use super::{World, Elem, DIRS4, DIRS8};

impl World {
    #[must_use]
    pub fn try_reaction(&mut self, index: usize, elem: Elem, cell_x: i32, cell_y: i32) -> bool {
        match elem {
            Elem::Water => return self.react_with_water(index, cell_x, cell_y),
            Elem::Lava => return self.react_with_lava(index, cell_x, cell_y),
            Elem::Magma => return self.react_with_magma(index, cell_x, cell_y),
            Elem::Acid => return self.react_with_acid(index, cell_x, cell_y),
            Elem::Ice => return self.react_with_ice(index, cell_x, cell_y),
            Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Roots => {
                return self.react_plant_with_fire(index, cell_x, cell_y);
            }
            Elem::Fire => return self.react_with_fire(cell_x, cell_y),
            Elem::Oil => return self.react_with_oil(index, cell_x, cell_y),
            Elem::Sulfur => return self.react_with_sulfur(index, cell_x, cell_y),
            Elem::Mercury => {
                self.react_with_mercury(cell_x, cell_y);
            }
            Elem::Plasma => {
                self.react_with_plasma(index);
            }
            Elem::Chlorine => {
                self.react_with_chlorine(cell_x, cell_y);
            }
            Elem::Methane => return self.react_with_methane(cell_x, cell_y),
            Elem::Oxygen => return self.react_with_oxygen(index, cell_x, cell_y),
            Elem::Gunpowder => return self.react_with_gunpowder(cell_x, cell_y),
            Elem::Fertilizer => {
                self.react_with_fertilizer(cell_x, cell_y);
            }
            Elem::Virus => {
                self.react_with_virus(cell_x, cell_y);
            }
            Elem::Bacteria => {
                self.react_with_bacteria(cell_x, cell_y);
            }
            Elem::Egg => {
                self.react_with_egg(index);
            }
            Elem::Corpse => {
                self.react_with_corpse(index, cell_x, cell_y);
            }
            Elem::Mana => {
                self.react_with_mana(cell_x, cell_y);
            }
            Elem::Soul if self.rng.chance(0.02) => {
                self.react_with_soul(cell_x, cell_y);
            }
            _ => {}
        }
        false
    }

    /// The water meets the fire, the lava and the acid.
    fn react_with_water(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.5) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor == Elem::Lava || neighbor == Elem::Magma {
                    self.set_at(index, Elem::Steam);
                    self.set_xy(cell_x + dx, cell_y + dy, if neighbor == Elem::Lava { Elem::Stone } else { Elem::Obsidian });
                    return true;
                }
                if neighbor == Elem::Fire {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Steam);
                }
                if neighbor == Elem::Acid {
                    self.set_at(index, Elem::Steam);
                }
                if neighbor == Elem::Chlorine {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Poison);
                }
                if (neighbor == Elem::Lava || neighbor == Elem::Magma)
                    && self.rng.chance(0.3) {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Obsidian);
                    }
            }
        }

        if self.rng.chance(0.1) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Gunpowder {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Mud);
                }
            }
        }
        false
    }

    /// The lava eats its way into the rock.
    fn react_with_lava(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        for (dx, dy) in DIRS4 {
            match self.get(cell_x + dx, cell_y + dy) {
                Elem::Water => {
                    self.set_at(index, Elem::Stone);
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Steam);
                    return true;
                }
                Elem::Sand => {
                    if self.rng.chance(0.05) {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Glass);
                    }
                }
                Elem::Sulfur => {
                    if self.rng.chance(0.1) {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Gas);
                    }
                }
                Elem::Ice => {
                    if self.rng.chance(0.1) {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Water);
                    }
                }
                Elem::Metal | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Iron | Elem::Steel | Elem::Gold | Elem::Silver => {
                    if self.rng.chance(0.02) {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Magma);
                    }
                }
                Elem::Stone | Elem::Brick | Elem::Concrete
                    if self.rng.chance(0.0005) => {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Magma);
                    }
                _ => {}
            }
        }
        false
    }

    /// The magma is hotter still.
    fn react_with_magma(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        for (dx, dy) in DIRS4 {
            match self.get(cell_x + dx, cell_y + dy) {
                Elem::Water => {
                    self.set_at(index, Elem::Obsidian);
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Steam);
                    return true;
                }
                Elem::Stone
                    if self.rng.chance(0.0005) => {
                        self.set_xy(cell_x + dx, cell_y + dy, Elem::Magma);
                    }
                _ => {}
            }
        }

        if self.rng.chance(0.001) {
            let to_obsidian = self.rng.chance(0.3);
            self.set_at(index, if to_obsidian { Elem::Obsidian } else { Elem::Stone });
        }
        false
    }

    /// The acid eats the rock and the metal.
    fn react_with_acid(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.06) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor.is_solid() && neighbor != Elem::Void && neighbor != Elem::Glass && neighbor != Elem::Diamond && neighbor != Elem::Obsidian && neighbor != Elem::Sand {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Sand);
                    self.set_at(index, Elem::Salt);
                    return true;
                }
                if neighbor == Elem::Metal || neighbor == Elem::Iron || neighbor == Elem::Copper {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Salt);
                }
            }
        }
        false
    }

    /// The ice melts next to the fire.
    fn react_with_ice(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        for (dx, dy) in DIRS4 {
            let neighbor = self.get(cell_x + dx, cell_y + dy);
            if neighbor == Elem::Fire || neighbor == Elem::Lava || neighbor == Elem::Magma {
                self.set_at(index, Elem::Water);
                return true;
            }
        }
        false
    }

    /// The green catches fire.
    fn react_plant_with_fire(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        for (dx, dy) in DIRS4 {
            if self.get(cell_x + dx, cell_y + dy) == Elem::Fire && self.rng.chance(0.3) {
                self.set_at(index, Elem::Fire);
                return true;
            }
        }
        false
    }

    /// The fire spreads, and it meets the powder and the gas.
    fn react_with_fire(&mut self, cell_x: i32, cell_y: i32) -> bool {
        let mut air = false;
        for (dx, dy) in DIRS8 {
            if self.get(cell_x + dx, cell_y + dy) == Elem::Oxygen {
                air = true;
                break;
            }
        }

        if air && self.rng.chance(0.15) {
            for (dx, dy) in DIRS8 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor.flammable() && self.rng.chance(0.2) {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Fire);
                }
            }
        }

        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Sulfur {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Plasma);
                }
            }
        }

        if self.rng.chance(0.1) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Gunpowder {
                    self.explosion(cell_x + dx, cell_y + dy, 3);
                    return true;
                }
            }
        }

        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Methane {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Fire);
                }
            }
        }
        false
    }

    /// The oil catches fire.
    fn react_with_oil(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Fire {
                    self.set_at(index, Elem::Fire);
                    return true;
                }
            }
        }
        false
    }

    /// The sulfur gives off its gas over the lava.
    fn react_with_sulfur(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.1) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor == Elem::Lava || neighbor == Elem::Magma {
                    self.set_at(index, Elem::Gas);
                    return true;
                }
            }
        }
        false
    }

    /// The mercury takes the gold and the silver.
    fn react_with_mercury(&mut self, cell_x: i32, cell_y: i32) {
        for (dx, dy) in DIRS4 {
            let neighbor = self.get(cell_x + dx, cell_y + dy);
            if matches!(neighbor, Elem::Gold | Elem::Copper | Elem::Silver)
                && self.rng.chance(0.05) {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Metal);
            }
        }
    }

    /// The plasma fades into the gas.
    fn react_with_plasma(&mut self, index: usize) {
        if self.rng.chance(0.2) {
            self.set_at(index, Elem::Gas);
        }
    }

    /// The chlorine poisons the water and the living.
    fn react_with_chlorine(&mut self, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.1) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor == Elem::Water {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Poison);
                }
                if neighbor.is_organic() {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Poison);
                }
            }
        }
    }

    /// The methane goes up with a bang.
    fn react_with_methane(&mut self, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Fire || self.get(cell_x + dx, cell_y + dy) == Elem::Plasma {
                    self.explosion(cell_x + dx, cell_y + dy, 2);
                    return true;
                }
            }
        }
        false
    }

    /// The oxygen puts the fire out, and the methane up.
    fn react_with_oxygen(&mut self, index: usize, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.2) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Fire {
                    self.set_at(index, Elem::Empty);
                    return true;
                }
            }
        }

        if self.rng.chance(0.02) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Methane {
                    self.explosion(cell_x + dx, cell_y + dy, 3);
                    return true;
                }
            }
        }
        false
    }

    /// The powder goes up.
    fn react_with_gunpowder(&mut self, cell_x: i32, cell_y: i32) -> bool {
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Fire || self.get(cell_x + dx, cell_y + dy) == Elem::Plasma {
                    self.explosion(cell_x + dx, cell_y + dy, 4);
                    return true;
                }
            }
        }
        false
    }

    /// The fertilizer feeds the green.
    fn react_with_fertilizer(&mut self, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.01) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if matches!(neighbor, Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Roots) {

                }
            }
        }
    }

    /// The virus spreads through the living.
    fn react_with_virus(&mut self, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if neighbor.is_organic() && neighbor != Elem::Virus && self.rng.chance(0.03) {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Virus);
                }
            }
        }
    }

    /// The bacteria takes the meat.
    fn react_with_bacteria(&mut self, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.2) {
            for (dx, dy) in DIRS8 {
                let neighbor = self.get(cell_x + dx, cell_y + dy);
                if matches!(neighbor, Elem::Meat | Elem::Corpse | Elem::Blood | Elem::Food | Elem::Honey) && self.rng.chance(0.15) {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Bacteria);
                }
            }
        }
    }

    /// The old egg goes bad.
    fn react_with_egg(&mut self, index: usize) {
        if self.age[index] > 1800 && self.rng.chance(0.01) {
            self.set_at(index, Elem::Empty);

        }
    }

    /// The corpse turns to bone and to dust.
    fn react_with_corpse(&mut self, index: usize, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.001) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Empty {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Bone);
                }
            }
        }

        if self.age[index] > 3000 && self.rng.chance(0.01) {
            let to_bone = self.rng.chance(0.5);
            self.set_at(index, if to_bone { Elem::Bone } else { Elem::Dust });
        }
    }

    /// The mana wakes the runes.
    fn react_with_mana(&mut self, cell_x: i32, cell_y: i32) {
        if self.rng.chance(0.05) {
            for (dx, dy) in DIRS4 {
                if self.get(cell_x + dx, cell_y + dy) == Elem::Rune {
                    self.set_xy(cell_x + dx, cell_y + dy, Elem::Portal);
                }
            }
        }
    }

    /// The soul feeds the ward.
    fn react_with_soul(&mut self, cell_x: i32, cell_y: i32) {
        for (dx, dy) in DIRS4 {
            if self.get(cell_x + dx, cell_y + dy) == Elem::Ward {
                self.set_xy(cell_x + dx, cell_y + dy, Elem::Energy);
            }
        }
    }

    pub fn explosion(&mut self, cx: i32, cy: i32, radius: i32) {
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

}
