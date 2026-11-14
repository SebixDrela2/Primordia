use super::*;

pub const ZORG_CAVE_RX: i32 = 32;

pub const ZORG_CAVE_RY: i32 = 8;

const MIN_DEPTH: i32 = 150;

const ROCK_BELOW: i32 = 40;

const ZORG_LARDER_TARGET: u32 = 200;

const ZORG_LARDER_DEPTH: i32 = 4;

fn is_zorg_poison(e: Elem) -> bool {
    matches!(
        e,
        Elem::Acid | Elem::Poison | Elem::Tar | Elem::Mercury | Elem::Oil | Elem::Sulfur | Elem::Chlorine
    )
}

impl World {

    pub fn carve_zorg_cave(&mut self, w: i32, h: i32) {

        let floor_limit = h - ROCK_BELOW;

        let cx = (w as f32 * (0.34 + self.rng.f32() * 0.32)) as i32;

        let surface = self.surface_y(cx);
        let deepest = floor_limit - ZORG_CAVE_RY;
        let cy = deepest.max(surface + MIN_DEPTH);

        if cy - ZORG_CAVE_RY <= surface + 8 || cy + ZORG_CAVE_RY > floor_limit || cy <= 0 {
            return;
        }

        let plateau_half = ZORG_CAVE_RX * 72 / 100;
        let plateau_y = cy + ZORG_CAVE_RY / 2;
        for dx in -ZORG_CAVE_RX..=ZORG_CAVE_RX {
            let t = dx as f32 / ZORG_CAVE_RX as f32;

            let wobble = 1.0 + 0.16 * (t * 9.0).sin() + 0.10 * (t * 15.0 + 1.7).cos();
            let half = ((1.0 - t * t).max(0.0).sqrt() * wobble).clamp(0.0, 1.0);
            let top = cy - (ZORG_CAVE_RY as f32 * half) as i32;

            let bottom = if dx.abs() <= plateau_half {
                plateau_y
            } else {
                cy + (ZORG_CAVE_RY as f32 * half) as i32
            };
            if bottom < top {
                continue;
            }
            let x = cx + dx;
            if x <= 1 || x >= w - 2 {
                continue;
            }
            for y in top.max(1)..=bottom.min(h - 7) {

                let e = self.get(x, y);
                if e == Elem::Void || e == Elem::Water {
                    continue;
                }
                self.set_xy(x, y, Elem::Empty);
            }

            let below = bottom + 1;
            if below < h - 6 {
                let e = self.get(x, below);
                if e == Elem::Empty || e == Elem::Void {
                    self.set_xy(x, below, Elem::Granite);
                }
            }
        }

        let passage_limit = surface + MIN_DEPTH / 2;
        for k in 0..2 {
            let side = if k == 0 { -1.0 } else { 1.0 };
            let sx = cx + (side * ZORG_CAVE_RX as f32 * 0.55) as i32;
            let sy = cy - ZORG_CAVE_RY / 2;
            let mut x = sx;
            let mut y = sy;
            let mut angle = -std::f32::consts::FRAC_PI_2 + side * 0.5;
            let len = 30 + self.rng.range(20) as i32;
            for _ in 0..len {
                angle += (self.rng.f32() - 0.5) * 0.8;
                x += (angle.cos() * 2.0) as i32;
                y += (angle.sin() * 2.0) as i32;

                if y < passage_limit {
                    break;
                }
                let radius = 1 + self.rng.range(2) as i32;
                for py in (y - radius).max(1)..(y + radius).min(h - 8) {
                    for px in (x - radius).max(1)..(x + radius).min(w - 2) {
                        if px <= 1 || py <= 1 || px >= w - 2 || py >= h - 6 {
                            continue;
                        }
                        let e = self.get(px, py);
                        if e == Elem::Void {
                            continue;
                        }
                        self.set_xy(px, py, Elem::Empty);
                    }
                }

                if y + 1 < h - 6 && self.get(x, y + 1) == Elem::Empty {
                    self.set_xy(x, y + 1, Elem::Stone);
                }
            }
        }

        let poisons = [
            Elem::Acid,
            Elem::Poison,
            Elem::Tar,
            Elem::Mercury,
            Elem::Oil,
            Elem::Sulfur,
            Elem::Chlorine,
        ];
        for dx in -plateau_half..=plateau_half {
            for dy in 0..ZORG_LARDER_DEPTH {
                let x = cx + dx;
                let y = plateau_y - dy;
                if x <= 1 || y <= 1 || x >= w - 2 || y >= h - 6 {
                    continue;
                }
                if self.get(x, y) != Elem::Empty {
                    continue;
                }
                let e = poisons[self.rng.range(poisons.len() as u32) as usize];
                self.set_xy(x, y, e);
            }
        }

        for _ in 0..60 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let dy = self.rng.range_i(-ZORG_CAVE_RY, ZORG_CAVE_RY + 1);
            let x = cx + dx;
            let y = cy + dy;
            if x <= 1 || y <= 1 || x >= w - 2 || y >= h - 6 {
                continue;
            }
            if self.get(x, y) != Elem::Empty {
                continue;
            }

            if self.get(x, y + 1).is_solid() {
                let e = poisons[self.rng.range(poisons.len() as u32) as usize];
                self.set_xy(x, y, e);
            }
        }

        let ores = [
            Elem::Stone,
            Elem::Granite,
            Elem::Iron,
            Elem::Copper,
            Elem::Gold,
            Elem::Silver,
            Elem::Crystal,
            Elem::Gem,
            Elem::Diamond,
            Elem::Obsidian,
            Elem::Crystal,
        ];
        for _ in 0..260 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX - 2, ZORG_CAVE_RX + 3);
            let dy = self.rng.range_i(-ZORG_CAVE_RY - 2, ZORG_CAVE_RY + 3);
            let x = cx + dx;
            let y = cy + dy;
            if x <= 1 || y <= 1 || x >= w - 2 || y >= h - 6 {
                continue;
            }
            let e = self.get(x, y);
            if !e.is_solid() {
                continue;
            }

            let open_neighbour = DIRS8
                .iter()
                .any(|(dx, dy)| self.get(x + dx, y + dy) == Elem::Empty);
            if !open_neighbour {
                continue;
            }
            let o = ores[self.rng.range(ores.len() as u32) as usize];
            self.set_xy(x, y, o);
        }

        for _ in 0..26 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let dy = self.rng.range_i(-ZORG_CAVE_RY, ZORG_CAVE_RY + 1);
            let x = cx + dx;
            let y = cy + dy;
            if x <= 1 || y <= 1 || x >= w - 2 || y >= h - 6 {
                continue;
            }
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Crystal);
            }
        }

        self.zorg_cave = Some((cx, cy));
        self.zorg_cave_floor = plateau_y;
    }

    pub fn zorg_cave_floor_row(&self) -> i32 {
        self.zorg_cave_floor
    }

    pub fn zorg_cave_center(&self) -> Option<(i32, i32)> {
        self.zorg_cave
    }

    pub fn regrow_zorg_larder(&mut self) {
        let Some((cx, cy)) = self.zorg_cave else {
            return;
        };

        let mut have = 0u32;
        for y in (cy - ZORG_CAVE_RY)..(cy + ZORG_CAVE_RY) {
            for x in (cx - ZORG_CAVE_RX)..(cx + ZORG_CAVE_RX) {
                if is_zorg_poison(self.get(x, y)) {
                    have += 1;
                }
            }
        }
        if have >= ZORG_LARDER_TARGET {
            return;
        }

        for _ in 0..20 {
            let x = cx + self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let y = cy + self.rng.range_i(-ZORG_CAVE_RY / 2, ZORG_CAVE_RY + 1);
            if x <= 1 || y <= 1 || x >= self.w as i32 - 2 || y >= self.h as i32 - 6 {
                continue;
            }

            if self.get(x, y) != Elem::Empty || !self.get(x, y + 1).is_solid() {
                continue;
            }
            let e = match self.rng.range(5) {
                0 => Elem::Acid,
                1 => Elem::Poison,
                2 => Elem::Tar,
                3 => Elem::Mercury,
                _ => Elem::Sulfur,
            };
            self.set_xy(x, y, e);
        }
    }

    pub fn in_zorg_cave(&self, x: i32, y: i32) -> bool {
        let Some((cx, cy)) = self.zorg_cave else {
            return false;
        };
        let dx = (x - cx) as f32 / (ZORG_CAVE_RX as f32 * 1.15);
        let dy = (y - cy) as f32 / (ZORG_CAVE_RY as f32 * 1.15);
        dx * dx + dy * dy <= 1.0
    }

    pub fn find_zorg_cave_cell(&self, cx: i32, cy: i32) -> Option<(i32, i32)> {
        let w = self.w as i32;
        let h = self.h as i32;
        let mut fallback: Option<(i32, i32)> = None;

        for r in 0..(ZORG_CAVE_RY + 6) {
            let step = if r < 4 { 1 } else { 3 };
            let mut dy = -r;
            while dy <= r {
                let mut dx = -r;
                while dx <= r {
                    if dx.abs() != r && dy.abs() != r {
                        dx += step;
                        continue;
                    }
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 0 && y > 0 && x < w && y < h && self.in_zorg_cave(x, y) {
                        if let Some(cell) = self.standable_cell(x, y, &mut fallback) {
                            return Some(cell);
                        }
                    }
                    dx += step;
                }
                dy += step;
            }
        }

        if let Some((ccx, ccy)) = self.zorg_cave {
            for y in (ccy - ZORG_CAVE_RY)..=(ccy + ZORG_CAVE_RY) {
                for x in (ccx - ZORG_CAVE_RX)..=(ccx + ZORG_CAVE_RX) {
                    if x <= 0 || y <= 0 || x >= w || y >= h {
                        continue;
                    }
                    if !self.in_zorg_cave(x, y) {
                        continue;
                    }
                    if let Some(cell) = self.standable_cell(x, y, &mut fallback) {
                        return Some(cell);
                    }
                }
            }
        }
        fallback
    }

    fn standable_cell(
        &self,
        x: i32,
        y: i32,
        fallback: &mut Option<(i32, i32)>,
    ) -> Option<(i32, i32)> {
        let here = self.get(x, y);
        if here != Elem::Empty && !here.is_passable() {
            return None;
        }
        let under = self.get(x, y + 1);
        if under.is_solid() || crate::creature::zorg_food(under) {
            return Some((x, y));
        }
        if fallback.is_none() {
            *fallback = Some((x, y));
        }
        None
    }
}
