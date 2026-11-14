use super::*;

impl World {

    pub fn fresh_air(&self, x: i32, y: i32, radius: i32) -> f32 {
        let mut n = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if self.get(x + dx, y + dy) == Elem::Oxygen {
                    n += 1;
                }
            }
        }
        (n as f32 / ((radius * 2 + 1) * (radius * 2 + 1)) as f32 * 14.0).min(1.0)
    }

    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        !self.get(x, y).is_passable()
    }

    pub fn find_farm_site(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32, i32, i32)> {

        for r in 1..=radius {
            for dy in -r..=r {
                for dx in -r..=r {

                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 1 || ny < 1 || nx >= self.w as i32 - 1 || ny >= self.h as i32 - 1 {
                        continue;
                    }

                    if self.get(nx, ny) != Elem::Empty {
                        continue;
                    }
                    let surface = self.surface_y(nx);

                    if ny < surface - 1 || ny > surface + 4 {
                        continue;
                    }

                    if self.get(nx, ny + 1) == Elem::Empty {
                        continue;
                    }

                    if !self.has_water_near(nx, ny, FARM_WATER_RADIUS) {
                        continue;
                    }

                    for (ox, oy) in [(0, -1), (1, 0), (-1, 0), (0, 1)] {
                        let px = nx + ox;
                        let py = ny + oy;
                        if self.get(px, py) == Elem::Empty && self.get(px, py + 1) != Elem::Empty {
                            return Some((nx, ny, px, py));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn is_loose(e: Elem) -> bool {
        matches!(
            e,
            Elem::Sand
                | Elem::Sulfur
                | Elem::Salt
                | Elem::Ash
                | Elem::Dust
                | Elem::Gunpowder
                | Elem::Fertilizer
                | Elem::Mud
                | Elem::Ice
                | Elem::Plant
                | Elem::Moss
                | Elem::Spore
                | Elem::Roots
                | Elem::Wood
                | Elem::Meat
                | Elem::Bone
                | Elem::Leather
                | Elem::Food
                | Elem::Corpse
                | Elem::Egg
        )
    }

    pub fn clear_burrow(&mut self, x: i32, y: i32) -> bool {
        let e = self.get(x, y);
        if e == Elem::Empty || e == Elem::Void {
            return false;
        }
        self.set_xy(x, y, Elem::Empty);
        true
    }

    pub fn eat(&mut self, x: i32, y: i32) -> Option<(f32, f32)> {
        let i = self.idx_xy(x, y)?;
        let e = Elem::from_u8(self.kind[i]);
        if e == Elem::Empty || e == Elem::Void {
            return None;
        }
        let n = e.nutrition();
        let t = e.toxicity();
        if n <= 0.0 && t <= 0.0 {
            return None;
        }
        self.set_at(i, Elem::Empty);
        Some((n, t))
    }

    pub fn find_food(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }

                let sx = x + dx;
                let sy = y + dy;
                if sx < 0 || sy < 0 || sx >= self.w as i32 || sy >= self.h as i32 {
                    continue;
                }
                let k = self.kind[sy as usize * self.w + sx as usize];
                if k != Elem::Empty as u8 && self.food_table[k as usize] {
                    if best.map_or(true, |b| d2 < b.2) {
                        best = Some((sx, sy, d2));
                    }
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    pub fn find_zorg_food(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, f32, i32)> = None;
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let sx = x + dx;
                let sy = y + dy;
                if sx < 0 || sy < 0 || sx >= self.w as i32 || sy >= self.h as i32 {
                    continue;
                }
                let k = self.kind[sy as usize * self.w + sx as usize];
                if k == Elem::Empty as u8 || !self.zorg_food_table[k as usize] {
                    continue;
                }

                let touching = dx.abs() <= 1 && dy.abs() <= 1 && dy <= 0;
                if !touching && !Elem::from_u8(k).is_passable() {
                    continue;
                }
                let nutrition = self.zorg_nutrition_table[k as usize];

                let better = match best {
                    None => true,
                    Some((_, _, n, d)) => nutrition > n || (nutrition == n && d2 < d),
                };
                if better {
                    best = Some((sx, sy, nutrition, d2));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    pub fn eat_zorg(&mut self, x: i32, y: i32) -> Option<(f32, f32)> {
        let i = self.idx_xy(x, y)?;
        let e = Elem::from_u8(self.kind[i]);
        if e == Elem::Empty || e == Elem::Void {
            return None;
        }
        let n = self.zorg_nutrition_table[e as usize];
        if n <= 0.0 {
            return None;
        }
        self.set_at(i, Elem::Empty);

        Some((n, crate::creature::zorg_toxicity(e)))
    }

    pub fn find_water(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > radius * radius {
                    continue;
                }
                if self.get(x + dx, y + dy) == Elem::Water
                    && best.map_or(true, |b| d2 < b.2)
                {
                    best = Some((x + dx, y + dy, d2));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    pub fn find_resource(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let sx = x + dx;
                let sy = y + dy;
                if sx < 0 || sy < 0 || sx >= self.w as i32 || sy >= self.h as i32 {
                    continue;
                }
                let k = self.kind[sy as usize * self.w + sx as usize];
                if k != Elem::Empty as u8 && self.gatherable[k as usize] {
                    if best.map_or(true, |b| d2 < b.2) {
                        best = Some((sx, sy, d2));
                    }
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    pub fn scavenge(&mut self, x: i32, y: i32, species: u32) {
        if let Some(i) = self.idx_xy(x, y) {
            if self.kind[i] == Elem::Empty as u8 {
                self.set_at(i, Elem::Corpse);

                self.owner[i] = species.saturating_add(1).min(u8::MAX as u32) as u8;
            }
        }
    }

    pub fn owner_at(&self, x: i32, y: i32) -> Option<u32> {
        match self.idx_xy(x, y) {
            Some(i) => match self.owner[i] {
                0 => None,
                o => Some(o as u32 - 1),
            },
            None => None,
        }
    }

    pub fn find_empty_passable_near(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let w = self.w as i32;
        let h = self.h as i32;
        for r in 1..=radius {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let e = self.get(nx, ny);
                    if e == Elem::Empty || e.is_passable() {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }
}
