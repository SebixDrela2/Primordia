//! Landmarks: the places worth living in.
//!
//! The world is not uniform. A crystal grove hums with mana, a hot spring
//! keeps the cold out, a bone field is a good place to hunt and an old ruin
//! still holds the memory of the people who built it. A colony that settles
//! beside one of them does better than a colony that does not, and the
//! colonies that settle in the same place end up fighting over it.

use super::*;

/// The kind of place a landmark is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Landmark {
    /// A grove of crystal: mana flows through the ground here.
    CrystalGrove,
    /// A hot spring: the cold does not reach here.
    HotSpring,
    /// A field of bones: a good hunting ground.
    BoneField,
    /// Rich soil: food grows here.
    FertileSoil,
    /// An old ruin: the knowledge of the people who lived here.
    OldRuin,
}

impl Landmark {
    pub fn name(self) -> &'static str {
        match self {
            Landmark::CrystalGrove => "Crystal Grove",
            Landmark::HotSpring => "Hot Spring",
            Landmark::BoneField => "Bone Field",
            Landmark::FertileSoil => "Fertile Soil",
            Landmark::OldRuin => "Old Ruin",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Landmark::CrystalGrove => "mana flows here",
            Landmark::HotSpring => "the cold does not reach here",
            Landmark::BoneField => "good hunting, and old bones",
            Landmark::FertileSoil => "food grows here",
            Landmark::OldRuin => "the old people knew things",
        }
    }

    /// How much of the world the landmarks take up.
    pub fn radius(self) -> f32 {
        match self {
            Landmark::CrystalGrove => 26.0,
            Landmark::HotSpring => 22.0,
            Landmark::BoneField => 30.0,
            Landmark::FertileSoil => 34.0,
            Landmark::OldRuin => 26.0,
        }
    }
}

/// A place in the world with a name.
#[derive(Clone, Copy)]
pub struct LandmarkSite {
    pub kind: Landmark,
    pub x: f32,
    pub y: f32,
}

impl LandmarkSite {
    /// How far a point is from the heart of this landmark.
    pub fn distance_to(&self, x: f32, y: f32) -> f32 {
        ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt()
    }

    /// Is a point inside the landmark?
    pub fn covers(&self, x: f32, y: f32) -> bool {
        self.distance_to(x, y) <= self.kind.radius()
    }
}

impl World {
    /// Scatter the landmarks across the world.
    ///
    /// They are placed on the surface, spread out, so that no two of them
    /// fight over the same patch of ground.
    pub(crate) fn generate_landmarks(&mut self, w: i32, _h: i32) {
        let kinds = [
            Landmark::CrystalGrove,
            Landmark::HotSpring,
            Landmark::BoneField,
            Landmark::FertileSoil,
            Landmark::OldRuin,
        ];
        // How many the world gets: enough to divide the map between the
        // species, few enough that they stay worth fighting over.
        let count = 5 + self.rng.range(4);
        let mut placed: Vec<(f32, f32)> = Vec::new();
        for i in 0..count {
            let kind = kinds[(i as usize) % kinds.len()];
            // Find a spot on the surface that is far enough from the others.
            let mut spot: Option<(i32, i32)> = None;
            for _ in 0..60 {
                let x = self.rng.range(w as u32) as i32;
                if x < 40 || x > w - 40 {
                    continue;
                }
                let y = self.surface_y(x) - 2;
                let ok = placed.iter().all(|(px, py)| {
                    let d = ((px - x as f32).powi(2) + (py - y as f32).powi(2)).sqrt();
                    d > 180.0
                });
                if ok {
                    spot = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = spot else {
                continue;
            };
            placed.push((x as f32, y as f32));
            self.landmarks.push(LandmarkSite {
                kind,
                x: x as f32,
                y: y as f32,
            });
            self.paint_landmark(kind, x, y);
        }
    }

    /// Paint the cells of a landmark into the world.
    fn paint_landmark(&mut self, kind: Landmark, x: i32, y: i32) {
        let r = kind.radius() as i32;
        match kind {
            Landmark::CrystalGrove => {
                // A cluster of crystal spires.
                for _ in 0..7 {
                    let cx = x + self.rng.range_i(-r / 2, r / 2 + 1);
                    let cy = y + self.rng.range_i(-2, 3);
                    let height = 2 + self.rng.range(4) as i32;
                    for dy in 0..height {
                        if self.get(cx, cy + dy) == Elem::Empty
                            || self.get(cx, cy + dy).is_solid()
                        {
                            self.set_xy(cx, cy + dy, Elem::Crystal);
                        }
                    }
                }
            }
            Landmark::HotSpring => {
                // A pool of warm water with a ring of stone around it.
                for dy in -4..=4 {
                    for dx in -6..=6 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty {
                            self.set_xy(px, py, Elem::Water);
                        }
                    }
                }
                for a in 0..16 {
                    let ang = (a as f32 / 16.0) * std::f32::consts::TAU;
                    let px = x + (ang.cos() * 7.0) as i32;
                    let py = y + (ang.sin() * 4.0) as i32;
                    if self.get(px, py) == Elem::Empty {
                        self.set_xy(px, py, Elem::Stone);
                    }
                }
            }
            Landmark::BoneField => {
                // Old bones scattered on the ground.
                for _ in 0..26 {
                    let px = x + self.rng.range_i(-r, r + 1);
                    let py = y + self.rng.range_i(-2, 4);
                    if self.get(px, py) == Elem::Empty {
                        let e = if self.rng.chance(0.6) {
                            Elem::Bone
                        } else {
                            Elem::Corpse
                        };
                        self.set_xy(px, py, e);
                    }
                }
            }
            Landmark::FertileSoil => {
                // Rich ground: plants and crops, and soil that feeds them.
                for dy in -2..=2 {
                    for dx in -r..=r {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty {
                            let e = if self.rng.chance(0.55) {
                                Elem::Plant
                            } else {
                                Elem::Fertilizer
                            };
                            self.set_xy(px, py, e);
                        }
                    }
                }
            }
            Landmark::OldRuin => {
                // The remains of a settlement, with a totem at its heart.
                for dy in -3..=3 {
                    for dx in -7..=7 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.45) {
                            let e = match self.rng.range(4) {
                                0 => Elem::Brick,
                                1 => Elem::Concrete,
                                2 => Elem::Road,
                                _ => Elem::Stone,
                            };
                            self.set_xy(px, py, e);
                        }
                    }
                }
                if self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Totem);
                }
            }
        }
    }

    /// The landmark a point is standing in, if any.
    pub fn landmark_at(&self, x: f32, y: f32) -> Option<Landmark> {
        self.landmarks
            .iter()
            .find(|l| l.covers(x, y))
            .map(|l| l.kind)
    }

    /// How much a point is worth because of the landmark it is in.
    ///
    /// This is what a colony gets from settling in the right place.
    pub fn landmark_bonus(&self, x: f32, y: f32) -> LandmarkBonus {
        let mut bonus = LandmarkBonus::default();
        for l in self.landmarks.iter() {
            if !l.covers(x, y) {
                continue;
            }
            match l.kind {
                Landmark::CrystalGrove => bonus.mana = 2.0,
                Landmark::HotSpring => bonus.warmth = true,
                Landmark::BoneField => bonus.hunting = true,
                Landmark::FertileSoil => bonus.food = true,
                Landmark::OldRuin => bonus.craft = true,
            }
        }
        bonus
    }
}

/// What a piece of ground gives to the creature standing on it.
#[derive(Clone, Copy, Default)]
pub struct LandmarkBonus {
    /// A multiplier on the mana that comes back.
    pub mana: f32,
    /// The cold does not bite here.
    pub warmth: bool,
    /// Hunting is easier here.
    pub hunting: bool,
    /// Food is plentiful here.
    pub food: bool,
    /// The old knowledge helps here.
    pub craft: bool,
}
