use super::{World, Elem, Volcano, Weather, Rng, Census, Behavior};

/// Which elements can be eaten for nourishment.
fn build_food_table() -> [bool; Elem::TABLE_LEN] {
    let mut table = [false; Elem::TABLE_LEN];
    let mut idx = 0;
    while idx < Elem::TABLE_LEN {
        let elem = Elem::from_u8(idx as u8);
        table[idx] = elem.is_edible() && elem.nutrition() > 0.0;
        idx += 1;
    }
    table
}

/// Which elements a creature can pick up and carry.
fn build_gatherable_table() -> [bool; Elem::TABLE_LEN] {
    let mut table = [false; Elem::TABLE_LEN];
    let mut idx = 0;
    while idx < Elem::TABLE_LEN {
        table[idx] = matches!(
            Elem::from_u8(idx as u8),
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
        idx += 1;
    }
    table
}

/// Which elements the ZORGs will eat.
fn build_zorg_food_table() -> [bool; Elem::TABLE_LEN] {
    let mut table = [false; Elem::TABLE_LEN];
    let mut idx = 0;
    while idx < Elem::TABLE_LEN {
        table[idx] = crate::creature::zorg_food(Elem::from_u8(idx as u8));
        idx += 1;
    }
    table
}

/// How much the ZORGs get out of each element.
fn build_zorg_nutrition_table() -> [f32; Elem::TABLE_LEN] {
    let mut table = [0.0f32; Elem::TABLE_LEN];
    let mut idx = 0;
    while idx < Elem::TABLE_LEN {
        table[idx] = crate::creature::zorg_nutrition(Elem::from_u8(idx as u8));
        idx += 1;
    }
    table
}

impl World {
    #[must_use]
    pub fn new(width: usize, height: usize, seed: u64) -> Self {
        let cells = width * height;
        World {
            w: width,
            h: height,
            kind: vec![Elem::Empty as u8; cells],
            owner: vec![0u8; cells],
            col: vec![[10, 14, 23]; cells],
            heat: vec![0; cells],
            age: vec![0; cells],
            ground: vec![0; width],
            ground_inv: vec![1.0; width],
            biome_tint: vec![[1.0, 1.0, 1.0]; width],
            biome: vec![super::Biome::Grassland; width],
            active: Vec::with_capacity(cells / 8),
            next_active: Vec::with_capacity(cells / 8),
            listed: vec![false; cells],
            inert: Elem::inert_table(),
            food_table: build_food_table(),
            gatherable: build_gatherable_table(),
            emissive: Elem::emissive_table(),
            emission: Elem::emission_table(),
            zorg_food_table: build_zorg_food_table(),
            zorg_nutrition_table: build_zorg_nutrition_table(),
            zorg_cave_floor: 0,
            volcano: None,
            season: 0,
            daylight: 0.5,
            difficulty: 0.0,
            volcano_state: Volcano::Sleeping,
            volcano_timer: 1200,
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
            zorg_cave: None,
            zorg_cave_sealed: false,
            deep_mining: 0,
        }
    }

    #[inline]
    #[must_use]
    pub fn idx_xy(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            None
        } else {
            Some(y as usize * self.w + x as usize)
        }
    }

    #[inline]
    #[must_use]
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

        self.owner[i] = 0;
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
    pub fn swap_cells(&mut self, a: usize, b: usize) {
        self.kind.swap(a, b);
        self.owner.swap(a, b);
        self.col.swap(a, b);
        self.heat.swap(a, b);
        self.age.swap(a, b);
        self.mark_active(a);
        self.mark_active(b);
    }

    #[inline]
    #[must_use]
    pub fn can_move_into(&self, mover: Elem, target: Elem) -> bool {
        if target == Elem::Empty {
            return true;
        }
        let target_movable = matches!(
            target.behavior(),
            Behavior::Flow | Behavior::Rise | Behavior::Diffuse
        );
        mover.density() > target.density() && target_movable
    }

    #[must_use]
    pub fn try_move(&mut self, i: usize, dx: i32, dy: i32) -> Option<usize> {
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

}
