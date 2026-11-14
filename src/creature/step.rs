use super::*;

impl CreatureSystem {
    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        self.update_succession();
        self.rebuild_claims();

        self.day_night += 0.000139;
        if self.day_night > 1.0 { self.day_night -= 1.0; }

        let base_temp = match self.season { 0 => 18.0, 1 => 26.0, 2 => 16.0, _ => 6.0 };
        let swing = 6.0 + world.pressure.extremes * 16.0;
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * swing;

        self.temperature = base_temp + day_variance + world.weather.temperature_offset();

        self.season_timer += 1;
        if self.season_timer > 7200 { self.season_timer = 0; self.season = (self.season + 1) % 4; }

        world.season = self.season;

        world.daylight = 0.5 - 0.5 * (self.day_night * std::f32::consts::TAU).cos();

        {

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

        let mut structures_by_species: Vec<Vec<usize>> =
            vec![Vec::new(); self.next_species as usize + 8];
        for (i, s) in self.structures.iter().enumerate() {
            let idx = (s.species as usize).min(structures_by_species.len() - 1);
            structures_by_species[idx].push(i);
        }
        let structures_by_species = structures_by_species;

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

        let mut input_buf = vec![0.0f32; INPUTS];
        let mut net_act: Vec<f32> = Vec::with_capacity(INPUTS);
        let mut net_next: Vec<f32> = Vec::with_capacity(24);

        let agents: Vec<(u32, f32, f32, u32, u8, f32)> = self
            .list
            .iter()
            .map(|c| (c.id, c.x, c.y, c.species, c.role as u8, c.health))
            .collect();

        let lives: Vec<(Stage, u16)> =
            self.list.iter().map(|c| (c.stage, c.culture)).collect();

        let _shot_positions: Vec<(f32, f32, u32)> = self
            .projectiles
            .iter()
            .map(|p| (p.x, p.y, p.species))
            .collect();

        let _zorg_positions: Vec<(f32, f32)> = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .map(|c| (c.x, c.y))
            .collect();

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
        let _total_towers = self.structures.len() as u32;
        let pop_now = self.list.len();

        let mut zorg_pop = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .count() as u32;

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

        let mut diplo_willing: Vec<f32> = vec![0.0; self.list.len()];

        let mut proposals: Vec<(usize, usize, DiplomaticAction)> = Vec::new();

        let mut mate_willing: Vec<bool> = vec![false; self.list.len()];

        let mut discoveries_by_species: Vec<u32> = vec![0; self.next_species as usize + 8];
        for (sp, d) in &self.discoveries {
            if (*sp as usize) < discoveries_by_species.len() {
                discoveries_by_species[*sp as usize] = *d;
            }
        }

        let mut learned: Vec<(usize, Culture)> = Vec::new();

        let mut virus_spawns: Vec<(i32, i32)> = Vec::new();

        let fear_entries = FearTable::build(self.predation.entries(), self.next_species as usize + 8);

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

            let wants_attack: bool;
            let ate_something: bool;
            let built_something: bool;
            let mut died = false;
            let mut wants_child = false;
            let mut cast_spell = false;
            let mut traded = false;

            let mut child_count: u32 = 1;
            let mut child_value: f32 = 1.0;

            {
                let c = &mut self.list[index];
                c.age += 1;

                let hurt = c.health < c.health_seen - 2.0;
                let killed = c.kills > c.kills_seen;

                let lifespan = c.lifespan();
                grow_and_learn(c, &grid, &agents, &lives);

                let ds = diplomatic_sense(&self.diplomacy, c, &centroids, &species_counts);

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

                if crowd >= 10 && c.disease == Disease::None {

                    let risk = 0.00003 * (crowd - 9) as f32;
                    if self.rng.chance(risk) {
                        c.disease = Disease::Plague;
                        c.disease_timer = Plague::from(Disease::Plague).duration();
                        self.plagues_started += 1;

                        virus_spawns.push((
                            c.x.round() as i32,
                            c.y.round() as i32,
                        ));
                    }
                }

                let place = world.landmark_bonus(c.x, c.y);

                let hardy = if c.traits & trait_bits::HARDY != 0 { 0.7 } else { 1.0 };

                let stores = if c.knows_tech(Discovery::Pottery) { 0.85 } else { 1.0 };

                let hardship = world.hardship();

                let debility = if c.disease == Disease::None {
                    0.0
                } else {
                    Plague::from(c.disease).debility()
                };

                let hunger_rate =
                    0.06 * hardy * stores * (0.8 + 0.55 * hardship) * (1.0 + 0.45 * debility);
                c.hunger = (c.hunger + hunger_rate).clamp(0.0, 100.0);

                let crisis = crisis_drain_of(c.species, self.tick);
                if crisis > 0.0 {
                    c.energy = (c.energy - crisis).max(0.0);
                    c.mood = Mood::Depressed;
                }

                if place.food {
                    c.hunger = (c.hunger - 0.035).clamp(0.0, 100.0);
                }

                if place.purity {
                    c.immunity[Disease::Plague as usize] =
                        c.immunity[Disease::Plague as usize].max(1);
                    c.immunity[Disease::Infection as usize] =
                        c.immunity[Disease::Infection as usize].max(1);
                }

                let discoveries = discoveries_by_species
                    .get(c.species as usize)
                    .copied()
                    .unwrap_or(0);
                if discoveries & discovery_bits::HUSBANDRY != 0 {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                }

                let thirst_rate = 0.045 * hardy / world.weather.thirst_factor()
                    * (0.25 + 0.75 * hardship);
                c.thirst = (c.thirst + thirst_rate).clamp(0.0, 100.0);

                if c.faction == Faction::Zorg {
                    c.thirst = 0.0;
                }

                if c.stage == Stage::Juvenile {
                    c.stamina =
                        (c.stamina - 0.02 * (1.0 + 0.7 * debility)).clamp(0.0, c.max_stamina);
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                } else {
                    c.stamina =
                        (c.stamina - 0.05 * (1.0 + 0.7 * debility)).clamp(0.0, c.max_stamina);
                }

                if c.stage == Stage::Elder {
                    c.stamina = (c.stamina - 0.03).clamp(0.0, c.max_stamina);
                }

                let mut morale = false;
                let mut near_market = false;
                let mut near_workshop = false;
                let mut near_granary = false;
                
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
                                
                            }
                        }
                        _ => {}
                    }
                }
                if morale {
                    c.hunger = (c.hunger - 0.03).clamp(0.0, 100.0);
                }

                if near_granary {
                    c.hunger = (c.hunger - 0.05).clamp(0.0, 100.0);
                }

                if ds.alliances > 0.0 {
                    c.health = (c.health + 0.04).min(100.0);
                    c.stamina = (c.stamina + 0.01).min(c.max_stamina);
                }

                let mut air = world.fresh_air(
                    c.x.round() as i32,
                    c.y.round() as i32,
                    2,
                );

                if place.air {
                    air = 1.0;
                }

                let air_factor = 0.7 + 0.6 * air;

                let breath = 0.05 * (1.0 - air);
                if c.resting {

                    let heal = (1.0 - 0.4 * debility).max(0.2);
                    c.stamina = (c.stamina + STAMINA_REGEN * air_factor * heal)
                        .clamp(0.0, c.max_stamina);

                    c.energy = (c.energy
                        + (c.max_energy - c.energy) * 0.01
                        - breath)
                        .min(c.max_energy);
                } else {

                    c.energy -= 0.08
                        * world.weather.energy_factor()
                        * (1.4 - 0.4 * air)
                        * (0.55 + 0.45 * hardship)
                        * crowding
                        + breath;
                }

                if c.bond.is_some() {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                    c.energy = (c.energy + 0.01).min(c.max_energy);
                }

                let cap = c.max_health();
                if c.health < cap {

                    let mut mend = 0.05;
                    if c.knows_tech(Discovery::Surgery) {
                        mend *= 1.4;
                    }
                    if c.knows_tech(Discovery::Alchemy) {
                        mend *= 1.3;
                    }
                    c.health = (c.health + mend).min(cap);
                }

                if c.faction == Faction::Zorg
                    && world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32)
                {
                    c.health = (c.health + 0.08).min(cap);
                    c.stamina = (c.stamina + 0.05).min(c.max_stamina);
                }

                let crepuscular = c.traits & trait_bits::CREPUSCULAR != 0;
                let hour = (self.day_night * 24.0) as f32;

                let daylight = if crepuscular {
                    let dawn = (hour - 6.0).abs().min(12.0);
                    let dusk = (hour - 18.0).abs().min(12.0);
                    1.0 - (dawn.min(dusk) / 6.0).min(1.0) * 0.6
                } else {
                    1.0
                };

                if c.traits & trait_bits::PROVISIONER != 0 && kin_near > 0 {
                    c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
                }

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

                    c.stamina = (c.stamina + 0.015).min(c.max_stamina);
                }

                let (fx, fy) = (c.x.round() as i32, c.y.round() as i32);
                if !c.affinity.fire_immune() {
                    let here = world.get(fx, fy);
                    if here == Elem::Fire || here == Elem::Lava || here == Elem::Ember {
                        c.health -= 0.6;
                    }
                }
                c.mana = (c.mana + MANA_REGEN * c.affinity.mana_factor() * place.mana)
                    .clamp(0.0, c.max_mana);

                if place.energy {
                    c.mana = (c.mana + MANA_REGEN * 1.2).clamp(0.0, c.max_mana);
                    c.stamina = (c.stamina + STAMINA_REGEN * 1.5).min(c.max_stamina);
                }

                if discoveries & discovery_bits::SANCTUARY != 0 {
                    c.mana = (c.mana + MANA_REGEN * 0.8).clamp(0.0, c.max_mana);
                }

                if c.knows_tech(Discovery::Radiance) {
                    c.mana = (c.mana + MANA_REGEN * 0.6).clamp(0.0, c.max_mana);
                }

                if c.cooldown > 0 { c.cooldown -= 1; }
                if c.fire_cooldown > 0 { c.fire_cooldown -= 1; }
                if c.build_cooldown > 0 { c.build_cooldown -= 1; }
                if c.craft_cooldown > 0 { c.craft_cooldown -= 1; }
                if c.spell_cooldown > 0 { c.spell_cooldown -= 1; }
                if c.diplo_cooldown > 0 { c.diplo_cooldown -= 1; }
                for cd in &mut c.spell_cd { if *cd > 0 { *cd -= 1; } }
                if c.disease_timer > 0 { c.disease_timer -= 1; }
                else { c.disease = Disease::None; }

                if c.faction == Faction::Zorg {
                    if c.disease == Disease::None {
                        c.disease = zorg_carried_disease(self.rng.range(5));
                    }
                    c.disease_timer = Plague::from(c.disease).duration();
                }

                if c.mood_timer == 0 {
                    c.mood = match self.rng.range(10) {
                        0 => Mood::Happy, 1 => Mood::Angry, 2 => Mood::Fearful,
                        3 => Mood::Curious, 4 => Mood::Loyal, 5 => Mood::Rebellious,
                        6 => Mood::Depressed, 7 => Mood::Zealous, _ => Mood::Neutral,
                    };
                    c.mood_timer = (self.rng.range(120) + 60) as u16;
                } else { c.mood_timer -= 1; }

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

                if think {

                    write_percept(&percept, &mut input_buf);
                    write_body(
                        c,
                        world,
                        self.day_night,
                        self.temperature,
                        &mut input_buf,
                    );

                    input_buf[crate::creature::senses::BODY_BASE
                        + crate::creature::senses::BODY_CROWDED] =
                        (crowd as f32 / 16.0).clamp(0.0, 1.0);
                    write_kin(c, &agents, &grid, &mut input_buf);
                    c.net.forward_into(&input_buf, &mut net_act, &mut net_next);
                    c.out_cache.copy_from_slice(&net_act);
                    c.brain_ready = true;

                    {
                        let mut bits = 0u32;
                        if net_act[2] > 0.05 {
                            bits |= crate::creature::technology::activity::EAT;
                        }
                        if net_act[6] > 0.25 {
                            bits |= crate::creature::technology::activity::CRAFT;
                        }
                        if net_act[5] > 0.2 {
                            bits |= crate::creature::technology::activity::BUILD;
                        }
                        if net_act[9] > 0.4 {
                            bits |= crate::creature::technology::activity::TRADE;
                        }
                        if net_act[12] > 0.35 {
                            bits |= crate::creature::technology::activity::MINE;
                        }
                        if net_act[7] > 0.5 {
                            bits |= crate::creature::technology::activity::CAST;
                        }
                        if net_act[4] > 0.3 {
                            bits |= crate::creature::technology::activity::ATTACK;
                        }
                        if bits != 0 {
                            match self
                                .activity
                                .iter_mut()
                                .find(|(sp, _)| *sp == c.species)
                            {
                                Some(e) => e.1 |= bits,
                                None => self.activity.push((c.species, bits)),
                            }
                        }
                    }

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

                let mut out_buf = [0.0f32; OUTPUTS];
                out_buf.copy_from_slice(&c.out_cache);
                let out: &[f32] = &out_buf;

                let wants_talks = out[outputs::NEGOTIATE] > 0.3;

                let wants_talks = wants_talks && c.faction != Faction::Zorg;
                diplo_willing[index] = if wants_talks { 1.0 } else { 0.0 };
                if wants_talks && c.diplo_cooldown == 0 {
                    if let Some(partner_idx) =
                        find_diplomatic_partner(c, &grid, &agents)
                    {

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

                let speed_cap = c.speed_cap() * world.weather.speed_factor() * c.disease_speed_factor();

                let speed_cap = if c.knows_tech(Discovery::Navigation) {
                    speed_cap * 1.15
                } else {
                    speed_cap
                };

                let climbing = if c.traits & trait_bits::CLIMBER != 0
                    && world.get(c.x.round() as i32, c.y.round() as i32) == Elem::Empty
                {
                    1.25
                } else {
                    1.0
                };
                let speed_cap = speed_cap * climbing * daylight;

                c.vx = (steer_x * 1.0 + out[0] * 0.25).clamp(-speed_cap, speed_cap);
                c.vy = (steer_y * 1.0 + out[1] * 0.25 + 0.2).clamp(-speed_cap, speed_cap);

                act_move(world, c, speed_mult_extra);

                {
                    let depth =
                        (c.y.round() as i32 - world.surface_y(c.x.round() as i32)).max(0) as u32;
                    if depth > 0 {
                        match self.deepest.iter_mut().find(|(sp, _)| *sp == c.species) {
                            Some(e) => e.1 = e.1.max(depth),
                            None => self.deepest.push((c.species, depth)),
                        }
                    }
                }

                if c.faction == Faction::Zorg && self.rng.chance(0.05) {
                    let fx = c.last_x.round() as i32;
                    let fy = c.last_y.round() as i32;
                    if world.get(fx, fy) == Elem::Empty {
                        let filth = zorg_filth(self.rng.range(8));
                        world.set_xy(fx, fy, filth);
                    }
                }

                ate_something = act_eat(world, c, &out, &self.diplomacy);
                act_mine(world, c, &out, company);

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

                if wants_attack {
                    learned.push((index, Culture::Warfare));
                }
                if ate_something {

                    if c.last_meal == Elem::Corpse {
                        learned.push((index, Culture::Hunting));
                    }
                    if c.inventory[10] > 0 {
                        learned.push((index, Culture::Cultivation));
                    }
                }

                if out[7] > 0.5 && c.spell_cooldown == 0 && c.known_spells > 0 && c.mana >= 10.0 {

                    if out[11] > 0.5 {
                        switch_spell(c);
                    }
                    cast_spell = true;
                    learned.push((index, Culture::Ritual));
                }

                let exploratory_mount = c.mount_kind == MountKind::None && c.energy > 100.0
                    && (c.age * 11 + c.id * 7) % 60 < 1;
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

                let trade_wanted = out[9] > 0.4;
                if trade_wanted && c.craft_cooldown == 0 {

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

                        c.craft_cooldown = 25;
                        self.events.push(SimEvent::Trade {
                            x: (cx + ox) * 0.5,
                            y: (cy + oy) * 0.5,
                            color: c.team_color,
                        });
                        c.reputation = (c.reputation as i32 + 1).clamp(-100, 100) as i16;

                        let pact =
                            self.diplomacy.has_treaty(cspecies, ospecies, Treaty::TradePact);

                        let gain =
                            if pact { 18.0 } else { 5.0 } * c.culture_factor(Culture::Commerce);

                        let gain = if near_market { gain * 1.6 } else { gain };

                        let gain = if c.knows_tech(Discovery::Commerce) {
                            gain * 1.5
                        } else {
                            gain
                        };
                        c.energy = (c.energy + gain).min(c.max_energy);
                        c.energy = (c.energy - energy_cost::TRADE).max(0.0);

                        self.diplomacy.adjust_trust(cspecies, ospecies, 0.8);
                        learned.push((index, Culture::Commerce));
                    }
                }

                let spent = c.stamina < c.max_stamina * 0.2
                    || c.energy < c.max_energy * 0.15;
                if (out[10] > 0.05 && c.stamina < c.max_stamina * 0.5) || spent {
                    c.resting = true;
                } else {
                    c.resting = false;
                }

                {
                    let disc = discoveries_by_species
                        .get(c.species as usize)
                        .copied()
                        .unwrap_or(0);

                    let mut learn = if disc & discovery_bits::GRANDCOGNITION != 0 {
                        0.030
                    } else {
                        0.012
                    };

                    if place.mind {
                        learn *= 2.0;
                    }

                    let _targets: Vec<(usize, f32)> = Vec::new();

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

                        push(2, learn * 1.5);

                        push(outputs::DESIRE + c.desire_dir, learn * 2.5);
                    }
                    if built_something {
                        push(5, learn * 1.0);
                    }

                    if killed {
                        push(4, learn * 1.6);
                        if c.kill_dir >= 0 {
                            push(
                                outputs::DESIRE + c.kill_dir as usize,
                                learn * 2.0,
                            );
                        }
                    } else if wants_attack {

                        push(4, learn * 0.3);
                    }
                    if traded {
                        push(9, learn * 0.8);
                    }
                    if cast_spell {
                        push(7, learn * 0.8);
                    }

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

                    if crowd >= 8 {
                        let penalty = if c.hunger > 40.0 && !ate_something {
                            learn * 1.4
                        } else {

                            learn * 0.6
                        };
                        push(outputs::DESIRE + c.desire_dir, -penalty);
                    }

                    if c.hunger > 70.0 {
                        push(2, learn * 0.5);
                    }

                    if !targets.is_empty() {
                    if n > 0 {
                        c.net.learn(&targets[..n], learn);
                    }
                    }
                    }

                    if c.hunger > 25.0 && self.tick % 4 == 0 {
                        let drive = ((c.hunger - 25.0) / 75.0).min(1.0);
                        let (best, best_strength) =
                            strongest_direction(&percept, |_, smell| smell[crate::creature::senses::smell::SWEET]);

                        if best_strength > 0.08 {
                            c.net.credit_desire(best, learn * 2.0 * drive, learn);
                        }
                    }

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

                    if self.tick % 97 == 0 {
                        c.net.settle(0.004);
                    }
                }

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

                let need = |base: f32| base * (0.6 + 0.9 * hardship);
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

                let litter = c.litter_size();

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

                if cast_spell {
                    spells_cast_total += 1;

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

                if c.disease != Disease::None {
                    c.apply_disease();
                }

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

                c.health_seen = c.health;
                c.kills_seen = c.kills;

                if wants_child {

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

        if self.tick % 60 == 0 {
            self.tend_the_fires();
        }

        for (vx, vy) in virus_spawns {
            if world.get(vx, vy) == Elem::Empty {
                world.set_xy(vx, vy, Elem::Virus);
            }
        }

        self.resolve_turrets(world, &mut shots);
        self.resolve_bombs(world);
        self.resolve_projectiles(world, shots);
        self.stats.shots += shots_fired;
        self.resolve_attacks(&grid, &attackers);

        self.maybe_speciate(world.pressure.scarcity);

        self.spread_diseases(&grid);

        self.spread_zorg_plague(&grid);

        self.reseed_the_deep(world);

        if self.tick % 60 == 0 {
            self.refresh_tech();
            world.deep_mining = self.best_deep_mining();
        }

        self.run_wards(world, &grid);

        self.form_bonds(&grid);

        self.check_discoveries(world);
        self.share_discoveries();
        self.raid_structures(world);

        self.maybe_outbreak(world);

        for (index, culture) in learned {
            self.learn_culture(index, culture);
        }

        let conceived = self.resolve_mating(&mate_willing, &grid, pop_now);
        births.extend(conceived);

        self.resolve_deaths(world, &dead);
        self.resolve_births(world, births);

        self.conclude_negotiations(&proposals, &diplo_willing);
        self.tick_diplomacy();

        if self.tick % 120 == 0 {
            self.predation.decay();
        }

        self.update_succession();

        self.trades_total = self.trades_total.saturating_add(traded_total);
        self.stats.structures = self.stats.structures.saturating_add(structures_total);
        self.stats.mounts = self.stats.mounts.saturating_add(mounts_total);
        self.stats.spells_cast = self.stats.spells_cast.saturating_add(spells_cast_total);

        self.apply_status(spell_targets, disease_targets, heal_targets);

        if self.tick % 15 == 0 {
            self.update_stats();
        }
    }

    fn tend_the_fires(&mut self) {

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

                self.fire_lit[sidx] = false;
                continue;
            }

            let mut burn = 1u32 + (self.list.iter().filter(|c| c.species == sp).count() as u32) / 8;

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
