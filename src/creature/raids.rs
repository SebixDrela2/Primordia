//! Raids.
//!
//! Two species at war do not only fight in the open. A war party that
//! reaches the enemy's camp tears the camp down: the towers, farms and
//! workshops of the other species are broken, and the raiders carry the
//! materials home.

use super::*;
use crate::creature::diplomacy::RelationState;

impl CreatureSystem {
    /// Look for enemy buildings that can be reached and smash them.
    ///
    /// This runs now and then rather than every tick: a raid is a thing that
    /// happens, not a thing that happens sixty times a second.
    pub(crate) fn raid_structures(&mut self, world: &mut World) {
        if self.tick % 40 != 0 || self.structures.is_empty() {
            return;
        }

        // Which species is at war with whom.
        let mut wars: Vec<(u32, u32)> = Vec::new();
        for rel in self.diplomacy.relations.iter() {
            if rel.state == RelationState::War {
                wars.push((rel.a, rel.b));
            }
        }
        if wars.is_empty() {
            return;
        }

        // Find the buildings that are in reach.
        let mut targets: Vec<(usize, u32)> = Vec::new();
        for (i, s) in self.structures.iter().enumerate() {
            for (a, b) in wars.iter() {
                let enemy = if *a == s.species {
                    *b
                } else if *b == s.species {
                    *a
                } else {
                    continue;
                };
                let guarded = self.list.iter().any(|c| {
                    c.species == enemy
                        && (c.x - s.x).powi(2) + (c.y - s.y).powi(2) < 700.0
                });
                if guarded && self.rng.chance(0.35) {
                    targets.push((i, enemy));
                    break;
                }
            }
        }
        if targets.is_empty() {
            return;
        }

        // A raid wears a camp down rather than deleting it: the building
        // takes damage and is only lost when it is broken.
        let mut broken = 0u32;
        let mut wounded = 0u32;
        for (i, enemy) in targets.into_iter().rev() {
            if i >= self.structures.len() {
                continue;
            }
            let s = &mut self.structures[i];
            s.hp -= 34.0;
            if s.hp > 0.0 {
                wounded += 1;
                continue;
            }
            let s = self.structures.remove(i);
            let tx = s.x.round() as i32;
            let ty = s.y.round() as i32;
            // Smash it and scatter the rubble.
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if world.get(tx + dx, ty + dy) != Elem::Empty {
                        world.set_xy(tx + dx, ty + dy, Elem::Stone);
                    }
                }
            }
            // The raiders carry the loot away.
            for c in self.list.iter_mut() {
                if c.species == enemy {
                    c.carried = (c.carried + 25.0).min(CARRY_CAP);
                    c.energy = (c.energy + 30.0).min(c.max_energy);
                }
            }
            broken += 1;
        }

        if broken > 0 || wounded > 0 {
            self.diplomacy.stats.raids += broken;
            let color = self.species_color(self.structures.first().map(|s| s.species).unwrap_or(0));
            self.diplomacy.log_event(
                format!(
                    "RAID: {} buildings broken, {} under attack",
                    broken, wounded
                ),
                color,
                self.tick,
            );
        }
    }
}
