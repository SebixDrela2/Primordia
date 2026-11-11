use super::*;

impl World {
    pub(crate) fn maintain(&mut self) {
        let frame = self.frame;
        let w = self.w as i32;

        // Keep the cached ground level fresh (the renderer reads it).
        if frame % 15 == 0 {
            self.update_ground();
        }

        // ---- Weather ----
        if self.weather_timer == 0 {
            self.weather = match self.rng.range(100) {
                0..=39 => Weather::Clear,
                40..=69 => Weather::Rain,
                70..=81 => Weather::Storm,
                82..=91 => Weather::Snow,
                _ => Weather::Meteor,
            };
            self.weather_timer = 500 + self.rng.range(700);
        }
        self.weather_timer = self.weather_timer.saturating_sub(1);
        if self.lightning_flash > 0 {
            self.lightning_flash -= 1;
        }

        match self.weather {
            Weather::Rain | Weather::Storm => {
                // Rain falls from the sky (stormy weather falls harder).
                let period = if self.weather == Weather::Storm { 15 } else { 30 };
                let want = if self.weather == Weather::Storm { 3 } else { 2 };
                if frame % period == 0 {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < want && attempts < 40 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let y = self.rng.range(4) as i32;
                        if self.get(x, y) == Elem::Empty {
                            let is_acid = self.weather == Weather::Storm && self.rng.chance(0.05);
                            let e = if is_acid { Elem::Acid } else { Elem::Water };
                            self.set_xy(x, y, e);
                            added += 1;
                        }
                    }
                }
                // Storms throw lightning bolts that scorch the ground.
                if self.weather == Weather::Storm && self.rng.chance(0.02) {
                    self.lightning_flash = 10;
                    let x = self.rng.range(w as u32) as i32;
                    let surf = self.surface_y(x);
                    for dy in 0..6 {
                        let y = surf - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Fire);
                        }
                    }
                }
            }
            Weather::Snow => {
                // A light dusting of snow on the surface.
                if frame % 60 == 0 {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < 1 && attempts < 20 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let y = self.rng.range(4) as i32;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Ice);
                            added += 1;
                        }
                    }
                }
            }
            Weather::Meteor => {
                // Meteors streak down and explode where they land.
                if self.rng.chance(0.005) {
                    let x = self.rng.range(w as u32) as i32;
                    let surf = self.surface_y(x);
                    let impact_y = surf + self.rng.range(4) as i32;
                    for dy in 0..10 {
                        let y = surf - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Fire);
                        }
                    }
                    // Impact crater + scorched earth.
                    for dx in -2..=2 {
                        for dy in -1..=1 {
                            let (cx, cy) = (x + dx, impact_y + dy);
                            if self.get(cx, cy) == Elem::Empty {
                                self.set_xy(cx, cy, Elem::Lava);
                            }
                        }
                    }
                    self.meteor_events.push((x as f32, impact_y as f32));
                }
            }
            Weather::Clear => {}
        }

        // Underground springs - water seeping up from aquifers (rare)
        if frame % 200 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let y = self.h as i32 - 2 - self.rng.range(20) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        // Surface water bodies maintenance - keep lakes from drying (very rare, minimal)
        if frame % 1000 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x);
                if self.get(x, sy - 1) == Elem::Empty && self.rng.chance(0.1) {
                    self.set_xy(x, sy - 1, Elem::Water);
                }
            }
        }

        // Vegetation regrowth. Slower and pressure-sensitive: plants establish
        // near water, so over-grazing causes real scarcity that then recovers.
        if frame % 15 == 0 && self.census.plants < 4000 {
            let batches = if self.census.plants < 1200 {
                24
            } else if self.census.plants < 3000 {
                12
            } else {
                6
            };
            for _ in 0..batches {
                let x = self.rng.range((w - 4) as u32) as i32 + 2;
                let sy = self.surface_y(x);
                let y = sy - 1 - self.rng.range(3) as i32;
                if self.get(x, y) == Elem::Empty {
                    // Slightly favour spots near moisture.
                    if !self.has_water_near(x, y, 5) && !self.rng.chance(0.5) {
                        continue;
                    }
                    let e = if self.rng.chance(0.12) {
                        Elem::Spore
                    } else if self.rng.chance(0.10) {
                        Elem::Fungus
                    } else {
                        Elem::Plant
                    };
                    self.set_xy(x, y, e);
                }
            }
        }

        // Hard cap: thin out vegetation if it starts smothering the world.
        // NOTE: compare decoded elements, never raw byte ranges.
        if frame % 20 == 0 && self.census.plants > 2000 {
            for _ in 0..60 {
                let i = self.rng.range((self.w * self.h) as u32) as usize;
                if matches!(
                    Elem::from_u8(self.kind[i]),
                    Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss
                ) {
                    self.set_at(i, Elem::Empty);
                }
            }
        }

        // Food drifting just above the ground
        if frame % 25 == 0 && self.census.energy < 150 {
            for _ in 0..4 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x).max(14);
                let y = sy - 1 - self.rng.range(12) as i32;
                if self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Energy);
                }
            }
        }

        // Resource spawns near surface
        if frame % 50 == 0 {
            // Oil seeps
            if self.census.oil < 200 {
                for _ in 0..3 {
                    let x = self.rng.range(w as u32) as i32;
                    let sy = self.surface_y(x);
                    let y = sy + 2 + self.rng.range(8) as i32;
                    if self.get(x, y) == Elem::Empty {
                        self.set_xy(x, y, Elem::Oil);
                    }
                }
            }
            // Sulfur vents near lava
            if self.census.sulfur < 150 {
                for _ in 0..2 {
                    let x = self.rng.range(w as u32) as i32;
                    let y = self.h as i32 - 5 - self.rng.range(10) as i32;
                    if self.get(x, y) == Elem::Empty && self.get(x, y + 1) == Elem::Lava {
                        self.set_xy(x, y, Elem::Sulfur);
                    }
                }
            }
        }

        // Meteor strike
        if frame % 1800 == 900 {
            let cx = self.rng.range_i(8, w - 8);
            for dy in -2..13 {
                for dx in -6..7 {
                    if dx * dx + dy * dy <= 42 {
                        self.set_xy(cx + dx, dy, Elem::Lava);
                    }
                }
            }
        }

        // Geological: slow stone formation from mineral deposition
        if frame % 100 == 0 {
            for _ in 0..8 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Stone);
                }
            }
        }

        // Slow ore genesis: deep rock occasionally turns into a fresh vein, so
        // mining is sustainable instead of a one-way drain on the world.
        if frame % 250 == 0 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let sy = self.surface_y(x);
                let y = sy + 8 + self.rng.range((self.h as i32 - sy - 10).max(1) as u32) as i32;
                if y < self.h as i32 - 1 && self.get(x, y) == Elem::Stone {
                    let ore = match self.rng.range(6) {
                        0 => Elem::Copper,
                        1 => Elem::Iron,
                        2 => Elem::Gold,
                        3 => Elem::Silver,
                        4 => Elem::Crystal,
                        _ => Elem::Tin,
                    };
                    self.set_xy(x, y, ore);
                }
            }
        }

        // Crystal growth in caves
        if frame % 200 == 0 && self.census.crystal < 300 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 2) {
                    self.set_xy(x, y, Elem::Crystal);
                }
            }
        }

        // Trim runaway particle counts (firestorms can fill the sky)
        if frame % 300 == 0 {
            let total = self.census.particles as usize;
            if total > (self.w * self.h * 85) / 100 {
                self.compact();
            }
        }
    }

    // Removes some airborne noise so the world cannot jam completely.
    pub(crate) fn compact(&mut self) {
        for _ in 0..(self.w * self.h / 20) {
            let x = self.rng.range(self.w as u32) as i32;
            let y = self.rng.range(self.h as u32) as i32;
            let e = self.get(x, y);
            if matches!(e, Elem::Smoke | Elem::Steam | Elem::Gas | Elem::Fire) {
                self.set_xy(x, y, Elem::Empty);
            }
        }
    }

}
