use super::{CreatureSystem, World, Role, SpatialGrid, INPUTS, Faction, CRISIS_TICKS, Elem};

use super::step_creature::{
    StepCtx, StepOut, Tick, Turn, breath_and_warmth, build_view, cooldowns_and_mood, day_and_fire,
    surroundings, vitals, wake,
};

use super::step_mind::{
    breed, dig, fight, finish, forage, learn, mount_and_trade, spells_and_plague, steer, talks,
    think, weapons,
};

impl CreatureSystem {
    /// One tick of the simulation: the creatures act, the world
    /// answers, and the colony is counted.
    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        self.update_succession();
        self.rebuild_claims();

        self.advance_clock(world);
        self.assign_squads();

        let structures_by_species = self.structures_by_species();
        let grid = self.build_grid(world);

        let tick = self.run_creatures(world, &grid, &structures_by_species);
        self.close_tick(world, &grid, tick);
    }

    /// The day, the season and the temperature.
    fn advance_clock(&mut self, world: &mut World) {
        self.day_night += 0.000_139;
        if self.day_night > 1.0 {
            self.day_night -= 1.0;
        }

        let base_temp = match self.season {
            0 => 18.0,
            1 => 26.0,
            2 => 16.0,
            _ => 6.0,
        };
        let swing = 6.0 + world.pressure.extremes * 16.0;
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * swing;

        self.temperature = base_temp + day_variance + world.weather.temperature_offset();

        self.season_timer += 1;
        if self.season_timer > 7200 {
            self.season_timer = 0;
            self.season = (self.season + 1) % 4;
        }

        world.season = self.season;

        world.daylight = 0.5 - 0.5 * (self.day_night * std::f32::consts::TAU).cos();
    }

    /// The soldiers that join one another's squad.
    fn assign_squads(&mut self) {
        let mut anchors: Vec<Vec<(u16, f32, f32)>> = vec![Vec::new(); self.next_species as usize + 8];
        let mut next_squad = self.next_squad.max(1);
        for c in &self.list {
            if c.role == Role::Soldier && c.squad_id != 0 {
                let idx = (c.species as usize).min(anchors.len() - 1);
                anchors[idx].push((c.squad_id, c.x, c.y));
            }
        }
        for c in &mut self.list {
            if c.role != Role::Soldier || c.squad_id != 0 {
                continue;
            }
            let mut joined = false;
            let idx = (c.species as usize).min(anchors.len() - 1);
            for (sid, ax, ay) in &anchors[idx] {
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

    /// Which of the colony's buildings belong to which species.
    fn structures_by_species(&self) -> Vec<Vec<usize>> {
        let mut structures_by_species: Vec<Vec<usize>> =
            vec![Vec::new(); self.next_species as usize + 8];
        for (i, s) in self.structures.iter().enumerate() {
            let idx = (s.species as usize).min(structures_by_species.len() - 1);
            structures_by_species[idx].push(i);
        }
        structures_by_species
    }

    /// The grid the neighbours are found in.
    fn build_grid(&self, world: &World) -> SpatialGrid {
        let mut grid = SpatialGrid::new(world.w, world.h, 32);
        grid.clear();
        for (i, c) in self.list.iter().enumerate() {
            grid.insert(c.x, c.y, i as u32);
        }
        grid
    }

    /// The turn of every creature, in the order they stand.
    fn run_creatures(
        &mut self,
        world: &mut World,
        grid: &SpatialGrid,
        structures_by_species: &[Vec<usize>],
    ) -> Tick {
        let view = build_view(self);
        let succession = self.succession.clone();
        let pop_now = self.list.len();
        let mut zorg_pop = self.list.iter().filter(|c| c.faction == Faction::Zorg).count() as u32;

        let mut out = StepOut {
            rng: &mut self.rng,
            events: &mut self.events,
            structures: &mut self.structures,
            activity: &mut self.activity,
            deepest: &mut self.deepest,
            plagues_started: &mut self.plagues_started,
            tick: Tick {
                spell_targets: Vec::new(),
                disease_targets: Vec::new(),
                heal_targets: Vec::new(),
                proposals: Vec::new(),
                learned: Vec::new(),
                virus_spawns: Vec::new(),
                shots: Vec::new(),
                births: Vec::new(),
                attackers: Vec::new(),
                dead: Vec::new(),
                diplo_willing: vec![0.0; self.list.len()],
                mate_willing: vec![false; self.list.len()],
                bombs: Vec::new(),
                input_buf: vec![0.0f32; INPUTS],
                net_act: Vec::with_capacity(INPUTS),
                net_next: Vec::with_capacity(24),
                structures_total: 0,
                mounts_total: 0,
                spells_cast_total: 0,
                shots_fired: 0,
                traded_total: 0,
                pop_now,
            },
        };

        let mut ctx = StepCtx {
            world,
            grid,
            agents: &view.agents,
            lives: &view.lives,
            centroids: &view.centroids,
            species_counts: &view.species_counts,
            structures_by_species,
            fire_lit: &self.fire_lit,
            fear: &view.fear,
            diplomacy: &mut self.diplomacy,
            discoveries: &view.discoveries,
            day_night: self.day_night,
            temperature: self.temperature,
            tick: self.tick,
        };

        for index in 0..self.list.len() {
            let mut turn = Turn::new();
            {
                let c = &mut self.list[index];
                turn.crisis = crisis_drain(&succession, c.species, self.tick);
                wake(c, &ctx, &mut out, &mut turn);
                vitals(c, &ctx, &mut turn);
                surroundings(c, &ctx, &out, &mut turn);
                breath_and_warmth(c, &ctx, &mut turn);
                day_and_fire(c, &ctx, &mut turn);
                cooldowns_and_mood(c, &mut out);
                let decisions = think(c, &ctx, &mut out, &mut turn);
                talks(c, &ctx, &mut out, index, &decisions);
                steer(c, &mut ctx, &mut out, &mut turn, &decisions);
                forage(c, &mut ctx, &mut out, &mut turn, index, &decisions);
                fight(c, &ctx, &mut out, &mut turn, index, &decisions);
                mount_and_trade(c, &mut ctx, &mut out, &mut turn, index, &decisions);
                learn(c, &ctx, &turn);
                dig(c, &mut ctx, &decisions);
                breed(c, &mut out, &mut turn, index, pop_now, &mut zorg_pop, &decisions);
                weapons(c, &ctx, &mut out);
                spells_and_plague(c, &mut ctx, &mut out, &mut turn, index);
                finish(c, &mut out, &mut turn);
            }

            if turn.has(Turn::ATTACKED) {
                out.tick.attackers.push(index);
            }
            if turn.has(Turn::ATE) {
                self.eats_total += 1;

                let meal = self.list[index].last_meal as usize;
                if meal < self.eats_by_elem.len() {
                    self.eats_by_elem[meal] += 1;
                }
            }
            if turn.has(Turn::BUILT) {
                self.builds_total += 1;
            }
            if turn.has(Turn::TRADED) {
                out.tick.traded_total += 1;
            }
            if turn.has(Turn::DIED) {
                out.tick.dead.push(index);
            }
        }

        out.tick
    }

    /// The end of the tick: the world answers, and the colony is
    /// counted.
    fn close_tick(&mut self, world: &mut World, grid: &SpatialGrid, mut tick: Tick) {
        if self.tick.is_multiple_of(60) {
            self.tend_the_fires();
        }

        for (vx, vy) in tick.virus_spawns {
            if world.get(vx, vy) == Elem::Empty {
                world.set_xy(vx, vy, Elem::Virus);
            }
        }

        self.bombs.append(&mut tick.bombs);

        self.resolve_turrets(world, &mut tick.shots);
        self.resolve_bombs(world);
        self.resolve_projectiles(world, tick.shots);
        self.stats.shots += tick.shots_fired;
        self.resolve_attacks(grid, &tick.attackers);

        self.maybe_speciate(world.pressure.scarcity);

        self.spread_diseases(grid);

        self.spread_zorg_plague(grid);

        self.reseed_the_deep(world);

        if self.tick.is_multiple_of(60) {
            self.refresh_tech();
            world.deep_mining = self.best_deep_mining();
        }

        self.run_wards(world, grid);

        self.form_bonds(grid);

        self.check_discoveries(world);
        self.share_discoveries();
        self.raid_structures(world);

        self.maybe_outbreak(world);

        for (index, culture) in tick.learned {
            self.learn_culture(index, culture);
        }

        let pop_now = tick.pop_now;
        let conceived = self.resolve_mating(&tick.mate_willing, grid, pop_now);
        tick.births.extend(conceived);

        self.resolve_deaths(world, &tick.dead);
        self.resolve_births(world, tick.births);

        self.conclude_negotiations(&tick.proposals, &tick.diplo_willing);
        self.tick_diplomacy();

        if self.tick.is_multiple_of(120) {
            self.predation.decay();
        }

        self.update_succession();

        self.trades_total = self.trades_total.saturating_add(tick.traded_total);
        self.stats.structures = self.stats.structures.saturating_add(tick.structures_total);
        self.stats.mounts = self.stats.mounts.saturating_add(tick.mounts_total);
        self.stats.spells_cast = self.stats.spells_cast.saturating_add(tick.spells_cast_total);

        self.apply_status(tick.spell_targets, &tick.disease_targets, tick.heal_targets);

        if self.tick.is_multiple_of(15) {
            self.update_stats();
        }
    }

    /// The drain of a colony that has lost its queen.
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
            for c in &self.list {
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
                if best.is_none_or(|(bd, _)| d < bd) {
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

/// The drain of a colony that has lost its queen.
fn crisis_drain(succession: &[(u32, u64)], species: u32, tick: u64) -> f32 {
    let start = succession
        .iter()
        .find(|(sp, _)| *sp == species)
        .map_or(tick, |(_, s)| *s);
    if tick.saturating_sub(start) < CRISIS_TICKS {
        let age = tick.saturating_sub(start) as f32;
        (0.004 + age * 0.000_012).min(0.05)
    } else {
        0.0
    }
}
