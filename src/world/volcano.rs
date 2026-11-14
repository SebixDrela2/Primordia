use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Volcano {

    Sleeping,

    Grumbling,

    Erupting,
}

impl World {

    pub fn generate_volcano(&mut self, w: i32, h: i32) {

        let vx = (w as f32 * 0.22) as i32;
        let vy = self.surface_y(vx);
        let radius = 16 + self.rng.range(8) as i32;

        for dy in 0..(radius / 2 + 2) {
            let r = radius - dy * 2;
            for dx in -r..=r {
                let x = vx + dx;
                let y = vy - dy;
                if x < 1 || x >= w - 1 || y < 1 {
                    continue;
                }
                if self.get(x, y) == Elem::Empty {
                    let e = if dy < radius / 3 {

                        Elem::Obsidian
                    } else if self.rng.chance(0.3) {
                        Elem::Ash
                    } else {
                        Elem::Stone
                    };
                    self.set_xy(x, y, e);
                }
            }
        }

        for dx in -4..=4 {
            for dy in -2..=2 {
                let x = vx + dx;
                let y = vy - radius / 2 + dy;
                if x > 0 && x < w - 1 && y > 0 && y < h - 1 {
                    let e = if dy >= 1 { Elem::Magma } else { Elem::Ash };
                    self.set_xy(x, y, e);
                }
            }
        }

        let cy = vy + 60 + self.rng.range(40) as i32;
        for dy in -8..=8 {
            for dx in -12..=12 {
                let x = vx + dx;
                let y = cy + dy;
                if x > 1 && x < w - 1 && y > 0 && y < h - 4 {
                    if (dx * dx) as f32 / 100.0 + (dy * dy) as f32 / 64.0 <= 1.0 {
                        self.set_xy(x, y, Elem::Magma);
                    }
                }
            }
        }

        let mut y = cy - 8;
        while y > vy - radius / 2 {
            let x = vx + self.rng.range_i(-2, 3);
            if x > 1 && x < w - 1 {
                self.set_xy(x, y, Elem::Magma);
            }
            y -= 1;
        }

        self.volcano = Some((vx, vy - radius / 2));
        self.volcano_state = Volcano::Sleeping;
        self.volcano_timer = 0;
    }

    pub fn volcano_state(&self) -> Volcano {
        self.volcano_state
    }

    pub fn volcano(&self) -> Option<(i32, i32)> {
        self.volcano
    }

    pub fn erupt(&mut self) {
        let Some((vx, vy)) = self.volcano else {
            return;
        };
        self.volcano_state = Volcano::Erupting;
        self.volcano_timer = 180;

        for _ in 0..120 {
            let dx = self.rng.range_i(-5, 6);
            let dy = -(self.rng.range(16) as i32) - 6;
            let x = vx + dx;
            let y = vy + dy;
            if x > 0 && x < self.w as i32 && y > 0 {
                if self.get(x, y) == Elem::Empty {
                    let e = if self.rng.chance(0.25) { Elem::Fire } else { Elem::Lava };
                    self.set_xy(x, y, e);
                }
            }
        }

        for dx in -10..=10 {
            for dy in -4..=3 {
                let x = vx + dx;
                let y = vy + dy;
                if x <= 0 || x >= self.w as i32 || y <= 0 {
                    continue;
                }
                let e = self.get(x, y);
                if e == Elem::Stone || e == Elem::Ash {
                    if self.rng.chance(0.45) {
                        self.set_xy(x, y, Elem::Lava);
                    }
                } else if e == Elem::Empty && self.rng.chance(0.2) {
                    self.set_xy(x, y, Elem::Fire);
                }
            }
        }

        for _ in 0..40 {
            let x = vx + self.rng.range_i(-4, 5);
            let y = vy - (self.rng.range(12) as i32) - 2;
            if x > 0 && x < self.w as i32 && y > 0 && self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Smoke);
            }
        }

        for _ in 0..90 {
            let x = self.rng.range(self.w as u32) as i32;
            let y = self.rng.range(6) as i32;
            if self.get(x, y) == Elem::Empty {
                let e = if self.rng.chance(0.5) { Elem::Ash } else { Elem::Smoke };
                self.set_xy(x, y, e);
            }
        }
    }

    pub fn tick_volcano(&mut self) {
        match self.volcano_state {
            Volcano::Erupting => {
                if self.volcano_timer > 0 {
                    self.volcano_timer -= 1;

                    if self.rng.chance(0.35) {
                        self.erupt();
                    }
                } else {
                    self.volcano_state = Volcano::Grumbling;
                    self.volcano_timer = 900 + self.rng.range(900);
                }
            }
            Volcano::Grumbling => {
                if self.volcano_timer > 0 {
                    self.volcano_timer -= 1;
                } else {
                    self.volcano_state = Volcano::Sleeping;
                    self.volcano_timer = 1800 + self.rng.range(1800);
                }
            }
            Volcano::Sleeping => {
                if self.volcano_timer > 0 {
                    self.volcano_timer -= 1;
                }
            }
        }
    }

    pub fn volcano_risk(&self) -> f32 {
        match self.volcano_state {
            Volcano::Erupting => 1.0,
            Volcano::Grumbling => 0.6,
            Volcano::Sleeping => self.pressure.volcanic.min(1.0),
        }
    }
}
