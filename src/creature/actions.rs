use super::{Creature, SpatialGrid, DiplomaticSense, FearTable, outputs, DIRS8, Faction, zorg_steer, World, MountKind, MOUNT_SPEED_BONUS, energy_cost, Elem, Tool, trait_bits, zorg_feed_on_rock, zorg_act_eat, Discovery, Culture, Role, Stage, SimEvent, discovery_bits, tool_gate, Projectile, Diplomacy, Treaty};

pub fn act_steer(
    c: &Creature,
    out: &[f32],
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
    foreign_bases: &[(u32, f32, f32)],
    ds: &DiplomaticSense,
    fear: &FearTable,
    weather: crate::world::Weather,
    place: &crate::world::LandmarkBonus,
    volcano: Option<(f32, f32)>,
) -> (f32, f32, f32) {

                let mut steer_x = 0.0f32;
                let mut steer_y = 0.0f32;
                let mut speed_mult_extra = 1.0f32;

                for d in 0..outputs::DESIRE_COUNT {
                    let v = out[outputs::DESIRE + d];
                    if v <= 0.0 {
                        continue;
                    }
                    let (dx, dy) = DIRS8[d];
                    steer_x += dx as f32 * v;
                    steer_y += dy as f32 * v;
                    if v > 0.5 {
                        speed_mult_extra = speed_mult_extra.max(1.15);
                    }
                }

                if c.faction == Faction::Zorg {
                    let (zx, zy, zs) = zorg_steer(c);
                    steer_x += zx;
                    steer_y += zy;
                    speed_mult_extra = speed_mult_extra.max(zs);
                }

                let _ = volcano;

                let bold = c.personality.aggression;
                let wary = c.personality.caution;

                let flee_range = 45.0 - bold * 22.0 + wary * 10.0 + weather.fear_factor();
                let mut flee_target: Option<(f32, f32, f32)> = None;

                let hunt_range = if place.hunting { 48.0 } else { 34.0 };
                let mut prey: Option<(f32, f32, f32)> = None;

                let search = (flee_range + 22.0).max(hunt_range) as usize;
                grid.near(c.x, c.y, search, |idx| {
                    let a = &agents[idx as usize];
                    let (id, ox, oy, ospecies) = (a.0, a.1, a.2, a.3);
                    if id == c.id || ospecies == c.species {
                        return;
                    }
                    let dx = ox - c.x;
                    let dy = oy - c.y;
                    let d = (dx * dx + dy * dy).sqrt();

                    let fright = fear.fear_of(c.species, ospecies) * 0.22;
                    let run = flee_range + fright;
                    if d < run && flee_target.is_none_or(|t| d < t.2) {
                        flee_target = Some((dx, dy, d));
                    }
                    if d < hunt_range && prey.is_none_or(|t| d < t.2) {
                        prey = Some((dx, dy, d));
                    }
                });

                let _ = (prey, place.hunting, fear.total_of(c.species));

                if out[15] > 0.35 {
                    if let Some((dx, dy, d)) = flee_target {
                        if d > 0.1 {
                            steer_x -= dx / d;
                            steer_y -= dy / d;
                            speed_mult_extra = speed_mult_extra.max(1.4);
                        }
                    }
                }

                let _ = out[18];

                if out[outputs::SEEK_MATE] > 0.35 {
                    let mut best: Option<(f32, f32, f32)> = None;
                    grid.near(c.x, c.y, 40, |idx| {
                        let a = &agents[idx as usize];
                        let (id, ox, oy, ospecies) = (a.0, a.1, a.2, a.3);
                        if id == c.id || ospecies != c.species {
                            return;
                        }
                        let dx = ox - c.x;
                        let dy = oy - c.y;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < 40.0 && best.is_none_or(|t| d < t.2) {
                            best = Some((dx, dy, d));
                        }
                    });
                    if let Some((dx, dy, d)) = best {
                        if d > 2.0 {
                            steer_x += dx / d;
                            steer_y += dy / d;
                            speed_mult_extra = speed_mult_extra.max(1.1);
                        }
                    }
                }

                if c.migrating {
                    let bx = c.x - c.base_x;
                    let by = c.y - c.base_y;
                    let bd = (bx * bx + by * by).sqrt().max(0.1);
                    steer_x += (bx / bd) * 1.3;
                    steer_y += (by / bd) * 1.3;
                    speed_mult_extra = speed_mult_extra.max(1.2);
                }

                let _ = (out[outputs::NEGOTIATE], ds.partner, foreign_bases);

    (steer_x, steer_y, speed_mult_extra)
}

pub fn act_move(world: &mut World, c: &mut Creature, behaviour_speed: f32) {

                let mut speed_mult = behaviour_speed;
                if c.mount_kind != MountKind::None && c.mount_stamina > 0.0 {
                    speed_mult = MOUNT_SPEED_BONUS * behaviour_speed;
                    c.mount_stamina = (c.mount_stamina - 0.3).max(0.0);
                }

                let ground = world.surface_y(c.x.round() as i32);
                let height = (ground - c.y.round() as i32).max(0) as f32;
                let distance = (c.vx.abs() + c.vy.abs()) * speed_mult;
                let step_cost = energy_cost::MOVE * (1.0 + height / 20.0);
                c.energy = (c.energy - distance * step_cost).max(0.0);

                if height > 1.0 {
                    let below = world.get(c.x.round() as i32, c.y.round() as i32 + 1);
                    if below.is_solid() {
                        c.air_time = 0;
                    } else {
                        c.air_time = c.air_time.saturating_add(1);
                        let hover = 0.02
                            * (1.0 + height / 20.0)
                            * (1.0 + c.air_time as f32 / 300.0);
                        c.energy = (c.energy - hover).max(0.0);
                    }
                } else {
                    c.air_time = 0;
                }
                let nx = c.x + c.vx * speed_mult;
                let ny = c.y + c.vy * speed_mult;

                let can_dig = |e: Elem, _tool: Tool| -> bool {

                    World::is_loose(e)
                };

                let zorg_chew = c.faction == Faction::Zorg;

                let tx = nx.round() as i32;
                let ty = ny.round() as i32;
                let mut ok = !world.is_blocked(tx, ty);

                if ok && c.traits & trait_bits::AMPHIBIOUS == 0 {
                    let here = world.get(c.x.round() as i32, c.y.round() as i32);
                    if here == Elem::Water || world.get(tx, ty) == Elem::Water {
                        c.stamina = (c.stamina - 0.22).max(0.0);
                    }
                }
                if !ok && can_dig(world.get(tx, ty), c.tool) {
                    world.set_xy(tx, ty, Elem::Empty);
                    c.stamina = (c.stamina - 0.35).max(0.0);
                    c.energy = (c.energy - energy_cost::SHOVE).max(0.0);
                    ok = true;
                }

                if !ok && zorg_chew {
                    let here = world.get(tx, ty);
                    if here.is_solid() {
                        world.set_xy(tx, ty, Elem::Empty);
                        c.stamina = (c.stamina - 1.2).max(0.0);
                        c.energy = (c.energy - energy_cost::CHEW).max(0.0);

                        zorg_feed_on_rock(c, here, c.vy < 0.0);
                        ok = true;
                    }
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
                        c.energy = (c.energy - energy_cost::SHOVE).max(0.0);
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
                        c.energy = (c.energy - energy_cost::SHOVE).max(0.0);
                        oky = true;
                    }
                    if oky {
                        c.y = (c.y + c.vy * speed_mult).clamp(1.0, world.h as f32 - 2.0);
                    }
                }

                let cx = c.x.round() as i32;
                let cy = c.y.round() as i32;
                if world.is_blocked(cx, cy) {
                    let here = world.get(cx, cy);
                    if World::is_loose(here) || matches!(c.tool, Tool::Pickaxe | Tool::Drill) {
                        let _ = world.clear_burrow(cx, cy);
                        c.stamina = (c.stamina - 0.6).max(0.0);
                        c.energy -= 0.5;
                    } else if let Some((ex, ey)) = world.find_empty_passable_near(cx, cy, 4) {
                        c.x = ex as f32;
                        c.y = ey as f32;
                        c.stamina = (c.stamina - 2.0).max(0.0);
                    }
                }
}

pub fn act_eat(
    world: &mut World,
    c: &mut Creature,
    out: &[f32],
    dip: &super::diplomacy::Diplomacy,
) -> bool {
    let mut ate_something = false;
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;

                    if out[2] > 0.05 || out[16] > 0.05 {
                        if c.faction == Faction::Zorg {

                            ate_something = zorg_act_eat(world, c, c.hunger > 55.0);
                        } else {
                            ate_something = eat_adjacent(world, c, gx, gy, dip);
                        }
                    }

                    if c.thirst > 40.0 && out[13] > 0.05 {
                        for (dx, dy) in &DIRS8 {
                            let wx = gx + dx;
                            let wy = gy + dy;
                            if world.get(wx, wy) == Elem::Water {
                                world.set_xy(wx, wy, Elem::Empty);
                                c.thirst = (c.thirst - 60.0).max(0.0);
                                c.energy = (c.energy + 5.0).min(c.max_energy);
                                c.inventory[8] = (c.inventory[8].saturating_add(2)).min(255);

                                c.last_meal = Elem::Water;
                                ate_something = true;
                                break;
                            }
                        }
                    }
    ate_something
}

fn eat_adjacent(world: &mut World, c: &mut Creature, gx: i32, gy: i32, dip: &super::diplomacy::Diplomacy) -> bool {

    let mut best: Option<(i32, i32, Elem)> = None;
    for (dx, dy) in DIRS8.iter().copied().chain([(0, 0)]) {
        let ex = gx + dx;
        let ey = gy + dy;
        let e = world.get(ex, ey);
        if e == Elem::Empty || e.nutrition() <= 0.0 {
            continue;
        }

        if !e.is_edible() {
            continue;
        }

        if e == Elem::Water {
            continue;
        }

        if e == Elem::Corpse {
            if let Some(owner) = world.owner_at(ex, ey) {
                if owner == c.species {
                    crate::world::counters::CORPSE_REFUSED
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    continue;
                }
                if dip.are_allied(c.species, owner) {
                    crate::world::counters::CORPSE_REFUSED
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    continue;
                }
            }
        }
        let better = match best {
            None => true,
            Some((_, _, be)) => e.nutrition() > be.nutrition(),
        };
        if better {
            best = Some((ex, ey, e));
        }
    }
    let Some((ex, ey, eaten_elem)) = best else {
        return false;
    };
    let Some((nutrition, toxicity)) = world.eat(ex, ey) else {
        return false;
    };

    c.last_meal = eaten_elem;
    crate::world::counters::MEALS[eaten_elem as usize]
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if eaten_elem == Elem::Corpse {
        crate::world::counters::CORPSE_EATEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    let mut nutrition = nutrition * c.hunting_bonus();

    if c.traits & trait_bits::SCAVENGER != 0 {
        nutrition *= 1.12;
    }

    if c.knows_tech(Discovery::Herbalism) && eaten_elem.is_organic() {
        nutrition *= 1.15;
    }
    c.energy = (c.energy + nutrition * 4.0).min(c.max_energy);
    c.hunger = (c.hunger - nutrition * 4.0).max(0.0);
    if eaten_elem == Elem::Water {
        c.thirst = (c.thirst - 40.0).max(0.0);
    } else if eaten_elem.nutrition() > 0.0 {
        c.thirst = (c.thirst - nutrition * 4.0).max(0.0);
    }

    match eaten_elem {
        Elem::Stone | Elem::Brick | Elem::Concrete | Elem::Wall => {
            c.inventory[0] = (c.inventory[0].saturating_add(5)).min(255);
            c.carried = (c.carried + 30.0).min(c.carry_cap());
        }
        Elem::Metal | Elem::Wire => {
            c.inventory[1] = (c.inventory[1].saturating_add(3)).min(255);
            c.carried = (c.carried + 40.0).min(c.carry_cap());
        }
        Elem::Copper => {
            c.inventory[2] = (c.inventory[2].saturating_add(3)).min(255);
            c.carried = (c.carried + 40.0).min(c.carry_cap());
        }
        Elem::Gold => {
            c.inventory[3] = (c.inventory[3].saturating_add(2)).min(255);
            c.carried = (c.carried + 50.0).min(c.carry_cap());
        }
        Elem::Crystal => {
            c.inventory[4] = (c.inventory[4].saturating_add(2)).min(255);
            c.carried = (c.carried + 30.0).min(c.carry_cap());
        }
        Elem::Gem | Elem::Diamond => {
            c.inventory[5] = (c.inventory[5].saturating_add(1)).min(255);
            c.carried = (c.carried + 60.0).min(c.carry_cap());
        }
        Elem::Wood => {

            c.inventory[6] = (c.inventory[6].saturating_add(8)).min(255);
            c.carried = (c.carried + 20.0).min(c.carry_cap());
        }
        Elem::Plant | Elem::Fungus => {

            c.inventory[6] = (c.inventory[6].saturating_add(1)).min(255);
            c.carried = (c.carried + 20.0).min(c.carry_cap());
        }
        Elem::Crop => {

            c.inventory[10] = (c.inventory[10].saturating_add(3)).min(255);
            c.carried = (c.carried + 15.0).min(c.carry_cap());
        }
        Elem::Glass | Elem::Circuit | Elem::Tool | Elem::Machine => {
            c.carried = (c.carried + 30.0).min(c.carry_cap());
        }
        Elem::Water => {
            c.inventory[8] = (c.inventory[8].saturating_add(5)).min(255);
        }
        _ => {}
    }
    if toxicity > 0.0 {

        let mut harm = 0.25;
        if c.knows_tech(Discovery::Antitoxins) {
            harm *= 0.5;
        }
        if c.knows_tech(Discovery::Deepdiving) {
            harm = 0.0;
        }
        c.health -= toxicity * harm;
    }
    if c.energy > c.max_energy * 0.75 {
        c.health = (c.health + 0.5).min(100.0);
    }
    true
}

fn desired_neighbour(world: &World, gx: i32, gy: i32, out: &[f32]) -> Option<(i32, i32)> {

    let mut best_dir = 0usize;
    let mut best = f32::MIN;
    for d in 0..outputs::DESIRE_COUNT {
        let v = out[outputs::DESIRE + d];
        if v > best {
            best = v;
            best_dir = d;
        }
    }
    let (dx, dy) = DIRS8[best_dir];
    let (mx, my) = (gx + dx, gy + dy);
    let e = world.get(mx, my);
    if e == Elem::Empty || !e.is_solid() {
        return None;
    }
    Some((mx, my))
}

pub fn act_mine(world: &mut World, c: &mut Creature, out: &[f32], company: f32) {

                if c.faction == Faction::Zorg {
                    return;
                }

                let skill = c.culture_factor(Culture::Mining) * c.affinity.mine_factor() * company;
                let wants_mine = (out[12] > 0.35 || out[17] > 0.3) && world.mine_budget > 0;
                let exploratory_mine = c.carried < 60.0
                    && c.stamina > 8.0
                    && matches!(c.role, Role::Worker | Role::Builder | Role::Explorer)
                    && (c.age.wrapping_add(c.id) % 400) < 1;

                if c.stage == Stage::Juvenile {
                    return;
                }
                if (wants_mine || (exploratory_mine && world.mine_budget > 0)) && c.stamina > 5.0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;

                    let Some((mx, my)) = desired_neighbour(world, gx, gy, out) else {
                        return;
                    };
                    {
                        world.mine_budget -= 1;
                        let me = world.get(mx, my);
                        world.set_xy(mx, my, Elem::Empty);
                        c.stamina = (c.stamina - 0.8).max(0.0);
                        c.energy = (c.energy - energy_cost::MINE * skill).max(0.0);
                        c.carried = (c.carried + 15.0 * skill).min(c.carry_cap());

                        c.inventory[0] = c.inventory[0].saturating_add(1).min(255);

                        let rich = world.landmark_bonus(c.x, c.y).metal;
                        let ore = if rich { 2 } else { 1 };
                        match me {
                            Elem::Stone | Elem::Sand => {
                                c.inventory[0] = c.inventory[0].saturating_add(3).min(255);
                            }
                            Elem::Metal | Elem::Iron | Elem::Steel | Elem::Wire => {
                                c.inventory[1] = c.inventory[1].saturating_add(2 * ore).min(255);
                            }
                            Elem::Copper => {
                                c.inventory[2] = c.inventory[2].saturating_add(2 * ore).min(255);
                            }
                            Elem::Gold => {
                                c.inventory[3] = c.inventory[3].saturating_add(2 * ore).min(255);
                            }
                            Elem::Crystal => {
                                c.inventory[4] = c.inventory[4].saturating_add(2 * ore).min(255);
                            }
                            Elem::Gem | Elem::Diamond => {
                                c.inventory[5] = c.inventory[5].saturating_add(ore).min(255);
                            }
                            Elem::Wood => {
                                c.inventory[6] = c.inventory[6].saturating_add(3).min(255);
                            }
                            Elem::Gunpowder => {
                                c.inventory[9] = c.inventory[9].saturating_add(2).min(255);
                                c.carried = (c.carried + 10.0).min(c.carry_cap());
                            }
                            _ => {}
                        }

                        let deep = c.traits & trait_bits::DEEP_DIGGER != 0;
                        if deep && world.rng.chance(0.06) {
                            let prize = match world.rng.range(4) {
                                0 => Elem::Crystal,
                                1 => Elem::Gem,
                                2 => Elem::Gold,
                                _ => Elem::Iron,
                            };
                            let _ = prize;
                            c.inventory[4] = c.inventory[4].saturating_add(1).min(255);
                            c.carried = (c.carried + 20.0).min(c.carry_cap());
                        }
                    }
                }
}

pub fn act_craft(
    c: &mut Creature,
    out: &[f32],
    events: &mut Vec<SimEvent>,
    workshop: bool,
    ruins: bool,
    discoveries: u32,
) -> bool {
    let mut crafted = false;

    if c.faction == Faction::Zorg {
        return crafted;
    }

                let skill = c.culture_factor(Culture::Craft)
                    * if workshop { 1.5 } else { 1.0 }
                    * if ruins { 1.35 } else { 1.0 };

                let knows_ranged = discoveries & discovery_bits::RANGED != 0;
                let knows_medicine = discoveries & discovery_bits::MEDICINE != 0;
                let knows_sanctify = discoveries & discovery_bits::SANCTIFY != 0;
                let knows_siegecraft = discoveries & discovery_bits::SIEGECRAFT != 0;

                let can_make = |t: Tool| -> bool {
                    match tool_gate(t) {
                        None => true,
                        Some(d) => c.tech & d.bit() != 0,
                    }
                };
                let nn_wants_craft = out[6] > 0.25;
                let exploratory_craft = c.tool == Tool::None && c.craft_cooldown == 0
                    && (c.age * 13 + c.id * 17) % 25 < 1;

                let wants_craft = c.craft_cooldown == 0
                    && (nn_wants_craft || exploratory_craft)
                    && c.stage != Stage::Juvenile;
                let tool_before = c.tool;
                let bombs_before = c.bombs;
                let salves_before = c.salves;
                let inv_before = c.inventory;
                if wants_craft {

                    if knows_sanctify
                        && c.tech_level >= 1
                        && c.inventory[4] >= 1
                        && c.inventory[6] >= 1
                    {
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.inventory[6] = c.inventory[6].saturating_sub(1);
                        c.tool = Tool::Sunfire;
                        c.tool_durability = 900;
                        c.craft_cooldown = 50;
                    }

                    else if c.tool == Tool::None && c.inventory[0] >= 5 && c.inventory[6] >= 2 {
                        c.inventory[0] = c.inventory[0].saturating_sub(5);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.tool = Tool::Pickaxe;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 30;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[1] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.tool = Tool::Sword;
                        c.tool_durability = 800;
                        c.craft_cooldown = 30;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[0] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[0] = c.inventory[0].saturating_sub(5);
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.tool = Tool::Hammer;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 30;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[2] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[2] = c.inventory[2].saturating_sub(3);
                        c.inventory[1] = c.inventory[1].saturating_sub(2);
                        c.tool = Tool::Drill;
                        c.tool_durability = 600;
                        c.craft_cooldown = 40;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[3] >= 2 && c.inventory[4] >= 1 {
                        c.inventory[3] = c.inventory[3].saturating_sub(2);
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.tool = Tool::Scanner;
                        c.tool_durability = 2000;
                        c.craft_cooldown = 50;
                    }

                    else if c.tool == Tool::None
                        && c.tech_level >= 1
                        && (knows_ranged || c.inventory[6] >= 3)
                    {
                        c.inventory[6] = c.inventory[6].saturating_sub(if knows_ranged { 3 } else { 5 });
                        c.tool = Tool::Bow;
                        c.tool_durability = if knows_ranged { 800 } else { 500 };
                        c.craft_cooldown = 30;
                    }

                    else if c.tool == Tool::None && c.inventory[6] >= 2 && c.inventory[0] >= 1 {
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.inventory[0] = c.inventory[0].saturating_sub(1);
                        c.tool = Tool::Spear;
                        c.tool_durability = 400;
                        c.craft_cooldown = 20;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[6] = c.inventory[6].saturating_sub(3);
                        c.inventory[1] = c.inventory[1].saturating_sub(2);
                        c.tool = Tool::Shield;
                        c.tool_durability = 700;
                        c.craft_cooldown = 30;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 1 && c.inventory[4] >= 1 {
                        c.inventory[6] = c.inventory[6].saturating_sub(1);
                        c.inventory[4] = c.inventory[4].saturating_sub(1);
                        c.tool = Tool::Staff;
                        c.tool_durability = 1200;
                        c.craft_cooldown = 40;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 4 && c.inventory[3] >= 1 && c.inventory[5] >= 1 {
                        c.inventory[3] = c.inventory[3].saturating_sub(1);
                        c.inventory[5] = c.inventory[5].saturating_sub(1);
                        c.tool = Tool::Amulet;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 50;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 2 {
                        c.inventory[6] = c.inventory[6].saturating_sub(4);
                        c.tool = Tool::Backpack;
                        c.tool_durability = 800;
                        c.craft_cooldown = 40;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[6] = c.inventory[6].saturating_sub(5);
                        c.inventory[1] = c.inventory[1].saturating_sub(3);
                        c.tool = Tool::Cart;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 60;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 6 {
                        c.inventory[6] = c.inventory[6].saturating_sub(6);
                        c.tool = Tool::Boat;
                        c.tool_durability = 900;
                        c.craft_cooldown = 60;
                    }

                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 4 {
                        c.inventory[6] = c.inventory[6].saturating_sub(4);
                        c.tool = Tool::Glider;
                        c.tool_durability = 700;
                        c.craft_cooldown = 70;
                    }

                    else if c.bombs < 3
                        && knows_siegecraft
                        && c.inventory[9] >= 3
                        && c.inventory[6] >= 2
                    {
                        c.inventory[9] = c.inventory[9].saturating_sub(3);
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        c.bombs += 1;
                        c.craft_cooldown = 40;
                    }

                    else if c.salves < 3
                        && c.inventory[9] == 0
                        && knows_medicine
                        && (c.inventory[6] >= 2 && c.inventory[4] >= 1)
                    {
                        c.inventory[6] = c.inventory[6].saturating_sub(2);
                        if !knows_medicine {
                            c.inventory[4] = c.inventory[4].saturating_sub(1);
                        }
                        c.salves += 1;
                        c.craft_cooldown = 40;
                    }

                    else {
                        c.craft_cooldown = 25;
                    }

                    if c.tool != tool_before && !can_make(c.tool) {
                        c.tool = tool_before;
                        c.inventory = inv_before;
                        c.craft_cooldown = 25;
                    }
                }

                if c.tool != tool_before || c.bombs != bombs_before || c.salves != salves_before {
                    c.energy = (c.energy - energy_cost::CRAFT / skill).max(0.0);
                    crafted = true;
                }

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

                if c.tech_level == 0 && (c.inventory[2] >= 1 || c.inventory[1] >= 1) { c.tech_level = 1; }
                else if c.tech_level == 1 && c.inventory[1] >= 3 { c.tech_level = 2; }
                else if c.tech_level == 2 && c.inventory[3] >= 1 { c.tech_level = 3; }
                else if c.tech_level == 3 && c.inventory[4] >= 1 { c.tech_level = 4; }
    crafted
}

pub fn act_combat(
    c: &mut Creature,
    out: &[f32],
    grid: &SpatialGrid,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    shots: &mut Vec<Projectile>,
    events: &mut Vec<SimEvent>,
    diplomacy: &Diplomacy,
) -> bool {
    let mut wants_attack = false;

                let mut nearest_foreign: Option<(u32, f32)> = None;
                grid.near(c.x, c.y, 16, |idx| {
                    let a = &agents[idx as usize];
                    if a.0 == c.id || a.3 == c.species {
                        return;
                    }
                    let d = ((a.1 - c.x).powi(2) + (a.2 - c.y).powi(2)).sqrt();
                    if nearest_foreign.is_none_or(|(_, bd)| d < bd) {
                        nearest_foreign = Some((a.3, d));
                    }
                });

                let truce = nearest_foreign.is_some_and(|(sp, _)| {
                    diplomacy.has_treaty(c.species, sp, Treaty::Ceasefire)
                        && !diplomacy.at_war(c.species, sp)
                });

                let bold = c.personality.aggression;
                let attack_threshold = 0.45 - (bold - 0.5) * 0.3;
                if out[4] > attack_threshold && !truce && c.stage.can_fight()

                    && c.traits & trait_bits::PEACEFUL == 0 {
                        wants_attack = true;
                    }

                if c.is_mad() {
                    wants_attack = true;
                }

                if out[4] > 0.25 || out[outputs::HOSTILE] > 0.5 {
                    let mut enemy_near = false;
                    grid.near(c.x, c.y, 18, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }
                        if diplomacy.at_war(c.species, a.3) {
                            enemy_near = true;
                        }
                    });
                    if enemy_near {
                        wants_attack = true;
                    }
                }

                if out[19] > 0.3 {

                    let mut intruder_near = false;
                    grid.near(c.base_x, c.base_y, 30, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }
                        let bx = a.1 - c.base_x;
                        let by = a.2 - c.base_y;
                        if bx * bx + by * by < 900.0 {
                            intruder_near = true;
                        }
                    });
                    if intruder_near {
                        wants_attack = true;
                    }
                }

                let ranged_range = match c.tool {
                    Tool::Bow => 30.0,
                    Tool::Staff => 24.0,
                    _ => 0.0,
                };

                let mut ranged_range = ranged_range;
                let mut shot_damage = 1.0f32;
                if c.knows_tech(Discovery::Ranged) && c.tool == Tool::Bow {
                    ranged_range *= 1.25;
                }
                if c.knows_tech(Discovery::Siegecraft) {
                    ranged_range *= 1.25;
                    shot_damage = 1.25;
                }
                let wants_shoot = out[14] > 0.3;
                if ranged_range > 0.0 && wants_shoot && c.fire_cooldown == 0 && c.energy > 40.0 {
                    let mut target: Option<(f32, f32, f32)> = None;
                    grid.near(c.x, c.y, 32, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }

                        if diplomacy.are_allied(c.species, a.3) {
                            return;
                        }
                        let dx = a.1 - c.x;
                        let dy = a.2 - c.y;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < ranged_range && target.is_none_or(|t| d < t.2) {
                            target = Some((dx, dy, d));
                        }
                    });
                    if let Some((dx, dy, d)) = target {
                        let inv = 1.0 / d.max(0.001);
                        let (ux, uy) = (dx * inv, dy * inv);
                        let (speed, dmg, life, elem) = if c.tool == Tool::Bow {
                            (3.2, 26.0 * shot_damage, 34, Elem::Wood)
                        } else {
                            (2.6, 34.0 * shot_damage, 30, Elem::Mana)
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
                        c.energy = (c.energy - energy_cost::SHOOT).max(0.0);
                        events.push(SimEvent::Shoot {
                            x: c.x,
                            y: c.y,
                            color: c.team_color,
                        });
                    }
                }
    wants_attack
}
