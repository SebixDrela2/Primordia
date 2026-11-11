use crate::elements::{Behavior, Elem};
use crate::rng::Rng;

pub const DIRS4: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
pub const DIRS8: [(i32, i32); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

#[derive(Default)]
pub struct Census {
    pub particles: u32,
    pub plants: u32,
    pub water: u32,
    pub energy: u32,
    pub stone: u32,
    pub metal: u32,
    pub copper: u32,
    pub gold: u32,
    pub crystal: u32,
    pub gem: u32,
    pub wood: u32,
    pub oil: u32,
    pub sulfur: u32,
}

pub struct World {
    pub w: usize,
    pub h: usize,
    pub kind: Vec<u8>,
    pub col: Vec<[u8; 3]>,
    pub heat: Vec<u8>,
    pub age: Vec<u32>,
    /// Indices of the cells that can still change. Rebuilt each step from a
    /// cheap scan so the simulation never touches the (vast) inert bulk of
    /// the world.
    pub active: Vec<u32>,
    /// Lookup: is this element byte completely inert?
    pub inert: [bool; 80],
    pub rng: Rng,
    pub frame: u64,
    /// Per-step budget for destructive resource gathering so creatures
    /// cannot strip-mine the entire world. Reset at the start of each step.
    pub mine_budget: i32,
    pub census: Census,
}

impl World {
    pub fn new(w: usize, h: usize, seed: u64) -> Self {
        let n = w * h;
        World {
            w,
            h,
            kind: vec![Elem::Empty as u8; n],
            col: vec![[10, 14, 23]; n],
            heat: vec![0; n],
            age: vec![0; n],
            active: Vec::with_capacity(n / 8),
            inert: Elem::inert_table(),
            rng: Rng::new(seed),
            frame: 0,
            mine_budget: 0,
            census: Census::default(),
        }
    }

    #[inline]
    pub fn idx_xy(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            None
        } else {
            Some(y as usize * self.w + x as usize)
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Elem {
        match self.idx_xy(x, y) {
            Some(i) => Elem::from_u8(self.kind[i]),
            None => Elem::Void,
        }
    }

    #[inline]
    pub fn set_xy(&mut self, x: i32, y: i32, e: Elem) {
        if let Some(i) = self.idx_xy(x, y) {
            self.set_at(i, e);
        }
    }

    pub fn set_at(&mut self, i: usize, e: Elem) {
        self.kind[i] = e as u8;
        let base = e.color();
        let jitter = (self.rng.range(17) as i32) - 8;
        self.col[i] = [
            (base[0] as i32 + jitter).clamp(0, 255) as u8,
            (base[1] as i32 + jitter).clamp(0, 255) as u8,
            (base[2] as i32 + jitter).clamp(0, 255) as u8,
        ];
        self.age[i] = 0;
        self.heat[i] = match e {
            Elem::Fire => 210,
            Elem::Lava => 235,
            Elem::Plasma => 255,
            Elem::Energy => 120,
            _ => 0,
        };
    }

    #[inline]
    fn swap_cells(&mut self, a: usize, b: usize) {
        self.kind.swap(a, b);
        self.col.swap(a, b);
        self.heat.swap(a, b);
        self.age.swap(a, b);
    }

    #[inline]
    fn can_move_into(&self, mover: Elem, target: Elem) -> bool {
        if target == Elem::Empty {
            return true;
        }
        let target_movable = matches!(
            target.behavior(),
            Behavior::Flow | Behavior::Rise | Behavior::Diffuse
        );
        mover.density() > target.density() && target_movable
    }

    /// Moves the cell at `i` by (dx,dy). Returns the new index if it moved.
    fn try_move(&mut self, i: usize, dx: i32, dy: i32) -> Option<usize> {
        if dx == 0 && dy == 0 {
            return None;
        }
        let x = (i % self.w) as i32 + dx;
        let y = (i / self.w) as i32 + dy;
        let to = self.idx_xy(x, y)?;
        let mover = Elem::from_u8(self.kind[i]);
        let target = Elem::from_u8(self.kind[to]);
        if self.can_move_into(mover, target) {
            self.swap_cells(i, to);
            Some(to)
        } else {
            None
        }
    }

    // ------------------------------------------------------------------
    // World generation
    // ------------------------------------------------------------------

    pub fn surface_y(&self, x: i32) -> i32 {
        let h = self.h as f32;
        let x = x as f32;
        let wobble = (x * 0.045).sin() * 5.0
            + (x * 0.013 + 1.7).sin() * 9.0
            + (x * 0.11 + 0.4).sin() * 2.0;
        (h * 0.66 + wobble).clamp(24.0, h - 26.0) as i32
    }

    pub fn seed(&mut self) {
        let w = self.w as i32;
        let h = self.h as i32;

        // Bedrock layer at very bottom (impermeable)
        for x in 0..w {
            for y in (h - 3)..h {
                self.set_xy(x, y, Elem::Void);
            }
        }

        for x in 0..w {
            let surface = self.surface_y(x);
            for y in surface..(h - 3) {
                let depth = y - surface;
                let mut e = if depth < 2 {
                    Elem::Sand
                } else if depth < 6 {
                    if self.rng.chance(0.25) {
                        Elem::Sand
                    } else {
                        Elem::Stone
                    }
                } else {
                    Elem::Stone
                };

                if depth > 6 {
                    let r = self.rng.f32();
                    if r < 0.020 {
                        e = Elem::Metal;
                    } else if r < 0.032 {
                        e = Elem::Copper;
                    } else if r < 0.035 {
                        e = Elem::Glass;
                    } else if r < 0.037 {
                        e = Elem::Ice;
                    }
                }
                self.set_xy(x, y, e);
            }
        }

        // Impermeable clay layer above bedrock (2 rows thick)
        for x in 0..w {
            for y in (h - 5)..(h - 3) {
                self.set_xy(x, y, Elem::Stone); // dense stone acts as clay
            }
        }

        // Underground lava chamber (ellipse) - randomized position
        let cx = self.rng.range((w as u32 * 35 / 100) - (w as u32 * 10 / 100)) as i32 + (w as u32 * 10 / 100) as i32;
        let cy = self.rng.range((h as u32 * 40 / 100) - (h as u32 * 15 / 100)) as i32 + (h as u32 * 15 / 100) as i32;
        let rx: i32 = 20 + self.rng.range(25) as i32;
        let ry: i32 = 10 + self.rng.range(15) as i32;
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let ny = dy as f32 / ry as f32;
                if nx * nx + ny * ny <= 1.0 && self.rng.chance(0.85) {
                    self.set_xy(cx + dx, cy + dy, Elem::Lava);
                }
            }
        }

        // Surface lake - smaller, so most of the world is land
        let lake_start = (w as f32 * 0.62) as i32;
        let lake_end = (w as f32 * 0.78) as i32;
        for x in lake_start..lake_end {
            let t = (x - lake_start) as f32 / (lake_end - lake_start) as f32;
            let depth = 3.0 + (t * std::f32::consts::PI).sin() * 5.0;
            let surface = self.surface_y(x);
            for y in (surface - depth as i32)..surface {
                self.set_xy(x, y, Elem::Water);
            }
        }

        // A few smaller ponds
        for _ in 0..2 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let pond_w = 2 + self.rng.range(3) as i32;
            for px in (x - pond_w / 2)..=(x + pond_w / 2) {
                let depth = 1 + self.rng.range(2) as i32;
                for y in (surface - depth)..surface {
                    if self.get(px, y) == Elem::Empty {
                        self.set_xy(px, y, Elem::Water);
                    }
                }
            }
        }

        // Vegetation and wood
        for x in 2..(w - 2) {
            if self.rng.chance(0.20) {
                let surface = self.surface_y(x);
                self.set_xy(x, surface - 1, Elem::Plant);
                if self.rng.chance(0.3) {
                    self.set_xy(x, surface - 2, Elem::Plant);
                }
            }
            if self.rng.chance(0.02) {
                let surface = self.surface_y(x);
                self.set_xy(x, surface - 1, Elem::Wood);
            }
        }

        // Energy drifting just above the ground
        for _ in 0..60 {
            let x = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(x).max(14);
            let y = sy - 1 - self.rng.range(12) as i32;
            self.set_xy(x, y, Elem::Energy);
        }

        // Copper deposits near surface (for early tech) - very abundant
        for _ in 0..200 {
            let x = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(x);
            let y = sy + self.rng.range(3) as i32; // 0-2 below surface
            if self.get(x, y) == Elem::Stone && self.rng.chance(0.6) {
                self.set_xy(x, y, Elem::Copper);
            }
        }

        // Surface copper veins - dense
        for x in (5..w-5).step_by(15) { // very frequent
            let sy = self.surface_y(x);
            for dy in 0..=3 {
                if self.get(x, sy + dy) == Elem::Stone {
                    self.set_xy(x, sy + dy, Elem::Copper);
                }
            }
        }

        // Gold deposits deeper
        for _ in 0..15 {
            let x = self.rng.range(w as u32) as i32;
            let y = self.h as i32 - 10 - self.rng.range(15) as i32;
            if self.get(x, y) == Elem::Stone && self.rng.chance(0.2) {
                self.set_xy(x, y, Elem::Gold);
            }
        }

        // Crystal formations in caves
        for _ in 0..20 {
            let x = self.rng.range(w as u32) as i32;
            let y = self.rng.range(self.h as u32) as i32;
            if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 3) {
                self.set_xy(x, y, Elem::Crystal);
            }
        }

        // ===== COMPREHENSIVE ELEMENT PLACEMENT =====
        self.place_diverse_elements(w, h);
        self.place_extreme_diversity(w, h);
        self.place_rare_vaults(w, h);
        self.place_element_museum(w, h);
        self.guarantee_water(w, h);
        self.bury_floating_resources(w, h);

        self.census = self.count();
    }

    /// Move solid/resource cells that ended up floating above the terrain down
    /// into the rock. Several placement passes historically used absolute
    /// heights (`h/2`) that sat above the surface, leaving ore and minerals
    /// hovering in the sky. This guarantees resources live underground.
    fn bury_floating_resources(&mut self, w: i32, h: i32) {
        let mut floating: Vec<Elem> = Vec::new();
        for x in 2..(w - 2) {
            let surf = self.surface_y(x);
            let sky_bottom = (surf - 3).max(2);
            for y in 2..sky_bottom {
                let e = self.get(x, y);
                let is_resource = e.is_solid()
                    || matches!(
                        e,
                        Elem::Oil
                            | Elem::Acid
                            | Elem::Mercury
                            | Elem::Blood
                            | Elem::Poison
                            | Elem::Alcohol
                            | Elem::Honey
                    );
                // Leave organic growth, airborne gases and water lakes alone.
                if is_resource
                    && !matches!(
                        e,
                        Elem::Plant | Elem::Wood | Elem::Moss | Elem::Spore | Elem::Roots
                    )
                {
                    floating.push(e);
                    self.set_xy(x, y, Elem::Empty);
                }
            }
        }

        for e in floating {
            for _ in 0..24 {
                let x = self.rng.range_i(3, w - 3);
                let surf = self.surface_y(x);
                let max_depth = (h - 8 - surf).max(2);
                let y = surf + 4 + self.rng.range(max_depth as u32) as i32;
                let cur = self.get(x, y);
                if cur == Elem::Stone || cur == Elem::Sand {
                    self.set_xy(x, y, e);
                    break;
                }
            }
        }
    }

    /// Guarantee water exists by placing an underground lake below the surface.
    fn guarantee_water(&mut self, w: i32, h: i32) {
        // Underground lake below surface level
        let lx = (w as f32 * 0.2) as i32;
        let ly = (h as f32 * 0.75) as i32;
        for dy in -6..=6 {
            for dx in -10..=10 {
                if dx*dx + dy*dy <= 40 {
                    let x = lx + dx;
                    let y = ly + dy;
                    // Only place below surface, never in the sky
                    if x > 1 && x < w - 1 && y > 1 && y < h - 5 && y > self.surface_y(x) {
                        self.set_xy(x, y, Elem::Water);
                    }
                }
            }
        }
    }

    /// Element museum: place one cell of EVERY element type so all 79 exist at seed time.
    /// Placed underground below the surface, surrounded by stone walls so they persist.
    fn place_element_museum(&mut self, w: i32, h: i32) {
        let start_x = (w as f32 * 0.05) as i32;
        let start_y = (h as f32 * 0.8) as i32;
        let mut idx = 0;
        for v in 1..=Elem::Food as u8 {
            let e = Elem::from_u8(v);
            if e == Elem::Empty || e == Elem::Void { continue; }
            // Place each element in its own column below the surface
            let mx = start_x + (idx % 20) * 4;
            let row = idx / 20;
            // Dig down below surface for this column
            let my = self.surface_y(mx.clamp(2, w - 3)) + 2 + row * 3;
            // Surround each museum cell with stone walls for protection
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let wx = mx + dx;
                    let wy = my + dy;
                    if wx > 1 && wx < w - 1 && wy > 1 && wy < h - 5 {
                        if dx == 0 && dy == 0 {
                            self.set_xy(wx, wy, e);
                        } else if self.get(wx, wy) == Elem::Empty {
                            self.set_xy(wx, wy, Elem::Stone);
                        }
                    }
                }
            }
            idx += 1;
        }
    }

    /// Place a huge variety of additional element types so the world is rich
    /// with many distinct materials for creatures to find, eat, and use.
    fn place_extreme_diversity(&mut self, w: i32, h: i32) {
        // === More powders ===
        // Salt flats (surface)
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(3) as i32;
            for dy in -2..=2 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Salt);
                        }
                    }
                }
            }
        }
        // Ash deposits (post-volcanic, near lava)
        for _ in 0..25 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(25) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }
        // Dust (dry surface areas)
        for _ in 0..40 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 1 + self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }
        // Gunpowder (rare, underground)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }
        // Fertilizer (farm patches)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.35) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }

        // === More liquids ===
        // Blood pools (surface, rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Blood);
                        }
                    }
                }
            }
        }
        // Poison pools (dangerous, swampy)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            for dy in -1..=1 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 4 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Poison);
                        }
                    }
                }
            }
        }
        // Alcohol surface pools
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Alcohol);
            }
        }
        // Honey pools (rare, near fungus)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 1 + self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Honey);
            }
        }
        // Acid pools (rare, dangerous)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Acid);
                        }
                    }
                }
            }
        }

        // === More gases ===
        // Chlorine gas (near poison/acid)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 16 && self.rng.chance(0.18) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Chlorine);
                        }
                    }
                }
            }
        }
        // Methane gas (swampy surface)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 2 - self.rng.range(6) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.22) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Methane);
                        }
                    }
                }
            }
        }
        // Oxygen pockets (caves)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Oxygen);
                        }
                    }
                }
            }
        }
        // Gas pockets
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Gas);
            }
        }

        // === More solids/metals ===
        // Tin veins (shallow)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 12 - self.rng.range(30) as i32;
            let len = 8 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(x, y, Elem::Tin);
                    }
                }
            }
        }
        // Bronze deposits (mixed copper+tin)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 16 - self.rng.range(28) as i32;
            let len = 6 + self.rng.range(12);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.22) {
                        self.set_xy(x, y, Elem::Bronze);
                    }
                }
            }
        }
        // Iron veins (deep)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 22 - self.rng.range(28) as i32;
            let len = 10 + self.rng.range(16);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Iron);
                    }
                }
            }
        }
        // Steel (very rare, deep)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 24 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Steel);
            }
        }
        // Silver veins (deep)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 20 - self.rng.range(26) as i32;
            let len = 8 + self.rng.range(14);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.28) {
                        self.set_xy(x, y, Elem::Silver);
                    }
                }
            }
        }
        // Gold veins (rare, deep)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 26 - self.rng.range(22) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Gold);
            }
        }
        // Glass (near lava/sand)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Sand && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Glass);
            }
        }
        // Obsidian (water meets lava, already happens but add more)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Obsidian);
            }
        }
        // Diamond (very rare, deep)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 28 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Diamond);
            }
        }
        // Gem (deep)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 24 - self.rng.range(18) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Gem);
            }
        }

        // === More organic ===
        // Wood (surface trees)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Wood);
                // tree trunk
                for dy in 1..=3 {
                    let ty = sy - dy;
                    if ty > 1 && self.get(cx, ty) == Elem::Empty && self.rng.chance(0.5) {
                        self.set_xy(cx, ty, Elem::Wood);
                    }
                }
            }
        }
        // Meat (creature remains)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Meat);
            }
        }
        // Bone
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Bone);
            }
        }
        // Leather (rare)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }
        // Roots (underground)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }
        // Food items
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }

        // === More energy/exotic ===
        // Ice (cold spots)
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 2 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ice);
            }
        }
        // Energy specks (above ground)
        for _ in 0..50 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx).max(10);
            let cy = sy - 1 - self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Energy);
            }
        }
        // Mana crystals (caves)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }
        // Soul (very rare, deep)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 6 - self.rng.range(12) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }
        // Fire (rare surface spots)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Fire);
            }
        }

        // === More constructed ===
        // Brick structures
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            let size = (4 + self.rng.range(6)) as i32;
            for dy in 0..size {
                for dx in -size/2..=size/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                        if self.get(x, y) == Elem::Empty && self.rng.chance(0.35) {
                            self.set_xy(x, y, Elem::Brick);
                        }
                    }
                }
            }
        }
        // Concrete
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.25) {
                self.set_xy(cx, cy, Elem::Concrete);
            }
        }
        // Chest
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Chest);
            }
        }
        // Forge
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Forge);
            }
        }
        // Farm
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(2) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Farm);
            }
        }
        // Temple
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(3) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.12) {
                self.set_xy(cx, cy, Elem::Temple);
            }
        }
        // Tower
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let height = (6 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let y = sy - dy;
                if y > 2 && self.get(cx, y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(cx, y, Elem::Tower);
                }
            }
        }
        // Bridge
        for _ in 0..2 {
            let x1 = self.rng.range((w / 3) as u32) as i32 + w / 6;
            let x2 = self.rng.range((w / 3) as u32) as i32 + 2 * w / 3;
            let sy = self.surface_y((x1 + x2) / 2);
            let y = sy - 1;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.7) {
                    self.set_xy(x, y, Elem::Bridge);
                }
            }
        }
        // Road
        for _ in 0..3 {
            let x1 = self.rng.range(w as u32) as i32;
            let x2 = self.rng.range(w as u32) as i32;
            let sy = self.surface_y((x1 + x2) / 2);
            let y = sy;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.6) {
                    self.set_xy(x, y, Elem::Road);
                }
            }
        }

        // === More magical ===
        // Runes
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }
        // Wards
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.18) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }
        // Portals (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        // === More biological ===
        // Eggs
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }
        // Virus patches
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.12) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                            let e = self.get(x, y);
                            if e.is_organic() {
                                self.set_xy(x, y, Elem::Virus);
                            }
                        }
                    }
                }
            }
        }
        // Bacteria (near meat/corpse)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 6 && self.rng.chance(0.18) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                            let e = self.get(x, y);
                            if e == Elem::Meat || e == Elem::Corpse || e == Elem::Food {
                                self.set_xy(x, y, Elem::Bacteria);
                            }
                        }
                    }
                }
            }
        }
    }

    fn place_diverse_elements(&mut self, w: i32, h: i32) {
        // === LIQUIDS ===
        // Mercury pools (rare, deep)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 5 - self.rng.range(20) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 10 && self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Mercury);
                        }
                    }
                }
            }
        }

        // Alcohol ponds (rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (h as i32 / 2) + (self.rng.range((h as u32 / 4) as u32) as i32);
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Alcohol);
                        }
                    }
                }
            }
        }

        // Honey pools (rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (h as i32 / 2) + (self.rng.range((h as u32 / 4) as u32) as i32);
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Honey);
                        }
                    }
                }
            }
        }

        // Blood pools (very rare, near surface)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            for dy in 0..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 5 && self.rng.chance(0.15) {
                        let x = cx + dx;
                        let y = sy - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Blood);
                        }
                    }
                }
            }
        }

        // Poison pools (rare, dangerous)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (h as i32 / 2) + (self.rng.range((h as u32 / 4) as u32) as i32);
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 9 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Poison);
                        }
                    }
                }
            }
        }

        // === GASES ===
        // Chlorine gas pockets (near acid/poison)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (h as i32 / 2) + (self.rng.range((h as u32 / 4) as u32) as i32);
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx*dx + dy*dy <= 20 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Chlorine);
                        }
                    }
                }
            }
        }

        // Methane gas pockets (swampy areas)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 2 - self.rng.range(8) as i32;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.25) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Methane);
                        }
                    }
                }
            }
        }

        // Oxygen pockets (caves)
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.2) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Oxygen);
                        }
                    }
                }
            }
        }

        // === POWDERS ===
        // Sulfur deposits (near lava)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(20) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.4) {
                self.set_xy(cx, cy, Elem::Sulfur);
            }
        }

        // Ash deposits (post-volcanic)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 5 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Stone && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Ash);
            }
        }

        // Dust (dry areas)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 3) as u32) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Dust);
            }
        }

        // Gunpowder (rare, man-made)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Gunpowder);
            }
        }

        // Fertilizer (farm areas)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Fertilizer);
            }
        }

        // Mud (wet areas)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1;
            if self.get(cx, cy) == Elem::Water && self.rng.chance(0.3) {
                self.set_xy(cx, cy, Elem::Mud);
            }
        }

        // === SOLIDS / METALS ===
        // Tin veins
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(35) as i32;
            let len = 10 + self.rng.range(20);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Tin);
                    }
                }
            }
        }

        // Bronze deposits (rare, mixed copper+tin)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 18 - self.rng.range(30) as i32;
            let len = 8 + self.rng.range(15);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.25) {
                        self.set_xy(x, y, Elem::Bronze);
                    }
                }
            }
        }

        // Iron veins (deeper)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 20 - self.rng.range(30) as i32;
            let len = 12 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.35) {
                        self.set_xy(x, y, Elem::Iron);
                    }
                }
            }
        }

        // Steel (rare, refined)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 22 - self.rng.range(25) as i32;
            let len = 6 + self.rng.range(10);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.2) {
                        self.set_xy(x, y, Elem::Steel);
                    }
                }
            }
        }

        // Silver veins
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 18 - self.rng.range(30) as i32;
            let len = 10 + self.rng.range(15);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Silver);
                    }
                }
            }
        }

        // Coal seams
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let len = 20 + self.rng.range(30);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.5) {
                        self.set_xy(x, y, Elem::Ash); // Using Ash as coal proxy
                    }
                }
            }
        }

        // === ORGANIC ===
        // Fungus (damp areas)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2) {
                if self.rng.chance(0.3) {
                    self.set_xy(cx, cy, Elem::Fungus);
                }
            }
        }

        // Moss (damp, shaded)
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(4) as i32;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 2) {
                if self.rng.chance(0.2) {
                    self.set_xy(cx, cy, Elem::Moss);
                }
            }
        }

        // Roots (underground)
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 2 + self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Roots);
            }
        }

        // Meat/Bone (creature remains)
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(8) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                if self.rng.chance(0.5) {
                    self.set_xy(cx, cy, Elem::Meat);
                } else {
                    self.set_xy(cx, cy, Elem::Bone);
                }
            }
        }

        // Leather (rare)
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Leather);
            }
        }

        // === ENERGY/EXOTIC ===
        // Mana crystals
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.has_water_near(cx, cy, 3) {
                self.set_xy(cx, cy, Elem::Mana);
            }
        }

        // Soul energy (very rare, deep)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 8 - self.rng.range(15) as i32;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Soul);
            }
        }

        // Magma (hotter than lava)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(20) as i32;
            let radius = (4 + self.rng.range(4)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.7) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            if self.get(x, y) == Elem::Stone {
                                self.set_xy(x, y, Elem::Magma);
                            }
                        }
                    }
                }
            }
        }

        // Void pockets (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            let radius = (2 + self.rng.range(3)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius && self.rng.chance(0.5) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            if self.rng.chance(0.5) {
                                self.set_xy(x, y, Elem::Void);
                            }
                        }
                    }
                }
            }
        }

        // === CONSTRUCTED ===
        // Ancient brick structures
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = (self.rng.range((h as u32 / 2) as u32) as i32) + h / 4;
            let width = (6 + self.rng.range(8)) as i32;
            let height = (4 + self.rng.range(4)) as i32;
            for dy in 0..height {
                for dx in -width/2..=width/2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                        if self.get(x, y) == Elem::Empty && self.rng.chance(0.4) {
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

        // Wire/Circuit ruins
        for _ in 0..6 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let len = 15 + self.rng.range(10);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Empty && self.rng.chance(0.3) {
                        match self.rng.range(3) {
                            0 => self.set_xy(x, y, Elem::Wire),
                            1 => self.set_xy(x, y, Elem::Circuit),
                            _ => self.set_xy(x, y, Elem::Machine),
                        }
                    }
                }
            }
        }

        // Tools scattered
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(10) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                match self.rng.range(5) {
                    0 => self.set_xy(cx, cy, Elem::Tool),
                    1 => self.set_xy(cx, cy, Elem::Furnace),
                    2 => self.set_xy(cx, cy, Elem::Anvil),
                    3 => self.set_xy(cx, cy, Elem::Forge),
                    _ => self.set_xy(cx, cy, Elem::Machine),
                }
            }
        }

        // Door/Chest ruins
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.5) {
                match self.rng.range(4) {
                    0 => self.set_xy(cx, cy, Elem::Door),
                    1 => self.set_xy(cx, cy, Elem::Chest),
                    2 => self.set_xy(cx, cy, Elem::Furnace),
                    _ => self.set_xy(cx, cy, Elem::Anvil),
                }
            }
        }

        // === MAGICAL ===
        // Runes
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Rune);
            }
        }

        // Wards
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Ward);
            }
        }

        // Portals (very rare)
        for _ in 0..2 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            if self.get(cx, cy) == Elem::Empty {
                self.set_xy(cx, cy, Elem::Portal);
            }
        }

        // === BIOLOGICAL ===
        // Eggs (rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(5) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.1) {
                self.set_xy(cx, cy, Elem::Egg);
            }
        }

        // Corpses
        for _ in 0..5 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(8) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.15) {
                self.set_xy(cx, cy, Elem::Corpse);
            }
        }

        // Virus patches
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx*dx + dy*dy <= 12 && self.rng.chance(0.1) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y).is_organic() {
                            self.set_xy(x, y, Elem::Virus);
                        }
                    }
                }
            }
        }

        // Bacteria
        for _ in 0..4 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range(h as u32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx*dx + dy*dy <= 6 && self.rng.chance(0.15) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Meat || self.get(x, y) == Elem::Corpse {
                            self.set_xy(x, y, Elem::Bacteria);
                        }
                    }
                }
            }
        }

        // Food items
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(cx);
            let cy = sy - 1 - self.rng.range(6) as i32;
            if self.get(cx, cy) == Elem::Empty && self.rng.chance(0.2) {
                self.set_xy(cx, cy, Elem::Food);
            }
        }
    }

    // ===== ENHANCED WORLD GENERATION METHODS =====

    fn generate_surface_water(&mut self, w: i32, h: i32) {
        // Main lake
        let lake_start = (w as f32 * 0.55) as i32;
        let lake_end = (w as f32 * 0.75) as i32;
        for x in lake_start..lake_end {
            let t = (x - lake_start) as f32 / (lake_end - lake_start) as f32;
            let depth = 4.0 + (t * std::f32::consts::PI).sin() * 8.0;
            let surface = self.surface_y(x);
            for y in (surface - depth as i32)..surface {
                self.set_xy(x, y, Elem::Water);
            }
        }

        // Additional ponds
        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let pond_w = 3 + self.rng.range(4) as i32;
            for px in (x - pond_w / 2)..=(x + pond_w / 2) {
                let depth = 1 + self.rng.range(3) as i32;
                for y in (surface - depth)..surface {
                    if self.get(px, y) == Elem::Empty {
                        self.set_xy(px, y, Elem::Water);
                    }
                }
            }
        }

        // River
        let river_y = self.surface_y(w / 2) - 2;
        for x in (w / 4)..(3 * w / 4) {
            if self.rng.chance(0.85) {
                let y = river_y as f32 + (self.rng.f32() - 0.5) * 3.0;
                if self.get(x, y as i32) == Elem::Empty {
                    self.set_xy(x, y as i32, Elem::Water);
                }
            }
        }

        // Hot springs
        for _ in 0..3 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            for dx in -2..=2 {
                for dy in 0..=2 {
                    let px = x + dx;
                    let py = surface - dy;
                    if px > 0 && px < w && py > 0 && py < h {
                        if self.rng.chance(0.3) {
                            self.set_xy(px, py, Elem::Water);
                        }
                    }
                }
            }
        }
    }

    fn generate_ore_veins(&mut self, w: i32, h: i32) {
        // Copper veins
        for _ in 0..25 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 10 - self.rng.range(40) as i32;
            let len = 15 + self.rng.range(25);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.4) {
                        self.set_xy(x, y, Elem::Copper);
                    }
                }
            }
        }

        // Tin veins
        for _ in 0..15 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 15 - self.rng.range(35) as i32;
            let len = 10 + self.rng.range(20);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Tin);
                    }
                }
            }
        }

        // Iron veins (deeper)
        for _ in 0..20 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 20 - self.rng.range(30) as i32;
            let len = 12 + self.rng.range(18);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.35) {
                        self.set_xy(x, y, Elem::Iron);
                    }
                }
            }
        }

        // Gold veins (rare, deep)
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 25 - self.rng.range(25) as i32;
            let len = 8 + self.rng.range(12);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Gold);
                    }
                }
            }
        }

        // Silver veins
        for _ in 0..10 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 18 - self.rng.range(30) as i32;
            let len = 10 + self.rng.range(15);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.3) {
                        self.set_xy(x, y, Elem::Silver);
                    }
                }
            }
        }

        // Coal seams
        for _ in 0..30 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let len = 20 + self.rng.range(30);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y) == Elem::Stone && self.rng.chance(0.5) {
                        self.set_xy(x, y, Elem::Ash);
                    }
                }
            }
        }

        // Sulfur vents near lava
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h - 12 - self.rng.range(20) as i32;
            let len = 8 + self.rng.range(12);
            let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
            let dx_f = angle.cos();
            let dy_f = angle.sin();
            for i in 0..len {
                let x = cx + (dx_f * i as f32) as i32;
                let y = cy + (dy_f * i as f32) as i32;
                if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                    if self.get(x, y).is_solid() && self.rng.chance(0.2) {
                        self.set_xy(x, y, Elem::Sulfur);
                    }
                }
            }
        }

        // Oil pools
        for _ in 0..8 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = h / 2 + self.rng.range((h / 4) as u32) as i32;
            for dy in -3..=3 {
                for dx in -5..=5 {
                    if dx * dx + dy * dy <= 25 && self.rng.chance(0.4) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Oil);
                        }
                    }
                }
            }
        }

        // Acid pools (rare)
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            for dy in -2..=2 {
                for dx in -3..=3 {
                    if dx * dx + dy * dy <= 9 && self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Acid);
                        }
                    }
                }
            }
        }
    }

    /// Rare vaults: place at least a few cells of EVERY element type so the
    /// world always contains full element variety, even if some are super rare.
    fn place_rare_vaults(&mut self, w: i32, h: i32) {
        let step = (w as f32 / 34.0) as i32;
        let vaults: &[(Elem, i32, i32, i32, i32)] = &[
            // (element, vault_index, cy_min, radius, count)
            (Elem::Sulfur, 1, h / 2, 3, 25),
            (Elem::Ash, 2, h / 2, 3, 25),
            (Elem::Salt, 3, h / 2, 3, 25),
            (Elem::Dust, 4, h / 2, 3, 25),
            (Elem::Gunpowder, 5, h / 2, 3, 12),
            (Elem::Fertilizer, 6, h / 2, 3, 20),
            (Elem::Blood, 7, h / 2, 3, 20),
            (Elem::Alcohol, 8, h / 2, 3, 12),
            (Elem::Honey, 9, h / 2, 3, 12),
            (Elem::Acid, 10, h / 2, 3, 20),
            (Elem::Chlorine, 11, h / 2, 3, 20),
            (Elem::Methane, 12, h / 2, 3, 20),
            (Elem::Oxygen, 13, h / 2, 3, 20),
            (Elem::Gas, 14, h / 2, 3, 20),
            (Elem::Steam, 15, h / 2, 3, 20),
            (Elem::Smoke, 16, h / 2, 3, 20),
            (Elem::Plasma, 17, h / 2, 3, 10),
            (Elem::Tin, 18, h / 2, 4, 25),
            (Elem::Bronze, 19, h / 2, 4, 20),
            (Elem::Iron, 20, h / 2, 4, 25),
            (Elem::Steel, 21, h / 2, 4, 10),
            (Elem::Silver, 22, h / 2, 4, 20),
            (Elem::Gold, 23, h / 2, 4, 15),
            (Elem::Glass, 24, h / 2, 4, 20),
            (Elem::Obsidian, 25, h / 2, 4, 15),
            (Elem::Gem, 26, h / 2, 4, 12),
            (Elem::Diamond, 27, h / 2, 4, 6),
            (Elem::Brick, 28, h / 2, 4, 25),
            (Elem::Concrete, 29, h / 2, 4, 25),
            (Elem::Chest, 30, h / 2, 3, 10),
            (Elem::Forge, 31, h / 2, 3, 6),
            (Elem::Farm, 32, h / 2, 3, 10),
            (Elem::Temple, 33, h / 2, 3, 6),
            (Elem::Machine, 34, h / 2, 3, 10),
            (Elem::Meat, 0, h / 2 + 40, 3, 12),
            (Elem::Leather, 1, h / 2 + 40, 3, 10),
            (Elem::Roots, 2, h / 2 + 40, 3, 20),
            (Elem::Food, 3, h / 2 + 40, 3, 15),
            (Elem::Energy, 4, h / 2 + 40, 3, 25),
            (Elem::Soul, 5, h / 2 + 40, 3, 6),
            (Elem::Fire, 6, h / 2 + 40, 2, 10),
            (Elem::Virus, 7, h / 2 + 40, 3, 3),
            (Elem::Bacteria, 8, h / 2 + 40, 3, 10),
            (Elem::Egg, 9, h / 2 + 40, 3, 6),
        ];
        for &(elem, vault_idx, cy_min, radius, count) in vaults {
            let mut placed = 0;
            let mut attempts = 0;
            while placed < count && attempts < count * 6 {
                attempts += 1;
                let cx = step * vault_idx + step / 2 + self.rng.range_i(-step / 4, step / 4);
                let angle = self.rng.f32() * std::f32::consts::PI * 2.0;
                let r = self.rng.range(radius as u32) as f32;
                let dx = angle.cos() * r;
                let dy = angle.sin() * r;
                let x = (cx as f32 + dx) as i32;
                // Depth below the terrain, varied per vault. Never absolute
                // heights, which used to float above the surface.
                let surf = self.surface_y(x);
                let depth = 8 + (vault_idx.rem_euclid(7)) * 12 + self.rng.range(6) as i32;
                let _ = cy_min;
                let y = surf + depth + dy as i32;
                if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                    let cur = self.get(x, y);
                    if matches!(cur, Elem::Empty | Elem::Stone | Elem::Sand) {
                        self.set_xy(x, y, elem);
                        placed += 1;
                    }
                }
            }
        }
    }

    fn generate_crystal_caves(&mut self, w: i32, h: i32) {
        for _ in 0..12 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let radius = (5 + self.rng.range(8)) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 2 && x < w - 2 && y > 2 && y < h - 5 {
                            let e = self.get(x, y);
                            if e == Elem::Empty || e == Elem::Stone {
                                let r = self.rng.f32();
                                if r < 0.25 {
                                    self.set_xy(x, y, Elem::Crystal);
                                } else if r < 0.4 {
                                    self.set_xy(x, y, Elem::Gem);
                                } else if r < 0.5 {
                                    self.set_xy(x, y, Elem::Diamond);
                                } else if r < 0.6 {
                                    self.set_xy(x, y, Elem::Mana);
                                } else if r < 0.7 {
                                    self.set_xy(x, y, Elem::Glass);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn generate_underground_structures(&mut self, w: i32, h: i32) {
        // Ancient ruins
        for _ in 0..5 {
            let cx = self.rng.range((w / 2) as u32) as i32 + w / 4;
            let cy = self.rng.range((h / 2) as u32) as i32 + h / 4;
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

        // Crystal towers
        for _ in 0..3 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.rng.range((h / 3) as u32) as i32;
            let height = (12 + self.rng.range(8)) as i32;
            for dy in 0..height {
                let x = cx;
                let y = cy + dy;
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.7) {
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
            let y = self.rng.range((h / 2) as u32) as i32 + h / 4;
            let start = x1.min(x2);
            let end = x1.max(x2);
            for x in start..=end {
                if self.get(x, y) == Elem::Empty && self.rng.chance(0.8) {
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
    }

    fn generate_surface_structures(&mut self, w: i32, h: i32) {
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

    pub fn step(&mut self) {
        self.frame += 1;
        self.mine_budget = 3;

        // Rebuild the list of cells that can actually change. The vast bulk of
        // the world is stone/ore, which never moves and never reacts, so
        // touching only the live cells is a huge win.
        self.active.clear();
        let n = self.w * self.h;
        for i in 0..n {
            let k = self.kind[i];
            if k != Elem::Empty as u8 && !self.inert[k as usize] {
                self.active.push(i as u32);
            }
        }
        self.rng.shuffle(&mut self.active);

        for k in 0..self.active.len() {
            let i = self.active[k] as usize;
            let e = Elem::from_u8(self.kind[i]);
            if e == Elem::Empty {
                continue;
            }
            self.update_cell(i, e);
        }

        self.maintain();
        if self.frame % 30 == 0 {
            self.census = self.count();
        }
    }

    fn update_cell(&mut self, i: usize, e: Elem) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        if self.try_reaction(i, e, x, y) {
            return;
        }

        let life = e.lifetime();
        if life != u32::MAX {
            self.age[i] = self.age[i].saturating_add(1);
            if self.age[i] > life {
                self.decay(i, e);
                return;
            }
        }

        match e.behavior() {
            Behavior::Static => {}
            Behavior::Fall => self.step_fall(i),
            Behavior::Flow => self.step_flow(i),
            Behavior::Rise => self.step_rise(i),
            Behavior::Diffuse => self.step_diffuse(i),
            Behavior::Burn => self.step_burn(i),
            Behavior::Grow => self.step_grow(i, e),
            Behavior::Energy => self.step_energy(i),
            Behavior::Conduct => {}, // Wire/Circuit - static but conducts
            Behavior::Spread => {},  // Virus/Bacteria - spread handled in reactions
            Behavior::Hatch => {},   // Egg - handled in reactions
            Behavior::Decay => {},   // Corpse - handled in reactions
        }
    }

    fn decay(&mut self, i: usize, e: Elem) {
        match e {
            Elem::Fire => self.set_at(i, Elem::Smoke),
            Elem::Steam => self.set_at(i, Elem::Water),
            _ => self.set_at(i, Elem::Empty),
        }
    }

    fn step_fall(&mut self, i: usize) {
        if self.try_move(i, 0, 1).is_some() {
            return;
        }
        let d = self.rng_sign();
        if self.try_move(i, d, 1).is_some() {
            return;
        }
        let _ = self.try_move(i, -d, 1);
    }

    fn step_flow(&mut self, i: usize) {
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

    fn step_rise(&mut self, i: usize) {
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

    fn step_diffuse(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        let _ = self.try_move(i, dx, dy);
    }

    fn step_burn(&mut self, i: usize) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        // Ignite fuel nearby
        for (dx, dy) in DIRS8 {
            let t = self.get(x + dx, y + dy);
            let fuel = matches!(
                t,
                Elem::Wood | Elem::Oil | Elem::Plant | Elem::Spore | Elem::Gas
            );
            if fuel && self.rng.chance(0.14) {
                self.set_xy(x + dx, y + dy, Elem::Fire);
            }
        }

        // Fire burns out faster when it has nothing to eat
        if self.age[i] > 24 && self.rng.chance(0.05) {
            self.set_at(i, Elem::Smoke);
            return;
        }

        if self.try_move(i, 0, -1).is_some() {
            return;
        }
        let d = self.rng_sign();
        let _ = self.try_move(i, d, -1);
    }

    fn step_grow(&mut self, i: usize, e: Elem) {
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
    }

    fn has_water_near(&self, x: i32, y: i32, radius: i32) -> bool {
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

    fn step_energy(&mut self, i: usize) {
        let dx = self.rng.range_i(-1, 2);
        let dy = self.rng.range_i(-1, 2);
        if self.try_move(i, dx, dy).is_none() {
            let _ = self.try_move(i, -dx, -dy);
        }
    }

    fn rng_sign(&mut self) -> i32 {
        if self.rng.next_u32() & 1 == 0 {
            -1
        } else {
            1
        }
    }

    /// Complex elemental reactions.
    fn try_reaction(&mut self, i: usize, e: Elem, x: i32, y: i32) -> bool {
        match e {
            Elem::Water => {
                if self.rng.chance(0.5) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if t == Elem::Lava || t == Elem::Magma {
                            self.set_at(i, Elem::Steam);
                            self.set_xy(x + dx, y + dy, if t == Elem::Lava { Elem::Stone } else { Elem::Obsidian });
                            return true;
                        }
                        if t == Elem::Fire {
                            self.set_xy(x + dx, y + dy, Elem::Steam);
                        }
                        if t == Elem::Acid {
                            self.set_at(i, Elem::Steam);
                        }
                        if t == Elem::Chlorine {
                            self.set_xy(x + dx, y + dy, Elem::Poison);
                        }
                        if t == Elem::Lava || t == Elem::Magma {
                            if self.rng.chance(0.3) {
                                self.set_xy(x + dx, y + dy, Elem::Obsidian);
                            }
                        }
                    }
                }
                // Water + gunpowder = wet gunpowder (useless)
                if self.rng.chance(0.1) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Gunpowder {
                            self.set_xy(x + dx, y + dy, Elem::Mud);
                        }
                    }
                }
            }
            Elem::Lava => {
                for (dx, dy) in DIRS4 {
                    match self.get(x + dx, y + dy) {
                        Elem::Water => {
                            self.set_at(i, Elem::Stone);
                            self.set_xy(x + dx, y + dy, Elem::Steam);
                            return true;
                        }
                        Elem::Sand => {
                            if self.rng.chance(0.05) {
                                self.set_xy(x + dx, y + dy, Elem::Glass);
                            }
                        }
                        Elem::Sulfur => {
                            if self.rng.chance(0.1) {
                                self.set_xy(x + dx, y + dy, Elem::Gas);
                            }
                        }
                        Elem::Ice => {
                            if self.rng.chance(0.1) {
                                self.set_xy(x + dx, y + dy, Elem::Water);
                            }
                        }
                        Elem::Metal | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Iron | Elem::Steel | Elem::Gold | Elem::Silver => {
                            if self.rng.chance(0.02) {
                                self.set_xy(x + dx, y + dy, Elem::Magma); // melted metal
                            }
                        }
                        Elem::Stone | Elem::Brick | Elem::Concrete => {
                            if self.rng.chance(0.0005) {
                                self.set_xy(x + dx, y + dy, Elem::Magma);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Elem::Magma => {
                for (dx, dy) in DIRS4 {
                    match self.get(x + dx, y + dy) {
                        Elem::Water => {
                            self.set_at(i, Elem::Obsidian);
                            self.set_xy(x + dx, y + dy, Elem::Steam);
                            return true;
                        }
                        Elem::Stone => {
                            if self.rng.chance(0.0005) {
                                self.set_xy(x + dx, y + dy, Elem::Magma); // spreads
                            }
                        }
                        _ => {}
                    }
                }
                // Magma cools to stone/obsidian
                if self.rng.chance(0.001) {
                    let to_obsidian = self.rng.chance(0.3);
                    self.set_at(i, if to_obsidian { Elem::Obsidian } else { Elem::Stone });
                }
            }
            Elem::Acid => {
                // Corrode slowly and conservatively: rock becomes sand (weathered
                // debris that still exists) and the acid is spent. This keeps
                // acid dangerous without letting it erase the whole world.
                if self.rng.chance(0.06) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if t.is_solid() && t != Elem::Void && t != Elem::Glass && t != Elem::Diamond && t != Elem::Obsidian && t != Elem::Sand {
                            self.set_xy(x + dx, y + dy, Elem::Sand);
                            self.set_at(i, Elem::Salt);
                            return true;
                        }
                        if t == Elem::Metal || t == Elem::Iron || t == Elem::Copper {
                            self.set_xy(x + dx, y + dy, Elem::Salt);
                        }
                    }
                }
            }
            Elem::Ice => {
                for (dx, dy) in DIRS4 {
                    let t = self.get(x + dx, y + dy);
                    if t == Elem::Fire || t == Elem::Lava || t == Elem::Magma {
                        self.set_at(i, Elem::Water);
                        return true;
                    }
                }
            }
            Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Roots => {
                for (dx, dy) in DIRS4 {
                    if self.get(x + dx, y + dy) == Elem::Fire && self.rng.chance(0.3) {
                        self.set_at(i, Elem::Fire);
                        return true;
                    }
                }
            }
            Elem::Fire => {
                // Fire spreads to flammable neighbors
                if self.rng.chance(0.15) {
                    for (dx, dy) in DIRS8 {
                        let t = self.get(x + dx, y + dy);
                        if t.flammable() && self.rng.chance(0.2) {
                            self.set_xy(x + dx, y + dy, Elem::Fire);
                        }
                    }
                }
                // Fire + sulfur = plasma (briefly)
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Sulfur {
                            self.set_xy(x + dx, y + dy, Elem::Plasma);
                        }
                    }
                }
                // Fire + gunpowder = explosion
                if self.rng.chance(0.1) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Gunpowder {
                            self.explosion(x + dx, y + dy, 3);
                            return true;
                        }
                    }
                }
                // Fire + methane = bigger fire
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Methane {
                            self.set_xy(x + dx, y + dy, Elem::Fire);
                        }
                    }
                }
            }
            Elem::Oil => {
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Fire {
                            self.set_at(i, Elem::Fire);
                            return true;
                        }
                    }
                }
            }
            Elem::Sulfur => {
                // Sulfur + lava/magma = gas
                if self.rng.chance(0.1) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if t == Elem::Lava || t == Elem::Magma {
                            self.set_at(i, Elem::Gas);
                            return true;
                        }
                    }
                }
            }
            Elem::Mercury => {
                // Mercury amalgamates with gold/copper/silver
                for (dx, dy) in DIRS4 {
                    let t = self.get(x + dx, y + dy);
                    if matches!(t, Elem::Gold | Elem::Copper | Elem::Silver) {
                        if self.rng.chance(0.05) {
                            self.set_xy(x + dx, y + dy, Elem::Metal);
                        }
                    }
                }
            }
            Elem::Plasma => {
                // Plasma cools to gas
                if self.rng.chance(0.2) {
                    self.set_at(i, Elem::Gas);
                }
            }
            Elem::Chlorine => {
                if self.rng.chance(0.1) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if t == Elem::Water {
                            self.set_xy(x + dx, y + dy, Elem::Poison);
                        }
                        if t.is_organic() {
                            self.set_xy(x + dx, y + dy, Elem::Poison);
                        }
                    }
                }
            }
            Elem::Methane => {
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Fire || self.get(x + dx, y + dy) == Elem::Plasma {
                            self.explosion(x + dx, y + dy, 2);
                            return true;
                        }
                    }
                }
            }
            Elem::Oxygen => {
                // Oxygen makes fire burn hotter
                if self.rng.chance(0.1) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Fire {
                            self.set_xy(x + dx, y + dy, Elem::Plasma);
                        }
                    }
                }
            }
            Elem::Gunpowder => {
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Fire || self.get(x + dx, y + dy) == Elem::Plasma {
                            self.explosion(x + dx, y + dy, 4);
                            return true;
                        }
                    }
                }
            }
            Elem::Fertilizer => {
                if self.rng.chance(0.01) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if matches!(t, Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Roots) {
                            // speeds growth (handled in grow step)
                        }
                    }
                }
            }
            Elem::Virus => {
                // Virus spreads slowly - rare and dangerous but not OP
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        let t = self.get(x + dx, y + dy);
                        if t.is_organic() && t != Elem::Virus && self.rng.chance(0.03) {
                            self.set_xy(x + dx, y + dy, Elem::Virus);
                        }
                    }
                }
            }
            Elem::Bacteria => {
                if self.rng.chance(0.2) {
                    for (dx, dy) in DIRS8 {
                        let t = self.get(x + dx, y + dy);
                        if matches!(t, Elem::Meat | Elem::Corpse | Elem::Blood | Elem::Food | Elem::Honey) && self.rng.chance(0.15) {
                            self.set_xy(x + dx, y + dy, Elem::Bacteria);
                        }
                    }
                }
            }
            Elem::Egg => {
                // Hatches after lifetime
                if self.age[i] > 1800 && self.rng.chance(0.01) {
                    self.set_at(i, Elem::Empty);
                    // Creature spawn handled elsewhere
                }
            }
            Elem::Corpse => {
                if self.rng.chance(0.001) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Empty {
                            self.set_xy(x + dx, y + dy, Elem::Bone);
                        }
                    }
                }
                // Decomposes to bones/dust
                if self.age[i] > 3000 && self.rng.chance(0.01) {
                    let to_bone = self.rng.chance(0.5);
                    self.set_at(i, if to_bone { Elem::Bone } else { Elem::Dust });
                }
            }
            Elem::Mana => {
                if self.rng.chance(0.05) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Rune {
                            self.set_xy(x + dx, y + dy, Elem::Portal);
                        }
                    }
                }
            }
            Elem::Soul => {
                if self.rng.chance(0.02) {
                    for (dx, dy) in DIRS4 {
                        if self.get(x + dx, y + dy) == Elem::Ward {
                            self.set_xy(x + dx, y + dy, Elem::Energy);
                        }
                    }
                }
            }
            Elem::Rune => {
                // Runes activate with mana
            }
            Elem::Ward => {
                // Wards repel viruses/bacteria
            }
            _ => {}
        }
        false
    }

    fn explosion(&mut self, cx: i32, cy: i32, radius: i32) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    let x = cx + dx;
                    let y = cy + dy;
                    let e = self.get(x, y);
                    if e != Elem::Void && e != Elem::Diamond && e != Elem::Obsidian {
                        let should_explode = self.rng.chance(0.7);
                        if should_explode {
                            let e2 = if self.rng.chance(0.5) { Elem::Fire } else { Elem::Smoke };
                            self.set_xy(x, y, e2);
                        }
                    }
                }
            }
        }
    }

    /// Keeps the ecosystem running without any input.
    fn maintain(&mut self) {
        let frame = self.frame;
        let w = self.w as i32;

        // Rain (water, occasionally acid rain) - light, so water doesn't take over
        if frame % 30 == 0 {
            let mut added = 0;
            let mut attempts = 0;
            while added < 2 && attempts < 30 {
                attempts += 1;
                let x = self.rng.range(w as u32) as i32;
                let y = self.rng.range(4) as i32;
                if self.get(x, y) == Elem::Empty {
                    let is_acid = self.rng.chance(0.03);
                    let e = if is_acid { Elem::Acid } else { Elem::Water };
                    self.set_xy(x, y, e);
                    added += 1;
                }
            }
        }

        // Underground springs - water seeping up from aquifers (rare)
        if frame % 200 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let y = self.h as i32 - 2 - self.rng.range(20) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        // Surface water bodies maintenance - keep lakes from drying (very rare, minimal)
        if frame % 1000 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x);
                if self.get(x, sy - 1) == Elem::Empty && self.rng.chance(0.1) {
                    self.set_xy(x, sy - 1, Elem::Water);
                }
            }
        }

        // Vegetation regrowth. Slower and pressure-sensitive: plants establish
        // near water, so over-grazing causes real scarcity that then recovers.
        if frame % 15 == 0 && self.census.plants < 1200 {
            let batches = if self.census.plants < 400 { 8 } else { 4 };
            for _ in 0..batches {
                let x = self.rng.range((w - 4) as u32) as i32 + 2;
                let sy = self.surface_y(x);
                let y = sy - 1 - self.rng.range(3) as i32;
                if self.get(x, y) == Elem::Empty {
                    // Slightly favour spots near moisture.
                    if !self.has_water_near(x, y, 5) && !self.rng.chance(0.5) {
                        continue;
                    }
                    let e = if self.rng.chance(0.12) {
                        Elem::Spore
                    } else if self.rng.chance(0.10) {
                        Elem::Fungus
                    } else {
                        Elem::Plant
                    };
                    self.set_xy(x, y, e);
                }
            }
        }

        // Hard cap: thin out vegetation if it starts smothering the world.
        // NOTE: compare decoded elements, never raw byte ranges.
        if frame % 20 == 0 && self.census.plants > 2000 {
            for _ in 0..60 {
                let i = self.rng.range((self.w * self.h) as u32) as usize;
                if matches!(
                    Elem::from_u8(self.kind[i]),
                    Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss
                ) {
                    self.set_at(i, Elem::Empty);
                }
            }
        }

        // Food drifting just above the ground
        if frame % 25 == 0 && self.census.energy < 150 {
            for _ in 0..4 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x).max(14);
                let y = sy - 1 - self.rng.range(12) as i32;
                if self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Energy);
                }
            }
        }

        // Resource spawns near surface
        if frame % 50 == 0 {
            // Oil seeps
            if self.census.oil < 200 {
                for _ in 0..3 {
                    let x = self.rng.range(w as u32) as i32;
                    let sy = self.surface_y(x);
                    let y = sy + 2 + self.rng.range(8) as i32;
                    if self.get(x, y) == Elem::Empty {
                        self.set_xy(x, y, Elem::Oil);
                    }
                }
            }
            // Sulfur vents near lava
            if self.census.sulfur < 150 {
                for _ in 0..2 {
                    let x = self.rng.range(w as u32) as i32;
                    let y = self.h as i32 - 5 - self.rng.range(10) as i32;
                    if self.get(x, y) == Elem::Empty && self.get(x, y + 1) == Elem::Lava {
                        self.set_xy(x, y, Elem::Sulfur);
                    }
                }
            }
        }

        // Meteor strike
        if frame % 1800 == 900 {
            let cx = self.rng.range_i(8, w - 8);
            for dy in -2..13 {
                for dx in -6..7 {
                    if dx * dx + dy * dy <= 42 {
                        self.set_xy(cx + dx, dy, Elem::Lava);
                    }
                }
            }
        }

        // Geological: slow stone formation from mineral deposition
        if frame % 100 == 0 {
            for _ in 0..8 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Stone);
                }
            }
        }

        // Slow ore genesis: deep rock occasionally turns into a fresh vein, so
        // mining is sustainable instead of a one-way drain on the world.
        if frame % 250 == 0 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let sy = self.surface_y(x);
                let y = sy + 8 + self.rng.range((self.h as i32 - sy - 10).max(1) as u32) as i32;
                if y < self.h as i32 - 1 && self.get(x, y) == Elem::Stone {
                    let ore = match self.rng.range(6) {
                        0 => Elem::Copper,
                        1 => Elem::Iron,
                        2 => Elem::Gold,
                        3 => Elem::Silver,
                        4 => Elem::Crystal,
                        _ => Elem::Tin,
                    };
                    self.set_xy(x, y, ore);
                }
            }
        }

        // Crystal growth in caves
        if frame % 200 == 0 && self.census.crystal < 300 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 2) {
                    self.set_xy(x, y, Elem::Crystal);
                }
            }
        }

        // Trim runaway particle counts (firestorms can fill the sky)
        if frame % 300 == 0 {
            let total = self.census.particles as usize;
            if total > (self.w * self.h * 85) / 100 {
                self.compact();
            }
        }
    }

    /// Removes some airborne noise so the world cannot jam completely.
    fn compact(&mut self) {
        for _ in 0..(self.w * self.h / 20) {
            let x = self.rng.range(self.w as u32) as i32;
            let y = self.rng.range(self.h as u32) as i32;
            let e = self.get(x, y);
            if matches!(e, Elem::Smoke | Elem::Steam | Elem::Gas | Elem::Fire) {
                self.set_xy(x, y, Elem::Empty);
            }
        }
    }

    pub fn count(&self) -> Census {
        let mut c = Census::default();
        for &k in &self.kind {
            if k == Elem::Empty as u8 {
                continue;
            }
            c.particles += 1;
            match Elem::from_u8(k) {
                Elem::Plant => c.plants += 1,
                Elem::Fungus => c.plants += 1,
                Elem::Water => c.water += 1,
                Elem::Energy => c.energy += 1,
                Elem::Stone => c.stone += 1,
                Elem::Metal => c.metal += 1,
                Elem::Copper => c.copper += 1,
                Elem::Gold => c.gold += 1,
                Elem::Crystal => c.crystal += 1,
                Elem::Gem => c.gem += 1,
                Elem::Wood => c.wood += 1,
                Elem::Oil => c.oil += 1,
                Elem::Sulfur => c.sulfur += 1,
                _ => {}
            }
        }
        c
    }

    // ------------------------------------------------------------------
    // Creature interaction
    // ------------------------------------------------------------------

    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        !self.get(x, y).is_passable()
    }

    /// Loose debris (powders, ice, organics) that a creature can shove aside or
    /// dig through with its bare hands. Used so falling material can never
    /// permanently entomb a creature.
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

    /// Remove loose debris that settled on a creature's cell. Returns true if
    /// anything was cleared.
    pub fn clear_burrow(&mut self, x: i32, y: i32) -> bool {
        let e = self.get(x, y);
        if e == Elem::Empty || e == Elem::Void {
            return false;
        }
        self.set_xy(x, y, Elem::Empty);
        true
    }

    /// Eats the cell at (x,y). Returns (nutrition, toxicity) if something was eaten.
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

    /// Nearest edible cell around (x,y) within `radius`, used by hungry creatures.
    /// Only real food is targeted, never rock/ore, so the terrain survives.
    pub fn find_food(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > radius * radius {
                    continue;
                }
                let e = self.get(x + dx, y + dy);
                if e.is_edible() && e.nutrition() > 0.0 {
                    if best.map_or(true, |b| d2 < b.2) {
                        best = Some((x + dx, y + dy, d2));
                    }
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    /// Nearest water cell within `radius`, used when a creature is thirsty.
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

    /// Nearest gatherable solid/resource cell within `radius`.
    pub fn find_resource(&self, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx * dx + dy * dy;
                if d2 > radius * radius {
                    continue;
                }
                let e = self.get(x + dx, y + dy);
                let gatherable = matches!(
                    e,
                    Elem::Metal
                        | Elem::Copper
                        | Elem::Tin
                        | Elem::Bronze
                        | Elem::Iron
                        | Elem::Steel
                        | Elem::Gold
                        | Elem::Silver
                        | Elem::Glass
                        | Elem::Crystal
                        | Elem::Gem
                        | Elem::Diamond
                        | Elem::Obsidian
                        | Elem::Wood
                        | Elem::Brick
                        | Elem::Concrete
                        | Elem::Wire
                        | Elem::Circuit
                );
                if gatherable && best.map_or(true, |b| d2 < b.2) {
                    best = Some((x + dx, y + dy, d2));
                }
            }
        }
        best.map(|b| (b.0, b.1))
    }

    /// Leaves remains where a creature died: a corpse/meat for scavengers plus
    /// a little energy, feeding the rest of the food web instead of nothing.
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

    /// Find an empty passable cell within `radius` of (x, y), scanning outward.
    /// Used so creatures never spawn inside solid blocks or deep water.
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
