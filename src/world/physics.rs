use super::*;

impl World {
    pub fn step(&mut self) {
        self.frame += 1;
        self.mine_budget = 3;

        // Process the live cells only. The list is maintained incrementally:
        // cells that change (or move) during this step are queued for the
        // next one, so the inert bulk of the world is never touched.
        self.next_active.clear();
        let n_active = self.active.len();
        // Random order keeps the physics fair (falling sand, flowing water).
        self.rng.shuffle(&mut self.active);
        for ai in 0..n_active {
            let i = self.active[ai] as usize;
            self.listed[i] = false;
            let e = Elem::from_u8(self.kind[i]);
            if e == Elem::Empty || self.inert[e as usize] {
                continue;
            }
            self.update_cell(i, e);
            // Re-queue if it is still able to change.
            self.mark_active(i);
        }
        std::mem::swap(&mut self.active, &mut self.next_active);

        self.maintain();
        if self.frame % 30 == 0 {
            self.census = self.count();
        }
    }

    pub(crate) fn update_cell(&mut self, i: usize, e: Elem) {
        let x = (i % self.w) as i32;
        let y = (i / self.w) as i32;

        // Only elements with a reaction branch pay for the (large) reaction
        // check; sand, ash and dust are the most numerous cells in the world
        // and they simply fall.
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
            Behavior::Conduct => {}, // Wire/Circuit - static but conducts
            Behavior::Spread => {},  // Virus/Bacteria - spread handled in reactions
            Behavior::Hatch => {},   // Egg - handled in reactions
            Behavior::Decay => {},   // Corpse - handled in reactions
        }
    }

}
