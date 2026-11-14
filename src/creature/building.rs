use super::*;

pub(crate) fn act_build(
    world: &mut World,
    c: &mut Creature,
    out: &[f32],
    structures: &mut Vec<Structure>,
    events: &mut Vec<SimEvent>,
    company: f32,
    sanctified: bool,
) -> (bool, u32) {
    let mut built_something = false;
    let mut structures_total = 0u32;

    if c.faction == Faction::Zorg {
        return (built_something, structures_total);
    }

    if structures.len() >= MAX_STRUCTURES {
        return (built_something, structures_total);
    }

                let skill = c.culture_factor(Culture::Architecture) * c.affinity.build_factor() * company;
                let nn_wants_build = out[5] > 0.2;
                let exploratory_build = c.carried > 80.0 && c.build_cooldown == 0
                    && (c.age * 17 + c.id * 23) % 100 < 1;

                let built = (nn_wants_build || exploratory_build)
                    && c.carried >= 20.0
                    && c.build_cooldown == 0
                    && c.stage != Stage::Juvenile;
                if built {
                    c.energy = (c.energy - energy_cost::BUILD / skill).max(0.0);

                    let farming = c.knows(Culture::Cultivation);
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    let bx = c.base_x.round() as i32;
                    let by = c.base_y.round() as i32;
                    let dist_to_base = ((gx - bx).pow(2) + (gy - by).pow(2)) as f32;
                    let mut placed = false;
                    let mut place_x = gx;
                    let mut place_y = gy;

                    let discount = if c.knows_tech(Discovery::Architecture) {
                        0.7
                    } else {
                        1.0
                    };

                    let sturdy = if c.knows_tech(Discovery::Masonry) {
                        1.4
                    } else {
                        1.0
                    };

                    let want_wall = c.inventory[0] >= 3 && c.tech_level < 2;
                    let want_door = c.inventory[0] >= 5 && c.role == Role::Builder;
                    let want_chest = c.inventory[1] >= 3 && c.carried > 150.0;

                    let farms_built = structures
                        .iter()
                        .filter(|s| s.species == c.species && s.kind == 3)
                        .count();
                    let want_farm = c.inventory[6] >= 4
                        && c.tech_level >= 1
                        && farms_built < MAX_FARMS;
                    let want_temple = c.inventory[4] >= 2 && c.tech_level >= 3 && c.role == Role::Priest;

                    let towers_built = structures
                        .iter()
                        .filter(|s| s.species == c.species && s.kind == 1)
                        .count();
                    let want_tower = c.tech_level >= 1
                        && c.inventory[6] >= 8
                        && towers_built < MAX_TOWERS
                        && matches!(c.role, Role::Explorer | Role::Soldier | Role::Builder | Role::Worker);

                    let want_market = c.tech_level >= 2
                        && c.inventory[6] >= 10
                        && c.inventory[1] >= 4;
                    let want_workshop = c.tech_level >= 2
                        && c.inventory[6] >= 8
                        && c.inventory[1] >= 6;
                    let want_granary = c.tech_level >= 1
                        && c.inventory[6] >= 12
                        && c.inventory[0] >= 4;
                    let want_beacon = c.tech_level >= 3
                        && c.inventory[0] >= 10
                        && c.inventory[4] >= 3;

                    let wards_built = structures
                        .iter()
                        .filter(|s| s.species == c.species && s.kind == structure_kind::WARD)
                        .count();
                    let want_ward = sanctified
                        && c.tech_level >= 2
                        && c.inventory[4] >= (2.0 * discount) as u16
                        && c.inventory[0] >= (6.0 * discount) as u16
                        && wards_built < 3;

                    let starving = c.hunger > 45.0;

                    let settled = c.knows(Culture::Commerce) && c.knows(Culture::Architecture);
                    let chosen = if want_farm && (starving || farming) { Buildable::Farm }
                        else if want_tower { Buildable::Tower }
                        else if want_temple { Buildable::Temple }
                        else if want_ward { Buildable::Ward }
                        else if want_market && settled { Buildable::Market }
                        else if want_workshop && settled { Buildable::Workshop }
                        else if want_granary && farming { Buildable::Granary }
                        else if want_beacon { Buildable::Beacon }
                        else if want_chest { Buildable::Chest }
                        else if want_farm { Buildable::Farm }
                        else if want_market { Buildable::Market }
                        else if want_workshop { Buildable::Workshop }
                        else if want_granary { Buildable::Granary }
                        else if want_door { Buildable::Door }
                        else if want_wall { Buildable::Wall }
                        else { Buildable::Bridge };

                    let struct_elem = match chosen {
                        Buildable::Wall => Elem::Wall,
                        Buildable::Door => Elem::Door,
                        Buildable::Chest => Elem::Chest,
                        Buildable::Furnace => Elem::Furnace,
                        Buildable::Farm => Elem::Farm,
                        Buildable::Temple => Elem::Temple,
                        Buildable::Tower => Elem::Tower,
                        Buildable::Bridge => Elem::Bridge,
                        Buildable::Market => Elem::Market,
                        Buildable::Workshop => Elem::Workshop,
                        Buildable::Granary => Elem::Granary,
                        Buildable::Beacon => Elem::Beacon,
                        Buildable::Ward => Elem::Ward,
                    };

                    if chosen == Buildable::Farm && !placed {
                        if let Some((tx, ty, px, py)) = world.find_farm_site(gx, gy, 14) {
                            world.set_xy(tx, ty, Elem::Farm);

                            world.set_xy(px, py, Elem::Plant);
                            place_x = tx;
                            place_y = ty;
                            c.carried = (c.carried - 15.0).max(0.0);
                            c.build_cooldown = if c.traits & trait_bits::INDUSTRIOUS != 0 { 6 } else { 10 };
                            c.build_cooldown = ((c.build_cooldown as f32) / skill).round() as u32;
                            placed = true;
                            built_something = true;
                            structures_total += 1;
                        }
                    }

                    if !placed && dist_to_base < (BASE_RADIUS * BASE_RADIUS) as f32 {
                        for r in 3..=BASE_RADIUS {
                            for angle in 0..24 {
                                let tx = bx + (r as f32 * (angle as f32 * 0.26).cos()) as i32;
                                let ty = by + (r as f32 * (angle as f32 * 0.26).sin()) as i32;
                                if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                    world.set_xy(tx, ty, struct_elem);
                                    place_x = tx;
                                    place_y = ty;
                                    c.carried = (c.carried - 15.0).max(0.0);
                                    c.build_cooldown = if c.traits & trait_bits::INDUSTRIOUS != 0 { 6 } else { 10 };
                            c.build_cooldown = ((c.build_cooldown as f32) / skill).round() as u32;
                                    placed = true;
                                    built_something = true;
                                    structures_total += 1;
                                    break;
                                }
                            }
                            if placed { break; }
                        }
                    }

                    if !placed {
                        for (dx, dy) in DIRS8 {
                            let tx = gx + dx;
                            let ty = gy + dy;
                            if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                world.set_xy(tx, ty, struct_elem);
                                place_x = tx;
                                place_y = ty;
                                c.carried = (c.carried - 15.0).max(0.0);
                                c.build_cooldown = if c.traits & trait_bits::INDUSTRIOUS != 0 { 6 } else { 10 };
                            c.build_cooldown = ((c.build_cooldown as f32) / skill).round() as u32;
                                built_something = true;
                                structures_total += 1;
                                placed = true;
                                break;
                            }
                        }
                    }

                    if !placed {
                        for dy in -2..=2 {
                            for dx in -2..=2 {
                                let tx = gx + dx;
                                let ty = gy + dy;
                                if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                    world.set_xy(tx, ty, struct_elem);
                                    place_x = tx;
                                    place_y = ty;
                                    c.carried = (c.carried - 15.0).max(0.0);
                                    c.build_cooldown = 10;
                                    built_something = true;
                                    structures_total += 1;
                                    placed = true;
                                    break;
                                }
                            }
                            if placed { break; }
                        }
                    }

                    if placed {

                        if world.landmark_bonus(gx as f32, gy as f32).stone && c.build_cooldown > 3 {
                            c.build_cooldown = 3;
                        }
                        if chosen == Buildable::Farm {

                            c.inventory[6] = c.inventory[6].saturating_sub((4 as f32 * discount) as u16);

                            structures.push(Structure {
                                x: place_x as f32,
                                y: place_y as f32,
                                species: c.species,
                                color: c.team_color,
                                hp: 100.0 * sturdy,
                                cooldown: 0,
                                armed: false,
                                kind: 3,
                            });
                        }
                        if chosen == Buildable::Tower {

                            let mine = structures
                                .iter()
                                .filter(|s| s.species == c.species && s.kind == 1)
                                .count();
                            c.inventory[6] = c.inventory[6].saturating_sub((8 as f32 * discount) as u16);
                            if mine < 6 {
                                structures.push(Structure {
                                    x: place_x as f32,
                                    y: place_y as f32,
                                    species: c.species,
                                    color: c.team_color,
                                    hp: 120.0 * sturdy,
                                    cooldown: 40,
                                    armed: true,
                                    kind: 1,
                                });
                            }
                        } else if chosen == Buildable::Temple {

                            c.inventory[4] = c.inventory[4].saturating_sub((2 as f32 * discount) as u16);
                            structures.push(Structure {
                                x: place_x as f32,
                                y: place_y as f32,
                                species: c.species,
                                color: c.team_color,
                                hp: 200.0 * sturdy,
                                cooldown: 0,
                                armed: false,
                                kind: structure_kind::TEMPLE,
                            });
                        } else {

                            let (kind, wood, metal, stone, crystal, hp) = match chosen {
                                Buildable::Market => {
                                    (structure_kind::MARKET, 10, 4, 0, 0, 140.0)
                                }
                                Buildable::Workshop => {
                                    (structure_kind::WORKSHOP, 8, 6, 0, 0, 160.0)
                                }
                                Buildable::Granary => {
                                    (structure_kind::GRANARY, 12, 0, 4, 0, 180.0)
                                }
                                Buildable::Beacon => {
                                    (structure_kind::BEACON, 0, 0, 10, 3, 200.0)
                                }

                                Buildable::Ward => {
                                    (structure_kind::WARD, 0, 0, 6, 2, 260.0)
                                }
                                _ => (structure_kind::PLAIN, 0, 0, 0, 0, 100.0),
                            };
                            c.inventory[6] = c.inventory[6].saturating_sub((wood as f32 * discount) as u16);
                            c.inventory[1] = c.inventory[1].saturating_sub((metal as f32 * discount) as u16);
                            c.inventory[0] = c.inventory[0].saturating_sub((stone as f32 * discount) as u16);
                            c.inventory[4] = c.inventory[4].saturating_sub((crystal as f32 * discount) as u16);
                            structures.push(Structure {
                                x: place_x as f32,
                                y: place_y as f32,
                                species: c.species,
                                color: c.team_color,
                                hp,
                                cooldown: 0,
                                armed: false,
                                kind,
                            });
                        }
                        events.push(SimEvent::Build {
                            x: place_x as f32,
                            y: place_y as f32,
                            color: c.team_color,
                        });
                    }
                }
    (built_something, structures_total)
}
