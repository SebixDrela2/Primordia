//! The game without the picture.
//!
//! This is the whole simulation: the world, the colony, the victory
//! and the revival. The game draws it, and the learner works it with
//! no drawing at all, so that the learner is the same game and not a
//! copy of it.
//!
//! A run goes on until a species is the last one standing (or the
//! colony is dead), and then the **revival** begins: a new world, and a
//! new colony that carries the champion of the last run - its mind, its
//! genes and what it had learnt. That is the cycle the learner works
//! through, and every revival is a generation of learning.

use crate::creature::{CreatureSystem, Faction, RevivalGenome};
use crate::world::World;
use crate::{H, W};

/// How the a run ended.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A species is the last one standing.
    Won { species: u32, zorgs: u32 },
    /// The colony is gone and the ZORGs are left.
    Lost { zorgs: u32 },
    /// Nothing is left at all.
    Extinct,
}

/// The simulation: the world, the colony, and the run around it.
pub struct Session {
    pub world: World,
    pub creatures: CreatureSystem,
    /// The seed of this world.
    pub seed: u64,
    /// How many times the game has been played out.
    pub revival: u32,
    /// The winner of the last run.
    pub champion: Option<RevivalGenome>,
    /// The best of the colony, kept in case the colony is lost.
    pub fallback_champion: Option<RevivalGenome>,
    /// The species that was last seen to be the strongest.
    pub last_species: Option<u32>,
    /// How the last run ended.
    pub outcome: Option<Outcome>,
    /// How many times the best of the colony has changed. The learner
    /// uses it to know when the lineage that would survive has become
    /// another one.
    pub champion_version: u64,
}

impl Session {
    /// A new world with a new colony in it.
    #[must_use]
    pub fn new(seed: u64, pop: usize) -> Self {
        let mut world = World::new(W, H, seed);
        world.seed();

        let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
        creatures.populate(&world, pop);

        Session {
            world,
            creatures,
            seed,
            revival: 1,
            champion: None,
            fallback_champion: None,
            last_species: None,
            outcome: None,
            champion_version: 0,
        }
    }

    /// One tick of the world and of the colony.
    pub fn step(&mut self) {
        self.world.step();
        self.creatures.step(&mut self.world);
    }

    /// Has the run ended? The champion is kept when a species wins.
    pub fn check_victory(&mut self) -> Option<Outcome> {
        if self.outcome.is_some() {
            return self.outcome;
        }

        if let Some(sp) = crate::ui::minimap::dominant_player_species(&self.creatures) {
            if self.last_species != Some(sp) || self.fallback_champion.is_none() {
                self.last_species = Some(sp);
                self.fallback_champion = self.creatures.champion_overall();
                // The lineage that would survive has become another
                // one.
                self.champion_version += 1;
            }
        }

        let mut players: Vec<u32> = self
            .creatures
            .list
            .iter()
            .filter(|c| c.faction == Faction::Player)
            .map(|c| c.species)
            .collect();
        players.sort_unstable();
        players.dedup();
        let zorgs = self.creatures.zorg_count();

        let outcome = if self.creatures.list.is_empty() {
            Outcome::Extinct
        } else if players.is_empty() {
            Outcome::Lost { zorgs }
        } else if players.len() == 1 && (zorgs == 0 || self.world.zorg_cave_sealed) {
            let species = players[0];
            self.champion = self.creatures.champion_from(species);
            Outcome::Won { species, zorgs }
        } else {
            return None;
        };
        self.outcome = Some(outcome);
        Some(outcome)
    }

    /// The lineage that is carried into the next run: the champion if
    /// a species won, and the best of the colony if it was lost.
    #[must_use]
    pub fn carried(&mut self) -> Option<RevivalGenome> {
        match self.outcome {
            Some(Outcome::Won { .. }) => self.champion.take(),
            _ => self.fallback_champion.take().or_else(|| self.champion.take()),
        }
    }

    /// The next run: a new world, and a new colony that carries the
    /// lineage of the last one.
    pub fn revive(&mut self, next_seed: u64, carried: Option<&RevivalGenome>) {
        self.world = World::new(W, H, next_seed);
        self.world.seed();
        self.creatures = CreatureSystem::new(next_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
        match carried {
            Some(genome) => self.creatures.populate_revival(&self.world, 36, genome),
            None => self.creatures.populate(&self.world, 64),
        }
        self.seed = next_seed;
        self.revival += 1;
        self.last_species = None;
        self.fallback_champion = None;
        self.outcome = None;
        self.champion_version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::{brain_sizes, Net, Personality};
    use crate::rng::Rng;

    /// A genome to carry.
    fn genome(generation: u32, fitness: f32) -> RevivalGenome {
        let mut rng = Rng::new(3);
        RevivalGenome {
            net: Net::random(&brain_sizes(), &mut rng),
            color: [10, 20, 30],
            genes: [0.5; 32],
            size: 1.0,
            generation,
            culture: 0,
            personality: Personality::default(),
            fitness,
        }
    }

    /// The lineage of a won run must be carried into the next one, and
    /// the next run must be a new world with a colony that carries it.
    #[test]
    fn a_won_run_is_carried_into_the_next_one() {
        let mut session = Session::new(7, 8);
        session.champion = Some(genome(4, 3.5));
        session.outcome = Some(Outcome::Won { species: 1, zorgs: 0 });

        let carried = session.carried().expect("the winner is carried");
        assert_eq!(carried.generation, 4);

        let old_seed = session.seed;
        session.revive(old_seed.wrapping_add(1), Some(&carried));
        assert_ne!(session.seed, old_seed);
        assert_eq!(session.revival, 2);
        assert!(!session.creatures.list.is_empty());
        assert!(session.outcome.is_none());
    }

    /// A lost run must carry the best of the colony, not nothing.
    #[test]
    fn a_lost_run_carries_the_best_of_the_colony() {
        let mut session = Session::new(9, 8);
        session.fallback_champion = Some(genome(2, 1.5));
        session.outcome = Some(Outcome::Lost { zorgs: 3 });
        let carried = session.carried().expect("the best is carried");
        assert_eq!(carried.generation, 2);
    }
}

