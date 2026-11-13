//! The ZORG cave: the one place in the world that belongs to the deep.
//!
//! Every other colony in the simulation starts on the surface, in the
//! open air, where the light is. The ZORGs are the opposite: they come from
//! the bottom of the world and they hold a cavern of their own, a long way
//! down, where the rock is old and the water is poison.
//!
//! The cave is carved during the world seed, so it is part of the terrain
//! from the very first tick. It is:
//!
//! - **deep**: it sits in the last stretch of rock above the bedrock,
//!   a hundred and fifty cells or more below the surface and far below
//!   the reach of the ordinary caves,
//! - **walled**: it is a chamber with a solid rock floor, so nothing in it
//!   floats and the terrain around it stays intact. Two narrow passages
//!   lead up out of it, and they stay in the deep as well,
//! - **provisioned**: its floor is lined with acid, poison, tar and
//!   mercury, and its walls are studded with ore. That is the ZORGs' larder:
//!   the things that would kill a player are the things a ZORG eats.
//!
//! All of the founding ZORGs spawn inside this cave, so the deep has a
//! heart of its own rather than a scattering of animals across the whole
//! bottom of the map.

use super::*;

/// How wide the ZORG cave is, in cells.
pub const ZORG_CAVE_RX: i32 = 32;
/// How tall the ZORG cave is, in cells.
///
/// The cave is a low, wide hall rather than a dome: a ZORG has to be able
/// to reach the rock it feeds on, so it must never be so far from the walls
/// and the floor that there is nothing within its reach.
pub const ZORG_CAVE_RY: i32 = 8;

/// How far below the surface the cave has to be, at the very least.
///
/// The ZORGs belong to the deep. Their cave is not a hole a colony stumbles
/// into a few cells under the grass: it is far below the ordinary caves,
/// in the oldest rock of the world.
const MIN_DEPTH: i32 = 150;

/// How much rock has to be left between the floor of the cave and the
/// bedrock at the bottom of the world.
const ROCK_BELOW: i32 = 40;

/// How much poison the cave holds. The deep seeps it back, so the ZORGs
/// always have something to eat.
const ZORG_LARDER_TARGET: u32 = 200;

/// How deep the pool of poison on the floor of the cave is.
const ZORG_LARDER_DEPTH: i32 = 4;

/// Is this element part of the ZORGs' larder?
fn is_zorg_poison(e: Elem) -> bool {
    matches!(
        e,
        Elem::Acid | Elem::Poison | Elem::Tar | Elem::Mercury | Elem::Oil | Elem::Sulfur | Elem::Chlorine
    )
}

impl World {
    /// Carve the ZORG cave into the deep rock.
    ///
    /// The cave is anchored to the bottom of the world: it sits in the last
    /// stretch of rock above the bedrock, far below the ordinary cave
    /// network, so the player colonies have to dig deep to find it. Its
    /// position is remembered in `zorg_cave`, and that is where the
    /// founding ZORGs are put.
    pub(crate) fn carve_zorg_cave(&mut self, w: i32, h: i32) {
        // The floor of the cave has to stay above the bedrock, and the roof
        // has to stay below the surface of the world.
        let floor_limit = h - ROCK_BELOW;

        // Pick the column: the middle of the map, but not so central that
        // the cave is directly under the busiest part of the surface.
        let cx = (w as f32 * (0.34 + self.rng.f32() * 0.32)) as i32;

        // Pick the depth: as deep as the rock allows, and never less than
        // MIN_DEPTH below the surface.
        let surface = self.surface_y(cx);
        let deepest = floor_limit - ZORG_CAVE_RY;
        let cy = deepest.max(surface + MIN_DEPTH);

        // A cave that is not below the surface would break the rule that
        // everything lives under the terrain, and one that reaches into the
        // bedrock would break the world open. If the terrain leaves no
        // room, there is nothing to do but leave the deep to the deep.
        if cy - ZORG_CAVE_RY <= surface + 8 || cy + ZORG_CAVE_RY > floor_limit || cy <= 0 {
            return;
        }

        // ---- The chamber ----
        //
        // The chamber is an almond: the roof arches over and the floor
        // dips away at the sides, so it reads as a natural cavern and not
        // as a dome somebody cut. The middle of it is a flat plateau, the
        // one level place in the deep where the ZORGs can gather.
        let plateau_half = ZORG_CAVE_RX * 72 / 100;
        let plateau_y = cy + ZORG_CAVE_RY / 2;
        for dx in -ZORG_CAVE_RX..=ZORG_CAVE_RX {
            let t = dx as f32 / ZORG_CAVE_RX as f32;
            // The width of the cavern at this column, wobbling so the walls
            // are not smooth.
            let wobble = 1.0 + 0.16 * (t * 9.0).sin() + 0.10 * (t * 15.0 + 1.7).cos();
            let half = ((1.0 - t * t).max(0.0).sqrt() * wobble).clamp(0.0, 1.0);
            let top = cy - (ZORG_CAVE_RY as f32 * half) as i32;
            // The floor: flat in the middle, dipping away at the sides.
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
                // The cave is cut through everything, granite included: it
                // is older than the rock around it.
                let e = self.get(x, y);
                if e == Elem::Void || e == Elem::Water {
                    continue;
                }
                self.set_xy(x, y, Elem::Empty);
            }
            // ---- A solid floor ----
            //
            // Every column of the cave gets a rock floor, so nothing in it
            // floats and the world above it is not undermined.
            let below = bottom + 1;
            if below < h - 6 {
                let e = self.get(x, below);
                if e == Elem::Empty || e == Elem::Void {
                    self.set_xy(x, below, Elem::Granite);
                }
            }
        }

        // ---- The way out ----
        //
        // Two narrow passages lead up out of the chamber into the deep
        // caves. The ZORGs come up through these; the players would have to
        // find them. The passages stay in the deep: they never climb back
        // up towards the surface.
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
                // The passage stops in the deep: it never reaches up
                // towards the light.
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
                // A floor under the passage, so the ZORGs can walk it.
                if y + 1 < h - 6 && self.get(x, y + 1) == Elem::Empty {
                    self.set_xy(x, y + 1, Elem::Stone);
                }
            }
        }

        // ---- The larder ----
        //
        // The floor of the cave is lined with the things that poison a
        // player and sustain a ZORG. This is why the ZORGs do not starve
        // down here, and why a colony that follows one down has to fight
        // for the ground.
        //
        // The pool is deep: several cells of it, so that a ZORG standing
        // anywhere in the hall has poison within its reach.
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
        // A few stray pools further out, so the whole floor is provisioned.
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
            // The poisons pool on the floor of the cave, where the ZORGs
            // walk, rather than floating in the middle of it.
            if self.get(x, y + 1).is_solid() {
                let e = poisons[self.rng.range(poisons.len() as u32) as usize];
                self.set_xy(x, y, e);
            }
        }

        // ---- The walls ----
        //
        // Ore in the walls of the cave: the ZORGs chew stone and ore for
        // food, so this rock is their bread as well as their shelter.
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
            // Only the rock that touches the open cave is worth eating.
            let open_neighbour = DIRS8
                .iter()
                .any(|(dx, dy)| self.get(x + dx, y + dy) == Elem::Empty);
            if !open_neighbour {
                continue;
            }
            let o = ores[self.rng.range(ores.len() as u32) as usize];
            self.set_xy(x, y, o);
        }

        // ---- A little light ----
        //
        // Crystal grows in the deep. A few bright cells keep the cave from
        // being a pitch black hole on the map.
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

        // Remember where it is: this is where the ZORGs are born.
        self.zorg_cave = Some((cx, cy));
        self.zorg_cave_floor = plateau_y;
    }

    /// The floor of the ZORG cave: the row the ZORGs keep to.
    pub fn zorg_cave_floor_row(&self) -> i32 {
        self.zorg_cave_floor
    }

    /// The heart of the ZORG cave, if the world has one.
    pub fn zorg_cave_center(&self) -> Option<(i32, i32)> {
        self.zorg_cave
    }

    /// Let the deep seep fresh poison into the ZORG cave.
    ///
    /// The cave is the ZORGs' larder, and a larder has to be restocked: the
    /// poison pools on its floor seep back, slowly, out of the rock. Without
    /// this the ZORGs would eat the cave bare in a few thousand ticks and
    /// then starve in the hole they had made.
    pub(crate) fn regrow_zorg_larder(&mut self) {
        let Some((cx, cy)) = self.zorg_cave else {
            return;
        };
        // How much of the larder is left?
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
        // A little more seeps onto the floor of the cave.
        for _ in 0..20 {
            let x = cx + self.rng.range_i(-ZORG_CAVE_RX, ZORG_CAVE_RX + 1);
            let y = cy + self.rng.range_i(-ZORG_CAVE_RY / 2, ZORG_CAVE_RY + 1);
            if x <= 1 || y <= 1 || x >= self.w as i32 - 2 || y >= self.h as i32 - 6 {
                continue;
            }
            // The poison pools on the floor of the cave, where the ZORGs
            // walk, rather than floating in the middle of it.
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

    /// Is this cell inside the ZORG cave?
    ///
    /// The test is a soft ellipse: the cave is irregular, so a point just
    /// outside the drawn chamber still counts as being in the deep.
    pub fn in_zorg_cave(&self, x: i32, y: i32) -> bool {
        let Some((cx, cy)) = self.zorg_cave else {
            return false;
        };
        let dx = (x - cx) as f32 / (ZORG_CAVE_RX as f32 * 1.15);
        let dy = (y - cy) as f32 / (ZORG_CAVE_RY as f32 * 1.15);
        dx * dx + dy * dy <= 1.0
    }

    /// A cell inside the ZORG cave that a creature can stand in.
    ///
    /// The search prefers a cell with something under it - rock, or a pool
    /// of the poison that the ZORGs live in - so a ZORG looks like it is
    /// standing on the floor of the cave. If the floor is crowded, any open
    /// cell of the cavern will do: the ZORGs swim through their own cave
    /// as readily as they walk on it.
    pub fn find_zorg_cave_cell(&self, cx: i32, cy: i32) -> Option<(i32, i32)> {
        let w = self.w as i32;
        let h = self.h as i32;
        let mut fallback: Option<(i32, i32)> = None;
        // The heart of the chamber first, then the rim.
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
        // Nothing near the point asked for: any open cell of the cave will
        // do, so the ZORGs never end up born outside their own home.
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

    /// Is this a cell a ZORG can stand in, and if so where?
    ///
    /// A cell is standable when it is open and there is something under it:
    /// rock, or the poison the ZORGs feast on.
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
