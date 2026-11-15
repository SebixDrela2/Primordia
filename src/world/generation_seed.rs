use super::{World, Elem, Biome, DepthZone};

/// The generation of a fresh world. The seed lays the ground down in
/// the same order every time, so the same seed gives the same world.
impl World {
    pub fn seed(&mut self) {
        let w = self.w as i32;
        let h = self.h as i32;

        self.cache_biomes();

        self.cache_biomes();

        self.seed_void_rows(w, h);
        self.seed_ground_layers(w, h);
        self.seed_surface_flora(w);
        self.seed_stone_floor(w, h);
        self.seed_volcano_lava(w);
        self.carve_caves(w, h);
        self.seed_granite_band(w, h);
        self.seed_lakes(w, h);
        self.seed_surface_grass(w);
        self.seed_scattered_resources(w);

        self.place_diverse_elements(w, h);
        self.place_extreme_diversity(w, h);
        self.place_rare_vaults(w, h);
        self.place_element_museum(w, h);
        self.guarantee_water(w, h);
        self.bury_floating_resources(w, h);

        self.generate_ore_veins(w, h);
        self.generate_crystal_caves(w, h);
        self.generate_underground_structures(w, h);
        self.bury_floating_resources(w, h);

        self.carve_zorg_cave(w, h);

        self.generate_landmarks(w, h);
        self.generate_volcano(w, h);
        self.bury_floating_resources(w, h);

        self.census = self.count();

        self.update_ground();
        self.rebuild_active();
    }

    /// The bottom of the map is empty space, below the world.
    fn seed_void_rows(&mut self, w: i32, h: i32) {
        for x in 0..w {
            for y in (h - 3)..h {
                self.set_xy(x, y, Elem::Void);
            }
        }
    }

    /// The body of the ground: what each column is made of, from the
    /// surface down to the stone.
    fn seed_ground_layers(&mut self, w: i32, h: i32) {
        for x in 0..w {
            let surface = self.surface_y(x);
            let biome = self.biome_at(x);
            for y in surface..(h - 3) {
                let depth = y - surface;

                let e = self.ground_element(biome, depth);
                let e = self.deep_element(depth, e);
                self.set_xy(x, y, e);
            }
        }
    }

    /// What the biome makes of the ground near the surface.
    fn ground_element(&mut self, biome: Biome, depth: i32) -> Elem {
        match biome {
            Biome::Desert => {
                if depth < 5 || (depth < 9 && self.rng.chance(0.3)) {
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
                if depth < 3 || (depth < 7 && self.rng.chance(0.4)) {
                    Elem::Mud
                } else {
                    Elem::Stone
                }
            }
            Biome::Ashland => {
                if depth < 2 || (depth < 6 && self.rng.chance(0.45)) {
                    Elem::Ash
                } else {
                    Elem::Stone
                }
            }
            Biome::Bloomwood => {
                if depth < 4 || (depth < 8 && self.rng.chance(0.3)) {
                    Elem::Mud
                } else {
                    Elem::Stone
                }
            }
            Biome::Forest | Biome::Grassland => {
                if depth < 3 || (depth < 7 && self.rng.chance(0.2)) {
                    Elem::Mud
                } else {
                    Elem::Stone
                }
            }
            Biome::Mire => {
                if depth < 4 {
                    Elem::Mud
                } else if depth < 8 && self.rng.chance(0.45) {
                    Elem::Fungus
                } else {
                    Elem::Stone
                }
            }
        }
    }

    /// The deep is different rock: metal, crystal and the treasure of
    /// the zones.
    fn deep_element(&mut self, depth: i32, e: Elem) -> Elem {
        if depth <= 6 {
            return e;
        }
        let r = self.rng.f32();
        if r < 0.020 {
            return Elem::Metal;
        }
        if r < 0.032 {
            return Elem::Copper;
        }
        if r < 0.035 {
            return Elem::Glass;
        }
        if r < 0.037 {
            return Elem::Ice;
        }
        if depth > 26 {
            let zone = DepthZone::of(depth);
            let rock = zone.rock(&mut self.rng);
            if self.rng.chance(zone.treasure_chance()) {
                if let Some(t) = zone.treasure(&mut self.rng) {
                    return t;
                }
            }
            return rock;
        }
        e
    }

    /// The plants, trees and pools that sit on the surface, one column
    /// at a time.
    fn seed_surface_flora(&mut self, w: i32) {
        for x in 0..w {
            let biome = self.biome_at(x);
            let surface = self.surface_y(x);
            match biome {
                Biome::Forest => self.seed_forest_flora(x, surface),
                Biome::Desert => self.seed_desert_flora(x, surface),
                Biome::Swamp => self.seed_swamp_flora(x, surface),
                Biome::Tundra => self.seed_tundra_flora(x, surface),
                Biome::Grassland => self.seed_grassland_flora(x, surface),
                Biome::Ashland => self.seed_ashland_flora(x, surface),
                Biome::Bloomwood => self.seed_bloomwood_flora(x, surface),
                Biome::Mire => self.seed_mire_flora(x, surface),
            }
        }
    }

    fn seed_forest_flora(&mut self, x: i32, surface: i32) {
        if !self.rng.chance(0.35) {
            return;
        }
        let h = 3 + self.rng.range(6) as i32;
        for k in 1..h {
            let y = surface - k;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Wood);
            }
        }

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

    fn seed_desert_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.25) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Sand);
            }
        }
    }

    fn seed_swamp_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.3) {
            for dx in 0..3 {
                let y = surface + self.rng.range(2) as i32;
                self.set_xy(x + dx, y, Elem::Water);
            }
        }
    }

    fn seed_tundra_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.2) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Ice);
            }
        }
    }

    fn seed_grassland_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.3) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Plant);
            }
        }
    }

    fn seed_ashland_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.3) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Charcoal);
            }
        }
        if self.rng.chance(0.04) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Ember);
            }
        }
    }

    fn seed_bloomwood_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.45) {
            let h = 4 + self.rng.range(7) as i32;
            for k in 1..h {
                let y = surface - k;
                if self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Wood);
                }
            }
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
        if self.rng.chance(0.45) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Fungus);
            }
        }
    }

    fn seed_mire_flora(&mut self, x: i32, surface: i32) {
        if self.rng.chance(0.45) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Fungus);
            }
        }
        if self.rng.chance(0.3) {
            let y = surface - 1;
            if self.get(x, y) == Elem::Empty {
                self.set_xy(x, y, Elem::Spore);
            }
        }
        if self.rng.chance(0.12) {
            for dx in 0..3 {
                let y = surface + self.rng.range(2) as i32;
                self.set_xy(x + dx, y, Elem::Water);
            }
        }
    }

    /// The floor of the world is solid stone.
    fn seed_stone_floor(&mut self, w: i32, h: i32) {
        for x in 0..w {
            for y in (h - 5)..(h - 3) {
                self.set_xy(x, y, Elem::Stone);
            }
        }
    }

    /// The pool of lava that the volcano sits over.
    fn seed_volcano_lava(&mut self, w: i32) {
        let cx = self.rng.range((w as u32 * 35 / 100) - (w as u32 * 10 / 100)) as i32
            + (w as u32 * 10 / 100) as i32;

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
    }

    /// The band of granite deep under the rock.
    fn seed_granite_band(&mut self, w: i32, h: i32) {
        for x in 0..w {
            let surface = self.surface_y(x);
            let granite_at = surface + 90 + (self.rng.range(40) as i32);
            for y in granite_at..(h - 6) {
                if self.get(x, y) == Elem::Stone {
                    self.set_xy(x, y, Elem::Granite);
                }
            }
        }
    }

    /// The lake and the ponds that are dug into the ground.
    fn seed_lakes(&mut self, w: i32, h: i32) {
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
    }

    /// The grass and the driftwood on the surface.
    fn seed_surface_grass(&mut self, w: i32) {
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
    }

    /// The loose bits that are sprinkled over the map: energy, copper,
    /// gold and crystal.
    fn seed_scattered_resources(&mut self, w: i32) {
        for _ in 0..60 {
            let x = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(x).max(14);
            let y = sy - 1 - self.rng.range(12) as i32;
            self.set_xy(x, y, Elem::Energy);
        }

        for _ in 0..200 {
            let x = self.rng.range(w as u32) as i32;
            let sy = self.surface_y(x);
            let y = sy + self.rng.range(3) as i32;
            if self.get(x, y) == Elem::Stone && self.rng.chance(0.6) {
                self.set_xy(x, y, Elem::Copper);
            }
        }

        for x in (5..w - 5).step_by(15) {
            let sy = self.surface_y(x);
            for dy in 0..=3 {
                if self.get(x, sy + dy) == Elem::Stone {
                    self.set_xy(x, sy + dy, Elem::Copper);
                }
            }
        }

        for _ in 0..15 {
            let x = self.rng.range(w as u32) as i32;
            let y = self.h as i32 - 10 - self.rng.range(15) as i32;
            if self.get(x, y) == Elem::Stone && self.rng.chance(0.2) {
                self.set_xy(x, y, Elem::Gold);
            }
        }

        for _ in 0..20 {
            let x = self.rng.range(w as u32) as i32;
            let y = self.rng.range(self.h as u32) as i32;
            if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 3) {
                self.set_xy(x, y, Elem::Crystal);
            }
        }
    }
}
