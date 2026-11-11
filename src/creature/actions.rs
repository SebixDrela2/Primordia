use super::*;

// The individual behaviours a creature can pick during its turn.

/// How strongly the creature wants to move in each direction.
pub(crate) fn act_steer(
    c: &Creature,
    out: &[f32],
    scan: &LocalScan,
    agents: &[(u32, f32, f32, u32, u8, f32)],
) -> (f32, f32, f32) {
                // Outputs: [0]=move_x, [1]=move_y, [2]=eat, [3]=reproduce, [4]=attack,
                // [5]=build, [6]=craft, [7]=cast_spell, [8]=mount/dismount, [9]=trade,
                // [10]=rest, [11]=switch_spell, [12]=dig/mine, [13]=gather_water,
                // [14]=shoot, [15]=flee, [16]=seek, [17]=forage, [18]=rally,
                // [19]=defend
                let mut steer_x = 0.0f32;
                let mut steer_y = 0.0f32;
                let mut speed_mult_extra = 1.0f32;

                // --- Flee: run from the closest enemy ---
                let mut flee_target: Option<(f32, f32, f32)> = None;
                for &(id, ox, oy, ospecies, _orole, _ohp) in agents {
                    if id == c.id || ospecies == c.species {
                        continue;
                    }
                    let dx = ox - c.x;
                    let dy = oy - c.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < 45.0 && flee_target.map_or(true, |t| d < t.2) {
                        flee_target = Some((dx, dy, d));
                    }
                }

                // --- Seek: head for whatever the creature needs most ---
                let seek_target: Option<(f32, f32, f32)> = if c.thirst > 45.0 && scan.water.2 < 30.0 {
                    Some(scan.water)
                } else if c.hunger > 45.0 && scan.food.2 < 30.0 {
                    Some(scan.food)
                } else if c.energy < c.max_energy * 0.5 && scan.food.2 < 30.0 {
                    Some(scan.food)
                } else {
                    None
                };

                // --- Forage: head for wood or ore to build with ---
                let forage_target: Option<(f32, f32, f32)> = {
                    let wood = scan.wood;
                    let ore = scan.ore;
                    if wood.2 <= ore.2 { Some(wood) } else { Some(ore) }
                };

                if out[15] > 0.35 {
                    if let Some((dx, dy, d)) = flee_target {
                        if d > 0.1 {
                            steer_x -= dx / d;
                            steer_y -= dy / d;
                            speed_mult_extra = 1.4;
                        }
                    }
                }
                if out[16] > 0.3 {
                    if let Some((dx, dy, d)) = seek_target {
                        if d > 0.1 && d < f32::MAX {
                            steer_x += dx / d;
                            steer_y += dy / d;
                            speed_mult_extra = speed_mult_extra.max(1.15);
                        }
                    }
                }
                if out[17] > 0.3 {
                    if let Some((dx, dy, d)) = forage_target {
                        if d > 0.1 && d < f32::MAX {
                            steer_x += dx / d;
                            steer_y += dy / d;
                        }
                    }
                }
                if out[18] > 0.3 {
                    let bx = c.base_x - c.x;
                    let by = c.base_y - c.y;
                    let bd = (bx * bx + by * by).sqrt().max(0.1);
                    steer_x += bx / bd;
                    steer_y += by / bd;
                }

    (steer_x, steer_y, speed_mult_extra)
}

/// Movement with collision (mounted moves faster).
pub(crate) fn act_move(world: &mut World, c: &mut Creature, behaviour_speed: f32) {
                // Movement with collision (mounted moves faster)
                let mut speed_mult = behaviour_speed;
                if c.mount_kind != MountKind::None && c.mount_stamina > 0.0 {
                    speed_mult = 1.6 * behaviour_speed;
                    c.mount_stamina = (c.mount_stamina - 0.3).max(0.0);
                }
                let nx = c.x + c.vx * speed_mult;
                let ny = c.y + c.vy * speed_mult;

                // A creature can shove through loose debris (sand, ice, ash...)
                // and can tunnel through solid rock only with a pickaxe/drill.
                let can_dig = |e: Elem, _tool: Tool| -> bool {
                    // Walking only shoves loose debris aside. Solid rock is never
                    // excavated just by pathing - that happens through the
                    // deliberate, budgeted mining action instead, otherwise a
                    // colony of pickaxe carriers hollows out the entire world.
                    World::is_loose(e)
                };

                let tx = nx.round() as i32;
                let ty = ny.round() as i32;
                let mut ok = !world.is_blocked(tx, ty);
                if !ok && can_dig(world.get(tx, ty), c.tool) {
                    world.set_xy(tx, ty, Elem::Empty);
                    c.stamina = (c.stamina - 0.35).max(0.0);
                    ok = true;
                }
                if ok {
                    c.x = nx.clamp(1.0, world.w as f32 - 2.0);
                    c.y = ny.clamp(1.0, world.h as f32 - 2.0);
                } else {
                    let ax = (c.x + c.vx * speed_mult).round() as i32;
                    let ay = c.y.round() as i32;
                    let mut okx = !world.is_blocked(ax, ay);
                    if !okx && can_dig(world.get(ax, ay), c.tool) {
                        world.set_xy(ax, ay, Elem::Empty);
                        c.stamina = (c.stamina - 0.35).max(0.0);
                        okx = true;
                    }
                    if okx {
                        c.x = (c.x + c.vx * speed_mult).clamp(1.0, world.w as f32 - 2.0);
                    }
                    let bx = c.x.round() as i32;
                    let by = (c.y + c.vy * speed_mult).round() as i32;
                    let mut oky = !world.is_blocked(bx, by);
                    if !oky && can_dig(world.get(bx, by), c.tool) {
                        world.set_xy(bx, by, Elem::Empty);
                        c.stamina = (c.stamina - 0.35).max(0.0);
                        oky = true;
                    }
                    if oky {
                        c.y = (c.y + c.vy * speed_mult).clamp(1.0, world.h as f32 - 2.0);
                    }
                }

                // If debris settled directly on top of us, clear our own cell so
                // we are never permanently entombed. Hard rock without a tool is
                // escaped by nudging to the nearest open cell instead.
                let cx = c.x.round() as i32;
                let cy = c.y.round() as i32;
                if world.is_blocked(cx, cy) {
                    let here = world.get(cx, cy);
                    if World::is_loose(here) || matches!(c.tool, Tool::Pickaxe | Tool::Drill) {
                        world.clear_burrow(cx, cy);
                        c.stamina = (c.stamina - 0.6).max(0.0);
                        c.energy -= 0.5;
                    } else if let Some((ex, ey)) = world.find_empty_passable_near(cx, cy, 4) {
                        c.x = ex as f32;
                        c.y = ey as f32;
                        c.stamina = (c.stamina - 2.0).max(0.0);
                    }
                }
}

/// Eat and drink. Returns true when something was consumed.
pub(crate) fn act_eat(world: &mut World, c: &mut Creature, out: &[f32]) -> bool {
    let mut ate_something = false;
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    // Hungry creatures actively search a little further for food.
                    let hungry = c.hunger > 55.0 || c.energy < c.max_energy * 0.45;
                    // Hungry creatures actively search a little further for food.
                    // Scavengers have a much better nose for food, and keen
                    // senses extend the search radius.
                    let scavenger = c.traits & trait_bits::SCAVENGER != 0;
                    let keen = c.traits & trait_bits::KEEN_SENSES != 0;
                    let radius = if out[2] > 0.3 || out[16] > 0.3 {
                        6 + if scavenger { 3 } else { 0 } + if keen { 2 } else { 0 }
                    } else if hungry {
                        5 + if scavenger { 2 } else { 0 } + if keen { 1 } else { 0 }
                    } else {
                        2
                    };
                    if let Some((fx, fy)) = world.find_food(gx, gy, radius) {
                        let eaten_elem = world.get(fx, fy);
                        if let Some((nutrition, toxicity)) = world.eat(fx, fy) {
                            // Modest returns: food is a real resource, so a
                            // colony has to keep foraging instead of trivially
                            // maxing out after a couple of bites.
                            c.energy = (c.energy + nutrition * 4.0).min(c.max_energy);
                            c.hunger = (c.hunger - nutrition * 4.0).max(0.0);
                            if eaten_elem == Elem::Water {
                                c.thirst = (c.thirst - 40.0).max(0.0);
                            } else if eaten_elem.nutrition() > 0.0 {
                                c.thirst = (c.thirst - nutrition * 4.0).max(0.0);
                            }
                            // Gathering resources gives inventory
                            match eaten_elem {
                                Elem::Stone | Elem::Brick | Elem::Concrete | Elem::Wall => {
                                    c.inventory[0] = (c.inventory[0].saturating_add(5)).min(255);
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Metal | Elem::Wire => {
                                    c.inventory[1] = (c.inventory[1].saturating_add(3)).min(255);
                                    c.carried = (c.carried + 40.0).min(CARRY_CAP);
                                }
                                Elem::Copper => {
                                    c.inventory[2] = (c.inventory[2].saturating_add(3)).min(255);
                                    c.carried = (c.carried + 40.0).min(CARRY_CAP);
                                }
                                Elem::Gold => {
                                    c.inventory[3] = (c.inventory[3].saturating_add(2)).min(255);
                                    c.carried = (c.carried + 50.0).min(CARRY_CAP);
                                }
                                Elem::Crystal => {
                                    c.inventory[4] = (c.inventory[4].saturating_add(2)).min(255);
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Gem => {
                                    c.inventory[5] = (c.inventory[5].saturating_add(1)).min(255);
                                    c.carried = (c.carried + 60.0).min(CARRY_CAP);
                                }
                                Elem::Wood | Elem::Plant | Elem::Fungus => {
                                    c.inventory[6] = (c.inventory[6].saturating_add(4)).min(255);
                                    c.carried = (c.carried + 20.0).min(CARRY_CAP);
                                }
                                Elem::Energy => {
                                    c.inventory[7] = (c.inventory[7].saturating_add(10)).min(255);
                                }
                                Elem::Glass | Elem::Circuit | Elem::Tool | Elem::Machine => {
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Water => {
                                    c.inventory[8] = (c.inventory[8].saturating_add(5)).min(255);
                                }
                                _ => {}
                            }
                            if toxicity > 0.0 {
                                c.health -= toxicity * 0.25;
                            }
                            if c.energy > c.max_energy * 0.75 {
                                c.health = (c.health + 0.5).min(100.0);
                            }
                            ate_something = true;
                        }
                    }
                    // Drink: look for nearby water when thirsty, rather than
                    // requiring the creature to already stand on a water cell.
                    if c.thirst > 40.0 {
                        let drink_radius = if out[13] > 0.5 { 6 } else { 4 };
                        if let Some((wx, wy)) = world.find_water(gx, gy, drink_radius) {
                            world.set_xy(wx, wy, Elem::Empty);
                            c.thirst = (c.thirst - 60.0).max(0.0);
                            c.energy = (c.energy + 5.0).min(c.max_energy);
                            c.inventory[8] = (c.inventory[8].saturating_add(2)).min(255);
                            ate_something = true;
                        }
                    }
    ate_something
}

/// Mine ore out of the rock (never food).
pub(crate) fn act_mine(world: &mut World, c: &mut Creature, out: &[f32]) {
                // Mining / gathering (output[12]). Takes solid resources out of
                // the world for building but is explicitly *not* food.
                let wants_mine = (out[12] > 0.35 || out[17] > 0.3) && world.mine_budget > 0;
                let exploratory_mine = c.carried < 60.0
                    && c.stamina > 8.0
                    && matches!(c.role, Role::Worker | Role::Builder | Role::Explorer)
                    && (c.age.wrapping_add(c.id) % 400) < 1;
                if (wants_mine || (exploratory_mine && world.mine_budget > 0)) && c.stamina > 5.0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    if let Some((mx, my)) = world.find_resource(gx, gy, 8) {
                        world.mine_budget -= 1;
                        let me = world.get(mx, my);
                        world.set_xy(mx, my, Elem::Empty);
                        c.stamina = (c.stamina - 0.8).max(0.0);
                        c.energy -= 0.8;
                        c.carried = (c.carried + 15.0).min(CARRY_CAP);
                        // Every excavation also yields a little stone.
                        c.inventory[0] = c.inventory[0].saturating_add(1).min(255);
                        match me {
                            Elem::Stone | Elem::Sand => {
                                c.inventory[0] = c.inventory[0].saturating_add(3).min(255);
                            }
                            Elem::Metal | Elem::Iron | Elem::Steel | Elem::Wire => {
                                c.inventory[1] = c.inventory[1].saturating_add(2).min(255);
                            }
                            Elem::Copper => {
                                c.inventory[2] = c.inventory[2].saturating_add(2).min(255);
                            }
                            Elem::Gold => {
                                c.inventory[3] = c.inventory[3].saturating_add(2).min(255);
                            }
                            Elem::Crystal => {
                                c.inventory[4] = c.inventory[4].saturating_add(2).min(255);
                            }
                            Elem::Gem | Elem::Diamond => {
                                c.inventory[5] = c.inventory[5].saturating_add(1).min(255);
                            }
                            Elem::Wood => {
                                c.inventory[6] = c.inventory[6].saturating_add(3).min(255);
                            }
                            Elem::Gunpowder => {
                                c.inventory[9] = c.inventory[9].saturating_add(2).min(255);
                                c.carried = (c.carried + 10.0).min(CARRY_CAP);
                            }
                            _ => {}
                        }
                    }
                }
}

/// Build a structure. Returns (built, structures_built).
pub(crate) fn act_build(
    world: &mut World,
    c: &mut Creature,
    out: &[f32],
    structures: &mut Vec<Structure>,
    events: &mut Vec<SimEvent>,
) -> (bool, u32) {
    let mut built_something = false;
    let mut structures_total = 0u32;
                // Build / place structures (output[5])
                let nn_wants_build = out[5] > 0.2;
                let exploratory_build = c.carried > 80.0 && c.build_cooldown == 0
                    && (c.age * 17 + c.id * 23) % 100 < 1;
                let built = (nn_wants_build || exploratory_build) && c.carried >= 20.0 && c.build_cooldown == 0;
                if built {
                    c.energy -= 8.0;
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    let bx = c.base_x.round() as i32;
                    let by = c.base_y.round() as i32;
                    let dist_to_base = ((gx - bx).pow(2) + (gy - by).pow(2)) as f32;
                    let mut placed = false;
                    let mut place_x = gx;
                    let mut place_y = gy;

                    // Choose structure type based on tech/carried
                    let want_wall = c.inventory[0] >= 3 && c.tech_level < 2;
                    let want_door = c.inventory[0] >= 5 && c.role == Role::Builder;
                    let want_chest = c.inventory[1] >= 3 && c.carried > 150.0;
                    let want_farm = c.inventory[6] >= 4 && c.tech_level >= 1;
                    let want_temple = c.inventory[4] >= 2 && c.tech_level >= 3 && c.role == Role::Priest;
                    // A turret needs a stockpile: 8 wood and tech 1.
                    let want_tower = c.tech_level >= 1
                        && c.inventory[6] >= 8
                        && matches!(c.role, Role::Explorer | Role::Soldier | Role::Builder | Role::Worker);

                    let chosen = if want_tower { Buildable::Tower }
                        else if want_temple { Buildable::Temple }
                        else if want_chest { Buildable::Chest }
                        else if want_farm { Buildable::Farm }
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
                    };

                    // Try base perimeter first if near base
                    if dist_to_base < (BASE_RADIUS * BASE_RADIUS) as f32 {
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
                                    placed = true;
                                    built_something = true;
                                    structures_total += 1;
                                    break;
                                }
                            }
                            if placed { break; }
                        }
                    }

                    // Fallback: build adjacent to self
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
                                built_something = true;
                                structures_total += 1;
                                placed = true;
                                break;
                            }
                        }
                    }

                    // Last resort: build anywhere in 5x5 area
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
                        if chosen == Buildable::Tower {
                            // Turrets are armed: they will shoot at other species.
                            // A colony keeps a limited battery of them.
                            let mine = structures
                                .iter()
                                .filter(|s| s.species == c.species && s.kind == 1)
                                .count();
                            c.inventory[6] = c.inventory[6].saturating_sub(8);
                            if mine < 6 {
                                structures.push(Structure {
                                    x: place_x as f32,
                                    y: place_y as f32,
                                    species: c.species,
                                    color: c.team_color,
                                    hp: 120.0,
                                    cooldown: 40,
                                    armed: true,
                                    kind: 1,
                                });
                            }
                        } else if chosen == Buildable::Temple {
                            // Temples rally the colony: nearby members recover
                            // faster and keep their spirits up.
                            c.inventory[4] = c.inventory[4].saturating_sub(2);
                            structures.push(Structure {
                                x: place_x as f32,
                                y: place_y as f32,
                                species: c.species,
                                color: c.team_color,
                                hp: 200.0,
                                cooldown: 0,
                                armed: false,
                                kind: 2,
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

/// Craft a tool, bomb or salve. Returns true when something was made.
pub(crate) fn act_craft(c: &mut Creature, out: &[f32], events: &mut Vec<SimEvent>) -> bool {
    let mut crafted = false;
                // Tool crafting (output[6] = craft)
                let nn_wants_craft = out[6] > 0.25;
                let exploratory_craft = c.tool == Tool::None && c.craft_cooldown == 0
                    && (c.age * 13 + c.id * 17) % 25 < 1;
                let wants_craft = nn_wants_craft || exploratory_craft;
                let tool_before = c.tool;
                if wants_craft {
                    // Pickaxe: 5 stone + 2 wood (tech 0)
                    if c.tool == Tool::None && c.inventory[0] >= 5 && c.inventory[6] >= 2 {
                        c.inventory[0] = c.inventory[0].saturating_sub(5);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.tool = Tool::Pickaxe;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 30;
                    }
                    // Sword: 3 metal + 2 wood (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[1] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.tool = Tool::Sword;
                        c.tool_durability = 800;
                        c.craft_cooldown = 30;
                    }
                    // Hammer: 5 stone + 3 metal (tech 1) - better building
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[0] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[0] = c.inventory[0].saturating_sub(5);
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.tool = Tool::Hammer;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 30;
                    }
                    // Drill: 3 copper + 2 metal (tech 2) - fast mining
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[2] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[2] = c.inventory[2].saturating_sub(3);
                        c.inventory[1] = c.inventory[1].saturating_sub(2);
                        c.tool = Tool::Drill;
                        c.tool_durability = 600;
                        c.craft_cooldown = 40;
                    }
                    // Scanner: 2 gold + 1 crystal (tech 3) - long range sensing
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[3] >= 2 && c.inventory[4] >= 1 {
                        c.inventory[3] = c.inventory[3].saturating_sub(2);
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.tool = Tool::Scanner;
                        c.tool_durability = 2000;
                        c.craft_cooldown = 50;
                    }
                    // Bow: 3 wood + 2 string(plant) (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[6] = c.inventory[6].saturating_sub(5);
                        c.tool = Tool::Bow;
                        c.tool_durability = 500;
                        c.craft_cooldown = 30;
                    }
                    // Spear: 2 wood + 1 stone (tech 0)
                    else if c.tool == Tool::None && c.inventory[6] >= 2 && c.inventory[0] >= 1 {
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.inventory[0] = c.inventory[0].saturating_sub(1);
                        c.tool = Tool::Spear;
                        c.tool_durability = 400;
                        c.craft_cooldown = 20;
                    }
                    // Shield: 3 wood + 2 metal (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[6] = c.inventory[6].saturating_sub(3);
                        c.inventory[1] = c.inventory[1].saturating_sub(2);
                        c.tool = Tool::Shield;
                        c.tool_durability = 700;
                        c.craft_cooldown = 30;
                    }
                    // Staff: 1 wood + 1 crystal (tech 3) - for shaman spells
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 1 && c.inventory[4] >= 1 {
                        c.inventory[6] = c.inventory[6].saturating_sub(1);
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.tool = Tool::Staff;
                        c.tool_durability = 1200;
                        c.craft_cooldown = 40;
                    }
                    // Amulet: 1 gold + 1 gem (tech 4)
                    else if c.tool == Tool::None && c.tech_level >= 4 && c.inventory[3] >= 1 && c.inventory[5] >= 1 {
                        c.inventory[3] = c.inventory[3].saturating_sub(1);
                        c.inventory[5] = c.inventory[5].saturating_sub(1);
                        c.tool = Tool::Amulet;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 50;
                    }
                    // Backpack: 4 leather + 2 wood (tech 2) - more carry
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 2 {
                        c.inventory[6] = c.inventory[6].saturating_sub(4);
                        c.tool = Tool::Backpack;
                        c.tool_durability = 800;
                        c.craft_cooldown = 40;
                    }
                    // Cart: 5 wood + 3 metal (tech 2)
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[6] = c.inventory[6].saturating_sub(5);
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.tool = Tool::Cart;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 60;
                    }
                    // Boat: 6 wood + 1 rope (tech 2)
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 6 {
                        c.inventory[6] = c.inventory[6].saturating_sub(6);
                        c.tool = Tool::Boat;
                        c.tool_durability = 900;
                        c.craft_cooldown = 60;
                    }
                    // Glider: 4 wood + 2 cloth (tech 3)
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 4 {
                        c.inventory[6] = c.inventory[6].saturating_sub(4);
                        c.tool = Tool::Glider;
                        c.tool_durability = 700;
                        c.craft_cooldown = 70;
                    }
                    // Bomb: 3 gunpowder + 2 wood - thrown at rivals
                    else if c.bombs < 3 && c.inventory[9] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[9] = c.inventory[9].saturating_sub(3);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.bombs += 1;
                        c.craft_cooldown = 40;
                    }
                    // Salve: 2 plant + 1 crystal - heals a wounded colonymate
                    else if c.salves < 3 && c.inventory[9] == 0 && c.inventory[6] >= 2 && c.inventory[4] >= 1 {
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.salves += 1;
                        c.craft_cooldown = 40;
                    }
                }

                // Tool durability decay
                if c.tool != tool_before {
                    events.push(SimEvent::Craft {
                        x: c.x,
                        y: c.y,
                        color: c.team_color,
                    });
                }
                if c.tool != Tool::None {
                    if c.tool_durability > 0 {
                        c.tool_durability = c.tool_durability.saturating_sub(1);
                    } else {
                        c.tool = Tool::None;
                    }
                }

                // Tech progression based on resources gathered
                if c.tech_level == 0 && (c.inventory[2] >= 1 || c.inventory[1] >= 1) { c.tech_level = 1; }
                else if c.tech_level == 1 && c.inventory[1] >= 3 { c.tech_level = 2; }
                else if c.tech_level == 2 && c.inventory[3] >= 1 { c.tech_level = 3; }
                else if c.tech_level == 3 && c.inventory[4] >= 1 { c.tech_level = 4; }
    crafted
}

/// Attack and shoot. Returns whether the creature wants to melee.
pub(crate) fn act_combat(
    c: &mut Creature,
    out: &[f32],
    grid: &SpatialGrid,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    shots: &mut Vec<Projectile>,
    events: &mut Vec<SimEvent>,
) -> bool {
    let mut wants_attack = false;
                // Attack intent (output[4] = attack)
                if out[4] > 0.45 {
                    // Peaceful species refuse to start a fight.
                    if c.traits & trait_bits::PEACEFUL == 0 {
                        wants_attack = true;
                    }
                }

                // Defend (output[19]): strike at anything closing in on the
                // colony, even without a general attack impulse.
                if out[19] > 0.3 {
                    for &(id, ox, oy, ospecies, _orole, _ohp) in agents {
                        if id == c.id || ospecies == c.species {
                            continue;
                        }
                        let bx = ox - c.base_x;
                        let by = oy - c.base_y;
                        if bx * bx + by * by < 900.0 {
                            wants_attack = true;
                            break;
                        }
                    }
                }

                // Ranged attack. A creature that managed to craft a bow or a
                // staff shoots at the nearest enemy instead of closing in.
                // The shoot output asks for it; soldiers open fire anyway.
                let ranged_range = match c.tool {
                    Tool::Bow => 30.0,
                    Tool::Staff => 24.0,
                    _ => 0.0,
                };
                let wants_shoot = out[14] > 0.3 || (out[4] > 0.6 && c.role == Role::Soldier);
                if ranged_range > 0.0 && wants_shoot && c.fire_cooldown == 0 && c.energy > 40.0 {
                    let mut target: Option<(f32, f32, f32)> = None;
                    grid.near(c.x, c.y, 32, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }
                        let dx = a.1 - c.x;
                        let dy = a.2 - c.y;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < ranged_range && target.map_or(true, |t| d < t.2) {
                            target = Some((dx, dy, d));
                        }
                    });
                    if let Some((dx, dy, d)) = target {
                        let inv = 1.0 / d.max(0.001);
                        let (ux, uy) = (dx * inv, dy * inv);
                        let (speed, dmg, life, elem) = if c.tool == Tool::Bow {
                            (3.2, 26.0, 34, Elem::Wood)
                        } else {
                            (2.6, 34.0, 30, Elem::Mana)
                        };
                        shots.push(Projectile {
                            x: c.x,
                            y: c.y,
                            px: c.x,
                            py: c.y,
                            vx: ux * speed,
                            vy: uy * speed,
                            life,
                            damage: dmg,
                            species: c.species,
                            color: c.team_color,
                            elem,
                        });
                        c.fire_cooldown = if c.tool == Tool::Bow { 40 } else { 55 };
                        c.energy = (c.energy - 4.0).max(0.0);
                        events.push(SimEvent::Shoot {
                            x: c.x,
                            y: c.y,
                            color: c.team_color,
                        });
                    }
                }
    wants_attack
}
