use super::{World, Elem, DIRS8};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Landmark {

    CrystalGrove,

    HotSpring,

    BoneField,

    FertileSoil,

    OldRuin,

    FungalWarren,

    SaltFlat,

    SunkenTemple,

    AmberField,

    WhisperingStone,

    IronWomb,

    Stormspire,

    Undergrove,
}

impl Landmark {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Landmark::CrystalGrove => "Crystal Grove",
            Landmark::HotSpring => "Hot Spring",
            Landmark::BoneField => "Bone Field",
            Landmark::FertileSoil => "Fertile Soil",
            Landmark::OldRuin => "Old Ruin",
            Landmark::FungalWarren => "Fungal Warren",
            Landmark::SaltFlat => "Salt Flat",
            Landmark::SunkenTemple => "Sunken Temple",
            Landmark::AmberField => "Amber Field",
            Landmark::WhisperingStone => "Whispering Stone",
            Landmark::IronWomb => "Iron Womb",
            Landmark::Stormspire => "Stormspire",
            Landmark::Undergrove => "Undergrove",
        }
    }

    #[must_use]
    pub fn blurb(self) -> &'static str {
        match self {
            Landmark::CrystalGrove => "mana flows here",
            Landmark::HotSpring => "the cold does not reach here",
            Landmark::BoneField => "good hunting, and old bones",
            Landmark::FertileSoil => "food grows here",
            Landmark::OldRuin => "the old people knew things",
            Landmark::FungalWarren => "sickness breeds here",
            Landmark::SaltFlat => "the deep's filth cannot take hold",
            Landmark::SunkenTemple => "the old learning lies here",
            Landmark::AmberField => "the stone of the deep, hardened",
            Landmark::WhisperingStone => "the mind is sharper here",
            Landmark::IronWomb => "the rock gives up its metal",
            Landmark::Stormspire => "the sky comes down here",
            Landmark::Undergrove => "the deep has air here",
        }
    }

    #[must_use]
    pub fn radius(self) -> f32 {
        match self {
            Landmark::CrystalGrove | Landmark::OldRuin => 26.0,
            Landmark::HotSpring | Landmark::Undergrove => 22.0,
            Landmark::BoneField => 30.0,
            Landmark::FertileSoil => 34.0,
            Landmark::FungalWarren | Landmark::IronWomb => 28.0,
            Landmark::SaltFlat => 32.0,
            Landmark::SunkenTemple | Landmark::WhisperingStone => 24.0,
            Landmark::AmberField => 27.0,
            Landmark::Stormspire => 25.0,
        }
    }

    #[must_use]
    pub fn all() -> [Landmark; 13] {
        [
            Landmark::CrystalGrove,
            Landmark::HotSpring,
            Landmark::BoneField,
            Landmark::FertileSoil,
            Landmark::OldRuin,
            Landmark::FungalWarren,
            Landmark::SaltFlat,
            Landmark::SunkenTemple,
            Landmark::AmberField,
            Landmark::WhisperingStone,
            Landmark::IronWomb,
            Landmark::Stormspire,
            Landmark::Undergrove,
        ]
    }
}

#[derive(Clone, Copy)]
pub struct LandmarkSite {
    pub kind: Landmark,
    pub x: f32,
    pub y: f32,
}

impl LandmarkSite {

    #[must_use]
    pub fn distance_to(&self, x: f32, y: f32) -> f32 {
        ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt()
    }

    #[must_use]
    pub fn covers(&self, x: f32, y: f32) -> bool {
        self.distance_to(x, y) <= self.kind.radius()
    }
}

impl World {

    pub fn generate_landmarks(&mut self, w: i32, _h: i32) {
        let kinds = Landmark::all();

        let count = 8 + self.rng.range(5);
        let mut placed: Vec<(f32, f32)> = Vec::new();
        for _ in 0..count {

            let kind = loop {
                let k = kinds[self.rng.range(kinds.len() as u32) as usize];
                if k != Landmark::Undergrove {
                    break k;
                }
            };

            let mut spot: Option<(i32, i32)> = None;
            for _ in 0..60 {
                let x = self.rng.range(w as u32) as i32;
                if x < 40 || x > w - 40 {
                    continue;
                }
                let y = self.surface_y(x) - 2;
                let ok = placed.iter().all(|(px, py)| {
                    let d = ((px - x as f32).powi(2) + (py - y as f32).powi(2)).sqrt();
                    d > 125.0
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

        self.generate_undergroves(w);
    }

    pub fn generate_undergroves(&mut self, w: i32) {
        let count = 2 + self.rng.range(3);
        for _ in 0..count {

            let mut spot: Option<(i32, i32)> = None;
            for _ in 0..400 {
                let x = self.rng.range(w as u32) as i32;
                let surface = self.surface_y(x);
                let y = surface + 25 + self.rng.range((self.h as i32 / 3) as u32) as i32;
                if y >= self.h as i32 - 12 {
                    continue;
                }

                if self.get(x, y) != Elem::Empty {
                    continue;
                }
                let mut open = 0;
                for (dx, dy) in DIRS8 {
                    if self.get(x + dx, y + dy) == Elem::Empty {
                        open += 1;
                    }
                }
                if open < 5 {
                    continue;
                }
                if self.get(x, y + 1) == Elem::Empty {
                    continue;
                }
                spot = Some((x, y));
                break;
            }
            let Some((x, y)) = spot else {
                continue;
            };
            let radius = Landmark::Undergrove.radius() as i32;

            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let d2 = dx * dx + dy * dy;
                    if d2 > radius * radius {
                        continue;
                    }
                    let px = x + dx;
                    let py = y + dy;
                    if px < 1 || px >= w - 1 || py < 1 || py >= self.h as i32 - 2 {
                        continue;
                    }
                    let e = self.get(px, py);
                    if e != Elem::Empty && e != Elem::Stone {
                        continue;
                    }
                    let roll = self.rng.f32();
                    if d2 < radius * radius / 3 {

                        if e == Elem::Empty && roll < 0.5 {
                            self.set_xy(px, py, Elem::Oxygen);
                        }
                    } else if roll < 0.3 {
                        self.set_xy(px, py, Elem::Fungus);
                    } else if roll < 0.38 && e == Elem::Empty {
                        self.set_xy(px, py, Elem::Spore);
                    }
                }
            }
            self.landmarks.push(LandmarkSite {
                kind: Landmark::Undergrove,
                x: x as f32,
                y: y as f32,
            });
        }
    }

    fn paint_ground(&mut self, px: i32, py: i32, _e: Elem) {
        if py < self.surface_y(px) {
            return;
        }
        if self.get(px, py) != Elem::Empty {
            return;
        }
        self.paint_ground(px, py, _e);
    }

    fn paint_landmark(&mut self, kind: Landmark, x: i32, y: i32) {
        let r = kind.radius() as i32;
        match kind {
            Landmark::CrystalGrove => {

                for _ in 0..7 {
                    let cx = x + self.rng.range_i(-r / 2, r / 2 + 1);
                    let surf = self.surface_y(cx);
                    let height = 2 + self.rng.range(4) as i32;
                    for dy in 0..height {
                        let py = surf - dy;
                        if self.get(cx, py) == Elem::Empty
                            || self.get(cx, py).is_solid()
                        {
                            self.set_xy(cx, py, Elem::Crystal);
                        }
                    }
                }
            }
            Landmark::HotSpring => {

                for dx in -6..=6 {
                    let px = x + dx;
                    let surf = self.surface_y(px);
                    for dy in 0..4 {
                        self.paint_ground(px, surf + dy, Elem::Water);
                    }
                }
                for a in 0..16 {
                    let ang = (a as f32 / 16.0) * std::f32::consts::TAU;
                    let px = x + (ang.cos() * 7.0) as i32;
                    let surf = self.surface_y(px);
                    self.paint_ground(px, surf + (ang.sin() * 2.0) as i32, Elem::Stone);
                }
            }
            Landmark::BoneField => {

                for _ in 0..26 {
                    let px = x + self.rng.range_i(-r, r + 1);
                    let py = y + self.rng.range_i(-2, 4);
                    if self.get(px, py) == Elem::Empty {
                        let e = if self.rng.chance(0.6) {
                            Elem::Bone
                        } else {
                            Elem::Corpse
                        };
                        self.paint_ground(px, py, e);
                    }
                }
            }
            Landmark::FertileSoil => {

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
                            self.paint_ground(px, py, e);
                        }
                    }
                }
            }
            Landmark::OldRuin => {

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
                            self.paint_ground(px, py, e);
                        }
                    }
                }
                let sx = self.surface_y(x);
                if self.get(x, sx) == Elem::Empty {
                    self.set_xy(x, sx, Elem::Totem);
                }
            }
            Landmark::FungalWarren => {

                for dy in -3..=2 {
                    for dx in -9..=9 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.4) {
                            let e = match self.rng.range(4) {
                                0 => Elem::Fungus,
                                1 => Elem::Spore,
                                2 => Elem::Moss,
                                _ => Elem::Fungus,
                            };
                            self.paint_ground(px, py, e);
                        }
                    }
                }
                let sx = self.surface_y(x);
                if self.get(x, sx) == Elem::Empty {
                    self.set_xy(x, sx, Elem::Fungus);
                }
            }
            Landmark::SaltFlat => {

                for dy in -1..=2 {
                    for dx in -13..=13 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty {
                            self.paint_ground(px, py, Elem::Salt);
                        }
                    }
                }
            }
            Landmark::SunkenTemple => {

                for dy in -2..=3 {
                    for dx in -8..=8 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.35) {
                            let e = match self.rng.range(4) {
                                0 => Elem::Brick,
                                1 => Elem::Concrete,
                                2 => Elem::Water,
                                _ => Elem::Rune,
                            };
                            self.paint_ground(px, py, e);
                        }
                    }
                }
                let sx = self.surface_y(x);
                if self.get(x, sx) == Elem::Empty {
                    self.set_xy(x, sx, Elem::Totem);
                }
            }
            Landmark::Undergrove => {

            }
            Landmark::AmberField => {

                for _ in 0..40 {
                    let px = x + self.rng.range_i(-r, r + 1);
                    let py = y + self.rng.range_i(-2, 4);
                    if self.get(px, py) == Elem::Empty {
                        let e = match self.rng.range(8) {
                            0 => Elem::Gold,
                            1 => Elem::Obsidian,
                            2 => Elem::Crystal,
                            _ => Elem::Stone,
                        };
                        self.paint_ground(px, py, e);
                    }
                }
            }
            Landmark::WhisperingStone => {

                for dy in -4..=1 {
                    for dx in -4..=4 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.3) {
                            self.paint_ground(px, py, Elem::Rune);
                        }
                    }
                }
                let sx = self.surface_y(x);
                for dy in 0..4 {
                    if self.get(x, sx - dy) == Elem::Empty || self.get(x, sx - dy).is_solid() {
                        self.set_xy(x, sx - dy, Elem::Crystal);
                    }
                }
            }
            Landmark::IronWomb => {

                for dy in -3..=3 {
                    for dx in -9..=9 {
                        let px = x + dx;
                        let py = y + dy;
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.5) {
                            let e = match self.rng.range(6) {
                                0 => Elem::Iron,
                                1 => Elem::Copper,
                                2 => Elem::Steel,
                                3 => Elem::Gold,
                                _ => Elem::Stone,
                            };
                            self.paint_ground(px, py, e);
                        }
                    }
                }
            }
            Landmark::Stormspire => {

                for dy in -6..=1 {
                    let px = x;
                    let py = y + dy;
                    if self.get(px, py) == Elem::Empty || self.get(px, py).is_solid() {
                        self.set_xy(px, py, if dy % 2 == 0 { Elem::Glass } else { Elem::Obsidian });
                    }
                }
                for _ in 0..34 {
                    let px = x + self.rng.range_i(-r, r + 1);
                    let py = y + self.rng.range_i(-1, 4);
                    if self.get(px, py) == Elem::Empty {
                        let e = match self.rng.range(5) {
                            0 => Elem::Energy,
                            1 => Elem::Ash,
                            2 => Elem::Obsidian,
                            _ => Elem::Stone,
                        };
                        self.paint_ground(px, py, e);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn landmark_at(&self, x: f32, y: f32) -> Option<Landmark> {
        self.landmarks
            .iter()
            .find(|l| l.covers(x, y))
            .map(|l| l.kind)
    }

    #[must_use]
    pub fn landmark_bonus(&self, x: f32, y: f32) -> LandmarkBonus {
        let mut bonus = LandmarkBonus::default();
        for l in &self.landmarks {
            if !l.covers(x, y) {
                continue;
            }
            match l.kind {
                Landmark::CrystalGrove => bonus.mana = 2.0,
                Landmark::HotSpring => bonus.warmth = true,
                Landmark::BoneField => bonus.hunting = true,
                Landmark::FertileSoil => bonus.food = true,
                Landmark::OldRuin => bonus.craft = true,
                Landmark::FungalWarren => bonus.plague = true,
                Landmark::SaltFlat => bonus.purity = true,
                Landmark::SunkenTemple => bonus.lore = true,
                Landmark::AmberField => bonus.stone = true,
                Landmark::WhisperingStone => bonus.mind = true,
                Landmark::IronWomb => bonus.metal = true,
                Landmark::Stormspire => bonus.energy = true,
                Landmark::Undergrove => bonus.air = true,
            }
        }
        bonus
    }
}

#[derive(Clone, Copy, Default)]
pub struct LandmarkBonus {

    pub mana: f32,

    pub warmth: bool,

    pub hunting: bool,

    pub food: bool,

    pub craft: bool,

    pub plague: bool,

    pub purity: bool,

    pub lore: bool,

    pub stone: bool,

    pub mind: bool,

    pub metal: bool,

    pub energy: bool,

    pub air: bool,
}
