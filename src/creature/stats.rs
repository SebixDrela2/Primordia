use super::*;

impl CreatureSystem {
    pub fn update_stats(&mut self) {
        let keep_shots = self.stats.shots;
        let keep_towers = self.structures.len() as u32;
        let pop = self.list.len();
        if pop == 0 {
            self.stats = CreatureStats {
                population: 0,
                species: 0,
                shots: keep_shots,
                towers: keep_towers,
                ..Default::default()
            };
            self.stats.births = self.births_total;
            self.stats.deaths = self.deaths_total;
            return;
        }

        let mut species_ids: Vec<u32> = self.list.iter().map(|c| c.species).collect();
        species_ids.sort_unstable();
        species_ids.dedup();

        let mut fit = 0.0;
        let mut energy = 0.0;
        let mut mana = 0.0;
        let mut loyalty_sum = 0;
        let mut max_gen = 0;
        let mut workers = 0;
        let mut soldiers = 0;
        let mut queens = 0;
        let mut priests = 0;
        let mut merchants = 0;
        let mut builders = 0;
        let mut explorers = 0;
        let mut shamans = 0;
        let mut envoys = 0;
        let mut tools_pickaxe = 0;
        let mut tools_sword = 0;
        let mut tools_hammer = 0;
        let mut tools_drill = 0;
        let mut tools_scanner = 0;
        let mut tools_bow = 0;
        let mut tools_spear = 0;
        let mut tools_shield = 0;
        let mut tools_staff = 0;
        let mut tools_amulet = 0;
        let mut tools_backpack = 0;
        let mut tools_cart = 0;
        let mut tools_boat = 0;
        let mut tools_glider = 0;
        let mut tech_stone = 0;
        let mut tech_copper = 0;
        let mut tech_iron = 0;
        let mut tech_gold = 0;
        let mut tech_crystal = 0;
        let mut disease_count = 0;
        let mut spell_count = 0u32;
        let mut trade_count = 0u32;
        let mut mount_count = 0u32;
        let mut hunger_sum = 0.0;
        let mut thirst_sum = 0.0;
        let mut stamina_sum = 0.0;
        let mut juveniles = 0;
        let mut elders = 0;

        for c in &self.list {
            fit += c.fitness;
            energy += c.energy;
            mana += c.mana;
            loyalty_sum += c.loyalty as i32;
            max_gen = max_gen.max(c.generation);
            hunger_sum += c.hunger;
            thirst_sum += c.thirst;
            stamina_sum += c.stamina;
            if c.active_spell != Spell::None { spell_count += 1; }
            if c.mount_kind != MountKind::None { mount_count += 1; }
            match c.stage {
                Stage::Juvenile => juveniles += 1,
                Stage::Elder => elders += 1,
                Stage::Adult => {}
            }
            if c.disease != Disease::None {
                disease_count += 1;
            }
            match c.role {
                Role::Worker => workers += 1,
                Role::Soldier => soldiers += 1,
                Role::Queen => queens += 1,
                Role::Priest => priests += 1,
                Role::Merchant => merchants += 1,
                Role::Builder => builders += 1,
                Role::Explorer => explorers += 1,
                Role::Shaman => shamans += 1,
                Role::Envoy => envoys += 1,
            }
            match c.tool {
                Tool::Pickaxe => tools_pickaxe += 1,
                Tool::Sword => tools_sword += 1,
                Tool::Hammer => tools_hammer += 1,
                Tool::Drill => tools_drill += 1,
                Tool::Scanner => tools_scanner += 1,
                Tool::Bow => tools_bow += 1,
                Tool::Spear => tools_spear += 1,
                Tool::Shield => tools_shield += 1,
                Tool::Staff => tools_staff += 1,
                Tool::Amulet => tools_amulet += 1,
                Tool::Backpack => tools_backpack += 1,
                Tool::Cart => tools_cart += 1,
                Tool::Boat => tools_boat += 1,
                Tool::Glider => tools_glider += 1,
                _ => {}
            }
            match c.tech_level {
                0 => tech_stone += 1,
                1 => tech_copper += 1,
                2 => tech_iron += 1,
                3 => tech_gold += 1,
                4 => tech_crystal += 1,
                _ => {}
            }
        }

        self.stats = CreatureStats {
            population: pop as u32,
            species: species_ids.len() as u32,
            shots: keep_shots,
            towers: keep_towers,
            avg_fitness: fit / pop as f32,
            avg_energy: energy / pop as f32,
            max_generation: max_gen,
            births: self.births_total,
            deaths: self.deaths_total,
            workers,
            soldiers,
            queens,
            priests,
            merchants,
            builders,
            explorers,
            shamans,
            envoys,
            tools_pickaxe,
            tools_sword,
            tools_hammer,
            tools_drill,
            tools_scanner,
            tools_bow,
            tools_spear,
            tools_shield,
            tools_staff,
            tools_amulet,
            tools_backpack,
            tools_cart,
            tools_boat,
            tools_glider,
            tech_stone,
            tech_copper,
            tech_iron,
            tech_gold,
            tech_crystal,
            avg_mana: mana / pop as f32,
            avg_loyalty: loyalty_sum as f32 / pop as f32,
            diseases: disease_count,
            spells_cast: spell_count,
            trades: self.trades_total as u32,
            mounts: mount_count,
            structures: self.stats.structures,
            avg_hunger: hunger_sum / pop as f32,
            avg_thirst: thirst_sum / pop as f32,
            avg_stamina: stamina_sum / pop as f32,
            juveniles,
            elders,
            matings: self.matings_total,
            splits: self.splits_total,
            practices: self
                .culture_pool
                .iter()
                .map(|(_, c)| c.count_ones())
                .sum(),
            discoveries: self
                .discoveries
                .iter()
                .map(|(_, d)| d.count_ones())
                .sum(),
        };
    }
}

