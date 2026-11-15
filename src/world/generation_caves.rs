use super::{World, Elem};

/// How deep the caves start, below the surface.
const CAVE_DEPTH: i32 = 14;

/// The size of one cell of the cave seed grid.
const CAVE_CELL: usize = 24;

impl World {
    /// The caves: a wandering tunnel out of every seed point, eroded
    /// so that only the rooms with enough neighbours survive.
    pub fn carve_caves(&mut self, w: i32, h: i32) {
        let gw = (w as usize).div_ceil(CAVE_CELL) + 1;
        let gh = (h as usize).div_ceil(CAVE_CELL) + 1;

        let seeds = self.cave_seeds(h, gw, gh);
        let cave = self.carve_cave_tunnels(w, h, &seeds);
        let eroded = erode_cave_map(&cave, w, h);
        self.open_cave_cells(&eroded, w, h);
    }

    /// Which cells of the coarse grid want a cave.
    fn cave_seeds(&mut self, h: i32, gw: usize, gh: usize) -> Vec<bool> {
        let mut seeds = vec![false; gw * gh];
        for gy in 0..gh {
            for gx in 0..gw {
                let wy = (gy * CAVE_CELL) as i32;
                let wx = (gx * CAVE_CELL) as i32;

                let top = self.surface_y(wx) + CAVE_DEPTH;
                let bottom = (h - 24).max(top + 1);
                if wy < top || wy > bottom {
                    continue;
                }

                let depth = (wy - top).max(0) as f32;
                let chance = 0.30 - (depth / (bottom - top).max(1) as f32) * 0.12;
                if self.rng.chance(chance) {
                    seeds[gy * gw + gx] = true;
                }
            }
        }
        seeds
    }

    /// The tunnel out of every seed point.
    fn carve_cave_tunnels(&mut self, w: i32, h: i32, seeds: &[bool]) -> Vec<bool> {
        let cw = w as usize;
        let gw = (w as usize).div_ceil(CAVE_CELL) + 1;
        let gh = (h as usize).div_ceil(CAVE_CELL) + 1;

        let mut cave = vec![false; cw * h as usize];
        for gy in 0..gh {
            for gx in 0..gw {
                if !seeds[gy * gw + gx] {
                    continue;
                }
                let cx = (gx * CAVE_CELL) as i32;
                let cy = (gy * CAVE_CELL) as i32;

                let mut tx = cx;
                let mut ty = cy;
                let mut angle = self.rng.f32() * std::f32::consts::TAU;
                let len = 14 + self.rng.range(26) as i32;
                for _ in 0..len {
                    angle += (self.rng.f32() - 0.5) * 1.1;
                    tx += (angle.cos() * 2.0) as i32;
                    ty += (angle.sin() * 2.0) as i32;
                    let radius = 1 + self.rng.range(3) as i32;

                    for y in (ty - radius).max(0)..(ty + radius).min(h) {
                        for x in (tx - radius).max(0)..(tx + radius).min(w) {
                            let dx = (x - tx) as f32 / (radius + 1) as f32;
                            let dy = (y - ty) as f32 / (radius + 1) as f32;
                            if dx * dx + dy * dy <= 1.0 {
                                cave[y as usize * cw + x as usize] = true;
                            }
                        }
                    }
                }

                let rr = 3 + self.rng.range(3) as i32;
                for y in (cy - rr).max(0)..(cy + rr).min(h) {
                    for x in (cx - rr).max(0)..(cx + rr).min(w) {
                        let dx = (x - cx) as f32 / rr as f32;
                        let dy = (y - cy) as f32 / rr as f32;
                        if dx * dx + dy * dy <= 1.0 {
                            cave[y as usize * cw + x as usize] = true;
                        }
                    }
                }
            }
        }
        cave
    }

    /// The caves that are too thin to stand in fall in.
    fn open_cave_cells(&mut self, eroded: &[bool], w: i32, h: i32) {
        let cw = w as usize;
        for x in 0..w {
            let surf = self.surface_y(x);
            let floor = surf + CAVE_DEPTH;
            for y in floor..(h - 4) {
                let i = y as usize * cw + x as usize;
                if !eroded[i] {
                    continue;
                }

                let below = self.get(x, y + 1);
                if below == Elem::Void || below == Elem::Water {
                    continue;
                }
                let e = self.get(x, y);
                if e == Elem::Stone || e == Elem::Sand || e == Elem::Mud || e == Elem::Granite {
                    self.set_xy(x, y, Elem::Empty);
                }
            }
        }
    }
}

/// Take the cave map and knock out the passages that have too few
/// neighbours: a cave needs solid rock around it to stand up.
fn erode_cave_map(cave: &[bool], w: i32, h: i32) -> Vec<bool> {
    let cw = w as usize;
    let mut eroded = cave.to_vec();
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let i = y as usize * cw + x as usize;
            if !cave[i] {
                continue;
            }
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if cave[(y + dy) as usize * cw + (x + dx) as usize] {
                        n += 1;
                    }
                }
            }
            if n < 4 {
                eroded[i] = false;
            }
        }
    }
    eroded
}
