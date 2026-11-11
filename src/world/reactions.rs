use super::*;

impl World {
    pub(crate) fn try_reaction(&mut self, i: usize, e: Elem, x: i32, y: i32) -> bool {
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

    pub(crate) fn explosion(&mut self, cx: i32, cy: i32, radius: i32) {
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

    // Keeps the ecosystem running without any input.
}
