//! Speciation: how one species becomes two.
//!
//! A species that has grown too large for one patch of ground does not
//! simply exist forever. Part of the colony leaves: a few animals walk
//! away from the others, and if they get far enough they become a species
//! of their own — different genes, a different colour, a different
//! alliance table and a different diplomatic record.
//!
//! This is what keeps a world varied. Without it a run can only ever lose
//! species; with it, lineages split, drift apart and then compete for the
//! same ground.

use super::*;

/// How many members a colony must have before it can split.
pub const MIN_SPLIT_POP: usize = 14;

/// How many members the old colony must keep when some of them leave.
pub const MIN_REMAINING: usize = 8;

/// The most species that may split off in one run.
///
/// The cap keeps a run from fragmenting into an unmanageable number of
/// lineages, and it keeps the victory condition reachable.
pub const MAX_SPLITS: u32 = 6;

/// How many members a species needs to count as viable.
pub const VIABLE_MEMBERS: usize = 4;

/// How far the emigrants have to get from the old colony, in cells.
pub const SPLIT_DISTANCE: f32 = 80.0;

/// How long the emigrants have to be away before they become a species.
pub const SPLIT_ELAPSED: u32 = 300;

/// A group of animals that has left its colony.
#[derive(Clone)]
pub struct Migration {
    /// The species they came from.
    pub species: u32,
    /// The animals that left, by creature id.
    pub members: Vec<u32>,
    /// Where they came from.
    pub origin_x: f32,
    pub origin_y: f32,
    /// How long they have been on the road.
    pub elapsed: u32,
    /// How far they have got from the old colony.
    pub distance: f32,
    /// The species they have become, once they have split.
    pub new_species: Option<u32>,
}

impl Migration {
    /// Has the group travelled far enough to be its own species?
    pub fn is_ready(&self) -> bool {
        self.distance > SPLIT_DISTANCE && self.elapsed > SPLIT_ELAPSED
    }
}

impl CreatureSystem {
    /// Start a migration out of a colony that has outgrown its ground.
    ///
    /// Returns the migration if one was started.
    pub(crate) fn begin_migration(&mut self, species: u32, pressure: f32) -> Option<Migration> {
        // The world may not fragment without limit.
        if self.splits_total >= MAX_SPLITS {
            return None;
        }
        // The colony has to be big enough to spare some members.
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
        // The colony only splits when the ground is crowded or scarce: a
        // comfortable colony has no reason to send anyone away.
        if pressure < 0.15 && members.len() < 20 {
            return None;
        }
        // Only one migration per species at a time.
        if self.migrations.iter().any(|m| m.species == species) {
            return None;
        }
        // A handful of the colony's members leave. The rest stay, and the
        // old colony must still be a colony afterwards.
        let leaving = 3 + self.rng.range(2) as usize;
        if members.len() - leaving < MIN_REMAINING {
            return None;
        }
        let mut chosen: Vec<u32> = Vec::new();
        let mut ids: Vec<u32> = Vec::new();
        for &i in members.iter() {
            ids.push(self.list[i].id);
        }
        // Take a scattered sample rather than the first few in the list.
        for k in 0..leaving.min(ids.len()) {
            let pick = (self.rng.range(ids.len() as u32) as usize + k) % ids.len();
            let id = ids[pick];
            ids.swap_remove(pick);
            chosen.push(id);
        }
        if chosen.len() < 3 {
            return None;
        }
        // Mark the emigrants.
        for c in self.list.iter_mut() {
            if chosen.contains(&c.id) {
                c.migrating = true;
            }
        }
        // The colony they left.
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

    /// Move the migrations on and split the ones that have travelled far
    /// enough.
    ///
    /// `elapsed_ticks` is how many simulation ticks have passed since the
    /// last call, so the age of a migration is measured in ticks and not in
    /// updates.
    pub(crate) fn update_migrations(&mut self, elapsed_ticks: u32) {
        let mut finished: Vec<usize> = Vec::new();
        let count = self.migrations.len();
        for mi in 0..count {
            // ---- Advance the migration ----
            let (species, origin_x, origin_y) = {
                let m = &self.migrations[mi];
                let t = (m.species, m.origin_x, m.origin_y);
                self.migrations[mi].elapsed += elapsed_ticks;
                t
            };
            // How far the group has got from the colony it left.
            let mut far = 0.0f32;
            let mut members = 0u32;
            for c in self.list.iter() {
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

            // ---- The group has become a species of its own ----
            let new_species = self.new_species_id();
            let mut first = true;
            for c in self.list.iter_mut() {
                if c.species != species || !c.migrating {
                    continue;
                }
                c.species = new_species;
                c.team_color = species_team_color(&mut self.rng, new_species);
                // A little more drift, now that they are a separate
                // lineage: this is where a new trait can appear.
                for gi in c.genes.iter_mut() {
                    *gi = (*gi + (self.rng.f32() * 2.0 - 1.0) * 0.10).clamp(0.0, 1.0);
                }
                c.traits = decode_traits(&c.genes);
                c.affinity = Affinity::from_roll(c.genes[16]);
                c.migrating = false;
                // The first of them founds the new colony.
                if first {
                    c.role = Role::Queen;
                    c.base_x = c.x;
                    c.base_y = c.y;
                    first = false;
                }
            }
            self.migrations[mi].new_species = Some(new_species);
            self.splits_total += 1;
            // The birth of a species is a moment worth seeing: a burst of
            // light where the new colony begins.
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
        // The migrations that have split are done.
        for i in finished.into_iter().rev() {
            self.migrations.remove(i);
        }
    }

    /// Give out a brand new species id.
    pub fn new_species_id(&mut self) -> u32 {
        self.next_species += 1;
        self.next_species
    }
}
