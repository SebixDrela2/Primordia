use super::*;

pub const MIN_SPLIT_POP: usize = 14;

pub const MIN_REMAINING: usize = 8;

pub const MAX_SPLITS: u32 = 6;

pub const VIABLE_MEMBERS: usize = 4;

pub const SPLIT_DISTANCE: f32 = 80.0;

pub const SPLIT_ELAPSED: u32 = 300;

#[derive(Clone)]
pub struct Migration {

    pub species: u32,

    pub members: Vec<u32>,

    pub origin_x: f32,
    pub origin_y: f32,

    pub elapsed: u32,

    pub distance: f32,

    pub new_species: Option<u32>,
}

impl Migration {

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.distance > SPLIT_DISTANCE && self.elapsed > SPLIT_ELAPSED
    }
}

impl CreatureSystem {

    pub fn maybe_speciate(&mut self, pressure: f32) {
        if self.tick % 120 != 0 {
            return;
        }
        let species: Vec<u32> = {
            let mut v: Vec<u32> = self.list.iter().map(|c| c.species).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        for sp in species {
            if self.rng.chance(0.06) {
                let _ = self.begin_migration(sp, pressure);
            }
        }
        self.update_migrations(120);
    }

    #[must_use]
    pub fn begin_migration(&mut self, species: u32, pressure: f32) -> Option<Migration> {

        if species == ZORG_SPECIES {
            return None;
        }

        if self.splits_total >= MAX_SPLITS {
            return None;
        }

        let members: Vec<usize> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, c)| c.species == species && !c.migrating)
            .map(|(i, _)| i)
            .collect();
        if members.len() < MIN_SPLIT_POP {
            return None;
        }

        if pressure < 0.15 && members.len() < 20 {
            return None;
        }

        if self.migrations.iter().any(|m| m.species == species) {
            return None;
        }

        let leaving = 3 + self.rng.range(2) as usize;
        if members.len() - leaving < MIN_REMAINING {
            return None;
        }
        let mut chosen: Vec<u32> = Vec::new();
        let mut ids: Vec<u32> = Vec::new();
        for &i in members.iter() {
            ids.push(self.list[i].id);
        }

        for k in 0..leaving.min(ids.len()) {
            let pick = (self.rng.range(ids.len() as u32) as usize + k) % ids.len();
            let id = ids[pick];
            ids.swap_remove(pick);
            chosen.push(id);
        }
        if chosen.len() < 3 {
            return None;
        }

        for c in self.list.iter_mut() {
            if chosen.contains(&c.id) {
                c.migrating = true;
            }
        }

        let (origin_x, origin_y) = self
            .list
            .iter()
            .find(|c| c.species == species && c.role == Role::Queen)
            .map(|c| (c.base_x, c.base_y))
            .unwrap_or((0.0, 0.0));
        let migration = Migration {
            species,
            members: chosen,
            origin_x,
            origin_y,
            elapsed: 0,
            distance: 0.0,
            new_species: None,
        };
        self.migrations.push(migration.clone());
        self.migrations_started += 1;
        Some(migration)
    }

    pub fn update_migrations(&mut self, elapsed_ticks: u32) {
        let mut finished: Vec<usize> = Vec::new();
        let count = self.migrations.len();
        for mi in 0..count {

            let (species, origin_x, origin_y) = {
                let m = &self.migrations[mi];
                let t = (m.species, m.origin_x, m.origin_y);
                self.migrations[mi].elapsed += elapsed_ticks;
                t
            };

            let mut far = 0.0f32;
            let mut members = 0u32;
            for c in &self.list {
                if c.species == species && c.migrating {
                    let d = ((c.x - origin_x).powi(2) + (c.y - origin_y).powi(2)).sqrt();
                    far += d;
                    members += 1;
                }
            }
            self.migrations[mi].distance =
                if members > 0 { far / members as f32 } else { 0.0 };

            if !self.migrations[mi].is_ready() {
                continue;
            }

            let new_species = self.new_species_id();
            let mut first = true;
            for c in self.list.iter_mut() {
                if c.species != species || !c.migrating {
                    continue;
                }
                c.species = new_species;
                c.team_color = species_team_color(&mut self.rng, new_species);

                for gi in c.genes.iter_mut() {
                    *gi = (*gi + (self.rng.f32() * 2.0 - 1.0) * 0.10).clamp(0.0, 1.0);
                }
                c.traits = decode_traits(&c.genes);
                c.affinity = Affinity::from_roll(c.genes[16]);
                c.migrating = false;

                if first {
                    c.role = Role::Queen;
                    c.base_x = c.x;
                    c.base_y = c.y;
                    first = false;
                }
            }
            self.migrations[mi].new_species = Some(new_species);
            self.splits_total += 1;

            if let Some(c) = self
                .list
                .iter()
                .find(|c| c.species == new_species)
            {
                self.events.push(SimEvent::Speciation {
                    x: c.x,
                    y: c.y,
                    color: c.team_color,
                });
            }
            finished.push(mi);
        }

        for i in finished.into_iter().rev() {
            self.migrations.remove(i);
        }
    }

    #[must_use]
    pub fn new_species_id(&mut self) -> u32 {
        self.next_species += 1;
        self.next_species
    }
}
