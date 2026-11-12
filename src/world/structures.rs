use super::*;

impl World {
    pub(crate) fn generate_underground_structures(&mut self, w: i32, h: i32) {
        // Ancient ruins
        for _ in 0..5 {
            let cx = self.rng.range((w / 2) as u32) as i32 + w / 4;
            let cy = self.surface_y(cx) + 25 + self.rng.range(60) as i32;
            let width = (8 + self.rng.range(6)) as i32;
            let height = (6 + self.rng.range(5)) as i32;
            for dy in 0..height {
                for dx in -width/2..=width/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                        if self.rng.chance(0.4) {
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

        // Crystal towers. The base is placed relative to the terrain, so the
        // tower grows out of the ground instead of hanging in the sky.
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 20 + self.rng.range(60) as i32;
            let height = (12 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let x = cx;
                let y = cy + dy;
                if y < h - 5 && self.get(x, y) == Elem::Empty && self.rng.chance(0.7) {
                    match self.rng.range(4) {
                        0 => self.set_xy(x, y, Elem::Crystal),
                        1 => self.set_xy(x, y, Elem::Glass),
                        2 => self.set_xy(x, y, Elem::Gem),
                        _ => self.set_xy(x, y, Elem::Mana),
                    }
                }
            }
        }

        // Ancient bridges
        for _ in 0..2 {
            let x1 = self.rng.range((w / 3) as u32) as i32 + w / 6;
            let x2 = self.rng.range((w / 3) as u32) as i32 + 2 * w / 3;
            // The bridge follows the terrain so it never floats above it.
            let y = self.surface_y((x1 + x2) / 2) + 12 + self.rng.range(40) as i32;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if y < h - 5 && self.get(x, y) == Elem::Empty && self.rng.chance(0.8) {
                    match self.rng.range(3) {
                        0 => self.set_xy(x, y, Elem::Bridge),
                        1 => self.set_xy(x, y, Elem::Brick),
                        _ => self.set_xy(x, y, Elem::Concrete),
                    }
                }
            }
        }

        // Dungeon entrances
        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Door);
                // Add some loot inside
                for dy in 1..=3 {
                    if self.get(x, y + dy) == Elem::Empty {
                        match self.rng.range(5) {
                            0 => self.set_xy(x, y + dy, Elem::Chest),
                            1 => self.set_xy(x, y + dy, Elem::Tool),
                            2 => self.set_xy(x, y + dy, Elem::Machine),
                            3 => self.set_xy(x, y + dy, Elem::Mana),
                            _ => self.set_xy(x, y + dy, Elem::Energy),
                        }
                    }
                }
            }
        }

        // The scattered remains of older settlements: a hearth still
        // burning somewhere, a totem raised by a tribe that is long gone.
        // These are what the first creatures find, and what they learn to
        // build for themselves.
        for _ in 0..5 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let y = surface - 1 - self.rng.range(2) as i32;
            if y < 2 || self.get(x, y) != Elem::Empty {
                continue;
            }
            match self.rng.range(4) {
                0 => self.set_xy(x, y, Elem::Hearth),
                1 => self.set_xy(x, y, Elem::Totem),
                2 => self.set_xy(x, y, Elem::Granary),
                _ => self.set_xy(x, y, Elem::Market),
            }
            // A little of the old settlement around it.
            for (dx, dy) in DIRS8 {
                if self.rng.chance(0.35) {
                    let px = x + dx;
                    let py = y + dy;
                    if py > 1 && self.get(px, py) == Elem::Empty {
                        self.set_xy(px, py, Elem::Brick);
                    }
                }
            }
        }
    }

    pub(crate) fn generate_surface_structures(&mut self, w: i32, h: i32) {
        // Ancient towers
        for _ in 0..3 {
            let cx = self.rng.range((w / 2) as u32) as i32 + w / 4;
            let surface = self.surface_y(cx);
            let height = (10 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let y = surface - dy;
                let x = cx;
                if y > 2 && y < h - 5 {
                    if self.rng.chance(0.8) {
                        match self.rng.range(4) {
                            0 => self.set_xy(x, y, Elem::Tower),
                            1 => self.set_xy(x, y, Elem::Brick),
                            2 => self.set_xy(x, y, Elem::Stone),
                            _ => self.set_xy(x, y, Elem::Wall),
                        }
                    }
                }
            }
            // Tower top
            if self.get(cx, surface - height) == Elem::Empty {
                self.set_xy(cx, surface - height, Elem::Crystal);
            }
        }

        // Ancient statues
        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                match self.rng.range(5) {
                    0 => self.set_xy(x, y, Elem::Tool),
                    1 => self.set_xy(x, y, Elem::Machine),
                    2 => self.set_xy(x, y, Elem::Tool),
                    3 => self.set_xy(x, y, Elem::Anvil),
                    _ => self.set_xy(x, y, Elem::Furnace),
                }
            }
        }

        // Geysers
        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Water);
                self.set_xy(x, y - 1, Elem::Steam);
                self.set_xy(x, y - 2, Elem::Steam);
            }
        }

        // Tar pits
        for _ in 0..2 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            for dy in 0..=2 {
                for dx in -2..=2 {
                    let px = x + dx;
                    let py = surface - dy;
                    if px > 2 && px < w - 2 && py > 2 && py < h - 5 {
                        if self.get(px, py) == Elem::Empty && self.rng.chance(0.5) {
                            self.set_xy(px, py, Elem::Oil);
                        }
                    }
                }
            }
        }

        // Mushroom circles
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(cx);
            let cy = surface - 1;
            let radius = (3 + self.rng.range(3)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius && self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            if self.get(x, y) == Elem::Empty {
                                match self.rng.range(4) {
                                    0 => self.set_xy(x, y, Elem::Fungus),
                                    1 => self.set_xy(x, y, Elem::Spore),
                                    2 => self.set_xy(x, y, Elem::Moss),
                                    _ => self.set_xy(x, y, Elem::Plant),
                                }
                            }
                        }
                    }
                }
            }
        }

        // Crystal formations
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) - 2;
            let height = (5 + self.rng.range(4)) as i32;
            for dy in 0..height {
                let y = cy - dy;
                let x = cx;
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.6) {
                    match self.rng.range(5) {
                        0 => self.set_xy(x, y, Elem::Crystal),
                        1 => self.set_xy(x, y, Elem::Gem),
                        2 => self.set_xy(x, y, Elem::Diamond),
                        3 => self.set_xy(x, y, Elem::Glass),
                        _ => self.set_xy(x, y, Elem::Mana),
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Simulation
    // ------------------------------------------------------------------

}
