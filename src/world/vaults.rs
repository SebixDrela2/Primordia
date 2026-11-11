use super::*;

impl World {
    pub(crate) fn place_rare_vaults(&mut self, w: i32, h: i32) {
        let step = (w as f32 / 34.0) as i32;
        let vaults: &[(Elem, i32, i32, i32, i32)] = &[
            // (element, vault_index, cy_min, radius, count)
            (Elem::Sulfur, 1, h / 2, 3, 25),
            (Elem::Ash, 2, h / 2, 3, 25),
            (Elem::Salt, 3, h / 2, 3, 25),
            (Elem::Dust, 4, h / 2, 3, 25),
            (Elem::Gunpowder, 5, h / 2, 3, 12),
            (Elem::Fertilizer, 6, h / 2, 3, 20),
            (Elem::Blood, 7, h / 2, 3, 20),
            (Elem::Alcohol, 8, h / 2, 3, 12),
            (Elem::Honey, 9, h / 2, 3, 12),
            (Elem::Acid, 10, h / 2, 3, 20),
            (Elem::Chlorine, 11, h / 2, 3, 20),
            (Elem::Methane, 12, h / 2, 3, 20),
            (Elem::Oxygen, 13, h / 2, 3, 20),
            (Elem::Gas, 14, h / 2, 3, 20),
            (Elem::Steam, 15, h / 2, 3, 20),
            (Elem::Smoke, 16, h / 2, 3, 20),
            (Elem::Plasma, 17, h / 2, 3, 10),
            (Elem::Tin, 18, h / 2, 4, 25),
            (Elem::Bronze, 19, h / 2, 4, 20),
            (Elem::Iron, 20, h / 2, 4, 25),
            (Elem::Steel, 21, h / 2, 4, 10),
            (Elem::Silver, 22, h / 2, 4, 20),
            (Elem::Gold, 23, h / 2, 4, 15),
            (Elem::Glass, 24, h / 2, 4, 20),
            (Elem::Obsidian, 25, h / 2, 4, 15),
            (Elem::Gem, 26, h / 2, 4, 12),
            (Elem::Diamond, 27, h / 2, 4, 6),
            (Elem::Brick, 28, h / 2, 4, 25),
            (Elem::Concrete, 29, h / 2, 4, 25),
            (Elem::Chest, 30, h / 2, 3, 10),
            (Elem::Forge, 31, h / 2, 3, 6),
            (Elem::Farm, 32, h / 2, 3, 10),
            (Elem::Temple, 33, h / 2, 3, 6),
            (Elem::Machine, 34, h / 2, 3, 10),
            (Elem::Meat, 0, h / 2 + 40, 3, 12),
            (Elem::Leather, 1, h / 2 + 40, 3, 10),
            (Elem::Roots, 2, h / 2 + 40, 3, 20),
            (Elem::Food, 3, h / 2 + 40, 3, 15),
            (Elem::Energy, 4, h / 2 + 40, 3, 25),
            (Elem::Soul, 5, h / 2 + 40, 3, 6),
            (Elem::Fire, 6, h / 2 + 40, 2, 10),
            (Elem::Virus, 7, h / 2 + 40, 3, 3),
            (Elem::Bacteria, 8, h / 2 + 40, 3, 10),
            (Elem::Egg, 9, h / 2 + 40, 3, 6),
        ];
        for &(elem, vault_idx, cy_min, radius, count) in vaults {
            let mut placed = 0;
            let mut attempts = 0;
            while placed < count && attempts < count * 6 {
                attempts += 1;
                let cx = step * vault_idx + step / 2 + self.rng.range_i(-step / 4, step / 4);
                let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
                let r = self.rng.range(radius as u32) as f32;
                let dx = angle.cos() * r;
                let dy = angle.sin() * r;
                let x = (cx as f32 + dx) as i32;
                // Depth below the terrain, varied per vault. Never absolute
                // heights, which used to float above the surface.
                let surf = self.surface_y(x);
                let depth = 8 + (vault_idx.rem_euclid(7)) * 12 + self.rng.range(6) as i32;
                let _ = cy_min;
                let y = surf + depth + dy as i32;
                if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                    let cur = self.get(x, y);
                    if matches!(cur, Elem::Empty | Elem::Stone | Elem::Sand) {
                        self.set_xy(x, y, elem);
                        placed += 1;
                    }
                }
            }
        }
    }

}
