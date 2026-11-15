use super::{World, Elem};

/// Where a vein of ore starts.
enum VeinOrigin {
    /// `height - top - rng.range(range)`: a vein deep in the rock.
    Deep { top: i32, range: u32 },

    /// `rng.range(height / 2) + height / 4`: a vein in the middle.
    Middle,
}

/// One kind of ore vein: how many there are, how long they run and
/// how likely they are to take hold in the stone.
struct VeinSpec {
    count: u32,
    len_base: u32,
    len_range: u32,
    ore: Elem,
    chance: f32,
    any_solid: bool,
    origin: VeinOrigin,
}

impl VeinSpec {
    fn copper() -> Self {
        Self {
            count: 25,
            len_base: 15,
            len_range: 25,
            ore: Elem::Copper,
            chance: 0.4,
            any_solid: false,
            origin: VeinOrigin::Deep { top: 10, range: 40 },
        }
    }

    fn tin() -> Self {
        Self {
            count: 15,
            len_base: 10,
            len_range: 20,
            ore: Elem::Tin,
            chance: 0.3,
            any_solid: false,
            origin: VeinOrigin::Deep { top: 15, range: 35 },
        }
    }

    fn iron() -> Self {
        Self {
            count: 20,
            len_base: 12,
            len_range: 18,
            ore: Elem::Iron,
            chance: 0.35,
            any_solid: false,
            origin: VeinOrigin::Deep { top: 20, range: 30 },
        }
    }

    fn gold() -> Self {
        Self {
            count: 8,
            len_base: 8,
            len_range: 12,
            ore: Elem::Gold,
            chance: 0.3,
            any_solid: false,
            origin: VeinOrigin::Deep { top: 25, range: 25 },
        }
    }

    fn silver() -> Self {
        Self {
            count: 10,
            len_base: 10,
            len_range: 15,
            ore: Elem::Silver,
            chance: 0.3,
            any_solid: false,
            origin: VeinOrigin::Deep { top: 18, range: 30 },
        }
    }

    fn ash() -> Self {
        Self {
            count: 30,
            len_base: 20,
            len_range: 30,
            ore: Elem::Ash,
            chance: 0.5,
            any_solid: false,
            origin: VeinOrigin::Middle,
        }
    }

    fn sulfur() -> Self {
        Self {
            count: 12,
            len_base: 8,
            len_range: 12,
            ore: Elem::Sulfur,
            chance: 0.2,
            any_solid: true,
            origin: VeinOrigin::Deep { top: 12, range: 20 },
        }
    }
}

impl World {
    /// The ore of the deep: a vein of each metal, the rust on the iron,
    /// the oil in the pockets and the acid in the wet rock.
    pub fn generate_ore_veins(&mut self, width: i32, height: i32) {
        self.spawn_ore_veins(width, height, &VeinSpec::copper());
        self.spawn_ore_veins(width, height, &VeinSpec::tin());
        self.spawn_ore_veins(width, height, &VeinSpec::iron());
        self.spawn_ore_veins(width, height, &VeinSpec::gold());
        self.spawn_ore_veins(width, height, &VeinSpec::silver());
        self.spawn_rust_patches(width, height);
        self.spawn_ore_veins(width, height, &VeinSpec::ash());
        self.spawn_ore_veins(width, height, &VeinSpec::sulfur());
        self.spawn_oil_pools(width);
        self.spawn_acid_pools(width, height);
    }

    /// A vein of ore: a wandering line that takes hold in the stone.
    fn spawn_ore_veins(&mut self, width: i32, height: i32, spec: &VeinSpec) {
        for _ in 0..spec.count {
            let cx = self.rng.range(width as u32) as i32;
            let cy = match spec.origin {
                VeinOrigin::Deep { top, range } => height - top - self.rng.range(range) as i32,
                VeinOrigin::Middle => self.rng.range((height / 2) as u32) as i32 + height / 4,
            };
            let len = spec.len_base + self.rng.range(spec.len_range);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            for step in 0..len {
                let cell_x = cx + (dir_x * step as f32) as i32;
                let cell_y = cy + (dir_y * step as f32) as i32;
                if cell_x <= 2 || cell_x >= width - 2 || cell_y <= 2 || cell_y >= height - 5 {
                    continue;
                }
                let elem = self.get(cell_x, cell_y);
                let holds = if spec.any_solid {
                    elem.is_solid()
                } else {
                    elem == Elem::Stone
                };
                if holds && self.rng.chance(spec.chance) {
                    self.set_xy(cell_x, cell_y, spec.ore);
                }
            }
        }
    }

    /// The rust that grows on the iron and the metal.
    fn spawn_rust_patches(&mut self, width: i32, height: i32) {
        for _ in 0..18 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.surface_y(cx) + 15 + self.rng.range(45) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx * dx + dy * dy <= 6 && self.rng.chance(0.3) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 {
                            let elem = self.get(cell_x, cell_y);
                            if elem == Elem::Iron || elem == Elem::Metal {
                                self.set_xy(cell_x, cell_y, Elem::Rust);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The oil in the pockets of the deep.
    fn spawn_oil_pools(&mut self, width: i32) {
        for _ in 0..8 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -3..=3 {
                for dx in -5..=5 {
                    if dx * dx + dy * dy <= 25 && self.rng.chance(0.4) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Oil);
                        }
                    }
                }
            }
        }
    }

    /// The acid in the wet rock of the middle depths.
    fn spawn_acid_pools(&mut self, width: i32, height: i32) {
        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.rng.range((height / 2) as u32) as i32 + height / 4;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx * dx + dy * dy <= 9 && self.rng.chance(0.3) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Acid);
                        }
                    }
                }
            }
        }
    }
}
