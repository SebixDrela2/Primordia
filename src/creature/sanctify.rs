use super::{discovery_bits, Elem, CreatureSystem, World, SpatialGrid, structure_kind, Faction, Role, SimEvent};

pub const SUNFIRE_VS_ZORG: f32 = 2.6;

pub const SUNFIRE_VS_OTHERS: f32 = 0.45;

pub const WARD_RADIUS: f32 = 30.0;

pub const WARD_BURN: f32 = 0.9;

pub const WARD_DEPTH: i32 = 35;

#[must_use]
pub fn sunfire_multiplier(target_is_zorg: bool) -> f32 {
    if target_is_zorg {
        SUNFIRE_VS_ZORG
    } else {
        SUNFIRE_VS_OTHERS
    }
}

#[must_use]
pub fn sunfire_damage(tech: u32, target_is_zorg: bool) -> f32 {
    let mut mult = sunfire_multiplier(target_is_zorg);

    if tech & discovery_bits::SUNFIRECRAFT != 0 {
        mult *= 1.3;
    }
    mult
}

#[must_use]
pub fn is_corruption(e: Elem) -> bool {
    matches!(
        e,
        Elem::Virus | Elem::Bacteria | Elem::Poison | Elem::Tar | Elem::Chlorine
    )
}

impl CreatureSystem {

    pub fn run_wards(&mut self, world: &mut World, grid: &SpatialGrid) {
        if !self.tick.is_multiple_of(5) {
            return;
        }

        let mut radius = WARD_RADIUS;
        for (_, d) in &self.discoveries {
            if *d & discovery_bits::WARDCRAFT != 0 {
                radius *= 1.5;
            }
        }
        let wards: Vec<(f32, f32, u32)> = self
            .structures
            .iter()
            .filter(|s| s.kind == structure_kind::WARD)
            .map(|s| (s.x, s.y, s.species))
            .collect();
        if wards.is_empty() {
            return;
        }

        for (wx, wy, species) in &wards {

            let mut near: Vec<usize> = Vec::new();
            grid.near(*wx, *wy, radius as usize, |idx| near.push(idx as usize));
            for j in near {
                if j >= self.list.len() {
                    continue;
                }
                let c = &self.list[j];
                if c.faction != Faction::Zorg || c.health <= 0.0 {
                    continue;
                }
                let d = ((c.x - wx).powi(2) + (c.y - wy).powi(2)).sqrt();
                if d > radius {
                    continue;
                }

                let burn = WARD_BURN * (1.0 - d / radius);
                self.list[j].health -= burn;
            }

            for c in &mut self.list {
                if c.species == *species && c.role == Role::Queen {
                    c.health = (c.health + 0.01).min(c.max_health());
                }
            }
        }

        if self.tick.is_multiple_of(20) {
            for (wx, wy, _) in &wards {
                let r = radius as i32;
                let cx = *wx as i32;
                let cy = *wy as i32;
                for y in (cy - r)..(cy + r) {
                    for x in (cx - r)..(cx + r) {
                        let dx = (x - cx) as f32;
                        let dy = (y - cy) as f32;
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }
                        if is_corruption(world.get(x, y)) {
                            world.set_xy(x, y, Elem::Empty);
                        }
                    }
                }
            }
        }

        if !world.zorg_cave_sealed {
            for (wx, wy, _) in &wards {
                let surf = world.surface_y(*wx as i32);
                if (*wy as i32) - surf < WARD_DEPTH {
                    continue;
                }
                world.zorg_cave_sealed = true;
                self.diplomacy.log_event(
                    "THE DEEP IS SEALED: the cave of the ZORGs is closed".to_string(),
                    [255, 230, 150],
                    self.tick,
                );
                self.events.push(SimEvent::Discovery {
                    x: *wx,
                    y: *wy,
                    color: [255, 230, 150],
                });
                break;
            }
        }
    }
}
