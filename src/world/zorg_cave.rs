use super::{Elem, World, DIRS8};

pub const ZORG_CAVE_RX: i32 = 32;

pub const ZORG_CAVE_RY: i32 = 8;

const MIN_DEPTH: i32 = 150;

const ROCK_BELOW: i32 = 40;

const ZORG_LARDER_TARGET: u32 = 200;

const ZORG_LARDER_DEPTH: i32 = 4;

/// The poisons the ZORGs live on.
const ZORG_POISONS: [Elem; 7] = [
    Elem::Acid,
    Elem::Poison,
    Elem::Tar,
    Elem::Mercury,
    Elem::Oil,
    Elem::Sulfur,
    Elem::Chlorine,
];

/// The rock and the ore of the deep, as the ZORGs see it.
const ZORG_ORES: [Elem; 11] = [
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

fn is_zorg_poison(elem: Elem) -> bool {
    matches!(
        elem,
        Elem::Acid | Elem::Poison | Elem::Tar | Elem::Mercury | Elem::Oil | Elem::Sulfur | Elem::Chlorine
    )
}

impl World {

    /// The cave of the ZORGs: a chamber in the deepest rock, with a
    /// passage down to it, a larder of poison and its own ore.
    pub fn carve_zorg_cave(&mut self, width: i32, height: i32) {
        let floor_limit = height - ROCK_BELOW;

        let cave_x = (width as f32 * (0.34 + self.rng.f32() * 0.32)) as i32;

        let surface = self.surface_y(cave_x);
        let deepest = floor_limit - ZORG_CAVE_RY;
        let cave_y = deepest.max(surface + MIN_DEPTH);

        if cave_y - ZORG_CAVE_RY <= surface + 8 || cave_y + ZORG_CAVE_RY > floor_limit || cave_y <= 0 {
            return;
        }

        let plateau_half = ZORG_CAVE_RX * 72 / 100;
        let plateau_y = cave_y + ZORG_CAVE_RY / 2;

        self.carve_zorg_chamber(width, height, cave_x, cave_y, plateau_y, plateau_half);
        self.carve_zorg_passages(width, height, cave_x, cave_y, surface);
        self.fill_zorg_larder(width, height, cave_x, plateau_y, plateau_half);
        self.scatter_zorg_poison(width, height, cave_x, cave_y);
        self.scatter_zorg_ores(width, height, cave_x, cave_y);
        self.scatter_zorg_crystals(width, height, cave_x, cave_y);

        self.zorg_cave = Some((cave_x, cave_y));
        self.zorg_cave_floor = plateau_y;
    }

    /// The chamber itself: an oval of empty rock with a granite floor.
    fn carve_zorg_chamber(
        &mut self,
        width: i32,
        height: i32,
        cave_x: i32,
        cave_y: i32,
        plateau_y: i32,
        plateau_half: i32,
    ) {
        for dx in -ZORG_CAVE_RX..=ZORG_CAVE_RX {
            let ratio = dx as f32 / ZORG_CAVE_RX as f32;

            let wobble = 1.0 + 0.16 * (ratio * 9.0).sin() + 0.10 * (ratio * 15.0 + 1.7).cos();
            let half = ((1.0 - ratio * ratio).max(0.0).sqrt() * wobble).clamp(0.0, 1.0);
            let top = cave_y - (ZORG_CAVE_RY as f32 * half) as i32;

            let bottom = if dx.abs() <= plateau_half {
                plateau_y
            } else {
                cave_y + (ZORG_CAVE_RY as f32 * half) as i32
            };
            if bottom < top {
                continue;
            }
            let cell_x = cave_x + dx;
            if cell_x <= 1 || cell_x >= width - 2 {
                continue;
            }
            for cell_y in top.max(1)..=bottom.min(height - 7) {

                let elem = self.get(cell_x, cell_y);
                if elem == Elem::Void || elem == Elem::Water {
                    continue;
                }
                self.set_xy(cell_x, cell_y, Elem::Empty);
            }

            let below = bottom + 1;
            if below < height - 6 {
                let elem = self.get(cell_x, below);
                if elem == Elem::Empty || elem == Elem::Void {
                    self.set_xy(cell_x, below, Elem::Granite);
                }
            }
        }
    }

    /// The passages that lead down to the chamber.
    fn carve_zorg_passages(
        &mut self,
        width: i32,
        height: i32,
        cave_x: i32,
        cave_y: i32,
        surface: i32,
    ) {
        let passage_limit = surface + MIN_DEPTH / 2;
        for side_index in 0..2 {
            let side = if side_index == 0 { -1.0 } else { 1.0 };
            let start_x = cave_x + (side * ZORG_CAVE_RX as f32 * 0.55) as i32;
            let start_y = cave_y - ZORG_CAVE_RY / 2;
            let mut walk_x = start_x;
            let mut walk_y = start_y;
            let mut angle = -std::f32::consts::FRAC_PI_2 + side * 0.5;
            let len = 30 + self.rng.range(20) as i32;
            for _ in 0..len {
                angle += (self.rng.f32() - 0.5) * 0.8;
                walk_x += (angle.cos() * 2.0) as i32;
                walk_y += (angle.sin() * 2.0) as i32;

                if walk_y < passage_limit {
                    break;
                }
                let radius = 1 + self.rng.range(2) as i32;
                for py in (walk_y - radius).max(1)..(walk_y + radius).min(height - 8) {
                    for px in (walk_x - radius).max(1)..(walk_x + radius).min(width - 2) {
                        if px <= 1 || py <= 1 || px >= width - 2 || py >= height - 6 {
                            continue;
                        }
                        let elem = self.get(px, py);
                        if elem == Elem::Void {
                            continue;
                        }
                        self.set_xy(px, py, Elem::Empty);
                    }
                }

                if walk_y + 1 < height - 6 && self.get(walk_x, walk_y + 1) == Elem::Empty {
                    self.set_xy(walk_x, walk_y + 1, Elem::Stone);
                }
            }
        }
    }

    /// The larder: the poison the ZORGs eat, on the plateau floor.
    fn fill_zorg_larder(
        &mut self,
        width: i32,
        height: i32,
        cave_x: i32,
        plateau_y: i32,
        plateau_half: i32,
    ) {
        for dx in -plateau_half..=plateau_half {
            for dy in 0..ZORG_LARDER_DEPTH {
                let cell_x = cave_x + dx;
                let cell_y = plateau_y - dy;
                if cell_x <= 1 || cell_y <= 1 || cell_x >= width - 2 || cell_y >= height - 6 {
                    continue;
                }
                if self.get(cell_x, cell_y) != Elem::Empty {
                    continue;
                }
                let elem = ZORG_POISONS[self.rng.range(ZORG_POISONS.len() as u32) as usize];
                self.set_xy(cell_x, cell_y, elem);
            }
        }
    }

    /// The poison that is scattered over the floor of the cave.
    fn scatter_zorg_poison(&mut self, width: i32, height: i32, cave_x: i32, cave_y: i32) {
        for _ in 0..60 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let dy = self.rng.range_i(-ZORG_CAVE_RY, ZORG_CAVE_RY + 1);
            let cell_x = cave_x + dx;
            let cell_y = cave_y + dy;
            if cell_x <= 1 || cell_y <= 1 || cell_x >= width - 2 || cell_y >= height - 6 {
                continue;
            }
            if self.get(cell_x, cell_y) != Elem::Empty {
                continue;
            }

            if self.get(cell_x, cell_y + 1).is_solid() {
                let elem = ZORG_POISONS[self.rng.range(ZORG_POISONS.len() as u32) as usize];
                self.set_xy(cell_x, cell_y, elem);
            }
        }
    }

    /// The ore that shows in the walls of the cave.
    fn scatter_zorg_ores(&mut self, width: i32, height: i32, cave_x: i32, cave_y: i32) {
        for _ in 0..260 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX - 2, ZORG_CAVE_RX + 3);
            let dy = self.rng.range_i(-ZORG_CAVE_RY - 2, ZORG_CAVE_RY + 3);
            let cell_x = cave_x + dx;
            let cell_y = cave_y + dy;
            if cell_x <= 1 || cell_y <= 1 || cell_x >= width - 2 || cell_y >= height - 6 {
                continue;
            }
            let elem = self.get(cell_x, cell_y);
            if !elem.is_solid() {
                continue;
            }

            let open_neighbour = DIRS8
                .iter()
                .any(|(dx, dy)| self.get(cell_x + dx, cell_y + dy) == Elem::Empty);
            if !open_neighbour {
                continue;
            }
            let ore = ZORG_ORES[self.rng.range(ZORG_ORES.len() as u32) as usize];
            self.set_xy(cell_x, cell_y, ore);
        }
    }

    /// The crystal that glitters in the cave.
    fn scatter_zorg_crystals(&mut self, width: i32, height: i32, cave_x: i32, cave_y: i32) {
        for _ in 0..26 {
            let dx = self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let dy = self.rng.range_i(-ZORG_CAVE_RY, ZORG_CAVE_RY + 1);
            let cell_x = cave_x + dx;
            let cell_y = cave_y + dy;
            if cell_x <= 1 || cell_y <= 1 || cell_x >= width - 2 || cell_y >= height - 6 {
                continue;
            }
            if self.get(cell_x, cell_y) == Elem::Empty {
                self.set_xy(cell_x, cell_y, Elem::Crystal);
            }
        }
    }

    #[must_use]
    pub fn zorg_cave_floor_row(&self) -> i32 {
        self.zorg_cave_floor
    }

    #[must_use]
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

    #[must_use]
    pub fn in_zorg_cave(&self, x: i32, y: i32) -> bool {
        let Some((cx, cy)) = self.zorg_cave else {
            return false;
        };
        let dx = (x - cx) as f32 / (ZORG_CAVE_RX as f32 * 1.15);
        let dy = (y - cy) as f32 / (ZORG_CAVE_RY as f32 * 1.15);
        dx * dx + dy * dy <= 1.0
    }

    #[must_use]
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
