use super::*;

// Combat resolution: turrets, bombs, projectiles, melee and deaths.
// These run after every creature has decided what it wants to do.

impl CreatureSystem {
    pub(crate) fn resolve_turrets(&mut self, world: &mut World, shots: &mut Vec<Projectile>) {
    // Turrets of extinct species decay away, armed turrets shoot.
        // ---- Turrets of extinct species decay away ----
        if self.tick % 60 == 0 {
            let alive: Vec<u32> = self.list.iter().map(|c| c.species).collect();
            let mut removed: Vec<Structure> = Vec::new();
            self.structures.retain(|s| {
                if alive.contains(&s.species) {
                    true
                } else {
                    removed.push(Structure {
                        x: s.x,
                        y: s.y,
                        species: s.species,
                        color: s.color,
                        hp: s.hp,
                        cooldown: s.cooldown,
                        armed: false,
                        kind: s.kind,
                    });
                    false
                }
            });
            for s in removed {
                let tx = s.x.round() as i32;
                let ty = s.y.round() as i32;
                if world.get(tx, ty) == Elem::Tower {
                    world.set_xy(tx, ty, Elem::Empty);
                }
            }
        }

        // ---- Turrets fire at other species ----
        for s in self.structures.iter_mut() {
            if !s.armed { continue; }
            if s.cooldown > 0 {
                s.cooldown -= 1;
                continue;
            }
            let mut target: Option<(f32, f32, f32)> = None;
            for other in self.list.iter() {
                if other.species == s.species { continue; }
                let dx = other.x - s.x;
                let dy = other.y - s.y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < 48.0 && target.map_or(true, |t| d < t.2) {
                    target = Some((dx, dy, d));
                }
            }
            if let Some((dx, dy, d)) = target {
                let inv = 1.0 / d.max(0.001);
                shots.push(Projectile {
                    x: s.x,
                    y: s.y,
                    px: s.x,
                    py: s.y,
                    vx: dx * inv * 3.6,
                    vy: dy * inv * 3.6,
                    life: 45,
                    damage: 30.0,
                    species: s.species,
                    color: s.color,
                    elem: Elem::Energy,
                });
                self.stats.shots += 1;

                s.cooldown = 55;
                self.events.push(SimEvent::Shoot {
                    x: s.x,
                    y: s.y,
                    color: s.color,
                });
            }
        }
    }
}

impl CreatureSystem {
    pub(crate) fn resolve_bombs(&mut self, world: &mut World) {
    // Bombs arc through the air and detonate on impact.
        // ---- Bombs in flight ----
        if !self.bombs.is_empty() {
            let current = std::mem::take(&mut self.bombs);
            for mut b in current {
                b.x += b.vx;
                b.y += b.vy;
                b.vy += 0.08; // gravity
                b.life = b.life.saturating_sub(1);
                let tx = b.x.round() as i32;
                let ty = b.y.round() as i32;
                let hit_wall = b.life == 0 || world.is_blocked(tx, ty);
                if hit_wall {
                    // Detonation: everything close by takes damage.
                    let bx = b.x;
                    let by = b.y;
                    for j in 0..self.list.len() {
                        let (ox, oy, ospecies) = {
                            let a = &self.list[j];
                            (a.x, a.y, a.species)
                        };
                        if ospecies == b.owner {
                            continue;
                        }
                        let dx = ox - bx;
                        let dy = oy - by;
                        if dx * dx + dy * dy < 36.0 {
                            self.list[j].health -= 55.0;
                        }
                    }
                    // Scorch the ground and release energy.
                    self.spawn_energy_burst(world, tx, ty, 16, b.color);
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            if world.get(tx + dx, ty + dy) == Elem::Empty {
                                world.set_xy(tx + dx, ty + dy, Elem::Fire);
                            }
                        }
                    }
                    self.events.push(SimEvent::Explosion {
                        x: bx,
                        y: by,
                        color: b.color,
                    });
                } else {
                    self.bombs.push(b);
                }
            }
        }
    }
}

impl CreatureSystem {
    pub(crate) fn resolve_projectiles(&mut self, world: &mut World, shots: Vec<Projectile>) {
    // Arrows, mana bolts and turret fire travel and hit things.
        // ---- Projectiles in flight ----
        let current: Vec<Projectile> = std::mem::take(&mut self.projectiles);
        let mut survivors: Vec<Projectile> = Vec::new();
        for mut p in current {
            p.px = p.x;
            p.py = p.y;
            p.x += p.vx;
            p.y += p.vy;
            p.life = p.life.saturating_sub(1);

            let tx = p.x.round() as i32;
            let ty = p.y.round() as i32;
            if world.is_blocked(tx, ty) {
                // Impact: a puff of smoke where the shot hit rock.
                self.spawn_impact_puff(world, tx, ty, 2);
                continue;
            }

            let mut hit: Option<usize> = None;
            for (j, other) in self.list.iter().enumerate() {
                if other.species == p.species { continue; }
                if other.health <= 0.0 { continue; }
                if segment_distance(other.x, other.y, p.px, p.py, p.x, p.y) < other.size.max(1.6) {
                    hit = Some(j);
                    break;
                }
            }
            if let Some(j) = hit {
                self.list[j].health -= p.damage;
                self.events.push(SimEvent::Hit {
                    x: p.x,
                    y: p.y,
                    color: p.color,
                    damage: p.damage,
                });
                self.spawn_energy_burst(world, tx, ty, 4, p.color);
                continue;
            }

            if p.life == 0 {
                self.spawn_impact_puff(world, tx, ty, 1);
                continue;
            }
            survivors.push(p);
        }
        self.projectiles = survivors;
        self.projectiles.extend(shots);
    }
}

impl CreatureSystem {
    pub(crate) fn resolve_attacks(&mut self, grid: &SpatialGrid, attackers: &[usize]) {
    // Melee: soldiers target enemy queens, squads focus fire.
        // Resolve attacks - soldiers target enemy queens/bases, workers defend.
        // Squads coordinate: whoever spots a target first calls it out, and
        // the rest of the squad concentrates fire on the same victim.
        let mut squad_targets: Vec<(u16, usize)> = Vec::new();
        for &a in attackers {
            let (ax, ay, a_species, a_role, a_squad) = {
                let c = &self.list[a];
                (c.x, c.y, c.species, c.role, c.squad_id)
            };
            let mut best: Option<(usize, f32)> = None;
            grid.near(ax, ay, 16, |ji| {
                let j = ji as usize;
                if j == a {
                    return;
                }
                let other = &self.list[j];
                if other.species == a_species {
                    return;
                }
                let priority = match (a_role, other.role) {
                    (Role::Soldier, Role::Queen) => 0.0,
                    (Role::Soldier, Role::Soldier) => 10.0,
                    (Role::Soldier, Role::Worker) => 20.0,
                    (Role::Worker, _) => 50.0,
                    _ => 30.0,
                };
                let d = ((other.x - ax).powi(2) + (other.y - ay).powi(2)).sqrt() + priority;
                let max_range = if a_role == Role::Soldier { 14.0 } else { 6.0 };
                if d < max_range && best.map_or(true, |b| d < b.1) {
                    best = Some((j, d));
                }
            });
            // No target of its own? Follow the squad's call.
            if best.is_none() && a_squad > 0 {
                if let Some((_, tj)) = squad_targets.iter().find(|(sq, _)| *sq == a_squad) {
                    let d = {
                        let t = &self.list[*tj];
                        ((t.x - ax).powi(2) + (t.y - ay).powi(2)).sqrt()
                    };
                    if d < 26.0 && self.list[*tj].species != a_species {
                        best = Some((*tj, d));
                    }
                }
            }
            if let Some((j, _)) = best {
                if a_squad > 0 && !squad_targets.iter().any(|(sq, _)| *sq == a_squad) {
                    squad_targets.push((a_squad, j));
                }
                if self.list[j].health > 0.0 {
                    let attacker = &self.list[a];
                    let mut dmg = if a_role == Role::Soldier { 45.0 } else { 25.0 };
                    dmg += match attacker.tool {
                        Tool::Sword => 35.0,
                        Tool::Pickaxe => 18.0,
                        Tool::Hammer => 22.0,
                        Tool::Drill => 28.0,
                        Tool::Spear => 20.0,
                        Tool::Staff => 15.0,
                        _ => 0.0,
                    };
                    // Ferocious species hit harder.
                    if attacker.traits & trait_bits::FEROCIOUS != 0 {
                        dmg *= 1.35;
                    }
                    if attacker.active_spell == Spell::Fireball && attacker.spell_cd[1] == 0 { dmg += 20.0; }
                    self.list[j].health -= dmg;
                    self.list[a].energy = (self.list[a].energy + 20.0).min(self.list[a].max_energy);
                    self.events.push(SimEvent::Hit {
                        x: self.list[j].x,
                        y: self.list[j].y,
                        color: self.list[a].team_color,
                        damage: dmg,
                    });
                    if self.list[j].health <= 0.0 {
                        self.list[a].kills += 1;
                        self.list[a].energy = (self.list[a].energy + 140.0).min(self.list[a].max_energy);
                    }
                }
            }
        }
    }
}

impl CreatureSystem {
    pub(crate) fn resolve_deaths(&mut self, world: &mut World, dead: &[usize]) {
    // The dead are removed and burst into energy.
        // Remove the dead
        for &i in dead.iter().rev() {
            let (cx, cy, color, size, dx, dy) = {
                let c = &self.list[i];
                (
                    c.x.round() as i32,
                    c.y.round() as i32,
                    c.team_color,
                    c.size,
                    c.x,
                    c.y,
                )
            };
            world.scavenge(cx, cy);
            // A dying organism bursts into a spray of energy particles.
            let burst = 8 + self.rng.range(9);
            self.spawn_energy_burst(world, cx, cy, burst, color);
            self.events.push(SimEvent::Death {
                x: dx,
                y: dy,
                color,
                size,
            });
            self.list.remove(i);
            self.deaths_total += 1;
        }
    }
}

impl CreatureSystem {
    pub(crate) fn resolve_births(&mut self, world: &mut World, births: Vec<(f32, f32, Creature)>) {
    // Newborns are placed next to the colony.
        // Births
        for (x, y, mut child) in births {
            if self.list.len() >= MAX_POP { break; }
            // Summon spell creates a minion at caster pos
            let cx = x + self.rng.range_i(-4, 5) as f32;
            let cy = y + self.rng.range_i(-4, 5) as f32;
            if world.is_blocked(cx.round() as i32, cy.round() as i32) { continue; }
            if self.spawn(world, cx, cy, CreatureGenesis::Child(&child)) {
                self.events.push(SimEvent::Birth {
                    x: cx,
                    y: cy,
                    color: child.team_color,
                });
            }
        }

        // No automatic re-seeding: species must be able to go extinct (and one
        // last lineage to win). A fully dead world is handled by the
        // extinction/revival screen in the main loop instead.
    }
}

impl CreatureSystem {
    pub(crate) fn apply_status(&mut self, spell_targets: Vec<(usize, usize, f32)>, disease_targets: Vec<usize>, heal_targets: Vec<(u32, f32, f32)>) {
    // Spell damage, curses and salve healing.
        // Apply spell damage and disease to targets
        for (caster_idx, target_idx, damage) in spell_targets {
            if let Some(target) = self.list.get_mut(target_idx) {
                target.health -= damage;
                if target.health <= 0.0 {
                    if let Some(caster) = self.list.get_mut(caster_idx) {
                        caster.kills += 1;
                        caster.energy = (caster.energy + 140.0).min(caster.max_energy);
                    }
                }
            }
        }
        for &target_idx in &disease_targets {
            if let Some(target) = self.list.get_mut(target_idx) {
                target.disease = Disease::Curse;
                target.disease_timer = 300;
            }
        }

        // Apply healing from salves.
        for (target_id, _hx, _hy) in heal_targets {
            if let Some(target) = self.list.iter_mut().find(|t| t.id == target_id) {
                target.health = (target.health + 35.0).min(100.0);
                target.energy = (target.energy + 40.0).min(target.max_energy);
            }
        }
    }
}
