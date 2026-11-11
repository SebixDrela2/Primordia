use super::*;

impl World {
    pub fn count(&self) -> Census {
        let mut c = Census::default();
        for &k in &self.kind {
            if k == Elem::Empty as u8 {
                continue;
            }
            c.particles += 1;
            match Elem::from_u8(k) {
                Elem::Plant => c.plants += 1,
                Elem::Fungus => c.plants += 1,
                Elem::Water => c.water += 1,
                Elem::Energy => c.energy += 1,
                Elem::Stone => c.stone += 1,
                Elem::Metal => c.metal += 1,
                Elem::Copper => c.copper += 1,
                Elem::Gold => c.gold += 1,
                Elem::Crystal => c.crystal += 1,
                Elem::Gem => c.gem += 1,
                Elem::Wood => c.wood += 1,
                Elem::Oil => c.oil += 1,
                Elem::Sulfur => c.sulfur += 1,
                _ => {}
            }
        }
        c
    }

    // ------------------------------------------------------------------
    // Creature interaction
    // ------------------------------------------------------------------

}
