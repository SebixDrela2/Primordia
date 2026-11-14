use super::*;

const PLANT_SPREAD: f32 = 0.003;

impl World {
    pub fn decay(&mut self, i: usize, e: Elem) {
        match e {
            Elem::Fire => self.set_at(i, Elem::Smoke),
            Elem::Steam => self.set_at(i, Elem::Water),
            _ => self.set_at(i, Elem::Empty),
        }
    }

    pub fn step_fall(&mut self, i: usize) {
        if self.try_move(i, 0, 1).is_some() {
            return;
        }
        let d = self.rng_sign();
        if self.try_move(i, d, 1).is_some() {
            return;
        }
        let _ = self.try_move(i, -d, 1);
    }

    pub fn step_flow(&mut self, i: usize) {
        if self.try_move(i, 0, 1).is_some() {
            return;
        }
        let d = self.rng_sign();
        if self.try_move(i, d, 1).is_some() {
            return;
        }
        if self.try_move(i, -d, 1).is_some() {
            return;
        }

        let mut cur = i;
        let spread = 1 + self.rng.range(3) as i32;
        for _ in 0..spread {
            match self.try_move(cur, d, 0) {
                Some(n) => cur = n,
                None => break,
            }
        }
    }

    pub fn step_rise(&mut self, i: usize) {
        if self.try_move(i, 0, -1).is_some() {
            return;
        }
        let d = self.rng_sign();
        if self.try_move(i, d, -1).is_some() {
            return;
        }
        if self.try_move(i, -d, -1).is_some() {
            return;
        }
        if self.rng.chance(0.4) {
            let _ = self.try_move(i, d, 0);
        }
    }

    pub fn step_diffuse(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        let _ = self.try_move(i, dx, dy);
    }

    pub fn step_burn(&mut self, i: usize) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        if y <= 0 {
            self.set_at(i, Elem::Smoke);
            return;
        }

        let mut air = false;
        for (dx, dy) in DIRS8 {
            if self.get(x + dx, y + dy) == Elem::Oxygen {
                air = true;
                if self.rng.chance(0.35) {
                    self.set_xy(x + dx, y + dy, Elem::Empty);
                }
            }
        }

        for (dx, dy) in DIRS8 {
            let t = self.get(x + dx, y + dy);
            let fuel = matches!(
                t,
                Elem::Wood | Elem::Oil | Elem::Tar | Elem::Plant | Elem::Spore | Elem::Gas
            );
            if fuel && self.rng.chance(0.14) {
                self.set_xy(x + dx, y + dy, Elem::Fire);
            }
        }

        let (limit, chance) = if air { (60, 0.04) } else { (10, 0.25) };
        if self.age[i] > limit && self.rng.chance(chance) {
            self.set_at(i, Elem::Smoke);
            return;
        }

        if self.try_move(i, 0, -1).is_some() {
            return;
        }
        let d = self.rng_sign();
        let _ = self.try_move(i, d, -1);
    }

    pub fn step_grow(&mut self, i: usize, e: Elem) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        let mut neighbors = 0;
        for (dx, dy) in DIRS8 {
            if self.get(x + dx, y + dy) == e {
                neighbors += 1;
            }
        }
        if neighbors >= 5 && self.rng.chance(0.004) {
            self.set_at(i, Elem::Empty);
            return;
        }

        for (dx, dy) in DIRS4 {
            if self.get(x + dx, y + dy) == Elem::Water && self.rng.chance(0.0015) {
                self.set_xy(x + dx, y + dy, e);
                return;
            }
        }

        if self.rng.chance(0.004) && self.get(x, y + 1) != Elem::Empty {
            if !self.has_water_near(x, y, 2) {
                return;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, -1)] {
                if self.get(x + dx, y + dy) == Elem::Empty {
                    self.set_xy(x + dx, y + dy, e);
                    return;
                }
            }
        }

        if self.rng.chance(PLANT_SPREAD) && self.get(x, y + 1) != Elem::Empty {
            for (dx, dy) in [(1, 0), (-1, 0)] {
                let nx = x + dx;
                let ny = y + dy;
                if self.get(nx, ny) == Elem::Empty && self.get(nx, ny + 1) != Elem::Empty {
                    self.set_xy(nx, ny, e);
                    return;
                }
            }
        }

        if matches!(e, Elem::Plant | Elem::Moss | Elem::Crop)
            && self.daylight > 0.3
            && self.rng.chance(0.012 * self.daylight)
        {
            let up = self.get(x, y - 1);
            if up == Elem::Empty {
                self.set_xy(x, y - 1, Elem::Oxygen);
                return;
            }
        }
    }

    pub fn step_cultivate(&mut self, i: usize) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        if !self.has_water_near(x, y, FARM_WATER_RADIUS) {
            return;
        }

        if self.rng.chance(0.08) {
            for (dx, dy) in DIRS8 {
                if self.get(x + dx, y + dy) == Elem::Plant {
                    self.set_xy(x + dx, y + dy, Elem::Crop);
                    return;
                }
            }
        }

        if self.rng.chance(0.08) {
            for (dx, dy) in DIRS8 {
                let nx = x + dx;
                let ny = y + dy;
                if self.get(nx, ny) == Elem::Empty && self.get(nx, ny + 1) != Elem::Empty {
                    self.set_xy(nx, ny, Elem::Plant);
                    return;
                }
            }
        }
    }

    pub fn has_water_near(&self, x: i32, y: i32, radius: i32) -> bool {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.get(x + dx, y + dy) == Elem::Water {
                    return true;
                }
            }
        }
        false
    }

    pub fn step_energy(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        if self.try_move(i, dx, dy).is_none() {
            let _ = self.try_move(i, -dx, -dy);
        }
    }

    pub fn rng_sign(&mut self) -> i32 {
        if self.rng.next_u32() & 1 == 0 {
            -1
        } else {
            1
        }
    }

}
