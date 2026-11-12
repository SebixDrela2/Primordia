use super::*;

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
            ground: vec![0; w],
            ground_inv: vec![1.0; w],
            biome_tint: vec![[1.0, 1.0, 1.0]; w],
            biome: vec![super::Biome::Grassland; w],
            active: Vec::with_capacity(n / 8),
            next_active: Vec::with_capacity(n / 8),
            listed: vec![false; n],
            inert: Elem::inert_table(),
            food_table: {
                let mut t = [false; Elem::TABLE_LEN];
                let mut i = 0;
                while i < Elem::TABLE_LEN {
                    let e = Elem::from_u8(i as u8);
                    t[i] = e.is_edible() && e.nutrition() > 0.0;
                    i += 1;
                }
                t
            },
            gatherable: {
                let mut t = [false; Elem::TABLE_LEN];
                let mut i = 0;
                while i < Elem::TABLE_LEN {
                    t[i] = matches!(
                        Elem::from_u8(i as u8),
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
                            | Elem::Granite
                            | Elem::Rust
                            | Elem::Wood
                            | Elem::Charcoal
                            | Elem::Brick
                            | Elem::Concrete
                            | Elem::Wire
                            | Elem::Circuit
                            | Elem::Gunpowder
                            | Elem::Tar
                    );
                    i += 1;
                }
                t
            },
            emissive: Elem::emissive_table(),
            emission: Elem::emission_table(),
            weather: Weather::Clear,
            weather_timer: 900,
            meteor_events: Vec::new(),
            rng: Rng::new(seed),
            frame: 0,
            mine_budget: 0,
            pressure: super::Pressure::default(),
            plant_capacity: 0.0,
            census: Census::default(),
            landmarks: Vec::new(),
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
        self.mark_active(i);
    }

    #[inline]
    pub(crate) fn swap_cells(&mut self, a: usize, b: usize) {
        self.kind.swap(a, b);
        self.col.swap(a, b);
        self.heat.swap(a, b);
        self.age.swap(a, b);
        self.mark_active(a);
        self.mark_active(b);
    }

    #[inline]
    pub(crate) fn can_move_into(&self, mover: Elem, target: Elem) -> bool {
        if target == Elem::Empty {
            return true;
        }
        let target_movable = matches!(
            target.behavior(),
            Behavior::Flow | Behavior::Rise | Behavior::Diffuse
        );
        mover.density() > target.density() && target_movable
    }

    // Moves the cell at `i` by (dx,dy). Returns the new index if it moved.
    pub(crate) fn try_move(&mut self, i: usize, dx: i32, dy: i32) -> Option<usize> {
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

}
