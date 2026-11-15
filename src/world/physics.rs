use super::{World, Elem, Behavior};

impl World {
    pub fn step(&mut self) {
        self.frame += 1;

        self.mine_budget = if self.deep_mining > 0 {
            3 + self.deep_mining as i32 * 2
        } else {
            3
        };

        self.active.clear();
        std::mem::swap(&mut self.active, &mut self.next_active);

        let n_active = self.active.len();

        self.rng.shuffle(&mut self.active);
        for ai in 0..n_active {
            let i = self.active[ai] as usize;

            self.listed[i] = false;
            let e = Elem::from_u8(self.kind[i]);
            if e == Elem::Empty || self.inert[e as usize] {
                continue;
            }
            self.update_cell(i, e);

            self.mark_active(i);
        }

        self.maintain();
        if self.frame.is_multiple_of(30) {
            self.census = self.count();
        }
    }

    pub fn update_cell(&mut self, i: usize, e: Elem) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        if e.is_reactive() && self.try_reaction(i, e, x, y) {
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
            Behavior::Conduct | Behavior::Spread => {},
            Behavior::Cultivate => self.step_cultivate(i),
            Behavior::Hatch | Behavior::Decay => {},
        }
    }

}
