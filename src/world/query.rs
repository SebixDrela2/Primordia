use super::*;

impl World {
    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        !self.get(x, y).is_passable()
    }

    /// Find a good place to put a farm near `(x, y)`.
    ///
    /// A farm only works if it stands on open ground with water within
    /// reach, so the search looks for a surface cell that satisfies both.
    /// It returns the cell to build on and the cell to plant the first
    /// seedling in.
    pub fn find_farm_site(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32, i32, i32)> {
        // Try the cells closest to the creature first, so the farm ends up
        // near the colony rather than at the far edge of the world.
        for r in 1..=radius {
            for dy in -r..=r {
                for dx in -r..=r {
                    // Only the ring at this radius.
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 1 || ny < 1 || nx >= self.w as i32 - 1 || ny >= self.h as i32 - 1 {
                        continue;
                    }
                    // The plot itself must be free and stand on the ground.
                    if self.get(nx, ny) != Elem::Empty {
                        continue;
                    }
                    let surface = self.surface_y(nx);
                    // A farm belongs on the surface, not buried or floating.
                    if ny < surface - 1 || ny > surface + 4 {
                        continue;
                    }
                    // There must be ground under it.
                    if self.get(nx, ny + 1) == Elem::Empty {
                        continue;
                    }
                    // And there must be water within reach, or the plot is
                    // dead ground.
                    if !self.has_water_near(nx, ny, FARM_WATER_RADIUS) {
                        continue;
                    }
                    // Find a free cell next to the plot to sow the first
                    // seedling in.
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

    // Loose debris (powders, ice, organics) that a creature can shove aside or
    // dig through with its bare hands. Used so falling material can never
    // permanently entomb a creature.
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

    // Remove loose debris that settled on a creature's cell. Returns true if
    // anything was cleared.
    pub fn clear_burrow(&mut self, x: i32, y: i32) -> bool {
        let e = self.get(x, y);
        if e == Elem::Empty || e == Elem::Void {
            return false;
        }
        self.set_xy(x, y, Elem::Empty);
        true
    }

    // Eats the cell at (x,y). Returns (nutrition, toxicity) if something was eaten.
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

    // Nearest edible cell around (x,y) within `radius`, used by hungry creatures.
    // Only real food is targeted, never rock/ore, so the terrain survives.
    pub fn find_food(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                // Direct indexing plus a lookup table: this runs for every
                // hungry creature every step.
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

    // Nearest water cell within `radius`, used when a creature is thirsty.
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

    // Nearest gatherable solid/resource cell within `radius`.
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

    // Leaves remains where a creature died: a corpse/meat for scavengers plus
    // a little energy, feeding the rest of the food web instead of nothing.
    pub fn scavenge(&mut self, x: i32, y: i32) {
        if self.get(x, y) == Elem::Empty {
            let remains = if self.rng.chance(0.5) { Elem::Corpse } else { Elem::Meat };
            self.set_xy(x, y, remains);
        }
        for _ in 0..4 {
            let dx = self.rng.range_i(-1, 2);
            let dy = self.rng.range_i(-1, 2);
            if self.get(x + dx, y + dy) == Elem::Empty {
                self.set_xy(x + dx, y + dy, Elem::Energy);
            }
        }
    }

    // Find an empty passable cell within `radius` of (x, y), scanning outward.
    // Used so creatures never spawn inside solid blocks or deep water.
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
