//! Sanctification: the answer to the deep.
//!
//! The ZORGs cannot be out-fought. There are always more of them, the
//! cave breeds more while the colony is still alive, and they are a
//! faction of their own: they never ally, never surrender and never die
//! out for good. A colony that wants to finish the deep off has to do
//! something else — it has to *unmake* the deep's hold on the world.
//!
//! Two things come out of the Sanctification discovery:
//!
//! - the **Sunfire**, a brand of sanctified fire. It burns the corruption
//!   of the deep and scours the ZORGs where they lie — but it is a poor
//!   weapon against ordinary flesh. A colony that leans on it against its
//!   rivals will find itself at a disadvantage: the fire is made for the
//!   deep, not for a rival with a sword.
//! - the **Ward**, a pillar of the same fire. It burns the ZORGs that
//!   come near it and cleanses the ground around it. Raised inside the
//!   ZORG cave it seals the cave, and the deep stops breeding. That is
//!   the only way a run can actually end against the ZORGs.

use super::*;

/// How much a Sunfire hurts a ZORG.
///
/// The ZORGs are the deep's own filth, and the light of the surface
/// scours them. A Sunfire goes through a ZORG like a blade through wet
/// cloth.
pub const SUNFIRE_VS_ZORG: f32 = 2.6;

/// How much a Sunfire hurts anything else.
///
/// It burns the corruption, not the flesh. Against a rival with a sword
/// this is a candle in the wind.
pub const SUNFIRE_VS_OTHERS: f32 = 0.45;

/// How far a Ward reaches.
pub const WARD_RADIUS: f32 = 30.0;

/// How much a Ward burns a ZORG that stands in its light, per tick.
pub const WARD_BURN: f32 = 0.9;

/// How far below the surface a ward has to stand for its fire to reach
/// down into the cave of the ZORGs.
///
/// A ward has to be raised in the deep, not on the surface: the fire
/// reaches down through the rock, but it has to be under the ground for
/// it to reach the cave.
pub const WARD_DEPTH: i32 = 35;

/// The damage a Sunfire does against a target.
pub fn sunfire_multiplier(target_is_zorg: bool) -> f32 {
    if target_is_zorg {
        SUNFIRE_VS_ZORG
    } else {
        SUNFIRE_VS_OTHERS
    }
}

/// The damage a Sunfire does, with the craft of the colony taken into
/// account.
pub fn sunfire_damage(tech: u32, target_is_zorg: bool) -> f32 {
    let mut mult = sunfire_multiplier(target_is_zorg);
    // Sunfirecraft: the fire is made to burn.
    if tech & discovery_bits::SUNFIRECRAFT != 0 {
        mult *= 1.3;
    }
    mult
}

/// Is this cell part of the corruption of the deep?
///
/// The things the ZORGs leave behind them: the virus, the bacteria, the
/// poison and the tar. Sanctified fire scours them away.
pub fn is_corruption(e: Elem) -> bool {
    matches!(
        e,
        Elem::Virus | Elem::Bacteria | Elem::Poison | Elem::Tar | Elem::Chlorine
    )
}

impl CreatureSystem {
    /// The wards of the colonies, and what they do.
    ///
    /// A ward is a pillar of sanctified fire. It burns the ZORGs that come
    /// near it, scours the corruption out of the ground around it, and -
    /// raised inside the ZORG cave - it seals the cave, so that the deep
    /// can breed no more of them.
    ///
    /// This runs every few ticks rather than every tick: the fire does
    /// not need to be tended.
    pub(crate) fn run_wards(&mut self, world: &mut World, grid: &SpatialGrid) {
        if self.tick % 5 != 0 {
            return;
        }
        // Wardcraft: the fire of a ward reaches further.
        let mut radius = WARD_RADIUS;
        for (_, d) in self.discoveries.iter() {
            if *d & discovery_bits::WARDCRAFT != 0 {
                radius *= 1.5;
            }
        }
        let wards: Vec<(f32, f32, u32)> = self
            .structures
            .iter()
            .filter(|s| s.kind == structure_kind::WARD)
            .map(|s| (s.x, s.y, s.species))
            .collect();
        if wards.is_empty() {
            return;
        }

        // ---- The fire burns the ZORGs that come near it ----
        for (wx, wy, species) in wards.iter() {
            // The ZORGs inside the light of the ward.
            let mut near: Vec<usize> = Vec::new();
            grid.near(*wx, *wy, radius as usize, |idx| near.push(idx as usize));
            for j in near {
                if j >= self.list.len() {
                    continue;
                }
                let c = &self.list[j];
                if c.faction != Faction::Zorg || c.health <= 0.0 {
                    continue;
                }
                let d = ((c.x - wx).powi(2) + (c.y - wy).powi(2)).sqrt();
                if d > radius {
                    continue;
                }
                // The nearer it stands to the fire, the harder it burns.
                let burn = WARD_BURN * (1.0 - d / radius);
                self.list[j].health -= burn;
            }
            // A ward belongs to a colony: its owners are heartened by it.
            for c in self.list.iter_mut() {
                if c.species == *species && c.role == Role::Queen {
                    c.health = (c.health + 0.01).min(c.max_health());
                }
            }
        }

        // ---- The fire scours the ground clean ----
        //
        // The corruption the ZORGs leave behind them is eaten back by
        // the light of the ward, so a colony that holds one slowly makes
        // its own ground clean again. This is done less often than the
        // burning: the ground does not clean itself in a second.
        if self.tick % 20 == 0 {
            for (wx, wy, _) in wards.iter() {
                let r = radius as i32;
                let cx = *wx as i32;
                let cy = *wy as i32;
                for y in (cy - r)..(cy + r) {
                    for x in (cx - r)..(cx + r) {
                        let dx = (x - cx) as f32;
                        let dy = (y - cy) as f32;
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }
                        if is_corruption(world.get(x, y)) {
                            world.set_xy(x, y, Elem::Empty);
                        }
                    }
                }
            }
        }

        // ---- And a ward in the deep seals the cave ----
        //
        // This is the only way a run can end against the ZORGs: the fire
        // of the ward reaches down through the rock and closes the cave
        // they breed in.
        if !world.zorg_cave_sealed {
            for (wx, wy, _) in wards.iter() {
                let surf = world.surface_y(*wx as i32);
                if (*wy as i32) - surf < WARD_DEPTH {
                    continue;
                }
                world.zorg_cave_sealed = true;
                self.diplomacy.log_event(
                    "THE DEEP IS SEALED: the cave of the ZORGs is closed".to_string(),
                    [255, 230, 150],
                    self.tick,
                );
                self.events.push(SimEvent::Discovery {
                    x: *wx,
                    y: *wy,
                    color: [255, 230, 150],
                });
                break;
            }
        }
    }
}
