//! The volcano: the one place in the world that fights back.
//!
//! A cone of ash and rock with a lava chamber under it and a shaft that
//! connects the two. When the pressure under the mountain builds up, it
//! erupts: lava fountains out of the crater, the rim melts, ash fills the
//! sky and the lava runs down the mountain and into the caves. Colonies that
//! settled too close learn what a volcano is.

use super::*;

/// How the mountain is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Volcano {
    /// Cold: nothing but rock and old ash.
    Sleeping,
    /// Grumbling: the crater glows and the ground trembles.
    Grumbling,
    /// Erupting: lava, fire and ash.
    Erupting,
}

impl World {
    /// Build the volcano. It sits on the surface with a chamber of magma
    /// underneath, joined by a shaft, so an eruption reaches the sky.
    pub(crate) fn generate_volcano(&mut self, w: i32, h: i32) {
        // Put it a quarter of the way in, away from the middle of the map
        // where most colonies live.
        let vx = (w as f32 * 0.22) as i32;
        let vy = self.surface_y(vx);
        let radius = 16 + self.rng.range(8) as i32;

        // The cone: a wide mound of dark volcanic rock and ash.
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
                        // The top of the mountain is bare basalt.
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

        // The crater: a hollow bowl with magma glowing at the bottom.
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

        // The chamber: a pocket of magma deep under the mountain.
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

        // The shaft: a narrow chimney from the chamber to the crater.
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

    /// How long until the mountain erupts again, and what it is doing now.
    pub fn volcano_state(&self) -> Volcano {
        self.volcano_state
    }

    /// The crater of the volcano, if the world has one.
    pub fn volcano(&self) -> Option<(i32, i32)> {
        self.volcano
    }

    /// Erupt. Lava fountains out of the crater, the rim melts and the air
    /// fills with ash.
    pub(crate) fn erupt(&mut self) {
        let Some((vx, vy)) = self.volcano else {
            return;
        };
        self.volcano_state = Volcano::Erupting;
        self.volcano_timer = 180;

        // The fountain: lava thrown up out of the crater, which then falls
        // back down the mountain.
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

        // The rim melts and the ground around the crater catches fire.
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

        // A smoke column rises out of the crater.
        for _ in 0..40 {
            let x = vx + self.rng.range_i(-4, 5);
            let y = vy - (self.rng.range(12) as i32) - 2;
            if x > 0 && x < self.w as i32 && y > 0 && self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Smoke);
            }
        }

        // The sky fills with ash and smoke.
        for _ in 0..90 {
            let x = self.rng.range(self.w as u32) as i32;
            let y = self.rng.range(6) as i32;
            if self.get(x, y) == Elem::Empty {
                let e = if self.rng.chance(0.5) { Elem::Ash } else { Elem::Smoke };
                self.set_xy(x, y, e);
            }
        }
    }

    /// Keep the eruption going for a while and let the mountain cool down
    /// afterwards.
    pub(crate) fn tick_volcano(&mut self) {
        match self.volcano_state {
            Volcano::Erupting => {
                if self.volcano_timer > 0 {
                    self.volcano_timer -= 1;
                    // Still spitting while the eruption runs.
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

    /// How close the volcano is to blowing. The deeper the pressure, the
    /// sooner it goes off.
    pub fn volcano_risk(&self) -> f32 {
        match self.volcano_state {
            Volcano::Erupting => 1.0,
            Volcano::Grumbling => 0.6,
            Volcano::Sleeping => self.pressure.volcanic.min(1.0),
        }
    }
}
