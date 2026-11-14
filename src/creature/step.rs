use super::*;

impl CreatureSystem {
    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        // Work out the ground every colony holds. Everything that asks for
        // a claim this tick reads the cache.
        self.update_succession();
        self.rebuild_claims();

        // Advance day/night cycle (full cycle = 7200 ticks ~ 2 min at 60fps)
        self.day_night += 0.000139;
        if self.day_night > 1.0 { self.day_night -= 1.0; }
        // Temperature follows season + day/night. The extremes pressure
        // widens the swing: later in a run the summers are fiercer and the
        // winters harder, without changing the base seasonal temperatures.
        let base_temp = match self.season { 0 => 18.0, 1 => 26.0, 2 => 16.0, _ => 6.0 };
        let swing = 6.0 + world.pressure.extremes * 16.0;
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * swing;
        // The weather has a say in the temperature too: a heatwave adds to
        // it and a snowfall takes off it.
        self.temperature = base_temp + day_variance + world.weather.temperature_offset();
        // Season progression
        self.season_timer += 1;
        if self.season_timer > 7200 { self.season_timer = 0; self.season = (self.season + 1) % 4; }
        // The world follows the same clock.
        world.season = self.season;
        // And it follows the sun: the plants of the surface breathe in the
        // daylight, and the air of a sealed cave never does.
        world.daylight = 0.5 - 0.5 * (self.day_night * std::f32::consts::TAU).cos();

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
        // Where the ZORGs are. There are only a handful of them, so the
        // list is short and every creature can look through it cheaply:
        // the deep is the one thing a player creature always wants to
        // know about.
        let zorg_positions: Vec<(f32, f32)> = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .map(|c| (c.x, c.y))
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
        // How many ZORGs there are. They are a faction of their own, and
        // they are kept in check: they are a threat the players have to
        // deal with, not a swarm that swallows the world. The count is
        // kept live, so a single tick cannot push them over the cap.
        let mut zorg_pop = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .count() as u32;

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
        let mut discoveries_by_species: Vec<u32> = vec![0; self.next_species as usize + 8];
        for (sp, d) in &self.discoveries {
            if (*sp as usize) < discoveries_by_species.len() {
                discoveries_by_species[*sp as usize] = *d;
            }
        }

        // The practices the creatures picked up this tick. They are applied
        // after the loop, because learning touches the whole colony.
        let mut learned: Vec<(usize, Culture)> = Vec::new();

        // The predation memory, laid out as a table so the behaviours can
        // look a fear up in constant time.
        let fear_entries = FearTable::build(self.predation.entries(), self.next_species as usize + 8);
        // The colonies that are in crisis, copied out so the loop can read
        // them without borrowing the whole system.
        let succession = self.succession.clone();
        let crisis_drain_of = |species: u32, tick: u64| -> f32 {
            let start = succession
                .iter()
                .find(|(sp, _)| *sp == species)
                .map(|(_, s)| *s)
                .unwrap_or(tick);
            if tick.saturating_sub(start) < CRISIS_TICKS {
                let age = tick.saturating_sub(start) as f32;
                (0.004 + age * 0.000012).min(0.05)
            } else {
                0.0
            }
        };

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
                // ---- What has happened to it since it last looked ----
                //
                // The harm that reaches a creature usually arrives from
                // outside its own turn: a shot or a bite is resolved after
                // every creature has had its turn. So the creature keeps
                // what its health and its kills were, and the difference
                // between then and now is what tells it that something
                // hit it, or that something died.
                let hurt = c.health < c.health_seen - 2.0;
                let killed = c.kills > c.kills_seen;

                // ---- Life stage and learning ----
                //
                // The young grow up, and the elders teach them.
                let lifespan = c.lifespan();
                grow_and_learn(c, &grid, &agents, &lives);

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

                // How crowded the place is.
                //
                // A creature that spends its life in a heap on top of its
                // own kind gets nothing done: it cannot find anything to
                // eat, it cannot see what is coming, and it burns more of
                // itself for want of room. That is the cost of idling in
                // one place, and it is what sends a colony out into the
                // world instead of letting it sit by its own fire.
                let mut crowd = 0u32;
                grid.near(c.x, c.y, 4, |idx| {
                    let a = &agents[idx as usize];
                    if a.0 == c.id {
                        return;
                    }
                    let dx = a.1 - c.x;
                    let dy = a.2 - c.y;
                    if dx * dx + dy * dy <= 16.0 {
                        crowd += 1;
                    }
                });
                let crowding = 1.0 + 0.08 * crowd.min(16) as f32;


                // The land under its feet.
                //
                // A colony that settles beside a crystal grove, a hot
                // spring, a bone field, rich soil or an old ruin does
                // better than one that does not.
                let place = world.landmark_bonus(c.x, c.y);

                // Needs decay
                // Hardy species tire of hunger more slowly.
                let hardy = if c.traits & trait_bits::HARDY != 0 { 0.7 } else { 1.0 };
                // Pottery: the colony's stores keep, so it goes hungry
                // more slowly.
                let stores = if c.knows_tech(Discovery::Pottery) { 0.85 } else { 1.0 };
                // The world gets harder as the run goes on, and hunger is
                // the first thing that gets harder. At the start of a run
                // a creature that has not yet learned to feed itself is
                // given the benefit of the doubt: hunger barely moves.
                // By the end there is no benefit of the doubt left, and
                // the same creature would have starved three times over.
                //
                // The rate is kept in step with what a meal is worth: a
                // plant fills a creature for a fixed number of ticks, and
                // if the two drift apart then a colony that is eating well
                // still goes hungry, which is not a difficulty but a bug.
                let hardship = world.hardship();
                // The hunger follows the hardship: a young world is gentle
                // and a colony can afford to be slow to learn, and a world
                // that has been picked clean and battered is not.
                //
                // The rate is kept in step with what a meal is worth: a
                // plant fills a creature for a fixed number of ticks, and
                // if the two drift apart then a colony that is eating well
                // still goes hungry, which is not a difficulty but a bug.
                let hunger_rate = 0.06 * hardy * stores * (0.8 + 0.55 * hardship);
                c.hunger = (c.hunger + hunger_rate).clamp(0.0, 100.0);
                // A colony without a queen loses heart: the animals burn
                // their energy faster and the colony falls apart.
                let crisis = crisis_drain_of(c.species, self.tick);
                if crisis > 0.0 {
                    c.energy = (c.energy - crisis).max(0.0);
                    c.mood = Mood::Depressed;
                }
                // Rich soil means the colony does not go hungry.
                if place.food {
                    c.hunger = (c.hunger - 0.035).clamp(0.0, 100.0);
                }
                // The salt of the deep burns the filth out of the air. A
                // colony that settles on a salt flat is much harder for the
                // ZORGs to poison.
                if place.purity {
                    c.immunity[Disease::Plague as usize] =
                        c.immunity[Disease::Plague as usize].max(1);
                    c.immunity[Disease::Infection as usize] =
                        c.immunity[Disease::Infection as usize].max(1);
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
                // The thirst follows the hunger: the world is gentler at
                // the start of a run and thirstier at the end of it.
                let thirst_rate = 0.045 * hardy / world.weather.thirst_factor()
                    * (0.25 + 0.75 * hardship);
                c.thirst = (c.thirst + thirst_rate).clamp(0.0, 100.0);
                // A ZORG lives in the wet deep and drinks the poison it
                // eats. It never needs water from a lake, so it does not
                // waste its time looking for any.
                if c.faction == Faction::Zorg {
                    c.thirst = 0.0;
                }
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
                // Fresh air. A creature that is standing in air the plants
                // have breathed recovers its wind faster, and one in a
                // sealed cave recovers it more slowly: the deep is a
                // breathless place, and working in it is hard going.
                let mut air = world.fresh_air(
                    c.x.round() as i32,
                    c.y.round() as i32,
                    2,
                );
                // An undergrove is the one place in the deep with air in
                // it, so a colony that has found one breathes easier than
                // one in stale rock.
                if place.air {
                    air = 1.0;
                }
                // ---- Breathing ----
                //
                // Every animal breathes. In fresh air it costs nothing
                // and the wind comes back quickly; in stale air the
                // breathing itself is work, and the animal tires for it.
                // This is the price of living in the deep: a colony that
                // digs too deep has to breathe for every step.
                let air_factor = 0.7 + 0.6 * air;
                // The cost of breathing in bad air, per tick.
                let breath = 0.05 * (1.0 - air);
                if c.resting {
                    c.stamina =
                        (c.stamina + STAMINA_REGEN * air_factor).clamp(0.0, c.max_stamina);
                    // A resting creature recovers what it is missing, not a
                    // fixed crumb. A tired animal that sleeps wakes up
                    // fit; one that only gets a little back never does.
                    c.energy = (c.energy
                        + (c.max_energy - c.energy) * 0.01
                        - breath)
                        .min(c.max_energy);
                } else {
                    // The weather makes every day harder, and stale air
                    // makes it harder still. So does the run itself: early
                    // on, the world lets a creature live a little easier.
                    // And a crowd is hard work: the more of them there are
                    // on the same spot, the more each of them burns.
                    c.energy -= 0.08
                        * world.weather.energy_factor()
                        * (1.4 - 0.4 * air)
                        * (0.55 + 0.45 * hardship)
                        * crowding
                        + breath;
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
                    // Surgery and alchemy mend the wounded faster.
                    let mut mend = 0.05;
                    if c.knows_tech(Discovery::Surgery) {
                        mend *= 1.4;
                    }
                    if c.knows_tech(Discovery::Alchemy) {
                        mend *= 1.3;
                    }
                    c.health = (c.health + mend).min(cap);
                }
                // A ZORG in its own cave is at home: the poison of the deep
                // restores it faster than the open air ever could.
                if c.faction == Faction::Zorg
                    && world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32)
                {
                    c.health = (c.health + 0.08).min(cap);
                    c.stamina = (c.stamina + 0.05).min(c.max_stamina);
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
                //
                // The cold is part of the difficulty: the first winters of
                // a run are mild, and by the end of a long one the cold is
                // the thing that decides which colonies survive.
                //
                // A lit fire at the colony's base keeps it out. That is
                // what the wood is for: a colony that huddles by a dead
                // fire finds the winter coming in, and a colony that
                // gathers its wood keeps the cold at the gate.
                let fire_warmth = self
                    .fire_lit
                    .get((c.species as usize).min(self.fire_lit.len().saturating_sub(1)))
                    .copied()
                    .unwrap_or(false)
                    && (c.x - c.base_x).abs() < 14.0
                    && (c.y - c.base_y).abs() < 14.0;
                if self.temperature < 4.0 && !place.warmth && !fire_warmth {
                    let chill = (4.0 - self.temperature) * c.affinity.cold_vulnerability();
                    c.health -= chill * 0.012 * (0.3 + 0.7 * hardship);
                }
                if fire_warmth {
                    // The warmth of a fire puts the wind back into an
                    // animal faster, but it is not a reason to sit in it:
                    // a colony that does nothing but huddle by its own
                    // fire goes hungry.
                    c.stamina = (c.stamina + 0.015).min(c.max_stamina);
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
                // A stormspire is full of charge: the colony draws on it
                // and rests on it well.
                if place.energy {
                    c.mana = (c.mana + MANA_REGEN * 1.2).clamp(0.0, c.max_mana);
                    c.stamina = (c.stamina + STAMINA_REGEN * 1.5).min(c.max_stamina);
                }
                // A colony that understands the old rites draws on the
                // world much more strongly.
                if discoveries & discovery_bits::SANCTUARY != 0 {
                    c.mana = (c.mana + MANA_REGEN * 0.8).clamp(0.0, c.max_mana);
                }
                // Radiance: the colony's magic is brighter.
                if c.knows_tech(Discovery::Radiance) {
                    c.mana = (c.mana + MANA_REGEN * 0.6).clamp(0.0, c.max_mana);
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
                // The sickness a ZORG carries never burns out: it is part
                // of the beast, and the deep keeps it in them.
                if c.faction == Faction::Zorg {
                    if c.disease == Disease::None {
                        c.disease = zorg_carried_disease(self.rng.range(5));
                    }
                    c.disease_timer = Plague::from(c.disease).duration();
                }

                // Mood drift
                if c.mood_timer == 0 {
                    c.mood = match self.rng.range(10) {
                        0 => Mood::Happy, 1 => Mood::Angry, 2 => Mood::Fearful,
                        3 => Mood::Curious, 4 => Mood::Loyal, 5 => Mood::Rebellious,
                        6 => Mood::Depressed, 7 => Mood::Zealous, _ => Mood::Neutral,
                    };
                    c.mood_timer = (self.rng.range(120) + 60) as u16;
                } else { c.mood_timer -= 1; }

                // The brain is only re-thought every other tick: a creature
                // moves slowly, so yesterday's decision is still a good one.
                // Halving the work here is the single biggest saving in the
                // simulation. The look at the world around the creature
                // rides along with it: on a tick where the brain does not
                // run, the picture from the last time it did is used.
                let think = self.tick % 2 == 0 || !c.brain_ready;
                if think {
                    c.percept_cache = scan_percept(
                        world,
                        c.x.round() as i32,
                        c.y.round() as i32,
                        c.species,
                        &agents,
                        &grid,
                    );
                }
                let percept = c.percept_cache;

                // The brain is only re-thought every other tick, with the
                // decision kept between the ticks it does not run on.
                if think {
                    // ---- The senses ----
                    //
                    // The creature sees, smells and hears the world, and
                    // knows its own body. That is all it is given.
                    write_percept(&percept, &mut input_buf);
                    write_body(
                        c,
                        world,
                        self.day_night,
                        self.temperature,
                        &mut input_buf,
                    );
                    write_kin(c, &agents, &grid, &mut input_buf);
                    c.net.forward_into(&input_buf, &mut net_act, &mut net_next);
                    c.out_cache.copy_from_slice(&net_act);
                    c.brain_ready = true;
                    // Remember which direction the creature most wanted to
                    // go in, so that a reward can be given to it later.
                    let mut best_dir = 0usize;
                    let mut best_desire = f32::MIN;
                    for d in 0..outputs::DESIRE_COUNT {
                        let v = c.out_cache[outputs::DESIRE + d];
                        if v > best_desire {
                            best_desire = v;
                            best_dir = d;
                        }
                    }
                    c.desire_dir = best_dir;
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
                // A ZORG never talks. It only fights.
                let wants_talks = wants_talks && c.faction != Faction::Zorg;
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

                let volcano = world.volcano().map(|(vx, vy)| (vx as f32, vy as f32));
                let (steer_x, steer_y, speed_mult_extra) = act_steer(
                    c,
                    &out,
                    &agents,
                    &grid,
                    &centroids,
                    &ds,
                    &fear_entries,
                    world.weather,
                    &place,
                    volcano,
                );
                // Standing in the blast of an eruption burns.
                if world.volcano_state() == crate::world::Volcano::Erupting {
                    if let Some((vx, vy)) = volcano {
                        let dx = c.x - vx;
                        let dy = c.y - vy;
                        let d2 = dx * dx + dy * dy;
                        if d2 < 900.0 {
                            c.health -= 3.0;
                        }
                    }
                }
                // Creatures move at a walking pace. The old clamp of 2.6 cells
                // per step made them dart across the screen. A juvenile moves
                // slower still, a hollow-boned one faster, and the wind and
                // the rain slow everybody down.
                let speed_cap = c.speed_cap() * world.weather.speed_factor() * c.disease_speed_factor();
                // Navigation: the colony finds its way about better.
                let speed_cap = if c.knows_tech(Discovery::Navigation) {
                    speed_cap * 1.15
                } else {
                    speed_cap
                };
                // A climber is quicker in the open air than under the rock.
                let climbing = if c.traits & trait_bits::CLIMBER != 0
                    && world.get(c.x.round() as i32, c.y.round() as i32) == Elem::Empty
                {
                    1.25
                } else {
                    1.0
                };
                let speed_cap = speed_cap * climbing * daylight;
                // The movement comes from the desires. The old "move x"
                // and "move y" outputs are still there, but they are now
                // only a small correction on top: the creature goes where
                // its desires point, which is what the perceptions feed.
                c.vx = (steer_x * 1.0 + out[0] * 0.25).clamp(-speed_cap, speed_cap);
                c.vy = (steer_y * 1.0 + out[1] * 0.25 + 0.2).clamp(-speed_cap, speed_cap);


                act_move(world, c, speed_mult_extra);

                // ---- The filth of the deep ----
                //
                // A ZORG does not walk over the world cleanly: it leaves
                // its poison and its microbes behind it. The ground it
                // crosses is corrupted, the plants in it rot, and the
                // corruption follows it up through the rock into the
                // surface colonies.
                if c.faction == Faction::Zorg && self.rng.chance(0.05) {
                    let fx = c.last_x.round() as i32;
                    let fy = c.last_y.round() as i32;
                    if world.get(fx, fy) == Elem::Empty {
                        let filth = zorg_filth(self.rng.range(8));
                        world.set_xy(fx, fy, filth);
                    }
                }

                // These results belong to the outer variables: a `let` here
                // would shadow them, and the melee attackers, the eat and
                // build counters and the structure total would never be
                // picked up after the block.
                ate_something = act_eat(world, c, &out);
                act_mine(world, c, &out, company);

                // A colony in crisis builds slowly: there is nobody to
                // decide what should be built, and nobody to keep it.
                let (built, built_count) = act_build(
                    world,
                    c,
                    &out,
                    &mut self.structures,
                    &mut self.events,
                    company,
                    discoveries_by_species
                        .get(c.species as usize)
                        .copied()
                        .unwrap_or(0)
                        & discovery_bits::SANCTIFY
                        != 0,
                );
                if built && crisis > 0.0 {
                    // The building goes up, but it takes twice as long to
                    // raise the next one.
                    c.build_cooldown = c.build_cooldown.max(60);
                }
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
                        switch_spell(c);
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
                    // The trade partner is looked for through the spatial
                    // grid: scanning the whole population for every trader
                    // was the slowest loop in the step.
                    let (cx, cy, cspecies) = (c.x, c.y, c.species);
                    let mut partner: Option<(f32, f32, u32)> = None;
                    grid.near(cx, cy, 6, |ji| {
                        if partner.is_some() {
                            return;
                        }
                        let other = &agents[ji as usize];
                        if other.3 == cspecies {
                            return;
                        }
                        let d = ((other.1 - cx).powi(2) + (other.2 - cy).powi(2)).sqrt();
                        if d < 6.0 {
                            partner = Some((other.1, other.2, other.3));
                        }
                    });
                    if let Some((ox, oy, ospecies)) = partner {
                        traded = true;
                        // A trade is a negotiation, not a per-tick income.
                        c.craft_cooldown = 25;
                        self.events.push(SimEvent::Trade {
                            x: (cx + ox) * 0.5,
                            y: (cy + oy) * 0.5,
                            color: c.team_color,
                        });
                        c.reputation = (c.reputation as i32 + 1).clamp(-100, 100) as i16;
                        // A signed trade pact makes the exchange far more
                        // valuable: the goods are the same, but the
                        // terms are better.
                        let pact =
                            self.diplomacy.has_treaty(cspecies, ospecies, Treaty::TradePact);
                        // A colony that has traded for generations gets
                        // better terms still.
                        let gain =
                            if pact { 18.0 } else { 5.0 } * c.culture_factor(Culture::Commerce);
                        // A market makes the exchange worth much more.
                        let gain = if near_market { gain * 1.6 } else { gain };
                        // Commerce: the colony knows what it is selling.
                        let gain = if c.knows_tech(Discovery::Commerce) {
                            gain * 1.5
                        } else {
                            gain
                        };
                        c.energy = (c.energy + gain).min(c.max_energy);
                        c.energy = (c.energy - energy_cost::TRADE).max(0.0);
                        // Trading warms the relations up: a pact is
                        // built out of repeated small exchanges.
                        self.diplomacy.adjust_trust(cspecies, ospecies, 0.8);
                        learned.push((index, Culture::Commerce));
                    }
                }

                // Rest (output[10] = rest) - recover stamina/energy but vulnerable
                //
                // The network may choose to rest, and it usually will once
                // it has learnt what resting is for. But an animal that is
                // simply exhausted lies down whatever its mind says: that
                // is the body, not the brain, and no amount of thinking
                // makes an exhausted creature any less tired.
                let spent = c.stamina < c.max_stamina * 0.2
                    || c.energy < c.max_energy * 0.15;
                if (out[10] > 0.05 && c.stamina < c.max_stamina * 0.5) || spent {
                    c.resting = true;
                } else {
                    c.resting = false;
                }

                // ---- What the mind learned from what it did ----
                //
                // This is the whole point of the new senses. The creature
                // is given raw impressions - a green something that smells
                // sweet - and nothing else. What it does with them has to
                // be worked out from what happens: it walks towards the
                // sweet smell, it eats, and it is fed. That is a good
                // outcome, so the connection between "sweet smell that
                // way" and "go that way" is strengthened.
                //
                // The credit is pushed back through the whole network, not
                // just the last layer, so the brain learns the *perception*
                // and not only the action.
                {
                    let disc = discoveries_by_species
                        .get(c.species as usize)
                        .copied()
                        .unwrap_or(0);
                    // A colony that has worked out how to learn learns
                    // twice as fast.
                    let mut learn = if disc & discovery_bits::GRANDCOGNITION != 0 {
                        0.030
                    } else {
                        0.012
                    };
                    // A whispering stone makes the animal that stands on it
                    // think more sharply.
                    if place.mind {
                        learn *= 2.0;
                    }

                    // ---- The lesson ----
                    //
                    // What happened, as a list of the decisions that
                    // should have been stronger (or weaker). This is the
                    // only channel between what happened and what the
                    // brain believes.
                    let mut targets: Vec<(usize, f32)> = Vec::new();
                    // The lesson is taken every fourth tick, not every
                    // tick. The traces keep the last few thoughts, so a
                    // decision that is four ticks old is still in there
                    // to be credited, and the arithmetic is a quarter
                    // of what it would be.
                    if self.tick % 4 == 0 {
                    let mut targets: [(usize, f32); 6] = [(0, 0.0); 6];
                    let mut n = 0usize;
                    let mut push = |out: usize, amount: f32| {
                        if n < targets.len() {
                            targets[n] = (out, amount);
                            n += 1;
                        }
                    };
                    if ate_something {
                        // Eating was the right thing to do.
                        push(2, learn * 1.5);
                        // And it was the direction it was heading in that
                        // brought it to the food.
                        push(outputs::DESIRE + c.desire_dir, learn * 2.5);
                    }
                    if built_something {
                        push(5, learn * 1.0);
                    }
                    // A kill is the one lesson that cannot be got wrong:
                    // the strike landed, and the prey is dead. So the
                    // attack is reinforced - and so is the direction the
                    // prey was in, because that is the direction the
                    // creature went to get it.
                    if killed {
                        push(4, learn * 1.6);
                        if c.kill_dir >= 0 {
                            push(
                                outputs::DESIRE + c.kill_dir as usize,
                                learn * 2.0,
                            );
                        }
                    } else if wants_attack {
                        // An attack that did not kill anything is worth
                        // very little: the creature swung at the air.
                        push(4, learn * 0.3);
                    }
                    if traded {
                        push(9, learn * 0.8);
                    }
                    if cast_spell {
                        push(7, learn * 0.8);
                    }
                    // Being hurt is the clearest lesson of all: run, and
                    // do not walk towards whatever did it.
                    //
                    // The direction it came from is remembered, so the
                    // credit goes to *that* direction and not to whatever
                    // the creature happened to want most when it happened.
                    if hurt {
                        push(15, learn * 1.2);
                        push(2, -learn * 0.8);
                        if c.hurt_dir >= 0 {
                            push(
                                outputs::DESIRE + c.hurt_dir as usize,
                                -learn * 1.5,
                            );
                        }
                    }
                    // Going hungry teaches it to want to eat.
                    if c.hunger > 70.0 {
                        push(2, learn * 0.5);
                    }

                    if !targets.is_empty() {
                    if n > 0 {
                        c.net.learn(&targets[..n], learn);
                    }
                    }
                    }

                    // ---- The shaping ----
                    //
                    // Learning only from meals is learning from a very
                    // rare event: the creature has to find food before it
                    // learns how. So the creature is also rewarded for
                    // *wanting* the right thing.
                    //
                    // A hungry creature that smells something sweet gets
                    // a credit for the direction it smells it in. It does
                    // not know the smell is food - that is the whole point
                    // - but the credit teaches it that wanting that
                    // direction is a good idea, and over many meals the
                    // connection between "sweet" and "food" is made.
                    //
                    // Only the strongest smell is credited, and only every
                    // few ticks: the lesson is the same either way, and
                    // the credit is not cheap.
                    if c.hunger > 25.0 && self.tick % 4 == 0 {
                        let drive = ((c.hunger - 25.0) / 75.0).min(1.0);
                        let (best, best_strength) =
                            strongest_direction(&percept, |_, smell| smell[crate::creature::senses::smell::SWEET]);
                        // The smell has to point one way and not all of them.
                        // A sweet smell that is everywhere is no smell at
                        // all, and crediting a direction on a tie teaches
                        // the animal to walk in circles.
                        if best_strength > 0.08 {
                            c.net.credit_desire(best, learn * 2.0 * drive, learn);
                        }
                    }
                    // A thirsty creature is drawn to the wet.
                    if c.thirst > 30.0 && self.tick % 8 == 0 {
                        let drive = ((c.thirst - 30.0) / 70.0).min(1.0);
                        let (best, best_strength) =
                            strongest_direction(&percept, |sight, _| {
                                sight[crate::creature::senses::sight::WET]
                                    * sight[crate::creature::senses::sight::BLUE]
                            });
                        if best_strength > 0.05 {
                            c.net.credit_desire(best, learn * 1.6 * drive, learn);
                        }
                    }

                    // Now and then the brain settles, so a life of
                    // reinforcement cannot turn a few weights into
                    // everything.
                    if self.tick % 97 == 0 {
                        c.net.settle(0.004);
                    }
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
                //
                // They also move with the difficulty. A creature that is
                // still learning to feed itself burns its energy walking
                // about looking for food, and asking it to have a full
                // tank before it can breed is asking the impossible. So
                // early on a creature needs less of itself to start a
                // family, and by the end of the run it has to be in good
                // shape to do it.
                let need = |base: f32| base * (0.45 + 0.55 * hardship);
                let well_fed = 60.0 + 35.0 * (1.0 - hardship);
                let can_reproduce = match c.role {
                    Role::Queen => c.energy > need(300.0),
                    Role::Worker => c.energy > need(430.0) && c.hunger < well_fed && (c.age * 17 + c.id * 23) % 220 < 2,
                    Role::Soldier => c.energy > need(460.0) && c.hunger < well_fed && (c.age * 19 + c.id * 29) % 220 < 2,
                    Role::Priest => c.energy > need(440.0) && c.hunger < well_fed && (c.age * 13 + c.id * 17) % 220 < 2,
                    Role::Merchant => c.energy > need(460.0) && c.hunger < well_fed && (c.age * 19 + c.id * 23) % 220 < 2,
                    Role::Builder => c.energy > need(430.0) && c.hunger < well_fed && (c.age * 11 + c.id * 13) % 220 < 2,
                    Role::Explorer => c.energy > need(410.0) && c.hunger < well_fed && (c.age * 7 + c.id * 11) % 220 < 2,
                    Role::Shaman => c.energy > need(440.0) && c.hunger < well_fed && (c.age * 17 + c.id * 19) % 220 < 2,
                    Role::Envoy => c.energy > need(420.0) && c.hunger < well_fed && (c.age * 11 + c.id * 29) % 220 < 2,
                };
                // The litter size and the cost of each young follow the
                // species' life history: a prolific species raises several
                // small ones, a prolonged one raises a single strong one.
                let litter = c.litter_size();
                // The ZORGs are capped. The deep has only so much room, and
                // the point of them is to be a threat that has to be met,
                // not a flood that ends every run on its own. The litter is
                // counted before the young are born, so the cap is a real
                // cap and not a mark they sail straight past.
                //
                // The cap grows with the difficulty: the deep starts as a
                // handful of beasts and becomes a swarm by the end of the
                // run. A colony has to be able to live with the deep
                // before the deep is worth fighting.
                let zorg_cap =
                    (30.0 + hardship * (ZORG_POP_CAP - 30) as f32) as u32;
                let zorg_room = zorg_pop + litter as u32 <= zorg_cap;
                if can_reproduce
                    && (nn_wants_reproduce || exploratory_reproduce)
                    && c.energy > 340.0
                    && c.cooldown == 0
                    && pop_now < REPRO_SOFT_CAP
                    && (c.faction != Faction::Zorg || zorg_room)
                    && c.stage.can_breed()
                {
                    // Fertile species raise young more cheaply.
                    let fertile = if c.traits & trait_bits::FERTILE != 0 { 0.6 } else { 1.0 };
                    let cost = c.brood_cost() * fertile;
                    if c.energy > cost {
                        if c.faction == Faction::Zorg {
                            zorg_pop += litter as u32;
                        }
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
                    spells_cast_total += 1;
                    // The spell is resolved with the creature borrowed on
                    // its own, so the borrow of the list is not disturbed.
                    let rng = &mut self.rng;
                    cast_spell_effects(
                        rng,
                        index,
                        c,
                        world,
                        &grid,
                        &agents,
                        &mut spell_targets,
                        &mut disease_targets,
                    );
                }

                // Disease spread
                //
                // A disease jumps to a neighbour of another species: the
                // sick ones and the well ones meet at the border.
                if c.disease != Disease::None && self.rng.chance(0.02) {
                    let (dx0, dy0, dsp) = (c.x, c.y, c.species);
                    grid.near(dx0, dy0, 4, |ji| {
                        let j = ji as usize;
                        let other = &agents[j];
                        if other.3 == dsp {
                            return;
                        }
                        let d = ((other.1 - dx0).powi(2) + (other.2 - dy0).powi(2)).sqrt();
                        if d < 4.0 && self.rng.chance(0.15) {
                            disease_targets.push(j);
                        }
                    });
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
                // Remember what it looks like now, so that the next turn
                // can tell what has happened to it in the meantime.
                c.health_seen = c.health;
                c.kills_seen = c.kills;

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
                // What it ate, for the diagnostic.
                let meal = self.list[index].last_meal as usize;
                if meal < self.eats_by_elem.len() {
                    self.eats_by_elem[meal] += 1;
                }
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

        // ---- The colony fires are fed ----
        //
        // A campfire burns wood, and the wood has to be carried home by
        // somebody. This is what punishes a colony that does nothing but
        // sit by its own fire: the fire goes out, and a colony in the cold
        // finds out that a warm nest is worth going out for.
        if self.tick % 60 == 0 {
            self.tend_the_fires();
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

        // ---- And the ZORGs carry theirs ----
        //
        // The deep does not need a war to bring a colony down: it only
        // has to get close to one.
        self.spread_zorg_plague(&grid);

        // ---- The deep is never empty ----
        // As long as the cave is there, the ZORGs breed back up out of it.
        // A player can thin them out, but the deep cannot be emptied for
        // good - unless a ward of sanctified fire is raised inside the
        // cave and seals it.
        self.reseed_the_deep(world);

        // ---- The knowledge of the species ----
        //
        // Every creature carries the tech of its species, so that the
        // effects of a discovery can be read where they are used.
        if self.tick % 60 == 0 {
            self.refresh_tech();
            world.deep_mining = self.best_deep_mining();
        }

        // ---- The wards burn the corruption and the ZORGs ----
        //
        // The only way a colony can finish the deep off.
        self.run_wards(world, &grid);

        // ---- The pairs that hold the colony together ----
        self.form_bonds(&grid);

        // ---- What the species have worked out for themselves ----
        self.check_discoveries(world);
        self.share_discoveries();
        self.raid_structures(world);

        // ---- An outbreak ----
        self.maybe_outbreak(world);

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

        // The fear of the prey fades slowly: a species that is not hunted
        // for a while forgets the danger.
        if self.tick % 120 == 0 {
            self.predation.decay();
        }

        // A colony that has lost its queen is in crisis: it holds no
        // ground, builds nothing and loses heart. The crisis ends the
        // moment a new queen takes the throne.
        self.update_succession();

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

    /// Feed the colony fires, and put out the ones that have no wood.
    ///
    /// A colony's fire burns a piece of wood every minute of the world's
    /// time, and the wood comes from the animals of that colony that are
    /// standing at home with it. A colony that has no wood left goes cold:
    /// the fire is out, and the animals feel it.
    fn tend_the_fires(&mut self) {
        // The base of every colony that still has members.
        let mut bases: Vec<(u32, f32, f32)> = Vec::new();
        for c in &self.list {
            if c.faction == Faction::Zorg {
                continue;
            }
            match bases.iter_mut().find(|(sp, _, _)| *sp == c.species) {
                Some(e) => {
                    e.1 = c.base_x;
                    e.2 = c.base_y;
                }
                None => bases.push((c.species, c.base_x, c.base_y)),
            }
        }
        for (sp, bx, by) in bases {
            let sidx = sp as usize;
            if sidx >= self.fire_lit.len() {
                self.fire_lit.resize(sidx + 8, false);
            }
            // The wood of the colony: what the animals have brought home.
            let mut wood: u32 = 0;
            for c in self.list.iter() {
                if c.species != sp {
                    continue;
                }
                if (c.x - bx).abs() + (c.y - by).abs() > 40.0 {
                    continue;
                }
                wood += c.inventory[6] as u32;
            }
            if wood == 0 {
                // Nothing to burn: the fire is out.
                self.fire_lit[sidx] = false;
                continue;
            }
            // The fire burns wood, and a big colony burns a lot of it:
            // the bigger the heap round the fire, the more the fire eats.
            let mut burn = 1u32 + (self.list.iter().filter(|c| c.species == sp).count() as u32) / 8;
            // The fire burns a piece of wood, taken from whoever is
            // standing nearest to the fire with some in their pack.
            let mut best: Option<(f32, usize)> = None;
            for (i, c) in self.list.iter().enumerate() {
                if c.species != sp || c.inventory[6] == 0 {
                    continue;
                }
                if (c.x - bx).abs() + (c.y - by).abs() > 40.0 {
                    continue;
                }
                let d = (c.x - bx).abs() + (c.y - by).abs();
                if best.map_or(true, |(bd, _)| d < bd) {
                    best = Some((d, i));
                }
            }
            let mut fed = false;
            if let Some((_, i)) = best {
                let take = burn.min(self.list[i].inventory[6] as u32) as u16;
                self.list[i].inventory[6] = self.list[i].inventory[6].saturating_sub(take);
                burn -= take as u32;
                fed = true;
            }
            // Any wood left in the pack of anybody else at home is fed to
            // the fire as well.
            if burn > 0 {
                for i in 0..self.list.len() {
                    if burn == 0 {
                        break;
                    }
                    if self.list[i].species != sp || self.list[i].inventory[6] == 0 {
                        continue;
                    }
                    if (self.list[i].x - bx).abs() + (self.list[i].y - by).abs() > 40.0 {
                        continue;
                    }
                    let take = burn.min(self.list[i].inventory[6] as u32) as u16;
                    self.list[i].inventory[6] = self.list[i].inventory[6].saturating_sub(take);
                    burn -= take as u32;
                    fed = true;
                }
            }
            self.fire_lit[sidx] = fed && burn == 0;
        }
    }
}

