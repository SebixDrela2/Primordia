use super::*;

pub const CRISIS_TICKS: u64 = 600;

pub fn in_crisis(succession: &[(u32, u64)], species: u32, tick: u64) -> bool {
    succession
        .iter()
        .any(|(sp, start)| *sp == species && tick.saturating_sub(*start) < CRISIS_TICKS)
}

impl CreatureSystem {

    pub fn species_in_crisis(&self) -> Vec<u32> {
        self.succession
            .iter()
            .filter(|(_sp, start)| self.tick.saturating_sub(*start) < CRISIS_TICKS)
            .map(|(sp, _)| *sp)
            .collect()
    }

    pub fn update_succession(&mut self) {
        let mut with_queen: Vec<u32> = Vec::new();
        let mut without_queen: Vec<u32> = Vec::new();
        for c in self.list.iter() {
            if c.role == Role::Queen {
                if !with_queen.contains(&c.species) {
                    with_queen.push(c.species);
                }
            } else if !without_queen.contains(&c.species) {
                without_queen.push(c.species);
            }
        }

        self.succession.retain(|(sp, _)| !with_queen.contains(sp));

        for sp in without_queen {

            if with_queen.contains(&sp) {
                continue;
            }
            if !self.succession.iter().any(|(s, _)| *s == sp) {
                self.succession.push((sp, self.tick));
            }
        }
    }

    pub fn begin_succession(&mut self, species: u32) {
        if self.succession.iter().any(|(sp, _)| *sp == species) {
            return;
        }
        self.succession.push((species, self.tick));
    }

    pub fn end_succession(&mut self, species: u32) {
        self.succession.retain(|(sp, _)| *sp != species);
    }

    pub fn crisis_penalty(&self, species: u32) -> f32 {
        if in_crisis(&self.succession, species, self.tick) {
            1.0
        } else {
            0.0
        }
    }
}
