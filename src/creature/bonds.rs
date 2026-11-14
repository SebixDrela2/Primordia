use super::*;

pub const BOND_RANGE: f32 = 12.0;

impl Creature {

    pub fn bondable(&self) -> bool {
        self.personality.sociability > 0.55
            && self.stage.can_breed()
            && self.bond.is_none()
            && !self.migrating
    }

    pub fn bond_morale(&self, partner_alive: bool) -> f32 {
        match (self.bond, partner_alive) {
            (Some(_), true) => 1.0,
            (Some(_), false) => -1.0,
            (None, _) => 0.0,
        }
    }
}

impl CreatureSystem {

    pub fn form_bonds(&mut self, grid: &SpatialGrid) {

        if self.tick % 24 != 0 {
            return;
        }
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for i in 0..self.list.len() {
            if !self.list[i].bondable() {
                continue;
            }
            let (x, y, species, id) = {
                let c = &self.list[i];
                (c.x, c.y, c.species, c.id)
            };
            grid.near(x, y, BOND_RANGE as usize, |idx| {
                let j = idx as usize;
                if j >= self.list.len() || j == i {
                    return;
                }
                let other = &self.list[j];
                if other.species != species || !other.bondable() {
                    return;
                }
                let d = ((other.x - x).powi(2) + (other.y - y).powi(2)).sqrt();
                if d > BOND_RANGE {
                    return;
                }

                if self.list[i].personality.sociability < other.personality.sociability {
                    return;
                }
                pairs.push((id, other.id));
            });
        }

        for (a, b) in pairs {
            let mut bonded = false;
            for c in self.list.iter_mut() {
                if c.id == a {
                    if c.bond.is_none() {
                        c.bond = Some(b);
                        bonded = true;
                    }
                }
            }
            if bonded {
                for c in self.list.iter_mut() {
                    if c.id == b {
                        if c.bond.is_none() {
                            c.bond = Some(a);
                        }
                    }
                }
            }
        }
    }

    pub fn bond_of(&self, id: u32) -> Option<(u32, bool)> {
        let c = self.list.iter().find(|c| c.id == id)?;
        let partner = c.bond?;
        let alive = self.list.iter().any(|o| o.id == partner && o.health > 0.0);
        Some((partner, alive))
    }

    pub fn bond_count(&self) -> u32 {
        self.list.iter().filter(|c| c.bond.is_some()).count() as u32 / 2
    }
}
