//! Bonds: the pairs that hold a colony together.
//!
//! A bond is a lasting partnership between two animals of the same kind.
//! It forms when two sociable creatures meet, and it changes how they
//! live: a bonded pair looks after each other, fends for each other and
//! finds each other when the colony is scattered.
//!
//! The bond is the social counterpart of the culture. A culture is what a
//! species has learned; a bond is what two animals have of each other.

use super::*;

/// How far a creature looks for a partner to bond with.
pub const BOND_RANGE: f32 = 12.0;

/// The bond a creature has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bond {
    /// The creature it is bonded to.
    pub partner: u32,
    /// How long the bond has lasted.
    pub age: u32,
}

impl Creature {
    /// Is this creature sociable enough to form a bond?
    pub fn bondable(&self) -> bool {
        self.personality.sociability > 0.55
            && self.stage.can_breed()
            && self.bond.is_none()
            && !self.migrating
    }

    /// How much a bond is worth to this creature right now.
    ///
    /// A creature with a partner is better fed and in better spirits: it
    /// has somebody to look after it.
    pub fn bond_morale(&self, partner_alive: bool) -> f32 {
        match (self.bond, partner_alive) {
            (Some(_), true) => 1.0,
            (Some(_), false) => -1.0,
            (None, _) => 0.0,
        }
    }
}

impl CreatureSystem {
    /// Form new bonds.
    ///
    /// Two animals of the same kind that meet, and that both like the
    /// company of their own kind, become partners. The bond is mutual: it
    /// is set on both of them at once.
    pub(crate) fn form_bonds(&mut self, grid: &SpatialGrid) {
        // Only now and then: a bond is a decision, not a reflex.
        if self.tick % 24 != 0 {
            return;
        }
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for i in 0..self.list.len() {
            if !self.list[i].bondable() {
                continue;
            }
            let (x, y, species, id) = {
                let c = &self.list[i];
                (c.x, c.y, c.species, c.id)
            };
            grid.near(x, y, BOND_RANGE as usize, |idx| {
                let j = idx as usize;
                if j >= self.list.len() || j == i {
                    return;
                }
                let other = &self.list[j];
                if other.species != species || !other.bondable() {
                    return;
                }
                let d = ((other.x - x).powi(2) + (other.y - y).powi(2)).sqrt();
                if d > BOND_RANGE {
                    return;
                }
                // The more sociable of the two decides.
                if self.list[i].personality.sociability < other.personality.sociability {
                    return;
                }
                pairs.push((id, other.id));
            });
        }
        // Set the bonds. A creature that has already bonded in this round
        // is left alone.
        for (a, b) in pairs {
            let mut bonded = false;
            for c in self.list.iter_mut() {
                if c.id == a {
                    if c.bond.is_none() {
                        c.bond = Some(b);
                        bonded = true;
                    }
                }
            }
            if bonded {
                for c in self.list.iter_mut() {
                    if c.id == b {
                        if c.bond.is_none() {
                            c.bond = Some(a);
                        }
                    }
                }
            }
        }
    }

    /// The bond a creature has, and whether the partner is still alive.
    pub fn bond_of(&self, id: u32) -> Option<(u32, bool)> {
        let c = self.list.iter().find(|c| c.id == id)?;
        let partner = c.bond?;
        let alive = self.list.iter().any(|o| o.id == partner && o.health > 0.0);
        Some((partner, alive))
    }

    /// How many bonds the colony holds.
    pub fn bond_count(&self) -> u32 {
        self.list.iter().filter(|c| c.bond.is_some()).count() as u32 / 2
    }
}
