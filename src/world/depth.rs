use crate::elements::Elem;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthZone {

    Shallows,

    CrystalDepths,

    SulphurWorks,

    BoneGalleries,

    MagmaLayer,
}

impl DepthZone {

    pub fn name(self) -> &'static str {
        match self {
            DepthZone::Shallows => "The Shallows",
            DepthZone::CrystalDepths => "The Crystal Depths",
            DepthZone::SulphurWorks => "The Sulphur Works",
            DepthZone::BoneGalleries => "The Bone Galleries",
            DepthZone::MagmaLayer => "The Magma Layer",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            DepthZone::Shallows => "ordinary rock and the odd seam of ore",
            DepthZone::CrystalDepths => "veins of crystal, and pockets of gem",
            DepthZone::SulphurWorks => "vents and seeps, and the gas they give off",
            DepthZone::BoneGalleries => "the old stone, and the old bones in it",
            DepthZone::MagmaLayer => "the rock is still hot enough to melt",
        }
    }

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

    pub const fn tint(self) -> [f32; 3] {
        match self {
            DepthZone::Shallows => [1.0, 1.0, 1.0],

            DepthZone::CrystalDepths => [0.88, 0.94, 1.10],

            DepthZone::SulphurWorks => [1.10, 1.02, 0.82],

            DepthZone::BoneGalleries => [1.04, 1.02, 0.96],

            DepthZone::MagmaLayer => [1.14, 0.94, 0.86],
        }
    }

    pub fn rock(self, rng: &mut crate::rng::Rng) -> Elem {
        match self {
            DepthZone::Shallows => Elem::Stone,
            DepthZone::CrystalDepths => {

                if rng.chance(0.07) {
                    Elem::Crystal
                } else {
                    Elem::Stone
                }
            }
            DepthZone::SulphurWorks => {

                Elem::Stone
            }
            DepthZone::BoneGalleries => {

                if rng.chance(0.02) {
                    Elem::Rust
                } else {
                    Elem::Granite
                }
            }
            DepthZone::MagmaLayer => {

                if rng.chance(0.05) {
                    Elem::Obsidian
                } else {
                    Elem::Granite
                }
            }
        }
    }

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

pub const DEPTH_TINT: [[f32; 3]; 32] = {
    let mut t = [[1.0f32, 1.0, 1.0]; 32];
    let mut i = 0;
    while i < 32 {
        t[i] = DepthZone::of((i as i32) * 8).tint();
        i += 1;
    }
    t
};
