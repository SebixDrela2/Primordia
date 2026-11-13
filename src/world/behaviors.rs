use super::*;

impl World {
    pub(crate) fn decay(&mut self, i: usize, e: Elem) {
        match e {
            Elem::Fire => self.set_at(i, Elem::Smoke),
            Elem::Steam => self.set_at(i, Elem::Water),
            _ => self.set_at(i, Elem::Empty),
        }
    }

    pub(crate) fn step_fall(&mut self, i: usize) {
        if self.try_move(i, 0, 1).is_some() {
            return;
        }
        let d = self.rng_sign();
        if self.try_move(i, d, 1).is_some() {
            return;
        }
        let _ = self.try_move(i, -d, 1);
    }

    pub(crate) fn step_flow(&mut self, i: usize) {
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

        // Lateral spreading
        let mut cur = i;
        let spread = 1 + self.rng.range(3) as i32;
        for _ in 0..spread {
            match self.try_move(cur, d, 0) {
                Some(n) => cur = n,
                None => break,
            }
        }
    }

    pub(crate) fn step_rise(&mut self, i: usize) {
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

    pub(crate) fn step_diffuse(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        let _ = self.try_move(i, dx, dy);
    }

    pub(crate) fn step_burn(&mut self, i: usize) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        // Fire needs something to burn. At the very top of the world there is
        // nothing but thin air, so the flame goes out instead of hovering at
        // row 0 for its whole lifetime.
        if y <= 0 {
            self.set_at(i, Elem::Smoke);
            return;
        }

        // ---- Fire breathes ----
        //
        // A flame takes the oxygen out of the air around it and puts
        // nothing back. A fire in the open air has a steady supply and
        // burns for a long time; a fire in a sealed cave eats its own air
        // and goes out. It is the reason the deep is a cold place.
        let mut air = false;
        for (dx, dy) in DIRS8 {
            if self.get(x + dx, y + dy) == Elem::Oxygen {
                air = true;
                if self.rng.chance(0.35) {
                    self.set_xy(x + dx, y + dy, Elem::Empty);
                }
            }
        }

        // Ignite fuel nearby
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

        // Fire burns out faster when it has nothing to eat, and much
        // faster when it has no air to breathe. A fire with air lasts
        // three times as long as one without it.
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

    pub(crate) fn step_grow(&mut self, i: usize, e: Elem) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        // Crowding: dense vegetation slowly dies back so it never fills the sky.
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

        // Plants convert nearby water into more plant (needs moisture).
        for (dx, dy) in DIRS4 {
            if self.get(x + dx, y + dy) == Elem::Water && self.rng.chance(0.0015) {
                self.set_xy(x + dx, y + dy, e);
                return;
            }
        }

        // Slow upward growth into air, only when there is water nearby.
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

        // ---- A plant breathes ----
        //
        // In the daylight a plant puts a little oxygen into the air above
        // it. That is where the fresh air of the surface comes from, and
        // it is why the deep is a stale, breathless place: nothing down
        // there ever makes any.
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

    /// A farm tends its plot.
    ///
    /// This is the whole farming mechanic. A farm:
    ///
    /// 1. **sows** a seedling into a free cell next to it, on solid ground;
    /// 2. **ripens** the seedlings it grew into crops.
    ///
    /// A crop is worth more food than the wild plant it grew from, and a
    /// farm keeps producing as long as there is water nearby. So a colony
    /// that builds farms turns the plants around its base into a
    /// renewable food supply, and it can do so even after the wild
    /// vegetation has been eaten bare.
    ///
    /// A farm with no water within reach is dormant and does nothing.
    pub(crate) fn step_cultivate(&mut self, i: usize) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        // A farm needs water to work. Without it the plot is dead ground.
        // The radius matches the one used when the plot is chosen, so a
        // farm that can be built is always a farm that can grow.
        if !self.has_water_near(x, y, FARM_WATER_RADIUS) {
            return;
        }

        // ---- Ripen -------------------------------------------------------
        // Turn one of the seedlings around the plot into a ripe crop.
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS8 {
                if self.get(x + dx, y + dy) == Elem::Plant {
                    self.set_xy(x + dx, y + dy, Elem::Crop);
                    return;
                }
            }
        }

        // ---- Sow ---------------------------------------------------------
        // Plant a new seedling in a free cell that has ground under it.
        if self.rng.chance(0.05) {
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

    pub(crate) fn has_water_near(&self, x: i32, y: i32, radius: i32) -> bool {
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

    pub(crate) fn step_energy(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        if self.try_move(i, dx, dy).is_none() {
            let _ = self.try_move(i, -dx, -dy);
        }
    }

    pub(crate) fn rng_sign(&mut self) -> i32 {
        if self.rng.next_u32() & 1 == 0 {
            -1
        } else {
            1
        }
    }

    // Complex elemental reactions.
}
