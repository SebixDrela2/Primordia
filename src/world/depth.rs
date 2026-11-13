//! The zones of the deep.
//!
//! The underground is not one long grey wall of stone. It is a journey:
//! the caves under the surface give way to the crystal depths, then to the
//! sulphur works, then to the old bone galleries, and at the very bottom
//! the rock is still hot enough to melt.
//!
//! Each zone has its own rock, its own deposits and its own colour. A
//! colony that digs down learns the world by its depth: the deeper it
//! goes, the richer the rock and the worse the things that live there.

use crate::elements::Elem;

/// The themed bands of the deep.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthZone {
    /// The shallow caves: ordinary rock, the odd seam of ore.
    Shallows,
    /// The crystal depths: veins of crystal and pockets of gem.
    CrystalDepths,
    /// The sulphur works: vents, seeps and toxic gas.
    SulphurWorks,
    /// The bone galleries: the old stone, and the old bones in it.
    BoneGalleries,
    /// The magma layer: the world is still hot down here.
    MagmaLayer,
}

impl DepthZone {
    /// The name shown in the HUD.
    pub fn name(self) -> &'static str {
        match self {
            DepthZone::Shallows => "The Shallows",
            DepthZone::CrystalDepths => "The Crystal Depths",
            DepthZone::SulphurWorks => "The Sulphur Works",
            DepthZone::BoneGalleries => "The Bone Galleries",
            DepthZone::MagmaLayer => "The Magma Layer",
        }
    }

    /// A one-line description of the zone.
    pub fn blurb(self) -> &'static str {
        match self {
            DepthZone::Shallows => "ordinary rock and the odd seam of ore",
            DepthZone::CrystalDepths => "veins of crystal, and pockets of gem",
            DepthZone::SulphurWorks => "vents and seeps, and the gas they give off",
            DepthZone::BoneGalleries => "the old stone, and the old bones in it",
            DepthZone::MagmaLayer => "the rock is still hot enough to melt",
        }
    }

    /// The zone a depth below the surface belongs to.
    ///
    /// The bands are measured from the terrain, never from an absolute
    /// row: the deep has to be the deep wherever the ground happens to be.
    pub const fn of(depth: i32) -> DepthZone {
        if depth < 58 {
            DepthZone::Shallows
        } else if depth < 104 {
            DepthZone::CrystalDepths
        } else if depth < 150 {
            DepthZone::SulphurWorks
        } else if depth < 215 {
            DepthZone::BoneGalleries
        } else {
            DepthZone::MagmaLayer
        }
    }

    /// The colour the zone lends to the rock in it.
    ///
    /// The renderer folds this into the depth grade, so the deep reads as
    /// a series of places rather than one long tunnel.
    pub const fn tint(self) -> [f32; 3] {
        match self {
            DepthZone::Shallows => [1.0, 1.0, 1.0],
            // The crystal depths are cold and blue.
            DepthZone::CrystalDepths => [0.88, 0.94, 1.10],
            // The sulphur works are yellow and sick.
            DepthZone::SulphurWorks => [1.10, 1.02, 0.82],
            // The bone galleries are pale and old.
            DepthZone::BoneGalleries => [1.04, 1.02, 0.96],
            // The magma layer is red.
            DepthZone::MagmaLayer => [1.14, 0.94, 0.86],
        }
    }

    /// The rock this zone is made of.
    pub fn rock(self, rng: &mut crate::rng::Rng) -> Elem {
        match self {
            DepthZone::Shallows => Elem::Stone,
            DepthZone::CrystalDepths => {
                // Veins of crystal run through the rock here.
                if rng.chance(0.07) {
                    Elem::Crystal
                } else {
                    Elem::Stone
                }
            }
            DepthZone::SulphurWorks => {
                // The rock is burnt and stained. The sulphur itself is
                // kept to the veins, because loose powder never settles.
                Elem::Stone
            }
            DepthZone::BoneGalleries => {
                // Old, hard rock with the odd rust in it. The rust is kept
                // to a trickle: loose powder never settles, and the deep
                // would spend all its time falling over.
                if rng.chance(0.02) {
                    Elem::Rust
                } else {
                    Elem::Granite
                }
            }
            DepthZone::MagmaLayer => {
                // The rock here has been cooked.
                if rng.chance(0.05) {
                    Elem::Obsidian
                } else {
                    Elem::Granite
                }
            }
        }
    }

    /// What is found in this zone, besides the rock.
    ///
    /// These are the treasures a colony digs for: the crystal depths have
    /// the gems, the bone galleries have the old bones, and the magma layer
    /// has the fire.
    pub fn treasure(self, rng: &mut crate::rng::Rng) -> Option<Elem> {
        match self {
            DepthZone::Shallows => {
                let roll = rng.range(100);
                if roll < 6 {
                    Some(Elem::Copper)
                } else if roll < 8 {
                    Some(Elem::Iron)
                } else {
                    None
                }
            }
            DepthZone::CrystalDepths => {
                let roll = rng.range(100);
                if roll < 5 {
                    Some(Elem::Gem)
                } else if roll < 7 {
                    Some(Elem::Diamond)
                } else if roll < 18 {
                    Some(Elem::Crystal)
                } else if roll < 22 {
                    Some(Elem::Mana)
                } else {
                    None
                }
            }
            DepthZone::SulphurWorks => {
                let roll = rng.range(100);
                if roll < 3 {
                    Some(Elem::Acid)
                } else if roll < 5 {
                    Some(Elem::Poison)
                } else if roll < 7 {
                    Some(Elem::Mercury)
                } else if roll < 14 {
                    Some(Elem::Sulfur)
                } else {
                    None
                }
            }
            DepthZone::BoneGalleries => {
                let roll = rng.range(100);
                if roll < 10 {
                    Some(Elem::Bone)
                } else if roll < 16 {
                    Some(Elem::Crystal)
                } else if roll < 19 {
                    Some(Elem::Gold)
                } else {
                    None
                }
            }
            DepthZone::MagmaLayer => {
                let roll = rng.range(100);
                if roll < 2 {
                    Some(Elem::Magma)
                } else if roll < 3 {
                    Some(Elem::Lava)
                } else if roll < 9 {
                    Some(Elem::Gold)
                } else {
                    None
                }
            }
        }
    }

    /// How much treasure lies in this zone.
    ///
    /// The deep is worth digging for, but not everywhere: the crystal
    /// depths are the richest, and the bone galleries are poor.
    pub fn treasure_chance(self) -> f32 {
        match self {
            DepthZone::Shallows => 0.04,
            DepthZone::CrystalDepths => 0.10,
            DepthZone::SulphurWorks => 0.07,
            DepthZone::BoneGalleries => 0.08,
            DepthZone::MagmaLayer => 0.09,
        }
    }
}

/// The tint of the deep, sampled every eight cells of depth.
///
/// The renderer reads it with a single lookup instead of working out the
/// zone for every pixel it paints.
pub const DEPTH_TINT: [[f32; 3]; 32] = {
    let mut t = [[1.0f32, 1.0, 1.0]; 32];
    let mut i = 0;
    while i < 32 {
        t[i] = DepthZone::of((i as i32) * 8).tint();
        i += 1;
    }
    t
};
