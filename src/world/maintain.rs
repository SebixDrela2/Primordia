use super::{World, Elem, Volcano, Weather};

impl World {
    pub fn maintain(&mut self) {
        let frame = self.frame;
        let w = self.w as i32;

        self.update_pressure();

        if frame.is_multiple_of(600) && frame > 0 && self.rng.chance(0.05 + self.pressure.extremes * 0.06) {
            let ex = self.rng.range(w as u32) as i32;
            let ey = self.surface_y(ex) + 20 + self.rng.range(40) as i32;
            let radius = 40 + self.rng.range(30) as i32;
            self.tremor(ex, ey, radius);
        }

        if frame.is_multiple_of(120) {
            match self.season {

                1 => {
                    for _ in 0..6 {
                        let x = self.rng.range(w as u32) as i32;
                        let sy = self.surface_y(x);
                        for y in (sy - 6..sy).rev() {
                            if self.get(x, y) == Elem::Water {
                                self.set_xy(x, y, Elem::Empty);
                                break;
                            }
                        }
                    }
                }

                3 => {
                    for _ in 0..4 {
                        let x = self.rng.range(w as u32) as i32;
                        let sy = self.surface_y(x);
                        for y in (sy - 4..sy).rev() {
                            if self.get(x, y) == Elem::Water {
                                self.set_xy(x, y, Elem::Ice);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.tick_volcano();
        if self.volcano_state == Volcano::Grumbling
            && self.rng.chance(0.004 + self.pressure.volcanic * 0.02)
        {
            self.erupt();
        }

        if frame.is_multiple_of(15) {
            self.update_ground();
        }

        if self.weather_timer == 0 {
            let harsh = self.pressure.harshness;
            let roll = self.rng.range(100) as f32;

            let summer = self.season == 1;
            let winter = self.season == 3;
            self.weather = if roll < 34.0 - harsh * 16.0 {
                Weather::Clear
            } else if roll < 58.0 - harsh * 6.0 {
                Weather::Rain
            } else if roll < 68.0 + harsh * 10.0 {
                Weather::Storm
            } else if roll < 76.0 + harsh * 4.0 {
                Weather::Snow
            } else if roll < 84.0 {
                if winter {
                    Weather::Snow
                } else if summer {
                    Weather::Heatwave
                } else {
                    Weather::Fog
                }
            } else if roll < 92.0 {
                if summer {
                    Weather::Heatwave
                } else {
                    Weather::Fog
                }
            } else if roll < 96.0 {

                if summer {
                    Weather::Sandstorm
                } else {
                    Weather::Fog
                }
            } else {
                Weather::Meteor
            };
            self.weather_timer = 500 + self.rng.range(700);
        }
        self.weather_timer = self.weather_timer.saturating_sub(1);

        match self.weather {
            Weather::Rain | Weather::Storm => {

                let period = if self.weather == Weather::Storm { 15 } else { 30 };
                let want = if self.weather == Weather::Storm { 3 } else { 2 };
                if frame.is_multiple_of(period) {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < want && attempts < 40 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(6) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            let is_acid = self.weather == Weather::Storm && self.rng.chance(0.05);
                            let e = if is_acid { Elem::Acid } else { Elem::Water };
                            self.set_xy(x, y, e);
                            added += 1;
                        }
                    }
                }

                if self.weather == Weather::Storm && self.rng.chance(0.02) {
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

                if frame.is_multiple_of(60) {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < 1 && attempts < 20 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(4) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Ice);
                            added += 1;
                        }
                    }
                }
            }
            Weather::Fog => {

                if frame.is_multiple_of(90) {
                    for _ in 0..3 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf + self.rng.range(3) as i32;
                        if y > 1 && y < self.h as i32 - 6 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Smoke);
                        }
                    }
                }
            }
            Weather::Heatwave => {

                if frame.is_multiple_of(70) {
                    for _ in 0..2 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(3) as i32;
                        if y > 1 && self.get(x, y) == Elem::Water {
                            self.set_xy(x, y, Elem::Steam);
                        }
                    }
                }
            }
            Weather::Meteor => {

                if self.rng.chance(0.005 + self.pressure.harshness * 0.02) {
                    let x = self.rng.range(w as u32) as i32;
                    let surf = self.surface_y(x);
                    let impact_y = surf + self.rng.range(4) as i32;
                    for dy in 0..10 {
                        let y = surf - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Fire);
                        }
                    }

                    for dx in -2..=2 {
                        for dy in -1..=1 {
                            let (cx, cy) = (x + dx, impact_y + dy);
                            if self.get(cx, cy) == Elem::Empty {
                                self.set_xy(cx, cy, Elem::Lava);
                            }
                        }
                    }
                    self.meteor_events.push((x as f32, impact_y as f32, 22));
                }
            }
            Weather::Sandstorm => {

                if frame.is_multiple_of(24) {
                    for _ in 0..4 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(5) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Dust);
                        }
                    }
                }
                if frame.is_multiple_of(45) {
                    for _ in 0..2 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(2) as i32;
                        if y > 1 && self.get(x, y) == Elem::Water {
                            self.set_xy(x, y, Elem::Steam);
                        }
                    }
                }
            }
            Weather::Clear => {}
        }

        if frame.is_multiple_of(200) {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let y = self.h as i32 - 2 - self.rng.range(20) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        if frame.is_multiple_of(1000) {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x);

                if self.get(x, sy) == Elem::Stone && self.rng.chance(0.1) {
                    self.set_xy(x, sy, Elem::Water);
                }
            }
        }

        if frame.is_multiple_of(15) && self.census.plants < 4000 {
            let scarcity = self.pressure.scarcity;
            let base = if self.census.plants < 1200 {
                32
            } else if self.census.plants < 3000 {
                18
            } else {
                9
            };

            let season_growth = match self.season {
                0 => 1.8,
                1 => 1.0,
                2 => 0.9,
                _ => 0.3,
            };

            let lean = 1.0 - 0.75 * self.difficulty;
            let batches = ((base as f32)
                * (1.0 - scarcity * 0.75)
                * season_growth
                * lean.max(0.05))
                as u32;
            for _ in 0..batches {
                let x = self.rng.range((w - 4) as u32) as i32 + 2;
                let sy = self.surface_y(x);

                let y = sy - 1;
                if self.get(x, y) != Elem::Empty {
                    continue;
                }

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

                if self.rng.chance(0.3) && self.get(x, y - 1) == Elem::Empty {
                    self.set_xy(x, y - 1, e);
                }
            }
        }

        if frame.is_multiple_of(20) && self.census.plants > 3000 {

            let over = (self.census.plants - 3000) as f32 / 1000.0;
            let n = (60.0 + over * 240.0) as u32;
            for _ in 0..n {
                let i = self.rng.range((self.w * self.h) as u32) as usize;
                if matches!(
                    Elem::from_u8(self.kind[i]),
                    Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss
                ) {
                    self.set_at(i, Elem::Empty);
                }
            }
        }

        if frame.is_multiple_of(25) && self.census.energy < 150 {
            for _ in 0..4 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x).max(14);

                let y = sy - 1 - self.rng.range(4) as i32;
                if y > 0 && self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Energy);
                }
            }
        }

        if frame.is_multiple_of(50) {

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

        if frame % 1800 == 900 {
            let cx = self.rng.range_i(8, w - 8);
            let surf = self.surface_y(cx);
            for dy in 0..6 {
                for dx in -6..7 {
                    if dx * dx + dy * dy <= 30 {
                        let x = cx + dx;
                        let y = surf + dy;

                        if self.get(x, y).is_solid() {
                            self.set_xy(x, y, Elem::Lava);
                        }
                    }
                }
            }

            for dy in 1..10 {
                let y = surf - dy;
                if self.get(cx, y) == Elem::Empty {
                    self.set_xy(cx, y, Elem::Fire);
                }
            }
        }

        if frame.is_multiple_of(100) {
            for _ in 0..8 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Stone);
                }
            }
        }

        if frame.is_multiple_of(10) {
            self.regrow_zorg_larder();
        }

        if frame.is_multiple_of(250) {
            let depletion = self.pressure.depletion;

            let attempts = ((3.0 * (1.0 - depletion)) as u32).max(1);
            for _ in 0..attempts {
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

        if frame.is_multiple_of(200) && self.census.crystal < 300 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 2) {
                    self.set_xy(x, y, Elem::Crystal);
                }
            }
        }

        if frame.is_multiple_of(600) && frame > 0 {
            let volcanic = self.pressure.volcanic;
            if volcanic > 0.05 && self.rng.chance(volcanic * 0.5) {
                let x = self.rng.range(w as u32) as i32;

                let mut y = self.h as i32 - 8;
                while y > self.surface_y(x) + 20 {
                    if self.get(x, y) == Elem::Stone {
                        self.set_xy(x, y, Elem::Magma);
                        break;
                    }
                    y -= 1;
                }
            }
        }

        for m in &mut self.meteor_events {
            m.2 = m.2.saturating_sub(1);
        }
        self.meteor_events.retain(|m| m.2 > 0);

        if frame.is_multiple_of(300) {
            let total = self.census.particles as usize;
            if total > (self.w * self.h * 85) / 100 {
                self.compact();
            }
        }
    }

    pub fn update_pressure(&mut self) {
        let frame = self.frame;
        let p = &mut self.pressure;

        let plants = self.census.plants as f32;

        if plants > self.plant_capacity {
            self.plant_capacity = plants;
        } else if frame.is_multiple_of(600) {
            self.plant_capacity = (self.plant_capacity * 0.995).max(plants);
        }

        let cover = if self.plant_capacity > 50.0 {
            (plants / self.plant_capacity).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if cover < 0.6 {

            p.scarcity = (p.scarcity + (0.6 - cover) * 0.0012).min(1.0);
        } else if cover > 0.85 {

            p.scarcity = (p.scarcity - 0.0006).max(0.0);
        }

        if frame.is_multiple_of(250) {
            p.depletion = (p.depletion + 0.012).min(1.0);
        }

        let severity = match self.weather {
            Weather::Storm => 0.00055,
            Weather::Meteor => 0.00030,
            Weather::Snow => 0.00008,
            _ => 0.0,
        };
        if severity > 0.0 {
            p.harshness = (p.harshness + severity).min(1.0);
        } else if self.weather == Weather::Clear {
            p.harshness = (p.harshness - 0.00018).max(0.0);
        }

        if frame.is_multiple_of(7200) && frame > 0 {
            p.extremes = (p.extremes + 0.25).min(1.0);
        }

        if frame.is_multiple_of(1200) && frame > 0 {
            p.volcanic = (p.volcanic + 0.09).min(1.0);
        }

        let t = (frame as f32 / 20_000.0).min(1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        let strain = p.overall();
        self.difficulty = (ease * 0.65 + strain * 0.35).clamp(0.0, 1.0);
    }

    #[must_use]
    pub fn hardship(&self) -> f32 {
        self.difficulty
    }

    pub fn compact(&mut self) {
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
