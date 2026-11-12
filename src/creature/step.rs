use super::*;

impl CreatureSystem {
    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        // Advance day/night cycle (full cycle = 7200 ticks ~ 2 min at 60fps)
        self.day_night += 0.000139;
        if self.day_night > 1.0 { self.day_night -= 1.0; }
        // Temperature follows season + day/night. The extremes pressure
        // widens the swing: later in a run the summers are fiercer and the
        // winters harder, without changing the base seasonal temperatures.
        let base_temp = match self.season { 0 => 18.0, 1 => 26.0, 2 => 16.0, _ => 6.0 };
        let swing = 6.0 + world.pressure.extremes * 16.0;
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * swing;
        self.temperature = base_temp + day_variance;
        // Season progression
        self.season_timer += 1;
        if self.season_timer > 7200 { self.season_timer = 0; self.season = (self.season + 1) % 4; }

        // Soldiers that stand together form a squad and fight as one. The
        // anchors are indexed per species so the lookup stays cheap.
        {
            // Indexed by species id: deterministic (no hash iteration order).
            let mut anchors: Vec<Vec<(u16, f32, f32)>> =
                vec![Vec::new(); self.next_species as usize + 8];
            let mut next_squad = self.next_squad.max(1);
            for c in self.list.iter() {
                if c.role == Role::Soldier && c.squad_id != 0 {
                    let idx = (c.species as usize).min(anchors.len() - 1);
                    anchors[idx].push((c.squad_id, c.x, c.y));
                }
            }
            for c in self.list.iter_mut() {
                if c.role != Role::Soldier || c.squad_id != 0 {
                    continue;
                }
                let mut joined = false;
                let idx = (c.species as usize).min(anchors.len() - 1);
                for (sid, ax, ay) in anchors[idx].iter() {
                    let d = (ax - c.x).powi(2) + (ay - c.y).powi(2);
                    if d < 400.0 {
                        c.squad_id = *sid;
                        joined = true;
                        break;
                    }
                }
                if !joined {
                    let id = if next_squad > 60 { 1 } else { next_squad };
                    next_squad = next_squad.wrapping_add(1).max(1);
                    c.squad_id = id;
                    anchors[idx].push((id, c.x, c.y));
                }
            }
            self.next_squad = next_squad;
        }

        // Structures indexed by species (temples grant morale, turrets fire).
        let mut structures_by_species: Vec<Vec<usize>> =
            vec![Vec::new(); self.next_species as usize + 8];
        for (i, s) in self.structures.iter().enumerate() {
            let idx = (s.species as usize).min(structures_by_species.len() - 1);
            structures_by_species[idx].push(i);
        }
        let structures_by_species = structures_by_species;

        // Spatial index for neighbour queries.
        let mut grid = SpatialGrid::new(world.w, world.h, 32);
        grid.clear();
        for (i, c) in self.list.iter().enumerate() {
            grid.insert(c.x, c.y, i as u32);
        }

        let mut dead: Vec<usize> = Vec::new();
        let mut births: Vec<(f32, f32, Creature)> = Vec::new();
        let mut attackers: Vec<usize> = Vec::new();
        let mut traded_total: u64 = 0;
        let mut structures_total: u32 = 0;
        let mut mounts_total: u32 = 0;
        let mut spells_cast_total: u32 = 0;
        let mut spell_targets: Vec<(usize, usize, f32)> = Vec::new();
        let mut heal_targets: Vec<(u32, f32, f32)> = Vec::new();
        let mut disease_targets: Vec<usize> = Vec::new();
        let mut shots: Vec<Projectile> = Vec::new();
        let mut shots_fired = 0u32;
        // Reused scratch buffers: allocating these per creature was the
        // single biggest cost of the simulation step.
        let mut input_buf = vec![0.0f32; INPUTS];
        let mut net_act: Vec<f32> = Vec::with_capacity(INPUTS);
        let mut net_next: Vec<f32> = Vec::with_capacity(24);
        let positions: Vec<(usize, u32, f32, f32, u32)> =
            self.list.iter().enumerate().map(|(i, c)| (i, c.id, c.x, c.y, c.species)).collect();
        // Richer per-agent view for the senses: id, x, y, species, role, health.
        let agents: Vec<(u32, f32, f32, u32, u8, f32)> = self
            .list
            .iter()
            .map(|c| (c.id, c.x, c.y, c.species, c.role as u8, c.health))
            .collect();
        // The stage and the culture of each creature, in the same order as
        // the agents. The young need them to find the elder that can teach
        // them.
        let lives: Vec<(Stage, u16)> =
            self.list.iter().map(|c| (c.stage, c.culture)).collect();
        // Shots currently in flight (used by the senses).
        let shot_positions: Vec<(f32, f32, u32)> = self
            .projectiles
            .iter()
            .map(|p| (p.x, p.y, p.species))
            .collect();
        // Per-species aggregates, computed once per step.
        let mut species_counts: Vec<(u32, u32)> = Vec::new();
        for c in &self.list {
            match species_counts.iter_mut().find(|(id, _)| *id == c.species) {
                Some(e) => e.1 += 1,
                None => species_counts.push((c.species, 1)),
            }
        }
        let mut tower_counts: Vec<(u32, u32)> = Vec::new();
        for s in &self.structures {
            match tower_counts.iter_mut().find(|(id, _)| *id == s.species) {
                Some(e) => e.1 += 1,
                None => tower_counts.push((s.species, 1)),
            }
        }
        let total_towers = self.structures.len() as u32;
        let pop_now = self.list.len();

        // ---- Diplomacy setup ----
        // The centroid of every species: where that colony lives. Envoys use
        // these to find the colony they have to travel to.
        let mut centroids: Vec<(u32, f32, f32)> = Vec::new();
        {
            let mut acc: Vec<(u32, f32, f32, u32)> = Vec::new();
            for c in &self.list {
                match acc.iter_mut().find(|(id, _, _, _)| *id == c.species) {
                    Some(e) => {
                        e.1 += c.x;
                        e.2 += c.y;
                        e.3 += 1;
                    }
                    None => acc.push((c.species, c.x, c.y, 1)),
                }
            }
            for (id, sx, sy, n) in acc {
                centroids.push((id, sx / n as f32, sy / n as f32));
            }
        }
        // How willing each creature is to hold talks, filled in during the
        // loop and read when the talks conclude.
        let mut diplo_willing: Vec<f32> = vec![0.0; self.list.len()];
        // The talks that are under way, resolved after the loop.
        let mut proposals: Vec<(usize, usize, DiplomaticAction)> = Vec::new();
        // Which creatures went looking for a mate.
        let mut mate_willing: Vec<bool> = vec![false; self.list.len()];
        // What each species has worked out for itself, indexed by species
        // id. It is a snapshot for this tick: a discovery made now takes
        // effect on the next one.
        let mut discoveries_by_species: Vec<u16> = vec![0; self.next_species as usize + 8];
        for (sp, d) in &self.discoveries {
            if (*sp as usize) < discoveries_by_species.len() {
                discoveries_by_species[*sp as usize] = *d;
            }
        }

        // The practices the creatures picked up this tick. They are applied
        // after the loop, because learning touches the whole colony.
        let mut learned: Vec<(usize, Culture)> = Vec::new();

        for index in 0..self.list.len() {
            // The results of the actions. They are assigned inside the
            // block below and read after it, so they are declared without a
            // value: a `let` inside the block would shadow them and the
            // result would be lost.
            let wants_attack: bool;
            let ate_something: bool;
            let built_something: bool;
            let mut died = false;
            let mut wants_child = false;
            let mut cast_spell = false;
            let mut traded = false;
            // How many young this creature raises, and how much it puts into
            // each of them.
            let mut child_count: u32 = 1;
            let mut child_value: f32 = 1.0;

            {
                let c = &mut self.list[index];
                c.age += 1;

                // ---- Life stage ----
                //
                // A creature grows up: it starts as a juvenile that cannot
                // fight or breed, becomes a full adult, and finally turns
                // into an elder that is slower but wiser. How long each
                // stage lasts depends on the species' life history.
                let lifespan = c.lifespan();
                let new_stage = Stage::of(c.age, lifespan, c.maturity_fraction());
                if new_stage != c.stage {
                    // Growing up costs energy, and the body changes with it.
                    if new_stage == Stage::Adult {
                        c.health = (c.health + 30.0).min(c.max_health());
                        c.energy = (c.energy - 80.0).max(0.0);
                    }
                    c.stage = new_stage;
                }

                // What this creature knows about the other species. It is
                // needed early, because an alliance changes how the
                // creature feels before it does anything else.
                let ds = diplomatic_sense(&self.diplomacy, c, &centroids, &species_counts);

                // How many of its own kind are close by. A gregarious
                // creature works much better in the company of its own.
                let mut kin_near = 0u32;
                grid.near(c.x, c.y, 30, |idx| {
                    let a = &agents[idx as usize];
                    if a.0 != c.id && a.3 == c.species {
                        kin_near += 1;
                    }
                });
                let company = if c.traits & trait_bits::GREGARIOUS != 0 {
                    1.0 + (kin_near as f32 / 6.0).min(1.0) * 0.5
                } else {
                    1.0
                };

                // ---- The elders teach the young ----
                //
                // A young animal that grows up near an old one learns from
                // it: it matures a little faster and picks up a practice
                // it did not know. This is what gives old age a purpose in
                // the colony.
                if c.stage == Stage::Juvenile {
                    let mut elder: Option<u16> = None;
                    let cx = c.x;
                    let cy = c.y;
                    grid.near(cx, cy, 24, |idx| {
                        let i = idx as usize;
                        if i >= lives.len() || !lives[i].0.can_teach() {
                            return;
                        }
                        let a = &agents[i];
                        if a.0 == c.id || a.3 != c.species {
                            return;
                        }
                        let d = ((a.1 - cx).powi(2) + (a.2 - cy).powi(2)).sqrt();
                        if d < 24.0 {
                            elder = Some(lives[i].1);
                        }
                    });
                    if let Some(culture) = elder {
                        // The lesson speeds the young up.
                        c.age += 1;
                        // And it teaches one practice the young did not
                        // know yet.
                        for bit in [
                            culture_bits::CULTIVATION,
                            culture_bits::MINING,
                            culture_bits::WARFARE,
                            culture_bits::COMMERCE,
                            culture_bits::CRAFT,
                            culture_bits::RITUAL,
                            culture_bits::ARCHITECTURE,
                            culture_bits::HUNTING,
                        ] {
                            if culture & bit != 0 && c.culture & bit == 0 {
                                c.culture |= bit;
                                break;
                            }
                        }
                    }
                }

                // The land under its feet.
                //
                // A colony that settles beside a crystal grove, a hot
                // spring, a bone field, rich soil or an old ruin does
                // better than one that does not.
                let place = world.landmark_bonus(c.x, c.y);

                // Needs decay
                // Hardy species tire of hunger more slowly.
                let hardy = if c.traits & trait_bits::HARDY != 0 { 0.7 } else { 1.0 };
                c.hunger = (c.hunger + 0.06 * hardy).clamp(0.0, 100.0);
                // Rich soil means the colony does not go hungry.
                if place.food {
                    c.hunger = (c.hunger - 0.035).clamp(0.0, 100.0);
                }
                // A colony that has learned to grow its crops properly eats
                // better from the same harvest.
                let discoveries = discoveries_by_species
                    .get(c.species as usize)
                    .copied()
                    .unwrap_or(0);
                if discoveries & discovery_bits::HUSBANDRY != 0 {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                }                // The rain quenches a thirst that would otherwise build up.
                c.thirst = (c.thirst + 0.045 * hardy / world.weather.thirst_factor())
                    .clamp(0.0, 100.0);
                // A juvenile grows instead of working: it burns less energy
                // and tires less, but it also needs feeding.
                if c.stage == Stage::Juvenile {
                    c.stamina = (c.stamina - 0.02).clamp(0.0, c.max_stamina);
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                } else {
                    c.stamina = (c.stamina - 0.05).clamp(0.0, c.max_stamina);
                }
                // Elders tire more easily than the young.
                if c.stage == Stage::Elder {
                    c.stamina = (c.stamina - 0.03).clamp(0.0, c.max_stamina);
                }
                // A temple nearby steadies the colony: less hunger, faster
                // recovery.
                //
                // The other buildings of the settlement do their own work: a
                // market makes trade profitable, a workshop makes tools
                // faster, a granary keeps the colony fed and a beacon lets
                // the colony see much further.
                let mut morale = false;
                let mut near_market = false;
                let mut near_workshop = false;
                let mut near_granary = false;
                let mut near_beacon = false;
                let sidx = (c.species as usize).min(structures_by_species.len() - 1);
                for &si in structures_by_species[sidx].iter() {
                    let s = &self.structures[si];
                    let d2 = (s.x - c.x).powi(2) + (s.y - c.y).powi(2);
                    match s.kind {
                        structure_kind::TEMPLE => {
                            if d2 < 625.0 {
                                morale = true;
                            }
                        }
                        structure_kind::MARKET => {
                            if d2 < 900.0 {
                                near_market = true;
                            }
                        }
                        structure_kind::WORKSHOP => {
                            if d2 < 900.0 {
                                near_workshop = true;
                            }
                        }
                        structure_kind::GRANARY => {
                            if d2 < 900.0 {
                                near_granary = true;
                            }
                        }
                        structure_kind::BEACON => {
                            if d2 < 2500.0 {
                                near_beacon = true;
                            }
                        }
                        _ => {}
                    }
                }
                if morale {
                    c.hunger = (c.hunger - 0.03).clamp(0.0, 100.0);
                }
                // A granary keeps the colony fed: beside one, hunger builds
                // far more slowly.
                if near_granary {
                    c.hunger = (c.hunger - 0.05).clamp(0.0, 100.0);
                }
                // An alliance is worth feeling: the allies cover each
                // other's wounds, so a creature with a partner heals a
                // little faster and tires a little slower.
                if ds.alliances > 0.0 {
                    c.health = (c.health + 0.04).min(100.0);
                    c.stamina = (c.stamina + 0.01).min(c.max_stamina);
                }
                if c.resting {
                    c.stamina = (c.stamina + STAMINA_REGEN).clamp(0.0, c.max_stamina);
                    c.energy = (c.energy + 0.02).min(c.max_energy);
                } else {
                    // The weather makes every day harder.
                    c.energy -= 0.08 * world.weather.energy_factor();
                }
                // A bonded pair looks after each other: a creature with a
                // living partner is better fed and in better spirits.
                //
                // The bond is cleared when the partner dies, so a creature
                // that still has one knows the other is alive.
                if c.bond.is_some() {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                    c.energy = (c.energy + 0.01).min(c.max_energy);
                }

                // A creature heals back towards the limit its body allows.
                // A juvenile that has just become an adult spends a long
                // time growing into its frame.
                let cap = c.max_health();
                if c.health < cap {
                    c.health = (c.health + 0.05).min(cap);
                }

                // ---- The rhythm of the day ----
                //
                // A crepuscular animal works at dawn and dusk and rests at
                // noon; a climber is quicker in the open air than under
                // the ground; a provisioner feeds the colony it belongs to.
                let crepuscular = c.traits & trait_bits::CREPUSCULAR != 0;
                let hour = (self.day_night * 24.0) as f32;
                // How well the creature is doing at this hour: full at dawn
                // and dusk, half at noon and midnight.
                let daylight = if crepuscular {
                    let dawn = (hour - 6.0).abs().min(12.0);
                    let dusk = (hour - 18.0).abs().min(12.0);
                    1.0 - (dawn.min(dusk) / 6.0).min(1.0) * 0.6
                } else {
                    1.0
                };
                // A provisioner shares its food with the colony. The number
                // of kin nearby is already known, so this costs nothing.
                if c.traits & trait_bits::PROVISIONER != 0 && kin_near > 0 {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                }
                // The cold bites. A species that has never felt a winter
                // suffers for it, and the ember people suffer more. A hot
                // spring keeps the cold out.
                if self.temperature < 4.0 && !place.warmth {
                    let chill = (4.0 - self.temperature) * c.affinity.cold_vulnerability();
                    c.health -= chill * 0.012;
                }
                // Fire does not hurt a creature that was born to it.
                let (fx, fy) = (c.x.round() as i32, c.y.round() as i32);
                if !c.affinity.fire_immune() {
                    let here = world.get(fx, fy);
                    if here == Elem::Fire || here == Elem::Lava || here == Elem::Ember {
                        c.health -= 0.6;
                    }
                }
                c.mana = (c.mana + MANA_REGEN * c.affinity.mana_factor() * place.mana)
                    .clamp(0.0, c.max_mana);
                // A colony that understands the old rites draws on the
                // world much more strongly.
                if discoveries & discovery_bits::SANCTUARY != 0 {
                    c.mana = (c.mana + MANA_REGEN * 0.8).clamp(0.0, c.max_mana);
                }

                // Cooldowns
                if c.cooldown > 0 { c.cooldown -= 1; }
                if c.fire_cooldown > 0 { c.fire_cooldown -= 1; }
                if c.build_cooldown > 0 { c.build_cooldown -= 1; }
                if c.craft_cooldown > 0 { c.craft_cooldown -= 1; }
                if c.spell_cooldown > 0 { c.spell_cooldown -= 1; }
                if c.diplo_cooldown > 0 { c.diplo_cooldown -= 1; }
                for cd in &mut c.spell_cd { if *cd > 0 { *cd -= 1; } }
                if c.disease_timer > 0 { c.disease_timer -= 1; }
                else { c.disease = Disease::None; }

                // Mood drift
                if c.mood_timer == 0 {
                    c.mood = match self.rng.range(10) {
                        0 => Mood::Happy, 1 => Mood::Angry, 2 => Mood::Fearful,
                        3 => Mood::Curious, 4 => Mood::Loyal, 5 => Mood::Rebellious,
                        6 => Mood::Depressed, 7 => Mood::Zealous, _ => Mood::Neutral,
                    };
                    c.mood_timer = (self.rng.range(120) + 60) as u16;
                } else { c.mood_timer -= 1; }

                let scan = scan_local(world, c.x.round() as i32, c.y.round() as i32);
                let own_pop = species_counts
                    .iter()
                    .find(|(id, _)| *id == c.species)
                    .map(|(_, n)| *n)
                    .unwrap_or(1);
                let own_towers = tower_counts
                    .iter()
                    .find(|(id, _)| *id == c.species)
                    .map(|(_, n)| *n)
                    .unwrap_or(0);

                // The brain is only re-thought every other tick: a creature
                // moves slowly, so yesterday's decision is still a good one.
                // Halving the work here is the single biggest saving in the
                // simulation.
                if self.tick % 2 == 0 || !c.brain_ready {
                    Self::sense(
                        &agents,
                        &grid,
                        c,
                        world,
                        self.day_night,
                        self.temperature,
                        self.season,
                        &scan,
                        &shot_positions,
                        own_towers,
                        total_towers.saturating_sub(own_towers),
                        own_pop,
                        (pop_now as u32).saturating_sub(own_pop),
                        &ds,
                        near_beacon,
                        discoveries_by_species
                            .get(c.species as usize)
                            .copied()
                            .unwrap_or(0),
                        &mut input_buf,
                    );
                    c.net.forward_into(&input_buf, &mut net_act, &mut net_next);
                    c.out_cache.copy_from_slice(&net_act);
                    c.brain_ready = true;
                }
                // A copy, so the behaviour helpers can still change the
                // creature while they read its decision.
                let mut out_buf = [0.0f32; OUTPUTS];
                out_buf.copy_from_slice(&c.out_cache);
                let out: &[f32] = &out_buf;

                // ---- Diplomacy: the creature decides whether it wants to
                // talk, and to whom ----
                let wants_talks = out[outputs::NEGOTIATE] > 0.3
                    || c.role == Role::Envoy
                    || (c.role == Role::Merchant && out[9] > 0.25)
                    || (c.role == Role::Priest && out[7] > 0.4);
                diplo_willing[index] = if wants_talks { 1.0 } else { 0.0 };
                if wants_talks && c.diplo_cooldown == 0 {
                    if let Some(partner_idx) =
                        find_diplomatic_partner(c, &grid, &agents)
                    {
                        // The agents list is built in the same order as the
                        // creature list, so the index carries the species.
                        let partner_species = agents[partner_idx].3;
                        let action = desired_action(
                            &self.diplomacy,
                            &out,
                            c.species,
                            partner_species,
                        );
                        proposals.push((index, partner_idx, action));
                    }
                }

                let (steer_x, steer_y, speed_mult_extra) =
                    act_steer(c, &out, &scan, &agents, &centroids, &ds, world.weather, &place);
                // Creatures move at a walking pace. The old clamp of 2.6 cells
                // per step made them dart across the screen. A juvenile moves
                // slower still, a hollow-boned one faster, and the wind and
                // the rain slow everybody down.
                let speed_cap = c.speed_cap() * world.weather.speed_factor() * c.disease_speed_factor();
                // A climber is quicker in the open air than under the rock.
                let climbing = if c.traits & trait_bits::CLIMBER != 0
                    && world.get(c.x.round() as i32, c.y.round() as i32) == Elem::Empty
                {
                    1.25
                } else {
                    1.0
                };
                let speed_cap = speed_cap * climbing * daylight;
                c.vx = (out[0] * 0.8 + steer_x * 0.8).clamp(-speed_cap, speed_cap);
                c.vy = (out[1] * 0.8 + steer_y * 0.8 + 0.2).clamp(-speed_cap, speed_cap);


                act_move(world, c, speed_mult_extra);

                // These results belong to the outer variables: a `let` here
                // would shadow them, and the melee attackers, the eat and
                // build counters and the structure total would never be
                // picked up after the block.
                ate_something = act_eat(world, c, &out);
                act_mine(world, c, &out, company);

                let (built, built_count) =
                    act_build(world, c, &out, &mut self.structures, &mut self.events, company);
                built_something = built;
                structures_total += built_count;

                act_craft(
                    c,
                    &out,
                    &mut self.events,
                    near_workshop,
                    place.craft,
                    discoveries_by_species
                        .get(c.species as usize)
                        .copied()
                        .unwrap_or(0),
                );
                // ---- What the colony learns from what it does ----
                //
                // Doing a thing again and again teaches the colony how to
                // do it. The practices accumulate and are inherited.
                if built_something {
                    learned.push((index, Culture::Architecture));
                }
                if c.tool != Tool::None {
                    learned.push((index, Culture::Craft));
                }
                if out[12] > 0.35 {
                    learned.push((index, Culture::Mining));
                }

                let shots_before = shots.len();
                wants_attack = act_combat(
                    c,
                    &out,
                    &grid,
                    &agents,
                    &mut shots,
                    &mut self.events,
                    &self.diplomacy,
                );
                shots_fired += (shots.len() - shots_before) as u32;

                // A colony that keeps fighting learns to fight.
                if wants_attack {
                    learned.push((index, Culture::Warfare));
                }
                if ate_something {
                    learned.push((index, Culture::Hunting));
                    if c.inventory[10] > 0 {
                        learned.push((index, Culture::Cultivation));
                    }
                }


                // Cast spell (output[7] = cast, [11] = switch spell)
                if out[7] > 0.5 && c.spell_cooldown == 0 && c.known_spells > 0 && c.mana >= 10.0 {
                    // Switch active spell if requested
                    if out[11] > 0.5 {
                        let next = (c.active_spell as u8 + 1) % 10;
                        c.active_spell = match next {
                            1 => Spell::Fireball, 2 => Spell::Heal, 3 => Spell::Teleport,
                            4 => Spell::Wall, 5 => Spell::Light, 6 => Spell::Summon,
                            7 => Spell::Curse, 8 => Spell::Bless, 9 => Spell::Lightning,
                            _ => Spell::None,
                        };
                    }
                    cast_spell = true;
                    learned.push((index, Culture::Ritual));
                }

                // Mount / dismount (output[8] = mount)
                let exploratory_mount = c.mount_kind == MountKind::None && c.energy > 100.0
                    && (c.age * 11 + c.id * 7) % 60 < 1; // ~1.6% chance per tick
                if out[8] > 0.45 || exploratory_mount {
                    if c.mount_kind == MountKind::None {
                        if c.energy > 100.0 {
                            c.mount_kind = MountKind::Beast;
                            c.mount_stamina = 100.0;
                            c.energy -= 30.0;
                            mounts_total += 1;
                        }
                    } else {
                        c.mount_kind = MountKind::None;
                        c.mount_stamina = 100.0;
                    }
                }

                // Trade (output[9] = trade) - with nearby creature
                // Peaceful species actively seek out trade partners instead
                // of fighting.
                let trade_wanted = out[9] > 0.4
                    || (c.traits & trait_bits::PEACEFUL != 0 && out[15] < 0.3);
                if trade_wanted && c.craft_cooldown == 0 {
                    for &(list_idx, oid, ox, oy, ospecies) in &positions {
                        if oid == c.id { continue; }
                        let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                        if d < 6.0 && ospecies != c.species {
                            traded = true;
                            // A trade is a negotiation, not a per-tick income.
                            c.craft_cooldown = 25;
                            self.events.push(SimEvent::Trade {
                                x: (c.x + ox) * 0.5,
                                y: (c.y + oy) * 0.5,
                                color: c.team_color,
                            });
                            c.reputation = (c.reputation as i32 + 1).clamp(-100, 100) as i16;
                            // A signed trade pact makes the exchange far more
                            // valuable: the goods are the same, but the
                            // terms are better.
                            let pact = self.diplomacy.has_treaty(
                                c.species,
                                ospecies,
                                Treaty::TradePact,
                            );
                            // A colony that has traded for generations gets
                            // better terms still.
                            let gain = if pact { 18.0 } else { 5.0 }
                                * c.culture_factor(Culture::Commerce);
                            // A market makes the exchange worth much more.
                            let gain = if near_market { gain * 1.6 } else { gain };
                            c.energy = (c.energy + gain).min(c.max_energy);
                            c.energy = (c.energy - energy_cost::TRADE).max(0.0);
                            // Trading warms the relations up: a pact is
                            // built out of repeated small exchanges.
                            self.diplomacy.adjust_trust(c.species, ospecies, 0.8);
                            learned.push((index, Culture::Commerce));
                            break;
                        }
                    }
                }

                // Rest (output[10] = rest) - recover stamina/energy but vulnerable
                if out[10] > 0.5 && c.stamina < c.max_stamina * 0.5 {
                    c.resting = true;
                } else {
                    c.resting = false;
                }

                // Dig / mine (output[12] = dig) - break through weak walls
                if out[12] > 0.5 && c.tool_durability > 0 && world.mine_budget > 0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    for (dx, dy) in DIRS4 {
                        let tx = gx + dx;
                        let ty = gy + dy;
                        let e = world.get(tx, ty);
                        if e == Elem::Wall || e == Elem::Stone {
                            if c.tool == Tool::Pickaxe || c.tool == Tool::Drill {
                                world.mine_budget -= 1;
                                world.set_xy(tx, ty, Elem::Empty);
                                c.tool_durability = c.tool_durability.saturating_sub(5);
                                c.energy = (c.energy - energy_cost::DIG).max(0.0);
                                c.inventory[0] = (c.inventory[0].saturating_add(2)).min(255);
                                break;
                            }
                        }
                    }
                }

                // Reproduction. Queens are the main lineage; every other role can
                // also occasionally bud off a child so a colony keeps evolving.
                // A soft population cap keeps the sim responsive while still
                // allowing real generational turnover.
                //
                // A juvenile is too young to breed, and a creature that is
                // looking for a mate will mate instead of budding.
                let seeking_mate = out[outputs::SEEK_MATE] > 0.35
                    && c.stage.can_breed()
                    && c.energy > 260.0
                    && c.cooldown == 0
                    && c.hunger < 70.0
                    && pop_now < REPRO_SOFT_CAP;
                mate_willing[index] = seeking_mate;
                let nn_wants_reproduce = out[3] > 0.05 && !seeking_mate;
                let exploratory_reproduce = c.energy > 380.0 && c.cooldown == 0
                    && c.hunger < 60.0 && c.thirst < 70.0
                    && (c.age * 13 + c.id * 29) % 120 < 2;
                // The thresholds are per role, so a colony's workers, soldiers
                // and scholars all get a chance to contribute to the next
                // generation. Without this the lineage stalls: only the
                // queens ever have enough energy to breed.
                let can_reproduce = match c.role {
                    Role::Queen => c.energy > 300.0,
                    Role::Worker => c.energy > 430.0 && c.hunger < 60.0 && (c.age * 17 + c.id * 23) % 220 < 2,
                    Role::Soldier => c.energy > 460.0 && c.hunger < 60.0 && (c.age * 19 + c.id * 29) % 220 < 2,
                    Role::Priest => c.energy > 440.0 && c.hunger < 60.0 && (c.age * 13 + c.id * 17) % 220 < 2,
                    Role::Merchant => c.energy > 460.0 && c.hunger < 60.0 && (c.age * 19 + c.id * 23) % 220 < 2,
                    Role::Builder => c.energy > 430.0 && c.hunger < 60.0 && (c.age * 11 + c.id * 13) % 220 < 2,
                    Role::Explorer => c.energy > 410.0 && c.hunger < 60.0 && (c.age * 7 + c.id * 11) % 220 < 2,
                    Role::Shaman => c.energy > 440.0 && c.hunger < 60.0 && (c.age * 17 + c.id * 19) % 220 < 2,
                    Role::Envoy => c.energy > 420.0 && c.hunger < 60.0 && (c.age * 11 + c.id * 29) % 220 < 2,
                };
                if can_reproduce
                    && (nn_wants_reproduce || exploratory_reproduce)
                    && c.energy > 340.0
                    && c.cooldown == 0
                    && pop_now < REPRO_SOFT_CAP
                    && c.stage.can_breed()
                {
                    // Fertile species raise young more cheaply.
                    let fertile = if c.traits & trait_bits::FERTILE != 0 { 0.6 } else { 1.0 };
                    // The litter size and the cost of each young follow the
                    // species' life history: a prolific species raises
                    // several small ones, a prolonged one raises a single
                    // strong one.
                    let litter = c.litter_size();
                    let cost = c.brood_cost() * fertile;
                    if c.energy > cost {
                        c.energy -= cost;
                        c.hunger = (c.hunger + 15.0).clamp(0.0, 100.0);
                        c.cooldown = if c.traits & trait_bits::FERTILE != 0 { 50 } else { 70 };
                        c.offspring += litter;
                        wants_child = true;
                        child_count = litter;
                        child_value = c.brood_value();
                    }
                }

                // Throw a bomb at the nearest rival.
                if c.bombs > 0 && c.craft_cooldown == 0 {
                    let mut target: Option<(f32, f32, f32)> = None;
                    grid.near(c.x, c.y, 18, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }
                        let dx = a.1 - c.x;
                        let dy = a.2 - c.y;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < 16.0 && target.map_or(true, |t| d < t.2) {
                            target = Some((dx, dy, d));
                        }
                    });
                    if let Some((dx, dy, d)) = target {
                        let ux = dx / d.max(0.1);
                        let uy = dy / d.max(0.1);
                        self.bombs.push(Bomb {
                            x: c.x,
                            y: c.y,
                            vx: ux * 2.0,
                            vy: uy * 2.0,
                            life: 40,
                            owner: c.species,
                            color: c.team_color,
                        });
                        c.bombs -= 1;
                        c.craft_cooldown = 25;
                        c.energy = (c.energy - energy_cost::BOMB).max(0.0);
                        self.events.push(SimEvent::Shoot {
                            x: c.x,
                            y: c.y,
                            color: c.team_color,
                        });
                    }
                }

                // Use a salve on a wounded colonymate.
                if c.salves > 0 && c.craft_cooldown == 0 {
                    for &(aid, ax, ay, aspecies, _arole, ahp) in &agents {
                        if aid == c.id || aspecies != c.species || ahp >= 95.0 {
                            continue;
                        }
                        let dx = ax - c.x;
                        let dy = ay - c.y;
                        if dx * dx + dy * dy < 64.0 {
                            heal_targets.push((aid, ax, ay));
                            c.salves -= 1;
                            c.craft_cooldown = 30;
                            c.energy = (c.energy - energy_cost::SALVE).max(0.0);
                            self.events.push(SimEvent::Heal {
                                x: ax,
                                y: ay,
                                color: c.team_color,
                            });
                            break;
                        }
                    }
                }

                // Spell casting effects
                if cast_spell {
                    c.energy = (c.energy - energy_cost::SPELL).max(0.0);
                    c.spell_cooldown = 30;
                    c.mana = (c.mana - 10.0).max(0.0);
                    c.spell_cd[c.active_spell as usize] = 60;
                    spells_cast_total += 1;
                    match c.active_spell {
                        Spell::Fireball => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 10.0 {
                                    spell_targets.push((index, list_idx, 35.0));
                                }
                            }
                        }
                        Spell::Heal => {
                            c.health = (c.health + 40.0).min(100.0);
                            c.energy = (c.energy + 30.0).min(c.max_energy);
                        }
                        Spell::Teleport => {
                            c.x = self.rng.range_i(10, world.w as i32 - 10) as f32;
                            c.y = self.rng.range_i(10, world.h as i32 - 10) as f32;
                        }
                        Spell::Wall => {
                            let gx = c.x.round() as i32;
                            let gy = c.y.round() as i32;
                            for (dx, dy) in DIRS4 {
                                if world.get(gx + dx, gy + dy) == Elem::Empty {
                                    world.set_xy(gx + dx, gy + dy, Elem::Wall);
                                    break;
                                }
                            }
                        }
                        Spell::Light => {
                            c.fitness += 1.0;
                        }
                        Spell::Summon => {
                            // Will spawn a minion (handled in births)
                        }
                        Spell::Curse => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 8.0 {
                                    disease_targets.push(list_idx);
                                }
                            }
                        }
                        Spell::Bless => {
                            c.health = (c.health + 20.0).min(100.0);
                            c.energy = (c.energy + 20.0).min(c.max_energy);
                        }
                        Spell::Lightning => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 6.0 {
                                    spell_targets.push((index, list_idx, 50.0));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Disease spread
                //
                // A disease jumps to a neighbour of another species: the
                // sick ones and the well ones meet at the border.
                if c.disease != Disease::None && self.rng.chance(0.02) {
                    for &(list_idx, oid, ox, oy, ospecies) in &positions {
                        if oid == c.id { continue; }
                        let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                        if d < 4.0 && ospecies != c.species && self.rng.chance(0.15) {
                            disease_targets.push(list_idx);
                        }
                    }
                }
                // The disease does its work: it burns energy, takes health
                // and slows the host down.
                if c.disease != Disease::None {
                    c.apply_disease();
                }

                // Fitness: colony success metrics
                c.fitness = (c.age as f32 / lifespan as f32) * 0.15
                    + (c.energy / c.max_energy) * 0.12
                    + c.offspring as f32 * 0.18
                    + c.kills as f32 * 0.12
                    + (c.carried / c.carry_cap()) * 0.08
                    + (c.tech_level as f32) * 0.1
                    + (c.tool as u8 as f32) * 0.04
                    + (c.mana / c.max_mana) * 0.06
                    + (c.stamina / c.max_stamina) * 0.05
                    - (c.hunger / 100.0) * 0.05
                    - (c.thirst / 100.0) * 0.05;

                if c.energy <= 0.0
                    || c.health <= 0.0
                    || c.age > lifespan + c.id.wrapping_mul(2654435761) % 1400
                {
                    died = true;
                }

                c.last_x = c.x;
                c.last_y = c.y;

                if wants_child {
                    // A litter of several small ones, or a single strong
                    // one: it depends on the species' life history.
                    for _ in 0..child_count.max(1) {
                        let mut child = c.child_blueprint();
                        child.energy = (child.energy * child_value).max(60.0);
                        child.max_energy = (child.max_energy * child_value).max(200.0);
                        child.size = (child.size * (0.85 + child_value * 0.15)).clamp(0.8, 2.4);
                        births.push((c.base_x, c.base_y, child));
                    }
                }
            }

            if wants_attack {
                attackers.push(index);
            }
            if ate_something {
                self.eats_total += 1;
            }
            if built_something {
                self.builds_total += 1;
            }
            if traded {
                traded_total += 1;
            }
            if died {
                dead.push(index);
            }
        }

        // Combat resolution (see combat.rs).
        self.resolve_turrets(world, &mut shots);
        self.resolve_bombs(world);
        self.resolve_projectiles(world, shots);
        self.stats.shots += shots_fired;
        self.resolve_attacks(&grid, &attackers);

        // ---- Speciation: a colony too big for its ground splits ----
        self.maybe_speciate(world.pressure.scarcity);

        // ---- The plagues spread through the colonies ----
        self.spread_diseases(&grid);

        // ---- The pairs that hold the colony together ----
        self.form_bonds(&grid);

        // ---- What the species have worked out for themselves ----
        self.check_discoveries();

        // ---- An outbreak ----
        self.maybe_outbreak();

        // ---- What the colony learned ----
        //
        // The practices are applied after the loop, because a discovery
        // joins the memory of the whole species, not just of one animal.
        for (index, culture) in learned {
            self.learn_culture(index, culture);
        }

        // ---- Courtship: the pairs that were made this tick ----
        //
        // A child with two parents inherits from both, so the mating is
        // resolved here, after every creature has decided what it wanted.
        // It has to happen before the dead are removed, because the grid
        // still points at the positions the creatures had when it was
        // built.
        let conceived = self.resolve_mating(&mate_willing, &grid, pop_now);
        births.extend(conceived);

        self.resolve_deaths(world, &dead);
        self.resolve_births(world, births);

        // ---- Diplomacy: conclude the talks that were under way ----
        self.conclude_negotiations(&proposals, &diplo_willing);
        self.tick_diplomacy();

        // Apply accumulated stats
        self.trades_total = self.trades_total.saturating_add(traded_total);
        self.stats.structures = self.stats.structures.saturating_add(structures_total);
        self.stats.mounts = self.stats.mounts.saturating_add(mounts_total);
        self.stats.spells_cast = self.stats.spells_cast.saturating_add(spells_cast_total);

        self.apply_status(spell_targets, disease_targets, heal_targets);

        if self.tick % 15 == 0 {
            self.update_stats();
        }
    }
}
