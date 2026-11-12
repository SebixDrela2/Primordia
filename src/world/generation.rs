use super::*;

impl World {
    pub fn seed(&mut self) {
        let w = self.w as i32;
        let h = self.h as i32;

        // The climate layout never changes, so it is cached once here.
        self.cache_biomes();

        // The climate layout never changes, so it is cached once here.
        self.cache_biomes();

        // Bedrock layer at very bottom (impermeable)
        for x in 0..w {
            for y in (h - 3)..h {
                self.set_xy(x, y, Elem::Void);
            }
        }

        for x in 0..w {
            let surface = self.surface_y(x);
            let biome = self.biome_at(x);
            for y in surface..(h - 3) {
                let depth = y - surface;
                // Each biome has its own soil profile.
                let mut e = match biome {
                    Biome::Desert => {
                        if depth < 5 {
                            Elem::Sand
                        } else if depth < 9 && self.rng.chance(0.3) {
                            Elem::Sand
                        } else {
                            Elem::Stone
                        }
                    }
                    Biome::Tundra => {
                        if depth < 1 {
                            Elem::Ice
                        } else if depth < 4 {
                            if self.rng.chance(0.35) {
                                Elem::Ice
                            } else {
                                Elem::Stone
                            }
                        } else {
                            Elem::Stone
                        }
                    }
                    Biome::Swamp => {
                        if depth < 3 {
                            Elem::Mud
                        } else if depth < 7 && self.rng.chance(0.4) {
                            Elem::Mud
                        } else {
                            Elem::Stone
                        }
                    }
                    Biome::Forest | Biome::Grassland => {
                        if depth < 3 {
                            // Soil: solid ground, so grazing never digs holes.
                            Elem::Mud
                        } else if depth < 7 && self.rng.chance(0.2) {
                            Elem::Mud
                        } else {
                            Elem::Stone
                        }
                    }
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

        // Forests grow actual trees, deserts grow dunes of sand.
        for x in 0..w {
            let biome = self.biome_at(x);
            let surface = self.surface_y(x);
            match biome {
                Biome::Forest => {
                    if self.rng.chance(0.35) {
                        let h = 3 + self.rng.range(6) as i32;
                        for k in 1..h {
                            let y = surface - k;
                            if self.get(x, y) == Elem::Empty {
                                self.set_xy(x, y, Elem::Wood);
                            }
                        }
                        // Canopy
                        let top = surface - h;
                        for dx in -2..=2 {
                            for dy in 0..3 {
                                let (cx, cy) = (x + dx, top + dy);
                                if self.get(cx, cy) == Elem::Empty {
                                    self.set_xy(cx, cy, Elem::Plant);
                                }
                            }
                        }
                    }
                }
                Biome::Desert => {
                    if self.rng.chance(0.25) {
                        let y = surface - 1;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Sand);
                        }
                    }
                }
                Biome::Swamp => {
                    if self.rng.chance(0.3) {
                        // Carve a shallow pool into the swamp floor so the
                        // water sits in the ground instead of floating.
                        for dx in 0..3 {
                            let y = surface + self.rng.range(2) as i32;
                            self.set_xy(x + dx, y, Elem::Water);
                        }
                    }
                }
                Biome::Tundra => {
                    if self.rng.chance(0.2) {
                        let y = surface - 1;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Ice);
                        }
                    }
                }
                Biome::Grassland => {
                    if self.rng.chance(0.3) {
                        let y = surface - 1;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Plant);
                        }
                    }
                }
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
        // The chamber must sit *below* the surface, otherwise the lava ends up
        // floating in the sky.
        let cy = self.surface_y(cx) + 45 + self.rng.range(70) as i32;
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

        // Carve caves into the rock. Without them the underground is a
        // solid block with nothing to explore, mine or light.
        self.carve_caves(w, h);

        // Granite: a deep igneous layer that only appears far underground.
        // It is much harder than stone, so it marks the oldest rock.
        for x in 0..w {
            let surface = self.surface_y(x);
            let granite_at = surface + 90 + (self.rng.range(40) as i32);
            for y in granite_at..(h - 6) {
                if self.get(x, y) == Elem::Stone {
                    self.set_xy(x, y, Elem::Granite);
                }
            }
        }

        // Surface lake - smaller, so most of the world is land.
        // The basin is *carved into* the terrain and the water fills it, so
        // the water surface sits level with the ground. Placing the water
        // above the ground instead left it floating in the sky.
        let lake_start = (w as f32 * 0.62) as i32;
        let lake_end = (w as f32 * 0.78) as i32;
        for x in lake_start..lake_end {
            let t = (x - lake_start) as f32 / (lake_end - lake_start) as f32;
            let depth = 3.0 + (t * std::f32::consts::PI).sin() * 5.0;
            let surface = self.surface_y(x);
            for dy in 0..depth as i32 {
                let y = surface + dy;
                if y < h - 6 {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        // A few smaller ponds, also carved into the ground.
        for _ in 0..2 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let pond_w = 2 + self.rng.range(3) as i32;
            for px in (x - pond_w / 2)..=(x + pond_w / 2) {
                let depth = 1 + self.rng.range(2) as i32;
                for dy in 0..depth {
                    let y = surface + dy;
                    if y < h - 6 {
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

        // Richer underground: ore veins, crystal caves and ancient ruins give
        // the world real places to dig and to fight over.
        self.generate_ore_veins(w, h);
        self.generate_crystal_caves(w, h);
        self.generate_underground_structures(w, h);
        self.bury_floating_resources(w, h);

        // The places worth living in. They go on the surface, so they are
        // painted before the ground level is cached.
        self.generate_landmarks(w, h);
        self.bury_floating_resources(w, h);

        self.census = self.count();
        // The world is fully built now: cache the ground level and collect
        // every cell that can still change.
        self.update_ground();
        self.rebuild_active();
    }

    /// Carve a network of caves into the rock below the surface.
    ///
    /// The caves are carved with a cellular automaton so they form winding,
    /// connected passages instead of round pockets. Every cave keeps a floor
    /// of solid rock, so the world stays stable and resources do not end up
    /// floating in the sky.
    pub(crate) fn carve_caves(&mut self, w: i32, h: i32) {
        const DEPTH: i32 = 14; // how far below the surface the caves start
        const CELL: usize = 24; // size of a cave seed cell
        let cw = w as usize;
        let ch = h as usize;
        let gw = (w as usize).div_ceil(CELL) + 1;
        let gh = (h as usize).div_ceil(CELL) + 1;

        // Seed: mark the coarse cells that become cave.
        let mut seed = vec![false; gw * gh];
        for gy in 0..gh {
            for gx in 0..gw {
                let wy = (gy * CELL) as i32;
                let wx = (gx * CELL) as i32;
                // Keep the caves below the surface and above the bedrock.
                let top = self.surface_y(wx) + DEPTH;
                let bottom = (h - 24).max(top + 1);
                if wy < top || wy > bottom {
                    continue;
                }
                // Denser in the middle depths, sparser near the surface.
                let depth = (wy - top).max(0) as f32;
                let chance = 0.30 - (depth / (bottom - top).max(1) as f32) * 0.12;
                if self.rng.chance(chance) {
                    seed[gy * gw + gx] = true;
                }
            }
        }

        // Expand the seeds into a fine grid. Each seed grows a pair of
        // winding tunnels rather than a round room, so the caves read as
        // passages that a creature could travel along.
        let mut cave = vec![false; cw * ch];
        for gy in 0..gh {
            for gx in 0..gw {
                if !seed[gy * gw + gx] {
                    continue;
                }
                let cx = (gx * CELL) as i32;
                let cy = (gy * CELL) as i32;
                // The tunnel starts here and wanders off in a random
                // direction, changing direction as it goes.
                let mut tx = cx;
                let mut ty = cy;
                let mut angle = self.rng.f32() * std::f32::consts::TAU;
                let len = 14 + self.rng.range(26) as i32;
                for _ in 0..len {
                    angle += (self.rng.f32() - 0.5) * 1.1;
                    tx += (angle.cos() * 2.0) as i32;
                    ty += (angle.sin() * 2.0) as i32;
                    let radius = 1 + self.rng.range(3) as i32;
                    // Carve a blob around the tunnel centre.
                    for y in (ty - radius).max(0)..(ty + radius).min(h as i32) {
                        for x in (tx - radius).max(0)..(tx + radius).min(w as i32) {
                            let dx = (x - tx) as f32 / (radius + 1) as f32;
                            let dy = (y - ty) as f32 / (radius + 1) as f32;
                            if dx * dx + dy * dy <= 1.0 {
                                cave[y as usize * cw + x as usize] = true;
                            }
                        }
                    }
                }
                // A slightly wider chamber where the tunnel starts.
                let rr = 3 + self.rng.range(3) as i32;
                for y in (cy - rr).max(0)..(cy + rr).min(h as i32) {
                    for x in (cx - rr).max(0)..(cx + rr).min(w as i32) {
                        let dx = (x - cx) as f32 / rr as f32;
                        let dy = (y - cy) as f32 / rr as f32;
                        if dx * dx + dy * dy <= 1.0 {
                            cave[y as usize * cw + x as usize] = true;
                        }
                    }
                }
            }
        }

        // Erode: drop cave cells with too few cave neighbours, so the
        // passages become thin and winding instead of wide rooms.
        let mut eroded = cave.clone();
        for y in 1..(h as i32 - 1) {
            for x in 1..(w as i32 - 1) {
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

        // Dig the caves out, but never through the bedrock, and never above
        // the surface.
        for x in 0..w {
            let surf = self.surface_y(x);
            let floor = surf + DEPTH;
            for y in floor..(h - 4) {
                let i = y as usize * cw + x as usize;
                if !eroded[i] {
                    continue;
                }
                // Keep a solid floor under every cave so nothing floats and
                // the surface stays intact.
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

    // Move solid/resource cells that ended up floating above the terrain down
    // into the rock. Several placement passes historically used absolute
    // heights (`h/2`) that sat above the surface, leaving ore and minerals
    // hovering in the sky. This guarantees resources live underground.
    pub(crate) fn bury_floating_resources(&mut self, w: i32, h: i32) {
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
                            | Elem::Tar
                    );
                // Leave organic growth, airborne gases and water lakes alone.
                if is_resource
                    && !matches!(
                        e,
                        Elem::Plant
                            | Elem::Wood
                            | Elem::Moss
                            | Elem::Spore
                            | Elem::Roots
                            | Elem::Water
                            | Elem::Ice
                            | Elem::Snow
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

    // Guarantee water exists by placing an underground lake below the surface.
    pub(crate) fn guarantee_water(&mut self, w: i32, h: i32) {
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

    // Element museum: place one cell of EVERY element type so all 79 exist at seed time.
    // Placed underground below the surface, surrounded by stone walls so they persist.
}

impl World {
    pub(crate) fn generate_surface_water(&mut self, w: i32, h: i32) {
        // Main lake, carved into the terrain so the water sits in a basin
        // instead of floating above the ground.
        let lake_start = (w as f32 * 0.55) as i32;
        let lake_end = (w as f32 * 0.75) as i32;
        for x in lake_start..lake_end {
            let t = (x - lake_start) as f32 / (lake_end - lake_start) as f32;
            let depth = 4.0 + (t * std::f32::consts::PI).sin() * 8.0;
            let surface = self.surface_y(x);
            for dy in 0..depth as i32 {
                let y = surface + dy;
                if y < h - 6 {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        // Additional ponds
        for _ in 0..4 {
            let x = self.rng.range(w as u32) as i32;
            let surface = self.surface_y(x);
            let pond_w = 3 + self.rng.range(4) as i32;
            for px in (x - pond_w / 2)..=(x + pond_w / 2) {
                let depth = 1 + self.rng.range(3) as i32;
                for dy in 0..depth {
                    let y = surface + dy;
                    if y < h - 6 {
                        self.set_xy(px, y, Elem::Water);
                    }
                }
            }
        }

        // River: a channel carved along the terrain, following its slope.
        let river_y = self.surface_y(w / 2);
        for x in (w / 4)..(3 * w / 4) {
            if self.rng.chance(0.85) {
                let y = river_y as f32 + (self.rng.f32() - 0.5) * 3.0;
                let y = y as i32;
                if y > 1 && y < h - 6 {
                    self.set_xy(x, y, Elem::Water);
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
                    let py = surface + dy;
                    if px > 0 && py > 0 && px < w && py < h - 6 {
                        if self.rng.chance(0.3) {
                            self.set_xy(px, py, Elem::Water);
                        }
                    }
                }
            }
        }
    }

}

impl World {
    pub(crate) fn generate_ore_veins(&mut self, w: i32, h: i32) {
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

        // Rust: corroded iron, found where iron meets damp ground.
        for _ in 0..18 {
            let cx = self.rng.range(w as u32) as i32;
            let cy = self.surface_y(cx) + 15 + self.rng.range(45) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx * dx + dy * dy <= 6 && self.rng.chance(0.3) {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x > 1 && x < w - 1 && y > 1 && y < h - 5 {
                            let e = self.get(x, y);
                            if e == Elem::Iron || e == Elem::Metal {
                                self.set_xy(x, y, Elem::Rust);
                            }
                        }
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
            let cy = self.surface_y(cx) + 20 + self.rng.range(50) as i32;
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

    // Rare vaults: place at least a few cells of EVERY element type so the
    // world always contains full element variety, even if some are super rare.
}

impl World {
    pub(crate) fn generate_crystal_caves(&mut self, w: i32, h: i32) {
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

}
