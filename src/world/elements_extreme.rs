use super::{World, Elem};

impl World {
    /// The extreme end of the world's variety: the odd materials, the
    /// rare veins, the old buildings and the sickness, all scattered
    /// over the ground that was already laid down.
    pub fn place_extreme_diversity(&mut self, width: i32, height: i32) {
        self.place_extreme_salts_and_dust(width, height);
        self.place_extreme_hazards(width, height);
        self.place_extreme_gases(width, height);
        self.place_extreme_metal_veins(width, height);
        self.place_extreme_rare_ores(width, height);
        self.place_extreme_organics(width);
        self.place_extreme_energy_and_cold(width, height);
        self.place_extreme_construction(width, height);
        self.place_extreme_paths(width);
        self.place_extreme_magic(width, height);
        self.place_extreme_plague(width, height);
    }

    /// Salt, ash, dust, gunpowder and fertilizer: the loose stuff that
    /// lies about near the surface.
    fn place_extreme_salts_and_dust(&mut self, width: i32, height: i32) {
        for _ in 0..12 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            for dy in -2..=2 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Salt);
                        }
                    }
                }
            }
        }

        for _ in 0..25 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 8 - self.rng.range(25) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }

        for _ in 0..40 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.surface_y(cx) + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }

        for _ in 0..45 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.45) {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }

        for _ in 0..15 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.35) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }
    }

    /// Blood, poison, alcohol and honey: what a colony leaves behind
    /// or what it keeps.
    fn place_extreme_hazards(&mut self, width: i32, height: i32) {
        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Blood);
                        }
                    }
                }
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Poison);
                        }
                    }
                }
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Alcohol);
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Honey);
            }
        }
    }

    /// Acid, chlorine, methane, oxygen and gas: the air of the deep.
    fn place_extreme_gases(&mut self, width: i32, height: i32) {
        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Acid);
                        }
                    }
                }
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 16 && self.rng.chance(0.18) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Chlorine);
                        }
                    }
                }
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.22) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Methane);
                        }
                    }
                }
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.2) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 && self.get(cell_x, cell_y) == Elem::Empty {
                            self.set_xy(cell_x, cell_y, Elem::Oxygen);
                        }
                    }
                }
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Gas);
            }
        }
    }

    /// The common veins: tin, bronze and iron.
    fn place_extreme_metal_veins(&mut self, width: i32, height: i32) {
        for _ in 0..15 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 12 - self.rng.range(30) as i32;
            let len = 8 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            for i in 0..len {
                let cell_x = cx + (dir_x * i as f32) as i32;
                let cell_y = cy + (dir_y * i as f32) as i32;
                if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5
                    && self.get(cell_x, cell_y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(cell_x, cell_y, Elem::Tin);
                    }
            }
        }

        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 16 - self.rng.range(28) as i32;
            let len = 6 + self.rng.range(12);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            for i in 0..len {
                let cell_x = cx + (dir_x * i as f32) as i32;
                let cell_y = cy + (dir_y * i as f32) as i32;
                if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5
                    && self.get(cell_x, cell_y) == Elem::Stone && self.rng.chance(0.22) {
                        self.set_xy(cell_x, cell_y, Elem::Bronze);
                    }
            }
        }

        for _ in 0..15 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 22 - self.rng.range(28) as i32;
            let len = 10 + self.rng.range(16);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            for i in 0..len {
                let cell_x = cx + (dir_x * i as f32) as i32;
                let cell_y = cy + (dir_y * i as f32) as i32;
                if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5
                    && self.get(cell_x, cell_y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(cell_x, cell_y, Elem::Iron);
                    }
            }
        }
    }

    /// The rare stuff: steel, silver, gold, glass, obsidian, diamond
    /// and gem.
    fn place_extreme_rare_ores(&mut self, width: i32, height: i32) {
        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 24 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Steel);
            }
        }

        for _ in 0..8 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 20 - self.rng.range(26) as i32;
            let len = 8 + self.rng.range(14);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dir_x = angle.cos();
            let dir_y = angle.sin();
            for i in 0..len {
                let cell_x = cx + (dir_x * i as f32) as i32;
                let cell_y = cy + (dir_y * i as f32) as i32;
                if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5
                    && self.get(cell_x, cell_y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(cell_x, cell_y, Elem::Silver);
                    }
            }
        }

        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 26 - self.rng.range(22) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Gold);
            }
        }

        for _ in 0..8 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Sand && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Glass);
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 8 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Obsidian);
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 28 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Diamond);
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 24 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Gem);
            }
        }
    }

    /// Wood, meat, bone, leather, roots and food: the living world.
    fn place_extreme_organics(&mut self, width: i32) {
        for _ in 0..30 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Wood);

                for dy in 1..=3 {
                    let ty = sy - dy;
                    if ty > 1 && self.get(cx, ty) == Elem::Empty && self.rng.chance(0.5) {
                        self.set_xy(cx, ty, Elem::Wood);
                    }
                }
            }
        }

        for _ in 0..8 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Meat);
            }
        }

        for _ in 0..8 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Bone);
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }

        for _ in 0..20 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }

        for _ in 0..12 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }
    }

    /// Ice, energy, mana, soul and fire.
    fn place_extreme_energy_and_cold(&mut self, width: i32, height: i32) {
        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ice);
            }
        }

        for _ in 0..50 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx).max(10);
            let cy = sy + 1 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Energy);
            }
        }

        for _ in 0..15 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height - 6 - self.rng.range(12) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Fire);
            }
        }
    }

    /// The old buildings: brick, concrete, chests, forges, farms,
    /// temples and towers.
    fn place_extreme_construction(&mut self, width: i32, height: i32) {
        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            let size = (4 + self.rng.range(6)) as i32;
            for dy in 0..size {
                for dx in -size/2..=size/2 {
                    let cell_x = cx + dx;
                    let cell_y = cy + dy;
                    if cell_x > 2 && cell_x < width - 2 && cell_y > 2 && cell_y < height - 5
                        && self.get(cell_x, cell_y) == Elem::Empty && self.rng.chance(0.35) {
                            self.set_xy(cell_x, cell_y, Elem::Brick);
                        }
                }
            }
        }

        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Concrete);
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Chest);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Forge);
            }
        }

        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Farm);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Temple);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let height = (6 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let cell_y = sy - dy;
                if cell_y > 2 && self.get(cx, cell_y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(cx, cell_y, Elem::Tower);
                }
            }
        }
    }

    /// The old ways: a bridge over the hollow and a road along the
    /// ground.
    fn place_extreme_paths(&mut self, width: i32) {
        for _ in 0..2 {
            let x1 = self.rng.range((width / 3) as u32) as i32 + width / 6;
            let x2 = self.rng.range((width / 3) as u32) as i32 + 2 * width / 3;
            let sy = self.surface_y(i32::midpoint(x1, x2));
            let cell_y = sy - 1;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for cell_x in start..=end {
                if self.get(cell_x, cell_y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(cell_x, cell_y, Elem::Bridge);
                }
            }
        }

        for _ in 0..3 {
            let x1 = self.rng.range(width as u32) as i32;
            let x2 = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(i32::midpoint(x1, x2));
            let cell_y = sy;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for cell_x in start..=end {
                if self.get(cell_x, cell_y) == Elem::Empty && self.rng.chance(0.6) {
                    self.set_xy(cell_x, cell_y, Elem::Road);
                }
            }
        }
    }

    /// Runes, wards, portals and eggs.
    fn place_extreme_magic(&mut self, width: i32, height: i32) {
        for _ in 0..6 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = height / 2 + self.rng.range((height / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }
    }

    /// The sickness: virus in the organic ground, bacteria in the
    /// meat.
    fn place_extreme_plague(&mut self, width: i32, height: i32) {
        for _ in 0..3 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.rng.range(height as u32) as i32;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.12) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 {
                            let elem = self.get(cell_x, cell_y);
                            if elem.is_organic() {
                                self.set_xy(cell_x, cell_y, Elem::Virus);
                            }
                        }
                    }
                }
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(width as u32) as i32;
            let cy = self.rng.range(height as u32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 6 && self.rng.chance(0.18) {
                        let cell_x = cx + dx;
                        let cell_y = cy + dy;
                        if cell_x > 1 && cell_x < width - 1 && cell_y > 1 && cell_y < height - 5 {
                            let elem = self.get(cell_x, cell_y);
                            if elem == Elem::Meat || elem == Elem::Corpse || elem == Elem::Food {
                                self.set_xy(cell_x, cell_y, Elem::Bacteria);
                            }
                        }
                    }
                }
            }
        }
    }
}
