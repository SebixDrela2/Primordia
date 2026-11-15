use super::{World, Elem};

impl World {
    pub fn place_element_museum(&mut self, w: i32, h: i32) {
        let start_x = (w as f32 * 0.05) as i32;
        let _start_y = (h as f32 * 0.8) as i32;
        let mut idx = 0;
        for v in 1..=Elem::Food as u8 {
            let e = Elem::from_u8(v);
            if e == Elem::Empty || e == Elem::Void { continue; }

            let mx = start_x + (idx % 20) * 4;
            let row = idx / 20;

            let my = self.surface_y(mx.clamp(2, w - 3)) + 2 + row * 3;

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

}

impl World {
    pub fn place_diverse_elements(&mut self, w: i32, h: i32) {

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

        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Sulfur);
            }
        }

        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 5 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }

        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 10 + self.rng.range(40) as i32;
            if cy < h - 5 && self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }

        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }

        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1;
            if self.get(cx, cy) == Elem::Water && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Mud);
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Tin);
                    }
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.25) {
                        self.set_xy(x, y, Elem::Bronze);
                    }
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.35) {
                        self.set_xy(x, y, Elem::Iron);
                    }
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.2) {
                        self.set_xy(x, y, Elem::Steel);
                    }
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Silver);
                    }
            }
        }

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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Stone && self.rng.chance(0.5) {
                        self.set_xy(x, y, Elem::Ash);
                    }
            }
        }

        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2)
                && self.rng.chance(0.3) {
                    self.set_xy(cx, cy, Elem::Fungus);
                }
        }

        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2)
                && self.rng.chance(0.2) {
                    self.set_xy(cx, cy, Elem::Moss);
                }
        }

        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }

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

        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }

        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 3) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(20) as i32;
            let radius = (4 + self.rng.range(4)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.7) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5
                            && self.get(x, y) == Elem::Stone {
                                self.set_xy(x, y, Elem::Magma);
                            }
                    }
                }
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            let radius = (2 + self.rng.range(3)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.5) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5
                            && self.rng.chance(0.5) {
                                self.set_xy(x, y, Elem::Void);
                            }
                    }
                }
            }
        }

        for _ in 0..40 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if self.rng.chance(0.35) {
                        let x = cx + dx;
                        let y = cy + dy;

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

        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (self.rng.range(h as u32 / 2) as i32) + h / 4;
            let width = (6 + self.rng.range(8)) as i32;
            let height = (4 + self.rng.range(4)) as i32;
            for dy in 0..height {
                for dx in -width/2..=width/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5
                        && self.get(x, y) == Elem::Empty && self.rng.chance(0.4) {
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
                if x > 2 && x < w - 2 && y > 2 && y < h - 5
                    && self.get(x, y) == Elem::Empty && self.rng.chance(0.3) {
                        match self.rng.range(3) {
                            0 => self.set_xy(x, y, Elem::Wire),
                            1 => self.set_xy(x, y, Elem::Circuit),
                            _ => self.set_xy(x, y, Elem::Machine),
                        }
                    }
            }
        }

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

        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }

        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }

        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }

        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(8) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Corpse);
            }
        }

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

        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }
    }

}
