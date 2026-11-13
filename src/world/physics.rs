use super::*;

impl World {
    pub fn step(&mut self) {
        self.frame += 1;
        // How much the world lets itself be dug into this tick. Deep
        // mining: a colony that knows how to go down digs a bigger hole.
        self.mine_budget = if self.deep_mining > 0 {
            (3 + self.deep_mining as i32 * 2)
        } else {
            3
        };

        // Process the live cells only. The list is maintained incrementally:
        // cells that change (or move) are queued for the next step, so the
        // inert bulk of the world is never touched.
        //
        // The queued cells are moved into the work list *here*, at the top
        // of the step, rather than at the bottom of the previous one. That
        // matters: anything that changes the world *between* steps - a
        // creature building a farm, the world regrowing a plant - queues
        // itself too, and it has to survive until this point. Clearing the
        // queue at the end of the step instead threw those away.
        self.active.clear();
        std::mem::swap(&mut self.active, &mut self.next_active);

        let n_active = self.active.len();
        // Random order keeps the physics fair (falling sand, flowing water).
        self.rng.shuffle(&mut self.active);
        for ai in 0..n_active {
            let i = self.active[ai] as usize;
            // The cell is being taken off the queue, so it may be queued
            // again if it changes.
            self.listed[i] = false;
            let e = Elem::from_u8(self.kind[i]);
            if e == Elem::Empty || self.inert[e as usize] {
                continue;
            }
            self.update_cell(i, e);
            // Re-queue if it is still able to change.
            self.mark_active(i);
        }

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
            Behavior::Cultivate => self.step_cultivate(i),
            Behavior::Hatch => {},   // Egg - handled in reactions
            Behavior::Decay => {},   // Corpse - handled in reactions
        }
    }

}
