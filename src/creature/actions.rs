use super::*;

// The individual behaviours a creature can pick during its turn.

/// How strongly the creature wants to move in each direction.
pub(crate) fn act_steer(
    c: &Creature,
    out: &[f32],
    scan: &LocalScan,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
    foreign_bases: &[(u32, f32, f32)],
    ds: &DiplomaticSense,
    fear: &FearTable,
    weather: crate::world::Weather,
    place: &crate::world::LandmarkBonus,
    volcano: Option<(f32, f32)>,
) -> (f32, f32, f32) {
                // Outputs: [0]=move_x, [1]=move_y, [2]=eat, [3]=reproduce, [4]=attack,
                // [5]=build, [6]=craft, [7]=cast_spell, [8]=mount/dismount, [9]=trade,
                // [10]=rest, [11]=switch_spell, [12]=dig/mine, [13]=gather_water,
                // [14]=shoot, [15]=flee, [16]=seek, [17]=forage, [18]=rally,
                // [19]=defend
                let mut steer_x = 0.0f32;
                let mut steer_y = 0.0f32;
                let mut speed_mult_extra = 1.0f32;

                // ---- The ZORGs ----
                //
                // A ZORG belongs to the deep, but it is a predator of the
                // surface, so it pushes upward: out of the caves and into
                // the world. It also does not care about its own kind and it
                // is never afraid of anything.
                //
                // The cave in the deep is its home and its larder, though.
                // A hungry ZORG goes down to it and feeds on the poison
                // pooled on its floor; a fed one goes up through the rock
                // after the colonies that live in the light.
                if c.faction == Faction::Zorg {
                    let (zx, zy, zs) = zorg_steer(c);
                    steer_x += zx;
                    steer_y += zy;
                    speed_mult_extra = speed_mult_extra.max(zs);
                }

                // ---- The volcano ----
                //
                // Nothing wants to live under a mountain that can erupt, so
                // everything near the crater walks away from it. A colony that
                // settled too close will move rather than burn.
                if let Some((vx, vy)) = volcano {
                    let dx = c.x - vx;
                    let dy = c.y - vy;
                    let d = (dx * dx + dy * dy).sqrt().max(0.1);
                    if d < 55.0 {
                        steer_x += dx / d * 2.2;
                        steer_y += dy / d * 2.2;
                        speed_mult_extra = speed_mult_extra.max(1.45);
                    }
                }

                // ---- The temperament of this particular animal ----
                //
                // A bold creature keeps its nerve when an enemy is close, a
                // curious one is willing to walk further from the colony,
                // a cautious one is the first to run and the last to go
                // underground, and a sociable one wants to be near its own
                // kind.
                let bold = c.personality.aggression;
                let wary = c.personality.caution;

                // --- Flee: run from the closest enemy ---
                //
                // The distance at which a creature gives in depends on how
                // brave it is and how close it is to being hurt.
                let flee_range = 45.0 - bold * 22.0 + wary * 10.0 + weather.fear_factor();
                let mut flee_target: Option<(f32, f32, f32)> = None;
                // The nearest creature of another kind. It is the thing to
                // run from, and for a hunter it is the thing to chase.
                // A bone field is good hunting ground: the prey comes
                // closer to the bones.
                let hunt_range = if place.hunting { 48.0 } else { 34.0 };
                let mut prey: Option<(f32, f32, f32)> = None;
                // The nearest creatures are looked for through the spatial
                // grid: walking the whole population for every animal was
                // the last O(n^2) loop in the simulation.
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
                    // A species that has been hunted keeps well away from
                    // the hunter: the fear of the herd lengthens the run.
                    let fright = fear.fear_of(c.species, ospecies) * 0.22;
                    let run = flee_range + fright;
                    if d < run && flee_target.map_or(true, |t| d < t.2) {
                        flee_target = Some((dx, dy, d));
                    }
                    if d < hunt_range && prey.map_or(true, |t| d < t.2) {
                        prey = Some((dx, dy, d));
                    }
                });

                // --- Hunt: stalk the nearest creature of another kind ---
                //
                // A colony that has learned to hunt does not wait for its
                // food to come to it. A hungry hunter goes after the nearest
                // animal of another species and closes on it.
                //
                // A species that has been hunted hard enough learns to hunt
                // itself: the ones that were eaten start eating. This is
                // how a prey species turns into a predator.
                let hunted_hard = fear.total_of(c.species);
                let will_hunt = c.knows(Culture::Hunting) || hunted_hard > 45.0;
                if will_hunt
                    && prey.is_some()
                    && (c.hunger > 25.0 || out[4] > 0.4)
                    && c.stage.can_fight()
                {
                    if let Some((dx, dy, d)) = prey {
                        if d > 1.5 {
                            steer_x += dx / d;
                            steer_y += dy / d;
                            speed_mult_extra = speed_mult_extra.max(1.3);
                        }
                    }
                }

                // --- Seek: head for whatever the creature needs most ---
                //
                // A curious creature will range further to find what it
                // needs; a cautious one settles for what is close by.
                let range_bonus = c.personality.curiosity * 8.0 - wary * 4.0;
                let seek_target: Option<(f32, f32, f32)> = if c.thirst > 45.0 && scan.water.2 < 30.0 + range_bonus {
                    Some(scan.water)
                } else if c.hunger > 45.0 && scan.food.2 < 30.0 + range_bonus {
                    Some(scan.food)
                } else if c.energy < c.max_energy * 0.5 && scan.food.2 < 30.0 + range_bonus {
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
                if out[18] > 0.3 && !c.migrating {
                    let bx = c.base_x - c.x;
                    let by = c.base_y - c.y;
                    let bd = (bx * bx + by * by).sqrt().max(0.1);
                    steer_x += bx / bd;
                    steer_y += by / bd;
                }

                // --- The pull of the colony ---
                //
                // A sociable creature does not like to be alone, and a
                // juvenile has no business wandering off: both are pulled
                // back towards the base. A solitary creature minds none of
                // this and is free to roam. An animal that has left its
                // colony is pulled the other way.
                let home_pull = if c.stage == Stage::Juvenile {
                    1.2
                } else {
                    // A sociable creature wants to be near its own kind.
                    (c.personality.sociability - 0.45).max(0.0) * 0.8
                };
                if home_pull > 0.0 && !c.migrating {
                    let bx = c.base_x - c.x;
                    let by = c.base_y - c.y;
                    let bd = (bx * bx + by * by).sqrt();
                    if bd > 12.0 {
                        steer_x += (bx / bd) * home_pull;
                        steer_y += (by / bd) * home_pull;
                    }
                }

                // --- Shelter from the weather ---
                //
                // A storm sends the colony home: the animals huddle near
                // their base instead of going out in it.
                if weather.shelters() && !c.migrating {
                    let bx = c.base_x - c.x;
                    let by = c.base_y - c.y;
                    let bd = (bx * bx + by * by).sqrt();
                    if bd > 6.0 {
                        steer_x += (bx / bd) * 1.1;
                        steer_y += (by / bd) * 1.1;
                    }
                }

                // --- Courtship: head for the nearest creature of its kind ---
                //
                // A creature that wants a mate goes looking for one. The
                // mating itself is handled in the step, but the walking
                // here is what brings two animals together at all.
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
                        if d < 40.0 && best.map_or(true, |t| d < t.2) {
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

                // --- Migration: walk away from the old colony ---
                //
                // The animals that have left their colony do not go back.
                // They push away from the place they came from, and if they
                // get far enough they become a species of their own.
                if c.migrating {
                    let bx = c.x - c.base_x;
                    let by = c.y - c.base_y;
                    let bd = (bx * bx + by * by).sqrt().max(0.1);
                    steer_x += (bx / bd) * 1.3;
                    steer_y += (by / bd) * 1.3;
                    speed_mult_extra = speed_mult_extra.max(1.2);
                }

                // --- Diplomacy: walk to the foreign colony to open talks ---
                //
                // A diplomat that wants to negotiate does not wait for a
                // stranger to bump into it: it travels to the nearest colony
                // of another species. This is what makes an embassy a real
                // journey across the map instead of a chance encounter.
                if out[outputs::NEGOTIATE] > 0.4 {
                    if let Some(partner) = ds.partner {
                        // A colony that is at war with us is a target, not a
                        // negotiating partner: head for it with intent.
                        let mut best: Option<(f32, f32, f32)> = None;
                        for (sp, bx, by) in foreign_bases {
                            if *sp != partner {
                                continue;
                            }
                            let dx = bx - c.x;
                            let dy = by - c.y;
                            let d = (dx * dx + dy * dy).sqrt();
                            if best.map_or(true, |t| d < t.2) {
                                best = Some((dx, dy, d));
                            }
                        }
                        if let Some((dx, dy, d)) = best {
                            if d > 1.0 {
                                let inv = 1.0 / d;
                                steer_x += dx * inv;
                                steer_y += dy * inv;
                                speed_mult_extra = speed_mult_extra.max(1.15);
                            }
                        }
                    }
                }

    (steer_x, steer_y, speed_mult_extra)
}

/// Movement with collision (mounted moves faster).
pub(crate) fn act_move(world: &mut World, c: &mut Creature, behaviour_speed: f32) {
                // Movement with collision (mounted moves faster)
                let mut speed_mult = behaviour_speed;
                if c.mount_kind != MountKind::None && c.mount_stamina > 0.0 {
                    speed_mult = MOUNT_SPEED_BONUS * behaviour_speed;
                    c.mount_stamina = (c.mount_stamina - 0.3).max(0.0);
                }
                // Moving costs energy, but only for the distance covered.
                let distance = (c.vx.abs() + c.vy.abs()) * speed_mult;
                c.energy = (c.energy - distance * energy_cost::MOVE).max(0.0);
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
                // A ZORG is the exception: it chews through the rock of the
                // deep on its way up, because the rock is its food and its
                // way out. It pays for the digging in stamina, so it is slow
                // and it tires, but nothing stops it.
                let zorg_chew = c.faction == Faction::Zorg;

                let tx = nx.round() as i32;
                let ty = ny.round() as i32;
                let mut ok = !world.is_blocked(tx, ty);
                // Swimming is hard work. A creature that is at home in the
                // water does not mind; the others tire quickly in a lake.
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
                // A ZORG chews its way through solid rock. It is slow and it
                // tires, but the way up is open to it.
                if !ok && zorg_chew {
                    let here = world.get(tx, ty);
                    if here.is_solid() {
                        world.set_xy(tx, ty, Elem::Empty);
                        c.stamina = (c.stamina - 1.2).max(0.0);
                        c.energy = (c.energy - energy_cost::CHEW).max(0.0);
                        // The rock it chews is its food as well as its way
                        // out: the tunnel it digs upwards feeds it. It
                        // does not eat its way down through the floor of
                        // its own cave, though - that would swallow the
                        // larder whole.
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
                    // Geodesy: the colony knows where the food is, and
                    // finds it further off.
                    let radius = if c.knows_tech(Discovery::Geodesy) {
                        (radius as f32 * 1.3) as i32
                    } else {
                        radius
                    };
                    if c.faction == Faction::Zorg {
                        // A ZORG has its own ecology: it eats the rock and
                        // the poison of the deep, and nothing it eats is
                        // ever bad for it.
                        ate_something = zorg_act_eat(world, c, hungry);
                    } else if let Some((fx, fy)) = world.find_food(gx, gy, radius) {
                        let eaten_elem = world.get(fx, fy);
                        if let Some((nutrition, toxicity)) = world.eat(fx, fy) {
                            // Modest returns: food is a real resource, so a
                            // colony has to keep foraging instead of trivially
                            // maxing out after a couple of bites.
                            //
                            // A colony that has hunted for generations gets
                            // more out of the same carcass.
                            let mut nutrition = nutrition * c.hunting_bonus();
                            // Herbalism: the colony knows which plants are
                            // worth eating.
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
                            // Gathering resources gives inventory
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
                                Elem::Gem => {
                                    c.inventory[5] = (c.inventory[5].saturating_add(1)).min(255);
                                    c.carried = (c.carried + 60.0).min(c.carry_cap());
                                }
                                Elem::Wood | Elem::Plant | Elem::Fungus => {
                                    c.inventory[6] = (c.inventory[6].saturating_add(4)).min(255);
                                    c.carried = (c.carried + 20.0).min(c.carry_cap());
                                }
                                Elem::Crop => {
                                    // A harvested crop is stored as grain: it can
                                    // be planted again or eaten later.
                                    c.inventory[10] = (c.inventory[10].saturating_add(3)).min(255);
                                    c.carried = (c.carried + 15.0).min(c.carry_cap());
                                }
                                Elem::Energy => {
                                    c.inventory[7] = (c.inventory[7].saturating_add(10)).min(255);
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
                                // Antitoxins: the colony can hold its own
                                // against poison. Deepdiving: the deep does
                                // no harm at all.
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
pub(crate) fn act_mine(world: &mut World, c: &mut Creature, out: &[f32], company: f32) {
                // A ZORG does not mine. It eats the rock it wants, rather
                // than carrying it away to build something with.
                if c.faction == Faction::Zorg {
                    return;
                }
                // Mining / gathering (output[12]). Takes solid resources out of
                // the world for building but is explicitly *not* food.
                //
                // A colony that has mined for generations digs better, and a
                // species that reads metal in the rock finds more of it.
                let skill = c.culture_factor(Culture::Mining) * c.affinity.mine_factor() * company;
                let wants_mine = (out[12] > 0.35 || out[17] > 0.3) && world.mine_budget > 0;
                let exploratory_mine = c.carried < 60.0
                    && c.stamina > 8.0
                    && matches!(c.role, Role::Worker | Role::Builder | Role::Explorer)
                    && (c.age.wrapping_add(c.id) % 400) < 1;
                // A juvenile is no use with a pickaxe.
                if c.stage == Stage::Juvenile {
                    return;
                }
                if (wants_mine || (exploratory_mine && world.mine_budget > 0)) && c.stamina > 5.0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    // A metallophilic creature reaches further for the ore,
                    // and a deep digger digs further still.
                    let deep = c.traits & trait_bits::DEEP_DIGGER != 0;
                    let reach = 8
                        + if c.traits & trait_bits::METALLOPHILIC != 0 { 5 } else { 0 }
                        + if deep { 4 } else { 0 };
                    if let Some((mx, my)) = world.find_resource(gx, gy, reach) {
                        world.mine_budget -= 1;
                        let me = world.get(mx, my);
                        world.set_xy(mx, my, Elem::Empty);
                        c.stamina = (c.stamina - 0.8).max(0.0);
                        c.energy = (c.energy - energy_cost::MINE * skill).max(0.0);
                        c.carried = (c.carried + 15.0 * skill).min(c.carry_cap());
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
                                c.carried = (c.carried + 10.0).min(c.carry_cap());
                            }
                            _ => {}
                        }
                        // A deep digger sometimes breaks through into a pocket
                        // of something better than plain rock.
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

/// Craft a tool, bomb or salve. Returns true when something was made.
pub(crate) fn act_craft(
    c: &mut Creature,
    out: &[f32],
    events: &mut Vec<SimEvent>,
    workshop: bool,
    ruins: bool,
    discoveries: u32,
) -> bool {
    let mut crafted = false;
    // A ZORG does not craft. It has no workshop and no need of one: it
    // digs with its own jaws and it fights with its own claws. Crafting
    // would cost it the energy it needs to live on the rock.
    if c.faction == Faction::Zorg {
        return crafted;
    }
                // Tool crafting (output[6] = craft)
                //
                // A colony that has crafted for generations makes better
                // tools: the craft costs less and the tool lasts longer. A
                // workshop beside the colony helps even more, and an old
                // ruin still holds the knowledge of the people who built
                // it.
                let skill = c.culture_factor(Culture::Craft)
                    * if workshop { 1.5 } else { 1.0 }
                    * if ruins { 1.35 } else { 1.0 };
                // A colony that has worked out the craft can make the thing
                // without the awkward materials.
                let knows_ranged = discoveries & discovery_bits::RANGED != 0;
                let knows_medicine = discoveries & discovery_bits::MEDICINE != 0;
                let knows_sanctify = discoveries & discovery_bits::SANCTIFY != 0;
                let knows_siegecraft = discoveries & discovery_bits::SIEGECRAFT != 0;
                // What the colony knows how to make. The sword comes with
                // smithing, the bow with ranged craft, and so on: a colony
                // cannot make a thing it has not worked out how to make.
                let can_make = |t: Tool| -> bool {
                    match tool_gate(t) {
                        None => true,
                        Some(d) => c.tech & d.bit() != 0,
                    }
                };
                let nn_wants_craft = out[6] > 0.25;
                let exploratory_craft = c.tool == Tool::None && c.craft_cooldown == 0
                    && (c.age * 13 + c.id * 17) % 25 < 1;
                // A craft can only be attempted when the creature is not still waiting
                // for the last one to finish. Without this the network can
                // ask for a craft on every single step.
                let wants_craft = c.craft_cooldown == 0
                    && (nn_wants_craft || exploratory_craft)
                    && c.stage != Stage::Juvenile;
                let tool_before = c.tool;
                let bombs_before = c.bombs;
                let salves_before = c.salves;
                let inv_before = c.inventory;
                if wants_craft {
                    // Sunfire: 1 crystal + 1 wood (tech 1).
                    //
                    // A colony that has worked out sanctification makes the
                    // light of the surface into a weapon before anything
                    // else. It scours the ZORGs and their corruption, but
                    // it is a poor weapon against ordinary flesh - which is
                    // exactly the point: this is a tool for the deep, not
                    // for a rival.
                    //
                    // Taking up the fire means giving up whatever the
                    // creature was carrying: a pickaxe, a sword, a bow.
                    // The colony trades its ordinary strength for the
                    // light that scours the deep, and it can only afford
                    // to do so as fast as it can find crystal.
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
                    // Pickaxe: 5 stone + 2 wood (tech 0)
                    else if c.tool == Tool::None && c.inventory[0] >= 5 && c.inventory[6] >= 2 {
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
                    // Bow: 3 wood + 2 string(plant) (tech 1).
                    // A colony that has worked out the craft can make one
                    // from wood alone.
                    else if c.tool == Tool::None
                        && c.tech_level >= 1
                        && (knows_ranged || (c.inventory[6] >= 3 && c.inventory[6] >= 2))
                    {
                        c.inventory[6] = c.inventory[6].saturating_sub(if knows_ranged { 3 } else { 5 });
                        c.tool = Tool::Bow;
                        c.tool_durability = if knows_ranged { 800 } else { 500 };
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
                    // Salve: 2 plant + 1 crystal - heals a wounded colonymate.
                    // A colony that has worked out medicine can make one
                    // from plants alone.
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
                    // Nothing was made: the materials are missing.
                    //
                    // A failed attempt must NOT cost the full price of a
                    // craft. Charging for work that never happened turned
                    // every toolless creature into an energy sink: it
                    // tried to craft every few steps, paid 5 energy each
                    // time, and starved next to the materials it could
                    // never afford. A failed attempt now costs nothing and
                    // puts the creature on a short cooldown so it stops
                    // hammering the same impossible recipe.
                    else {
                        c.craft_cooldown = 25;
                    }

                    // ---- What the colony knows how to make ----
                    //
                    // A colony cannot make a thing it has not worked out
                    // how to make: the sword comes with smithing, the bow
                    // with ranged craft, the sunfire with sanctification.
                    // If it tries, it puts the tools down again and keeps
                    // its materials.
                    if c.tool != tool_before && !can_make(c.tool) {
                        c.tool = tool_before;
                        c.inventory = inv_before;
                        c.craft_cooldown = 25;
                    }
                }

                // The energy is only spent when something is actually made.
                if c.tool != tool_before || c.bombs != bombs_before || c.salves != salves_before {
                    c.energy = (c.energy - energy_cost::CRAFT / skill).max(0.0);
                    crafted = true;
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
    diplomacy: &Diplomacy,
) -> bool {
    let mut wants_attack = false;
                // The nearest foreign creature: the species it belongs to
                // decides whether a first strike is allowed at all.
                let mut nearest_foreign: Option<(u32, f32)> = None;
                grid.near(c.x, c.y, 16, |idx| {
                    let a = &agents[idx as usize];
                    if a.0 == c.id || a.3 == c.species {
                        return;
                    }
                    let d = ((a.1 - c.x).powi(2) + (a.2 - c.y).powi(2)).sqrt();
                    if nearest_foreign.map_or(true, |(_, bd)| d < bd) {
                        nearest_foreign = Some((a.3, d));
                    }
                });
                // A ceasefire suppresses the first strike: the two species
                // have agreed to stop, and only a defensive blow breaks it.
                let truce = nearest_foreign.map_or(false, |(sp, _)| {
                    diplomacy.has_treaty(c.species, sp, Treaty::Ceasefire)
                        && !diplomacy.at_war(c.species, sp)
                });

                // Attack intent (output[4] = attack)
                //
                // A bold animal needs less provocation to start a fight, a
                // timid one needs more, and a juvenile has no business
                // fighting at all.
                let bold = c.personality.aggression;
                let attack_threshold = 0.45 - (bold - 0.5) * 0.3;
                if out[4] > attack_threshold && !truce && c.stage.can_fight() {
                    // Peaceful species refuse to start a fight.
                    if c.traits & trait_bits::PEACEFUL == 0 {
                        wants_attack = true;
                    }
                }

                // Madness drives a creature to attack its own kind. A colony
                // with madness in it tears itself apart.
                if c.is_mad() {
                    wants_attack = true;
                }

                // A war overrides the peaceful temperament: once the banners
                // are up, a peaceful species fights back. Only enemies that
                // are actually close count, otherwise the whole world would
                // be at war with the whole world.
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

                // Defend (output[19]): strike at anything closing in on the
                // colony, even without a general attack impulse.
                if out[19] > 0.3 {
                    // Anything of another kind that comes close to the
                    // colony is a target for the defenders.
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

                // Ranged attack. A creature that managed to craft a bow or a
                // staff shoots at the nearest enemy instead of closing in.
                // The shoot output asks for it; soldiers open fire anyway.
                let ranged_range = match c.tool {
                    Tool::Bow => 30.0,
                    Tool::Staff => 24.0,
                    _ => 0.0,
                };
                // Ranged craft: the bow flies further. Siegecraft: the
                // colony's shots fly further and hit harder.
                let mut ranged_range = ranged_range;
                let mut shot_damage = 1.0f32;
                if c.knows_tech(Discovery::Ranged) && c.tool == Tool::Bow {
                    ranged_range *= 1.25;
                }
                if c.knows_tech(Discovery::Siegecraft) {
                    ranged_range *= 1.25;
                    shot_damage = 1.25;
                }
                let wants_shoot = out[14] > 0.3 || (out[4] > 0.6 && c.role == Role::Soldier);
                if ranged_range > 0.0 && wants_shoot && c.fire_cooldown == 0 && c.energy > 40.0 {
                    let mut target: Option<(f32, f32, f32)> = None;
                    grid.near(c.x, c.y, 32, |idx| {
                        let a = &agents[idx as usize];
                        if a.0 == c.id || a.3 == c.species {
                            return;
                        }
                        // Allies are never shot at.
                        if diplomacy.are_allied(c.species, a.3) {
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
