use super::*;

impl World {
    pub(crate) fn place_element_museum(&mut self, w: i32, h: i32) {
        let start_x = (w as f32 * 0.05) as i32;
        let start_y = (h as f32 * 0.8) as i32;
        let mut idx = 0;
        for v in 1..=Elem::Food as u8 {
            let e = Elem::from_u8(v);
            if e == Elem::Empty || e == Elem::Void { continue; }
            // Place each element in its own column below the surface
            let mx = start_x + (idx % 20) * 4;
            let row = idx / 20;
            // Dig down below surface for this column
            let my = self.surface_y(mx.clamp(2, w - 3)) + 2 + row * 3;
            // Surround each museum cell with stone walls for protection
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let wx = mx + dx;
                    let wy = my + dy;
                    if wx > 1 && wx < w - 1 && wy > 1 && wy < h - 5 {
                        if dx == 0 && dy == 0 {
                            self.set_xy(wx, wy, e);
                        } else if self.get(wx, wy) == Elem::Empty {
                            self.set_xy(wx, wy, Elem::Stone);
                        }
                    }
                }
            }
            idx += 1;
        }
    }

    // Place a huge variety of additional element types so the world is rich
    // with many distinct materials for creatures to find, eat, and use.
}

impl World {
    pub(crate) fn place_diverse_elements(&mut self, w: i32, h: i32) {
        // === LIQUIDS ===
        // Mercury pools (rare, deep)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 5 - self.rng.range(20) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Mercury);
                        }
                    }
                }
            }
        }

        // Alcohol ponds (rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Alcohol);
                        }
                    }
                }
            }
        }

        // Honey pools (rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Honey);
                        }
                    }
                }
            }
        }

        // Blood pools (very rare, near surface)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            for dy in 0..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 5 && self.rng.chance(0.15) {
                        let x = cx + dx;
                        let y = sy - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Blood);
                        }
                    }
                }
            }
        }

        // Poison pools (rare, dangerous)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Poison);
                        }
                    }
                }
            }
        }

        // === GASES ===
        // Chlorine gas pockets (near acid/poison)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 20 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Chlorine);
                        }
                    }
                }
            }
        }

        // Methane gas pockets (swampy areas)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(8) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Methane);
                        }
                    }
                }
            }
        }

        // Oxygen pockets (caves)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Oxygen);
                        }
                    }
                }
            }
        }

        // === POWDERS ===
        // Sulfur deposits (near lava)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Sulfur);
            }
        }

        // Ash deposits (post-volcanic)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 5 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }

        // Dust (dry areas)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 10 + self.rng.range(40) as i32;
            if cy < h - 5 && self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }

        // Gunpowder (rare, man-made)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }

        // Fertilizer (farm areas)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }

        // Mud (wet areas)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1;
            if self.get(cx, cy) == Elem::Water && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Mud);
            }
        }

        // Tar: the heavy residue of ancient oil fields.
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.35) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Tar);
                        }
                    }
                }
            }
        }

        // === SOLIDS / METALS ===
        // Tin veins
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(35) as i32;
            let len = 10 + self.rng.range(20);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Tin);
                    }
                }
            }
        }

        // Bronze deposits (rare, mixed copper+tin)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 18 - self.rng.range(30) as i32;
            let len = 8 + self.rng.range(15);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.25) {
                        self.set_xy(x, y, Elem::Bronze);
                    }
                }
            }
        }

        // Iron veins (deeper)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 20 - self.rng.range(30) as i32;
            let len = 12 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.35) {
                        self.set_xy(x, y, Elem::Iron);
                    }
                }
            }
        }

        // Steel (rare, refined)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 22 - self.rng.range(25) as i32;
            let len = 6 + self.rng.range(10);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.2) {
                        self.set_xy(x, y, Elem::Steel);
                    }
                }
            }
        }

        // Silver veins
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 18 - self.rng.range(30) as i32;
            let len = 10 + self.rng.range(15);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Silver);
                    }
                }
            }
        }

        // Coal seams
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let len = 20 + self.rng.range(30);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.5) {
                        self.set_xy(x, y, Elem::Ash); // Using Ash as coal proxy
                    }
                }
            }
        }

        // === ORGANIC ===
        // Fungus (damp areas)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2) {
                if self.rng.chance(0.3) {
                    self.set_xy(cx, cy, Elem::Fungus);
                }
            }
        }

        // Moss (damp, shaded)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2) {
                if self.rng.chance(0.2) {
                    self.set_xy(cx, cy, Elem::Moss);
                }
            }
        }

        // Roots (underground)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }

        // Meat/Bone (creature remains)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(8) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                if self.rng.chance(0.5) {
                    self.set_xy(cx, cy, Elem::Meat);
                } else {
                    self.set_xy(cx, cy, Elem::Bone);
                }
            }
        }

        // Leather (rare)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }

        // === ENERGY/EXOTIC ===
        // Mana crystals
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 3) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }

        // Soul energy (very rare, deep)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }

        // Magma (hotter than lava)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(20) as i32;
            let radius = (4 + self.rng.range(4)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.7) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            if self.get(x, y) == Elem::Stone {
                                self.set_xy(x, y, Elem::Magma);
                            }
                        }
                    }
                }
            }
        }

        // Void pockets (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            let radius = (2 + self.rng.range(3)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.5) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            if self.rng.chance(0.5) {
                                self.set_xy(x, y, Elem::Void);
                            }
                        }
                    }
                }
            }
        }

        // Snow: old snow that never melted, lying in the deep caves.
        for _ in 0..40 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if self.rng.chance(0.35) {
                        let x = cx + dx;
                        let y = cy + dy;
                        // Snow settles on a floor, so it goes in the empty
                        // cell directly above solid ground.
                        if x > 1 && x < w - 1 && y > 1 && y < h - 6
                            && self.get(x, y) == Elem::Empty
                            && self.get(x, y + 1).is_solid()
                        {
                            self.set_xy(x, y, Elem::Snow);
                        }
                    }
                }
            }
        }

        // Embers: coals that still glow in the deep rock.
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 25 + self.rng.range(50) as i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Ember);
                        }
                    }
                }
            }
        }

        // === CONSTRUCTED ===
        // Ancient brick structures
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (self.rng.range((h as u32 / 2) as u32) as i32) + h / 4;
            let width = (6 + self.rng.range(8)) as i32;
            let height = (4 + self.rng.range(4)) as i32;
            for dy in 0..height {
                for dx in -width/2..=width/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                        if self.get(x, y) == Elem::Empty && self.rng.chance(0.4) {
                            match self.rng.range(4) {
                                0 => self.set_xy(x, y, Elem::Brick),
                                1 => self.set_xy(x, y, Elem::Concrete),
                                2 => self.set_xy(x, y, Elem::Stone),
                                _ => self.set_xy(x, y, Elem::Wall),
                            }
                        }
                    }
                }
            }
        }

        // Wire/Circuit ruins
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let len = 15 + self.rng.range(10);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Empty && self.rng.chance(0.3) {
                        match self.rng.range(3) {
                            0 => self.set_xy(x, y, Elem::Wire),
                            1 => self.set_xy(x, y, Elem::Circuit),
                            _ => self.set_xy(x, y, Elem::Machine),
                        }
                    }
                }
            }
        }

        // Tools scattered
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                match self.rng.range(5) {
                    0 => self.set_xy(cx, cy, Elem::Tool),
                    1 => self.set_xy(cx, cy, Elem::Furnace),
                    2 => self.set_xy(cx, cy, Elem::Anvil),
                    3 => self.set_xy(cx, cy, Elem::Forge),
                    _ => self.set_xy(cx, cy, Elem::Machine),
                }
            }
        }

        // Door/Chest ruins
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.5) {
                match self.rng.range(4) {
                    0 => self.set_xy(cx, cy, Elem::Door),
                    1 => self.set_xy(cx, cy, Elem::Chest),
                    2 => self.set_xy(cx, cy, Elem::Furnace),
                    _ => self.set_xy(cx, cy, Elem::Anvil),
                }
            }
        }

        // Charcoal: the residue of burnt wood, left where fires have been.
        for _ in 0..14 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 10 + self.rng.range(50) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if self.rng.chance(0.35) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Charcoal);
                        }
                    }
                }
            }
        }

        // === MAGICAL ===
        // Runes
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }

        // Wards
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }

        // Portals (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        // === BIOLOGICAL ===
        // Eggs (rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }

        // Corpses
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(8) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Corpse);
            }
        }

        // Virus patches
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.1) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y).is_organic() {
                            self.set_xy(x, y, Elem::Virus);
                        }
                    }
                }
            }
        }

        // Bacteria
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 6 && self.rng.chance(0.15) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Meat || self.get(x, y) == Elem::Corpse {
                            self.set_xy(x, y, Elem::Bacteria);
                        }
                    }
                }
            }
        }

        // Food items
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }
    }

    // ===== ENHANCED WORLD GENERATION METHODS =====

}
