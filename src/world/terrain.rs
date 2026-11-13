use super::*;

impl World {
    pub fn surface_y(&self, x: i32) -> i32 {
        let h = self.h as f32;
        let x = x as f32;
        let wobble = (x * 0.045).sin() * 5.0
            + (x * 0.013 + 1.7).sin() * 9.0
            + (x * 0.11 + 0.4).sin() * 2.0;
        // The surface is never above the top of the world and never below
        // the bedrock. The bounds are ordered so that a very small world
        // (a test world, say) still gets a sane answer instead of a panic.
        let lo = 24.0f32.min(h - 4.0);
        let hi = (h - 26.0).max(lo);
        (h * 0.66 + wobble).clamp(lo, hi) as i32
    }

    // Queue a cell for processing if it can still change.
    #[inline]
    pub(crate) fn mark_active(&mut self, i: usize) {
        let k = self.kind[i];
        if k != Elem::Empty as u8 && !self.inert[k as usize] && !self.listed[i] {
            self.listed[i] = true;
            self.next_active.push(i as u32);
        }
    }

    // Rebuild the full list of active cells (used after seeding).
    pub fn rebuild_active(&mut self) {
        // Fill the queue, not the work list: the next step moves the queue
        // into the work list.
        self.next_active.clear();
        for i in 0..self.listed.len() {
            self.listed[i] = false;
            let k = self.kind[i];
            if k != Elem::Empty as u8 && !self.inert[k as usize] {
                self.listed[i] = true;
                self.next_active.push(i as u32);
            }
        }
    }

    // Recompute the cached ground level of every column: the topmost solid
    // cell that actually rests on something. Using the real terrain (rather
    // than the analytic surface) keeps eroded gaps and crater floors from
    // being painted as black voids.
    pub fn update_ground(&mut self) {
        let solid = Elem::solid_table();
        let empty = Elem::Empty as u8;
        for x in 0..self.w {
            let mut g = self.h as i32 - 1;
            let mut found = false;
            for y in 0..self.h {
                let k = self.kind[y * self.w + x];
                if k != empty && solid[k as usize] {
                    let below = if y + 1 < self.h {
                        self.kind[(y + 1) * self.w + x]
                    } else {
                        k
                    };
                    if below != empty && solid[below as usize] {
                        g = y as i32;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                for y in 0..self.h {
                    let k = self.kind[y * self.w + x];
                    if k != empty && solid[k as usize] {
                        g = y as i32;
                        break;
                    }
                }
            }
            self.ground[x] = g;
            self.ground_inv[x] = 1.0 / (g.max(1) as f32);
        }
    }

    // Which climate band a column belongs to.
    pub fn biome_at(&self, x: i32) -> Biome {
        if x < 0 {
            return Biome::Tundra;
        }
        if (x as usize) < self.biome.len() {
            return self.biome[x as usize];
        }
        Biome::Grassland
    }

    // Smoothly blended biome colour for a column, so the borders between
    // climate bands fade instead of snapping.
    pub fn biome_tint_at(&self, x: i32) -> [f32; 3] {
        if x < 0 {
            return Biome::Tundra.tint();
        }
        if (x as usize) < self.biome_tint.len() {
            return self.biome_tint[x as usize];
        }
        Biome::Grassland.tint()
    }

    // Build the cached per-column biome layout. Called once when the world
    // is seeded; the layout never changes afterwards.
    pub fn cache_biomes(&mut self) {
        for x in 0..self.w {
            self.biome[x] = biome_of_column(x as i32, self.w as i32);
            self.biome_tint[x] = biome_tint_of_column(x as i32, self.w as i32);
        }
    }
}

/// The climate band a column belongs to, as a pure function of the column
/// index and the world width.
fn biome_of_column(x: i32, w: i32) -> Biome {
    let f = x as f32 / w as f32;
    // A few wide, wavy bands so the map reads as a journey.
    let v = ((f * 5.0 + (f * 2.3).sin() * 0.7).sin() * 0.5 + 0.5) as f32;
    if v < 0.13 {
        Biome::Tundra
    } else if v < 0.25 {
        Biome::Ashland
    } else if v < 0.37 {
        Biome::Forest
    } else if v < 0.50 {
        Biome::Bloomwood
    } else if v < 0.63 {
        Biome::Grassland
    } else if v < 0.77 {
        Biome::Swamp
    } else if v < 0.90 {
        Biome::Mire
    } else {
        Biome::Desert
    }
}

/// The blended tint of a column, as a pure function of the column index.
fn biome_tint_of_column(x: i32, w: i32) -> [f32; 3] {
    let f = x as f32 / w as f32;
    let v = ((f * 5.0 + (f * 2.3).sin() * 0.7).sin() * 0.5 + 0.5) as f32;
    // Representative positions of each band, in the same v space.
    const POS: [f32; 8] = [0.07, 0.19, 0.31, 0.44, 0.57, 0.70, 0.84, 0.95];
    let tints = [
        Biome::Tundra.tint(),
        Biome::Ashland.tint(),
        Biome::Forest.tint(),
        Biome::Bloomwood.tint(),
        Biome::Grassland.tint(),
        Biome::Swamp.tint(),
        Biome::Mire.tint(),
        Biome::Desert.tint(),
    ];
    if v <= POS[0] {
        return tints[0];
    }
    for i in 0..7 {
        if v <= POS[i + 1] {
            let t = (v - POS[i]) / (POS[i + 1] - POS[i]);
            let a = tints[i];
            let b = tints[i + 1];
            return [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ];
        }
    }
    tints[4]
}
