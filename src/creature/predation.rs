use super::CreatureSystem;

#[must_use]
pub fn fear_in(entries: &[(u32, u32, Fear)], prey: u32, predator: u32) -> Fear {
    entries
        .iter()
        .find(|(p, q, _)| *p == prey && *q == predator)
        .map_or(0.0, |(_, _, f)| *f)
}

#[must_use]
pub fn fear_total(entries: &[(u32, u32, Fear)], prey: u32) -> Fear {
    entries
        .iter()
        .filter(|(p, _, _)| *p == prey)
        .map(|(_, _, f)| *f)
        .sum()
}

pub struct FearTable {
    stride: usize,
    cells: Vec<Fear>,

    totals: Vec<Fear>,
}

impl FearTable {
    #[must_use]
    pub fn build(entries: &[(u32, u32, Fear)], species: usize) -> Self {
        let stride = species.max(1);
        let mut cells = vec![0.0f32; stride * stride];
        let mut totals = vec![0.0f32; stride];
        for (p, q, f) in entries {
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
    #[must_use]
    pub fn fear_of(&self, prey: u32, predator: u32) -> Fear {
        let (pi, qi) = (prey as usize, predator as usize);
        if pi >= self.stride || qi >= self.stride {
            return 0.0;
        }
        self.cells[pi * self.stride + qi]
    }

    #[inline]
    #[must_use]
    pub fn total_of(&self, prey: u32) -> Fear {
        let pi = prey as usize;
        if pi >= self.stride {
            return 0.0;
        }
        self.totals[pi]
    }
}

pub type Fear = f32;

#[derive(Clone, Default)]
pub struct PredationMemory {

    entries: Vec<(u32, u32, Fear)>,
}

impl PredationMemory {

    #[must_use]
    pub fn fear_of(&self, prey: u32, predator: u32) -> Fear {
        self.entries
            .iter()
            .find(|(p, q, _)| *p == prey && *q == predator)
            .map_or(0.0, |(_, _, f)| *f)
    }

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

    #[must_use]
    pub fn most_feared_by(&self, predator: u32) -> Option<(u32, Fear)> {
        self.entries
            .iter()
            .filter(|(_, q, _)| *q == predator)
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _, f)| (*p, *f))
    }

    pub fn decay(&mut self) {
        for e in &mut self.entries {
            e.2 = (e.2 - 0.05).max(0.0);
        }
        self.entries.retain(|(_, _, f)| *f > 0.5);
    }

    #[must_use]
    pub fn entries(&self) -> &[(u32, u32, Fear)] {
        &self.entries
    }
}

impl CreatureSystem {

    #[must_use]
    pub fn fear_of_species(&self, creature_species: u32, other_species: u32) -> Fear {
        self.predation.fear_of(creature_species, other_species)
    }

    #[must_use]
    pub fn fear_flee_range(&self, creature_species: u32, other_species: u32) -> f32 {
        let fear = self.predation.fear_of(creature_species, other_species);
        if fear <= 0.0 {
            return 0.0;
        }

        fear * 0.2
    }

    #[must_use]
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
