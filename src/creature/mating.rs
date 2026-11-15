use super::{Creature, gene_distance, CreatureSystem, SpatialGrid, REPRO_SOFT_CAP, Faction, trait_bits, ZORG_POP_CAP, Role};

pub const IDEAL_MATE_DISTANCE: f32 = 0.35;

pub const IDEAL_GENE_DISTANCE: f32 = 0.12;

#[must_use]
pub fn compatibility(a: &Creature, b: &Creature) -> f32 {
    let pd = a.personality.distance(&b.personality);
    let gd = gene_distance(&a.genes, &b.genes);
    let pd_fit = 1.0 - (pd - IDEAL_MATE_DISTANCE).abs() / 0.5;
    let gd_fit = 1.0 - (gd - IDEAL_GENE_DISTANCE).abs() / 0.35;
    (pd_fit * 0.5 + gd_fit * 0.5).clamp(0.0, 1.0)
}

impl CreatureSystem {

    pub fn resolve_mating(
        &mut self,
        willing: &[bool],
        grid: &SpatialGrid,
        pop_now: usize,
    ) -> Vec<(f32, f32, Creature)> {
        let mut children: Vec<(f32, f32, Creature)> = Vec::new();
        if pop_now >= REPRO_SOFT_CAP {
            return children;
        }

        let mut zorg_pop = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .count() as u32;

        let mut paired: Vec<u32> = Vec::new();

        for index in 0..self.list.len() {
            if !willing.get(index).copied().unwrap_or(false) {
                continue;
            }
            if paired.contains(&self.list[index].id) {
                continue;
            }

            let mut best: Option<(usize, f32)> = None;
            {
                let c = &self.list[index];
                grid.near(c.x, c.y, 16, |ji| {
                    let j = ji as usize;
                    if j == index || j >= self.list.len() {
                        return;
                    }
                    let other = &self.list[j];
                    if other.species != c.species || other.health <= 0.0 {
                        return;
                    }
                    if !other.stage.can_breed() {
                        return;
                    }
                    if paired.contains(&other.id) {
                        return;
                    }
                    let d = ((other.x - c.x).powi(2) + (other.y - c.y).powi(2)).sqrt();
                    if d > 16.0 {
                        return;
                    }

                    if !willing.get(j).copied().unwrap_or(false)
                        && other.personality.sociability < 0.6
                    {
                        return;
                    }
                    let fit = compatibility(c, other);
                    if fit <= 0.0 {
                        return;
                    }

                    let score = fit - d * 0.01;
                    if best.is_none_or(|(_, bs)| score > bs) {
                        best = Some((j, score));
                    }
                });
            }
            let Some((mate_idx, _)) = best else {
                continue;
            };

            let (a_id, b_id) = (self.list[index].id, self.list[mate_idx].id);
            paired.push(a_id);
            paired.push(b_id);

            let cost = 130.0;
            let fertile = if self.list[index].traits & trait_bits::FERTILE != 0 {
                0.6
            } else {
                1.0
            };
            if self.list[index].energy < cost * fertile
                || self.list[mate_idx].energy < cost * fertile
            {
                continue;
            }

            if self.list[index].faction == Faction::Zorg && zorg_pop + 1 > ZORG_POP_CAP {
                continue;
            }
            zorg_pop += 1;
            self.list[index].energy -= cost * fertile;
            self.list[mate_idx].energy -= cost * fertile;
            self.list[index].hunger = (self.list[index].hunger + 12.0).clamp(0.0, 100.0);
            self.list[mate_idx].hunger = (self.list[mate_idx].hunger + 12.0).clamp(0.0, 100.0);
            self.list[index].cooldown = 70;
            self.list[mate_idx].cooldown = 70;
            self.list[index].offspring += 1;
            self.list[mate_idx].offspring += 1;
            self.list[index].sired += 1;
            self.list[mate_idx].sired += 1;
            self.list[index].mate = Some(b_id);
            self.list[mate_idx].mate = Some(a_id);

            let parent = &self.list[index];
            let mate = &self.list[mate_idx];
            let mut child = parent.child_blueprint_with(mate);
            child.role = Role::Worker;
            child.x = (parent.x + mate.x) * 0.5;
            child.y = (parent.y + mate.y) * 0.5;
            self.matings_total += 1;
            self.two_parent_births += 1;
            children.push((child.x, child.y, child));
        }
        children
    }
}
