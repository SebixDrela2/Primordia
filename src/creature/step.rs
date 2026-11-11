use super::*;

impl CreatureSystem {
    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        // Advance day/night cycle (full cycle = 7200 ticks ~ 2 min at 60fps)
        self.day_night += 0.000139;
        if self.day_night > 1.0 { self.day_night -= 1.0; }
        // Temperature follows season + day/night
        let base_temp = match self.season { 0 => 18.0, 1 => 26.0, 2 => 16.0, _ => 6.0 };
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * 6.0;
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

        for index in 0..self.list.len() {
            let mut died = false;
            let mut wants_attack = false;
            let mut wants_child = false;
            let mut ate_something = false;
            let mut built_something = false;
            let mut cast_spell = false;
            let mut traded = false;

            {
                let c = &mut self.list[index];
                c.age += 1;

                // Needs decay
                // Hardy species tire of hunger more slowly.
                let hardy = if c.traits & trait_bits::HARDY != 0 { 0.7 } else { 1.0 };
                c.hunger = (c.hunger + 0.06 * hardy).clamp(0.0, 100.0);
                c.thirst = (c.thirst + 0.045 * hardy).clamp(0.0, 100.0);
                c.stamina = (c.stamina - 0.05).clamp(0.0, c.max_stamina);
                // A temple nearby steadies the colony: less hunger, faster
                // recovery.
                let mut morale = false;
                let sidx = (c.species as usize).min(structures_by_species.len() - 1);
                for &si in structures_by_species[sidx].iter() {
                    let s = &self.structures[si];
                    if s.kind == 2
                        && (s.x - c.x).powi(2) + (s.y - c.y).powi(2) < 625.0
                    {
                        morale = true;
                        break;
                    }
                }
                if morale {
                    c.hunger = (c.hunger - 0.03).clamp(0.0, 100.0);
                }
                if c.resting {
                    c.stamina = (c.stamina + STAMINA_REGEN).clamp(0.0, c.max_stamina);
                    c.energy = (c.energy + 0.02).min(c.max_energy);
                } else {
                    c.energy -= 0.08;
                }
                c.mana = (c.mana + MANA_REGEN).clamp(0.0, c.max_mana);

                // Cooldowns
                if c.cooldown > 0 { c.cooldown -= 1; }
                if c.fire_cooldown > 0 { c.fire_cooldown -= 1; }
                if c.build_cooldown > 0 { c.build_cooldown -= 1; }
                if c.craft_cooldown > 0 { c.craft_cooldown -= 1; }
                if c.spell_cooldown > 0 { c.spell_cooldown -= 1; }
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
                    &mut input_buf,
                );
                c.net.forward_into(&input_buf, &mut net_act, &mut net_next);
                let out = &net_act;

                let (steer_x, steer_y, speed_mult_extra) =
                    act_steer(c, &out, &scan, &agents);
                c.vx = (out[0] * 2.2 + steer_x * 2.0).clamp(-2.6, 2.6);
                c.vy = (out[1] * 2.2 + steer_y * 2.0 + 0.5).clamp(-2.6, 2.6);


                act_move(world, c, speed_mult_extra);

                let ate_something = act_eat(world, c, &out);
                act_mine(world, c, &out);

                let (built_something, structures_total) =
                    act_build(world, c, &out, &mut self.structures, &mut self.events);

                let crafted = act_craft(c, &out, &mut self.events);

                let shots_before = shots.len();
                let wants_attack = act_combat(
                    c,
                    &out,
                    &grid,
                    &agents,
                    &mut shots,
                    &mut self.events,
                );
                shots_fired += (shots.len() - shots_before) as u32;


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
                            c.energy = (c.energy + 5.0).min(c.max_energy);
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
                let nn_wants_reproduce = out[3] > 0.05;
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
                };
                if can_reproduce && (nn_wants_reproduce || exploratory_reproduce)
                    && c.energy > 340.0 && c.cooldown == 0 && pop_now < REPRO_SOFT_CAP {
                    // Fertile species raise young more cheaply.
                    let fertile = if c.traits & trait_bits::FERTILE != 0 { 0.6 } else { 1.0 };
                    c.energy -= 110.0 * fertile;
                    c.hunger = (c.hunger + 15.0).clamp(0.0, 100.0);
                    c.cooldown = if c.traits & trait_bits::FERTILE != 0 { 50 } else { 70 };
                    c.offspring += 1;
                    wants_child = true;
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
                if c.disease != Disease::None && self.rng.chance(0.02) {
                    for &(list_idx, oid, ox, oy, ospecies) in &positions {
                        if oid == c.id { continue; }
                        let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                        if d < 4.0 && ospecies != c.species && self.rng.chance(0.15) {
                            disease_targets.push(list_idx);
                        }
                    }
                }
                if c.disease != Disease::None {
                    c.health -= 0.5;
                    c.energy -= 0.3;
                }

                // Fitness: colony success metrics
                c.fitness = (c.age as f32 / MAX_AGE as f32) * 0.15
                    + (c.energy / c.max_energy) * 0.12
                    + c.offspring as f32 * 0.18
                    + c.kills as f32 * 0.12
                    + (c.carried / CARRY_CAP) * 0.08
                    + (c.tech_level as f32) * 0.1
                    + (c.tool as u8 as f32) * 0.04
                    + (c.mana / c.max_mana) * 0.06
                    + (c.stamina / c.max_stamina) * 0.05
                    - (c.hunger / 100.0) * 0.05
                    - (c.thirst / 100.0) * 0.05;

                if c.energy <= 0.0 || c.health <= 0.0 || c.age > MAX_AGE + c.id.wrapping_mul(2654435761) % 1400 {
                    died = true;
                }

                c.last_x = c.x;
                c.last_y = c.y;

                if wants_child {
                    let child = c.child_blueprint();
                    births.push((c.base_x, c.base_y, child));
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
        self.resolve_deaths(world, &dead);
        self.resolve_births(world, births);

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
