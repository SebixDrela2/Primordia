use super::{World, Creature, Structure, SimEvent, Faction, MAX_STRUCTURES, Culture, Stage, energy_cost, Discovery, Role, MAX_FARMS, MAX_TOWERS, structure_kind, Buildable, Elem, trait_bits, BASE_RADIUS, DIRS8};

/// Where the creature is standing and where its home is.
struct Site {
    gx: i32,
    gy: i32,
    bx: i32,
    by: i32,
    dist_to_base: f32,
}

/// The building the creature picks out of the ones it could want. The
/// hungry want a farm first, and a settled colony wants a market.
fn choose_buildable(
    c: &Creature,
    structures: &[Structure],
    sanctified: bool,
    discount: f32,
) -> Buildable {
    let want_wall = c.inventory[0] >= 3 && c.tech_level < 2;
    let want_door = c.inventory[0] >= 5 && c.role == Role::Builder;
    let want_chest = c.inventory[1] >= 3 && c.carried > 150.0;

    let farms_built = structures
        .iter()
        .filter(|s| s.species == c.species && s.kind == 3)
        .count();
    let want_farm = c.inventory[6] >= 4 && c.tech_level >= 1 && farms_built < MAX_FARMS;
    let want_temple = c.inventory[4] >= 2 && c.tech_level >= 3 && c.role == Role::Priest;

    let towers_built = structures
        .iter()
        .filter(|s| s.species == c.species && s.kind == 1)
        .count();
    let want_tower = c.tech_level >= 1
        && c.inventory[6] >= 8
        && towers_built < MAX_TOWERS
        && matches!(c.role, Role::Explorer | Role::Soldier | Role::Builder | Role::Worker);

    let want_market = c.tech_level >= 2 && c.inventory[6] >= 10 && c.inventory[1] >= 4;
    let want_workshop = c.tech_level >= 2 && c.inventory[6] >= 8 && c.inventory[1] >= 6;
    let want_granary = c.tech_level >= 1 && c.inventory[6] >= 12 && c.inventory[0] >= 4;
    let want_beacon = c.tech_level >= 3 && c.inventory[0] >= 10 && c.inventory[4] >= 3;

    let wards_built = structures
        .iter()
        .filter(|s| s.species == c.species && s.kind == structure_kind::WARD)
        .count();
    let want_ward = sanctified
        && c.tech_level >= 2
        && c.inventory[4] >= (2.0 * discount) as u16
        && c.inventory[0] >= (6.0 * discount) as u16
        && wards_built < 3;

    let farming = c.knows(Culture::Cultivation);
    let starving = c.hunger > 45.0;
    let settled = c.knows(Culture::Commerce) && c.knows(Culture::Architecture);

    if want_farm && (starving || farming) {
        return Buildable::Farm;
    }
    if want_tower {
        return Buildable::Tower;
    }
    if want_temple {
        return Buildable::Temple;
    }
    if want_ward {
        return Buildable::Ward;
    }
    if want_market && settled {
        return Buildable::Market;
    }
    if want_workshop && settled {
        return Buildable::Workshop;
    }
    if want_granary && farming {
        return Buildable::Granary;
    }
    if want_beacon {
        return Buildable::Beacon;
    }
    if want_chest {
        return Buildable::Chest;
    }
    if want_farm {
        return Buildable::Farm;
    }
    if want_market {
        return Buildable::Market;
    }
    if want_workshop {
        return Buildable::Workshop;
    }
    if want_granary {
        return Buildable::Granary;
    }
    if want_door {
        return Buildable::Door;
    }
    if want_wall {
        return Buildable::Wall;
    }
    Buildable::Bridge
}

/// What the building is made of in the ground.
fn struct_elem(chosen: Buildable) -> Elem {
    match chosen {
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
    }
}

/// The cooldown a building leaves behind, shortened for the industrious.
fn build_cooldown(c: &Creature, skill: f32) -> u32 {
    let base = if c.traits & trait_bits::INDUSTRIOUS != 0 { 6 } else { 10 };
    ((base as f32) / skill).round() as u32
}

/// Where the building goes: a farm beside the water, else a ring round
/// the home, else the ground under the creature's feet.
fn place_structure(
    world: &mut World,
    c: &mut Creature,
    chosen: Buildable,
    site: &Site,
    skill: f32,
) -> Option<(i32, i32)> {
    if chosen == Buildable::Farm {
        if let Some((tx, ty, px, py)) = world.find_farm_site(site.gx, site.gy, 14) {
            world.set_xy(tx, ty, Elem::Farm);

            world.set_xy(px, py, Elem::Plant);
            c.carried = (c.carried - 15.0).max(0.0);
            c.build_cooldown = build_cooldown(c, skill);
            return Some((tx, ty));
        }
    }

    if site.dist_to_base < (BASE_RADIUS * BASE_RADIUS) as f32 {
        for r in 3..=BASE_RADIUS {
            for angle in 0..24 {
                let tx = site.bx + (r as f32 * (angle as f32 * 0.26).cos()) as i32;
                let ty = site.by + (r as f32 * (angle as f32 * 0.26).sin()) as i32;
                if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                    world.set_xy(tx, ty, struct_elem(chosen));
                    c.carried = (c.carried - 15.0).max(0.0);
                    c.build_cooldown = build_cooldown(c, skill);
                    return Some((tx, ty));
                }
            }
        }
    }

    for (off_x, off_y) in DIRS8 {
        let tx = site.gx + off_x;
        let ty = site.gy + off_y;
        if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
            world.set_xy(tx, ty, struct_elem(chosen));
            c.carried = (c.carried - 15.0).max(0.0);
            c.build_cooldown = build_cooldown(c, skill);
            return Some((tx, ty));
        }
    }

    for dy in -2..=2 {
        for dx in -2..=2 {
            let tx = site.gx + dx;
            let ty = site.gy + dy;
            if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                world.set_xy(tx, ty, struct_elem(chosen));
                c.carried = (c.carried - 15.0).max(0.0);
                c.build_cooldown = 10;
                return Some((tx, ty));
            }
        }
    }
    None
}

/// The plain buildings: what they cost and how they look.
fn plain_cost(chosen: Buildable) -> (u8, u16, u16, u16, u16, f32) {
    match chosen {
        Buildable::Market => (structure_kind::MARKET, 10, 4, 0, 0, 140.0),
        Buildable::Workshop => (structure_kind::WORKSHOP, 8, 6, 0, 0, 160.0),
        Buildable::Granary => (structure_kind::GRANARY, 12, 0, 4, 0, 180.0),
        Buildable::Beacon => (structure_kind::BEACON, 0, 0, 10, 3, 200.0),
        Buildable::Ward => (structure_kind::WARD, 0, 0, 6, 2, 260.0),
        _ => (structure_kind::PLAIN, 0, 0, 0, 0, 100.0),
    }
}

/// What the building takes out of the creature's hands, and what it
/// leaves standing in the colony.
fn commit_structure(
    c: &mut Creature,
    structures: &mut Vec<Structure>,
    chosen: Buildable,
    place_x: i32,
    place_y: i32,
    discount: f32,
    sturdy: f32,
) {
    if chosen == Buildable::Farm {
        c.inventory[6] = c.inventory[6].saturating_sub((4_f32 * discount) as u16);

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
    } else if chosen == Buildable::Tower {
        let mine = structures
            .iter()
            .filter(|s| s.species == c.species && s.kind == 1)
            .count();
        c.inventory[6] = c.inventory[6].saturating_sub((8_f32 * discount) as u16);
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
        c.inventory[4] = c.inventory[4].saturating_sub((2_f32 * discount) as u16);
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
        let (kind, wood, metal, stone, crystal, hp) = plain_cost(chosen);
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
}

pub fn act_build(
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
    if !built {
        return (built_something, structures_total);
    }

    c.energy = (c.energy - energy_cost::BUILD / skill).max(0.0);

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

    let chosen = choose_buildable(c, structures, sanctified, discount);

    let gx = c.x.round() as i32;
    let gy = c.y.round() as i32;
    let bx = c.base_x.round() as i32;
    let by = c.base_y.round() as i32;
    let site = Site {
        gx,
        gy,
        bx,
        by,
        dist_to_base: ((gx - bx).pow(2) + (gy - by).pow(2)) as f32,
    };

    let Some((place_x, place_y)) = place_structure(world, c, chosen, &site, skill) else {
        return (built_something, structures_total);
    };

    built_something = true;
    structures_total += 1;

    if world
        .landmark_bonus(site.gx as f32, site.gy as f32)
        .has(crate::world::Landmark::AmberField)
        && c.build_cooldown > 3
    {
        c.build_cooldown = 3;
    }

    commit_structure(c, structures, chosen, place_x, place_y, discount, sturdy);

    events.push(SimEvent::Build {
        x: place_x as f32,
        y: place_y as f32,
        color: c.team_color,
    });

    (built_something, structures_total)
}
