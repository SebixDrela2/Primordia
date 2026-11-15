use super::{CreatureSystem, CreatureStats, Creature, Spell, MountKind, Stage, Disease, Role, Tool};

/// The running tally of a colony: every count the statistics panel
/// shows, gathered in one pass over the creatures.
struct Tally {
    fit: f32,
    energy: f32,
    mana: f32,
    loyalty: i32,
    max_gen: u32,
    hunger: f32,
    thirst: f32,
    stamina: f32,
    workers: u32,
    soldiers: u32,
    queens: u32,
    priests: u32,
    merchants: u32,
    builders: u32,
    explorers: u32,
    shamans: u32,
    envoys: u32,
    tools: [u32; 16],
    tech: [u32; 5],
    disease: u32,
    spells: u32,
    mounts: u32,
    juveniles: u32,
    elders: u32,
}

impl Tally {
    fn new() -> Self {
        Tally {
            fit: 0.0,
            energy: 0.0,
            mana: 0.0,
            loyalty: 0,
            max_gen: 0,
            hunger: 0.0,
            thirst: 0.0,
            stamina: 0.0,
            workers: 0,
            soldiers: 0,
            queens: 0,
            priests: 0,
            merchants: 0,
            builders: 0,
            explorers: 0,
            shamans: 0,
            envoys: 0,
            tools: [0; 16],
            tech: [0; 5],
            disease: 0,
            spells: 0,
            mounts: 0,
            juveniles: 0,
            elders: 0,
        }
    }

    fn add(&mut self, c: &Creature) {
        self.fit += c.fitness;
        self.energy += c.energy;
        self.mana += c.mana;
        self.loyalty += c.loyalty as i32;
        self.max_gen = self.max_gen.max(c.generation);
        self.hunger += c.hunger;
        self.thirst += c.thirst;
        self.stamina += c.stamina;
        if c.active_spell != Spell::None {
            self.spells += 1;
        }
        if c.mount_kind != MountKind::None {
            self.mounts += 1;
        }
        match c.stage {
            Stage::Juvenile => self.juveniles += 1,
            Stage::Elder => self.elders += 1,
            Stage::Adult => {}
        }
        if c.disease != Disease::None {
            self.disease += 1;
        }
        match c.role {
            Role::Worker => self.workers += 1,
            Role::Soldier => self.soldiers += 1,
            Role::Queen => self.queens += 1,
            Role::Priest => self.priests += 1,
            Role::Merchant => self.merchants += 1,
            Role::Builder => self.builders += 1,
            Role::Explorer => self.explorers += 1,
            Role::Shaman => self.shamans += 1,
            Role::Envoy => self.envoys += 1,
        }
        match c.tool {
            Tool::Pickaxe => self.tools[1] += 1,
            Tool::Sword => self.tools[2] += 1,
            Tool::Hammer => self.tools[3] += 1,
            Tool::Drill => self.tools[4] += 1,
            Tool::Scanner => self.tools[5] += 1,
            Tool::Bow => self.tools[6] += 1,
            Tool::Spear => self.tools[7] += 1,
            Tool::Shield => self.tools[8] += 1,
            Tool::Staff => self.tools[9] += 1,
            Tool::Amulet => self.tools[10] += 1,
            Tool::Backpack => self.tools[11] += 1,
            Tool::Cart => self.tools[12] += 1,
            Tool::Boat => self.tools[13] += 1,
            Tool::Glider => self.tools[14] += 1,
            _ => {}
        }
        match c.tech_level {
            0 => self.tech[0] += 1,
            1 => self.tech[1] += 1,
            2 => self.tech[2] += 1,
            3 => self.tech[3] += 1,
            4 => self.tech[4] += 1,
            _ => {}
        }
    }
}

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

        let mut tally = Tally::new();
        for c in &self.list {
            tally.add(c);
        }

        self.stats = self.build_stats(&tally, pop as u32, species_ids.len() as u32, keep_shots, keep_towers);
    }

    fn build_stats(
        &self,
        tally: &Tally,
        pop: u32,
        species: u32,
        keep_shots: u32,
        keep_towers: u32,
    ) -> CreatureStats {
        let f = pop as f32;
        CreatureStats {
            population: pop,
            species,
            shots: keep_shots,
            towers: keep_towers,
            avg_fitness: tally.fit / f,
            avg_energy: tally.energy / f,
            max_generation: tally.max_gen,
            births: self.births_total,
            deaths: self.deaths_total,
            workers: tally.workers,
            soldiers: tally.soldiers,
            queens: tally.queens,
            priests: tally.priests,
            merchants: tally.merchants,
            builders: tally.builders,
            explorers: tally.explorers,
            shamans: tally.shamans,
            envoys: tally.envoys,
            tools_pickaxe: tally.tools[1],
            tools_sword: tally.tools[2],
            tools_hammer: tally.tools[3],
            tools_drill: tally.tools[4],
            tools_scanner: tally.tools[5],
            tools_bow: tally.tools[6],
            tools_spear: tally.tools[7],
            tools_shield: tally.tools[8],
            tools_staff: tally.tools[9],
            tools_amulet: tally.tools[10],
            tools_backpack: tally.tools[11],
            tools_cart: tally.tools[12],
            tools_boat: tally.tools[13],
            tools_glider: tally.tools[14],
            tech_stone: tally.tech[0],
            tech_copper: tally.tech[1],
            tech_iron: tally.tech[2],
            tech_gold: tally.tech[3],
            tech_crystal: tally.tech[4],
            avg_mana: tally.mana / f,
            avg_loyalty: tally.loyalty as f32 / f,
            diseases: tally.disease,
            spells_cast: tally.spells,
            trades: self.trades_total as u32,
            mounts: tally.mounts,
            structures: self.stats.structures,
            avg_hunger: tally.hunger / f,
            avg_thirst: tally.thirst / f,
            avg_stamina: tally.stamina / f,
            juveniles: tally.juveniles,
            elders: tally.elders,
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
        }
    }
}
