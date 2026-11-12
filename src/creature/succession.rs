//! Succession: what happens to a colony that loses its queen.
//!
//! A colony is not just a heap of animals: it is organised around its
//! queen. When the queen dies the colony does not simply carry on. There
//! is a *crisis*:
//!
//! - the colony holds no ground, because there is nobody to defend it,
//! - it builds nothing, because there is nobody to decide what to build,
//! - its animals lose heart and their morale drains,
//! - and the rivals take the chance: a colony in crisis is an easy target.
//!
//! The crisis ends when a new queen takes the throne. That can take a
//! while, and if the colony is small enough it may never recover: the
//! animals drift away and the species dies out where it stood.

use super::*;

/// How long a colony stays in crisis before it can even try to find a new
/// queen.
pub const CRISIS_TICKS: u64 = 600;

/// Is this species in a succession crisis?
///
/// A species is in crisis while it has living members but no queen. The
/// start tick is only used to work out how far the crisis has run.
pub fn in_crisis(succession: &[(u32, u64)], species: u32, tick: u64) -> bool {
    succession
        .iter()
        .any(|(sp, start)| *sp == species && tick.saturating_sub(*start) < CRISIS_TICKS)
}

impl CreatureSystem {
    /// The species that are in crisis right now.
    pub fn species_in_crisis(&self) -> Vec<u32> {
        self.succession
            .iter()
            .filter(|(sp, start)| self.tick.saturating_sub(*start) < CRISIS_TICKS)
            .map(|(sp, _)| *sp)
            .collect()
    }

    /// Work out which colonies have lost their queen.
    ///
    /// This is the truth of the matter: a species with members but no
    /// queen is in crisis, and a species that has found a new queen is not.
    /// The list is rebuilt every tick, so a crisis ends the moment the
    /// colony gets a queen, however that happens.
    pub fn update_succession(&mut self) {
        let mut with_queen: Vec<u32> = Vec::new();
        let mut without_queen: Vec<u32> = Vec::new();
        for c in self.list.iter() {
            if c.role == Role::Queen {
                if !with_queen.contains(&c.species) {
                    with_queen.push(c.species);
                }
            } else if !without_queen.contains(&c.species) {
                without_queen.push(c.species);
            }
        }
        // A species that has a queen is never in crisis.
        self.succession.retain(|(sp, _)| !with_queen.contains(sp));
        // A species with members but no queen starts one.
        for sp in without_queen {
            // A species that has a queen is not in crisis, even though it
            // also has members that are not the queen.
            if with_queen.contains(&sp) {
                continue;
            }
            if !self.succession.iter().any(|(s, _)| *s == sp) {
                self.succession.push((sp, self.tick));
            }
        }
    }

    /// The colony enters a crisis: the queen has died and nobody has taken
    /// the throne yet.
    pub fn begin_succession(&mut self, species: u32) {
        if self.succession.iter().any(|(sp, _)| *sp == species) {
            return;
        }
        self.succession.push((species, self.tick));
    }

    /// A new queen has taken the throne. The crisis is over.
    pub fn end_succession(&mut self, species: u32) {
        self.succession.retain(|(sp, _)| *sp != species);
    }

    /// The crisis of a colony makes it weaker in every way that matters.
    ///
    /// The animals lose heart, they hold no ground, and they do not build.
    /// A colony in crisis is a colony that can be taken.
    pub fn crisis_penalty(&self, species: u32) -> f32 {
        if in_crisis(&self.succession, species, self.tick) {
            1.0
        } else {
            0.0
        }
    }

    /// How much the animals of a colony in crisis lose their nerve.
    ///
    /// The energy drains away: without a queen to organise them, the
    /// colony falls apart.
    pub fn crisis_drain(&self, species: u32) -> f32 {
        if in_crisis(&self.succession, species, self.tick) {
            // The longer the crisis goes on, the worse it gets.
            let start = self
                .succession
                .iter()
                .find(|(sp, _)| *sp == species)
                .map(|(_, s)| *s)
                .unwrap_or(self.tick);
            let age = self.tick.saturating_sub(start) as f32;
            (0.004 + age * 0.000012).min(0.05)
        } else {
            0.0
        }
    }
}
