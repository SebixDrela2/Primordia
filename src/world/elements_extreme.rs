use super::*;

impl World {
    pub(crate) fn place_extreme_diversity(&mut self, w: i32, h: i32) {
        // === More powders ===
        // Salt flats (surface)
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            for dy in -2..=2 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Salt);
                        }
                    }
                }
            }
        }
        // Ash deposits (post-volcanic, near lava)
        for _ in 0..25 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(25) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }
        // Dust (dry surface areas)
        for _ in 0..40 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }
        // Gunpowder (underground) - enough of it that colonies can craft bombs
        for _ in 0..45 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.45) {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }
        // Fertilizer (farm patches)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.35) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }

        // === More liquids ===
        // Blood pools (surface, rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Blood);
                        }
                    }
                }
            }
        }
        // Poison pools (dangerous, swampy)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Poison);
                        }
                    }
                }
            }
        }
        // Alcohol surface pools
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Alcohol);
            }
        }
        // Honey pools (rare, near fungus). The pool is dug into the ground so
        // the honey rests in a basin instead of hovering above the terrain.
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Honey);
            }
        }
        // Acid pools (rare, dangerous)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Acid);
                        }
                    }
                }
            }
        }

        // === More gases ===
        // Chlorine gas (near poison/acid)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 16 && self.rng.chance(0.18) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Chlorine);
                        }
                    }
                }
            }
        }
        // Methane gas (swampy surface)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.22) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Methane);
                        }
                    }
                }
            }
        }
        // Oxygen pockets (caves)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Oxygen);
                        }
                    }
                }
            }
        }
        // Gas pockets
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Gas);
            }
        }

        // === More solids/metals ===
        // Tin veins (shallow)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 12 - self.rng.range(30) as i32;
            let len = 8 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(x, y, Elem::Tin);
                    }
                }
            }
        }
        // Bronze deposits (mixed copper+tin)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 16 - self.rng.range(28) as i32;
            let len = 6 + self.rng.range(12);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.22) {
                        self.set_xy(x, y, Elem::Bronze);
                    }
                }
            }
        }
        // Iron veins (deep)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 22 - self.rng.range(28) as i32;
            let len = 10 + self.rng.range(16);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Iron);
                    }
                }
            }
        }
        // Steel (very rare, deep)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 24 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Steel);
            }
        }
        // Silver veins (deep)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 20 - self.rng.range(26) as i32;
            let len = 8 + self.rng.range(14);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(x, y, Elem::Silver);
                    }
                }
            }
        }
        // Gold veins (rare, deep)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 26 - self.rng.range(22) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Gold);
            }
        }
        // Glass (near lava/sand)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Sand && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Glass);
            }
        }
        // Obsidian (water meets lava, already happens but add more)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Obsidian);
            }
        }
        // Diamond (very rare, deep)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 28 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Diamond);
            }
        }
        // Gem (deep)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 24 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Gem);
            }
        }

        // === More organic ===
        // Wood (surface trees)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Wood);
                // tree trunk
                for dy in 1..=3 {
                    let ty = sy - dy;
                    if ty > 1 && self.get(cx, ty) == Elem::Empty && self.rng.chance(0.5) {
                        self.set_xy(cx, ty, Elem::Wood);
                    }
                }
            }
        }
        // Meat (creature remains)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Meat);
            }
        }
        // Bone
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Bone);
            }
        }
        // Leather (rare)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }
        // Roots (underground)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }
        // Food items
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }

        // === More energy/exotic ===
        // Ice (cold spots)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ice);
            }
        }
        // Energy specks (above ground)
        for _ in 0..50 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx).max(10);
            let cy = sy + 1 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Energy);
            }
        }
        // Mana crystals (caves)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }
        // Soul (very rare, deep)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 6 - self.rng.range(12) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }
        // Fire (rare surface spots)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Fire);
            }
        }

        // === More constructed ===
        // Brick structures
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            let size = (4 + self.rng.range(6)) as i32;
            for dy in 0..size {
                for dx in -size/2..=size/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                        if self.get(x, y) == Elem::Empty && self.rng.chance(0.35) {
                            self.set_xy(x, y, Elem::Brick);
                        }
                    }
                }
            }
        }
        // Concrete
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Concrete);
            }
        }
        // Chest
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Chest);
            }
        }
        // Forge
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Forge);
            }
        }
        // Farm
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Farm);
            }
        }
        // Temple
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Temple);
            }
        }
        // Tower
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let height = (6 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let y = sy - dy;
                if y > 2 && self.get(cx, y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(cx, y, Elem::Tower);
                }
            }
        }
        // Bridge
        for _ in 0..2 {
            let x1 = self.rng.range((w / 3) as u32) as i32 + w / 6;
            let x2 = self.rng.range((w / 3) as u32) as i32 + 2 * w / 3;
            let sy = self.surface_y((x1 + x2) / 2);
            let y = sy - 1;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(x, y, Elem::Bridge);
                }
            }
        }
        // Road
        for _ in 0..3 {
            let x1 = self.rng.range(w as u32) as i32;
            let x2 = self.rng.range(w as u32) as i32;
            let sy = self.surface_y((x1 + x2) / 2);
            let y = sy;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.6) {
                    self.set_xy(x, y, Elem::Road);
                }
            }
        }

        // === More magical ===
        // Runes
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }
        // Wards
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }
        // Portals (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        // === More biological ===
        // Eggs
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }
        // Virus patches
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.12) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                            let e = self.get(x, y);
                            if e.is_organic() {
                                self.set_xy(x, y, Elem::Virus);
                            }
                        }
                    }
                }
            }
        }
        // Bacteria (near meat/corpse)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 6 && self.rng.chance(0.18) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                            let e = self.get(x, y);
                            if e == Elem::Meat || e == Elem::Corpse || e == Elem::Food {
                                self.set_xy(x, y, Elem::Bacteria);
                            }
                        }
                    }
                }
            }
        }
    }

}
