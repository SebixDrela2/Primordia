//! Predation: who eats whom, and what the prey remembers.
//!
//! A hunt is not only a fight. When a species loses members to another
//! species, the survivors remember it: they keep a *fear* of the hunter,
//! and that fear is written into the animal itself as a personality trait.
//!
//! The fear has three effects:
//! - a frightened animal runs from the hunter much earlier,
//! - a frightened colony keeps closer together (it herds),
//! - a species that has been hunted hard learns to hunt itself, which is
//!   how a prey species turns into a predator.
//!
//! The memory belongs to the species, not to the individual: an animal
//! that never met the hunter still knows of it from its mother.

use super::*;

/// Look the fear up in a flat list of entries.
///
/// The behaviours read the memory through a plain slice, so they do not
/// have to borrow the whole creature system while a creature is borrowed.
pub fn fear_in(entries: &[(u32, u32, Fear)], prey: u32, predator: u32) -> Fear {
    entries
        .iter()
        .find(|(p, q, _)| *p == prey && *q == predator)
        .map(|(_, _, f)| *f)
        .unwrap_or(0.0)
}

/// How much fear a species holds for all of its hunters together.
pub fn fear_total(entries: &[(u32, u32, Fear)], prey: u32) -> Fear {
    entries
        .iter()
        .filter(|(p, _, _)| *p == prey)
        .map(|(_, _, f)| *f)
        .sum()
}

/// The fear of every species, laid out as a flat table.
///
/// The behaviours ask "how much does this species fear that one" for every
/// neighbour they see, so the answer has to come out in constant time. The
/// table is indexed by `prey * stride + predator`.
pub struct FearTable {
    stride: usize,
    cells: Vec<Fear>,
    /// How much each species fears all of its hunters together.
    totals: Vec<Fear>,
}

impl FearTable {
    pub fn build(entries: &[(u32, u32, Fear)], species: usize) -> Self {
        let stride = species.max(1);
        let mut cells = vec![0.0f32; stride * stride];
        let mut totals = vec![0.0f32; stride];
        for (p, q, f) in entries.iter() {
            let (pi, qi) = (*p as usize, *q as usize);
            if pi >= stride || qi >= stride {
                continue;
            }
            cells[pi * stride + qi] = *f;
            totals[pi] += *f;
        }
        FearTable {
            stride,
            cells,
            totals,
        }
    }

    #[inline]
    pub fn fear_of(&self, prey: u32, predator: u32) -> Fear {
        let (pi, qi) = (prey as usize, predator as usize);
        if pi >= self.stride || qi >= self.stride {
            return 0.0;
        }
        self.cells[pi * self.stride + qi]
    }

    #[inline]
    pub fn total_of(&self, prey: u32) -> Fear {
        let pi = prey as usize;
        if pi >= self.stride {
            return 0.0;
        }
        self.totals[pi]
    }
}

/// How much a species remembers a hunter, 0 .. 100.
pub type Fear = f32;

/// The predation memory of the whole world: for every species, the hunters
/// it remembers and how much it fears them.
#[derive(Clone, Default)]
pub struct PredationMemory {
    /// (prey species, predator species, fear).
    entries: Vec<(u32, u32, Fear)>,
}

impl PredationMemory {
    /// The fear a species holds for a hunter.
    pub fn fear_of(&self, prey: u32, predator: u32) -> Fear {
        self.entries
            .iter()
            .find(|(p, q, _)| *p == prey && *q == predator)
            .map(|(_, _, f)| *f)
            .unwrap_or(0.0)
    }

    /// Record that one of the prey lost an animal to the predator.
    pub fn record_kill(&mut self, prey: u32, predator: u32) {
        if prey == predator {
            return;
        }
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|(p, q, _)| *p == prey && *q == predator)
        {
            e.2 = (e.2 + 8.0).min(100.0);
            return;
        }
        if self.entries.len() > 4096 {
            return;
        }
        self.entries.push((prey, predator, 8.0));
    }

    /// The species that fear a given predator the most.
    pub fn most_feared_by(&self, predator: u32) -> Option<(u32, Fear)> {
        self.entries
            .iter()
            .filter(|(_, q, _)| *q == predator)
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _, f)| (*p, *f))
    }

    /// The fear fades. A species that is not hunted for a long time
    /// forgets the danger, and the herd becomes loose again.
    pub fn decay(&mut self) {
        for e in self.entries.iter_mut() {
            e.2 = (e.2 - 0.05).max(0.0);
        }
        self.entries.retain(|(_, _, f)| *f > 0.5);
    }

    /// The entries, for the panel.
    pub fn entries(&self) -> &[(u32, u32, Fear)] {
        &self.entries
    }
}

impl CreatureSystem {
    /// The fear this creature holds for the species it is looking at.
    ///
    /// The memory is the species' own, so every animal of it shares the
    /// lesson, whether or not it saw the kill itself.
    pub fn fear_of_species(&self, creature_species: u32, other_species: u32) -> Fear {
        self.predation.fear_of(creature_species, other_species)
    }

    /// How far a frightened animal runs.
    ///
    /// A prey species that has been hunted keeps well away from the hunter:
    /// the herd spots it much earlier and turns sooner.
    pub fn fear_flee_range(&self, creature_species: u32, other_species: u32) -> f32 {
        let fear = self.predation.fear_of(creature_species, other_species);
        if fear <= 0.0 {
            return 0.0;
        }
        // Up to twenty extra cells of caution.
        fear * 0.2
    }

    /// A species that has been hunted hard learns to hunt itself.
    ///
    /// This is how a prey species becomes a predator: the ones that were
    /// eaten start eating.
    pub fn learned_hunting(&self, species: u32) -> bool {
        let mut total = 0.0;
        for (prey, _predator, fear) in self.predation.entries() {
            if *prey == species {
                total += *fear;
            }
        }
        total > 45.0
    }
}
