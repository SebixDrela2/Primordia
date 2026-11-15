use super::{World, Elem};

impl World {
    pub fn bury_floating_resources(&mut self, w: i32, h: i32) {
        let mut floating: Vec<Elem> = Vec::new();
        for x in 2..(w - 2) {
            let surf = self.surface_y(x);
            let sky_bottom = (surf - 3).max(2);
            for y in 2..sky_bottom {
                let e = self.get(x, y);
                let is_resource = e.is_solid()
                    || matches!(
                        e,
                        Elem::Oil
                            | Elem::Acid
                            | Elem::Mercury
                            | Elem::Blood
                            | Elem::Poison
                            | Elem::Alcohol
                            | Elem::Honey
                            | Elem::Tar
                    );

                if is_resource
                    && !matches!(
                        e,
                        Elem::Plant
                            | Elem::Wood
                            | Elem::Moss
                            | Elem::Spore
                            | Elem::Roots
                            | Elem::Water
                            | Elem::Ice
                            | Elem::Snow
                    )
                {
                    floating.push(e);
                    self.set_xy(x, y, Elem::Empty);
                }
            }
        }

        for e in floating {
            for _ in 0..24 {
                let x = self.rng.range_i(3, w - 3);
                let surf = self.surface_y(x);
                let max_depth = (h - 8 - surf).max(2);
                let y = surf + 4 + self.rng.range(max_depth as u32) as i32;
                let cur = self.get(x, y);
                if cur == Elem::Stone || cur == Elem::Sand {
                    self.set_xy(x, y, e);
                    break;
                }
            }
        }
    }

    pub fn guarantee_water(&mut self, w: i32, h: i32) {

        let lx = (w as f32 * 0.2) as i32;
        let ly = (h as f32 * 0.75) as i32;
        for dy in -6..=6 {
            for dx in -10..=10 {
                if dx*dx + dy*dy <= 40 {
                    let x = lx + dx;
                    let y = ly + dy;

                    if x > 1 && x < w - 1 && y > 1 && y < h - 5 && y > self.surface_y(x) {
                        self.set_xy(x, y, Elem::Water);
                    }
                }
            }
        }
    }

}

impl World {
    pub fn generate_surface_water(&mut self, w: i32, h: i32) {

        let lake_start = (w as f32 * 0.55) as i32;
        let lake_end = (w as f32 * 0.75) as i32;
        for x in lake_start..lake_end {
            let t = (x - lake_start) as f32 / (lake_end - lake_start) as f32;
            let depth = 4.0 + (t * std::f32::consts::PI).sin() * 8.0;
            let surface = self.surface_y(x);
            for dy in 0..depth as i32 {
                let y = surface + dy;
                if y < h - 6 {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let pond_w = 3 + self.rng.range(4) as i32;
            for px in (x - pond_w / 2)..=(x + pond_w / 2) {
                let depth = 1 + self.rng.range(3) as i32;
                for dy in 0..depth {
                    let y = surface + dy;
                    if y < h - 6 {
                        self.set_xy(px, y, Elem::Water);
                    }
                }
            }
        }

        let river_y = self.surface_y(w / 2);
        for x in (w / 4)..(3 * w / 4) {
            if self.rng.chance(0.85) {
                let y = river_y as f32 + (self.rng.f32() - 0.5) * 3.0;
                let y = y as i32;
                if y > 1 && y < h - 6 {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        for _ in 0..3 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            for dx in -2..=2 {
                for dy in 0..=2 {
                    let px = x + dx;
                    let py = surface + dy;
                    if px > 0 && py > 0 && px < w && py < h - 6
                        && self.rng.chance(0.3) {
                            self.set_xy(px, py, Elem::Water);
                        }
                }
            }
        }
    }

}

impl World {
    pub fn generate_crystal_caves(&mut self, width: i32, height: i32) {
        for _ in 0..12 {
            let cave_x = self.rng.range(width as u32) as i32;
            let cave_y = self.rng.range((height / 2) as u32) as i32 + height / 4;
            let radius = (5 + self.rng.range(8)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius {
                        let cell_x = cave_x + dx;
                        let cell_y = cave_y + dy;
                        if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5 {
                            let elem = self.get(cell_x, cell_y);
                            if elem == Elem::Empty || elem == Elem::Stone {
                                let roll = self.rng.f32();
                                if roll < 0.25 {
                                    self.set_xy(cell_x, cell_y, Elem::Crystal);
                                } else if roll < 0.4 {
                                    self.set_xy(cell_x, cell_y, Elem::Gem);
                                } else if roll < 0.5 {
                                    self.set_xy(cell_x, cell_y, Elem::Diamond);
                                } else if roll < 0.6 {
                                    self.set_xy(cell_x, cell_y, Elem::Mana);
                                } else if roll < 0.7 {
                                    self.set_xy(cell_x, cell_y, Elem::Glass);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

}
