
use crate::creature::{CreatureSystem, Faction, ZORG_START};
use crate::elements::Elem;
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

const RUN: u64 = 6000;

/// What the cave itself is like.
struct Cave {
    width: i32,
    height: i32,
    open: u32,
    floor_ok: u32,
    food: u32,
}

/// Scan the hollow of the cave.
fn scan_cave(world: &World, cx: i32, cy: i32) -> Cave {
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut open = 0u32;
    let mut floor_ok = 0u32;
    for y in (cy - 30)..(cy + 30) {
        for x in (cx - 45)..(cx + 45) {
            if !world.in_zorg_cave(x, y) {
                continue;
            }
            if world.get(x, y) == Elem::Empty {
                open += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                if world.get(x, y + 1).is_solid() {
                    floor_ok += 1;
                }
            }
        }
    }
    Cave {
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
        open,
        floor_ok,
        food: count_cave_food(world, cx, cy),
    }
}

/// The cells of the cave that the ZORGs can eat.
fn count_cave_food(world: &World, cx: i32, cy: i32) -> u32 {
    let mut food = 0u32;
    for y in (cy - 30)..(cy + 30) {
        for x in (cx - 45)..(cx + 45) {
            if world.in_zorg_cave(x, y) && crate::creature::zorg_food(world.get(x, y)) {
                food += 1;
            }
        }
    }
    food
}

/// The cave has to be hollow, floored, stocked and deep.
fn check_cave(world: &World, cave: &Cave, depth: i32, cy: i32) -> u32 {
    let mut failures = 0;
    if cave.open < 200 {
        println!("FAIL: the cave is not hollow enough ({} open cells)", cave.open);
        failures += 1;
    }
    if cave.floor_ok < 30 {
        println!("FAIL: the cave has hardly any floor to stand on");
        failures += 1;
    }
    if cave.food < 50 {
        println!("FAIL: the cave has no larder for the ZORGs");
        failures += 1;
    }

    if depth < 120 {
        println!("FAIL: the cave is only {} below the surface", depth);
        failures += 1;
    }
    let floor_y = cy + crate::world::ZORG_CAVE_RY / 2 + 1;
    if floor_y >= world.h as i32 - 6 {
        println!("FAIL: the cave runs into the bedrock");
        failures += 1;
    }
    failures
}

/// Every founding ZORG has to be born in the cave and in the open.
fn check_spawns(world: &World, creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    let mut outside = 0;
    let mut inside = 0;
    let mut buried = 0;
    for c in &creatures.list {
        if c.faction != Faction::Zorg {
            continue;
        }
        let (x, y) = (c.x.round() as i32, c.y.round() as i32);
        if world.in_zorg_cave(x, y) {
            inside += 1;
        } else {
            outside += 1;
        }
        if !world.get(x, y).is_passable() {
            buried += 1;
        }
    }
    println!(
        "  {} ZORGs spawned inside the cave, {} outside it",
        inside, outside
    );
    if inside != ZORG_START {
        println!("FAIL: not every founding ZORG spawned in the cave");
        failures += 1;
    }
    if buried > 0 {
        println!("FAIL: {} ZORGs spawned inside the rock", buried);
        failures += 1;
    }
    failures
}

fn report_deep_step(
    world: &World,
    creatures: &CreatureSystem,
    i: u64,
    cx: i32,
    cy: i32,
    peak: &mut u32,
) {
    let zorgs: Vec<&crate::creature::Creature> = creatures
        .list
        .iter()
        .filter(|c| c.faction == Faction::Zorg)
        .collect();
    let n = zorgs.len().max(1) as f32;
    *peak = (*peak).max(zorgs.len() as u32);
    let hunger: f32 = zorgs.iter().map(|c| c.hunger).sum::<f32>() / n;
    let energy: f32 = zorgs.iter().map(|c| c.energy).sum::<f32>() / n;
    let in_cave = zorgs
        .iter()
        .filter(|c| world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32))
        .count();

    let left = count_cave_food(world, cx, cy);
    println!(
        "  step {:5}  zorgs {:3}  in cave {:3}  hunger {:5.1}  energy {:6.1}  food left {:5}  plants {:5}",
        i,
        zorgs.len(),
        in_cave,
        hunger,
        energy,
        left,
        world.census.plants
    );
}

/// Watch the deep over time.
fn run_deep(world: &mut World, seed: u64, cx: i32, cy: i32) -> (CreatureSystem, u32) {
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(world, 64);
    let mut peak = 0u32;
    for i in 0..RUN {
        world.step();
        creatures.step(world);
        if i % 1000 != 0 {
            continue;
        }
        report_deep_step(world, &creatures, i, cx, cy, &mut peak);
    }
    (creatures, peak)
}

/// The ZORGs have to survive and to keep breeding.
fn check_deep_population(creatures: &CreatureSystem, peak: u32) -> u32 {
    let mut failures = 0;
    let alive = creatures.zorg_count();
    println!("  {} ZORGs alive at the end (peak {})", alive, peak);
    if alive == 0 {
        println!("FAIL: the ZORGs died out");
        failures += 1;
    }
    if peak < 12 {
        println!("FAIL: the ZORGs never grew (peak {})", peak);
        failures += 1;
    }
    failures
}

fn report_players(world: &World, creatures: &CreatureSystem) {
    let player_species = {
        let mut v: Vec<u32> = creatures
            .list
            .iter()
            .filter(|c| c.faction != Faction::Zorg)
            .map(|c| c.species)
            .collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    println!(
        "  {} player species are left, the deep is {}",
        player_species,
        if world.zorg_cave_sealed { "SEALED" } else { "still open" }
    );
    if !world.zorg_cave_sealed {
        println!("NOTE: the deep is still open: the run cannot end against it");
    }
}

/// Who has worked out sanctification, and who is raising the wards.
fn report_sanctification(world: &World, creatures: &CreatureSystem) {
    let sanctified: Vec<u32> = creatures
        .discoveries
        .iter()
        .filter(|(_, d)| d & crate::creature::discovery_bits::SANCTIFY != 0)
        .map(|(sp, _)| *sp)
        .collect();
    let wards = creatures
        .structures
        .iter()
        .filter(|s| s.kind == crate::creature::structure_kind::WARD)
        .count();
    let sunfires = creatures
        .list
        .iter()
        .filter(|c| c.tool == crate::creature::Tool::Sunfire)
        .count();
    println!(
        "  {} species know sanctification, {} wards stand, {} carry a sunfire",
        sanctified.len(),
        wards,
        sunfires
    );
    for sp in &sanctified {
        let members = creatures.list.iter().filter(|c| c.species == *sp).count();
        let queen = creatures
            .list
            .iter()
            .any(|c| c.species == *sp && c.role == crate::creature::Role::Queen);
        println!(
            "    species {}: {} members, a queen: {}",
            sp, members, queen
        );
    }
    println!(
        "  the deep is {}",
        if world.zorg_cave_sealed { "SEALED" } else { "still open" }
    );
    if sanctified.is_empty() {
        println!("NOTE: no species has worked out sanctification yet");
    }
    if wards == 0 {
        println!("NOTE: no ward has been raised yet");
    }
}

/// Every ZORG carries a sickness out of the deep.
fn check_sickness(creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    let carriers = creatures
        .list
        .iter()
        .filter(|c| c.faction == Faction::Zorg && c.disease != crate::creature::Disease::None)
        .count();
    let sick_players = creatures
        .list
        .iter()
        .filter(|c| c.faction != Faction::Zorg && c.disease != crate::creature::Disease::None)
        .count();
    println!(
        "  {} ZORGs carry a sickness, {} other creatures are sick",
        carriers, sick_players
    );
    if carriers == 0 {
        println!("FAIL: not one ZORG is carrying a sickness");
        failures += 1;
    }
    failures
}

/// The ZORGs leave their filth behind them.
fn check_filth(world: &World) -> u32 {
    let mut failures = 0;
    let mut filth = 0u32;
    let mut on_the_surface = 0u32;
    for x in 0..world.w as i32 {
        let surf = world.surface_y(x);
        for y in 0..world.h as i32 {
            let e = Elem::from_u8(world.kind[y as usize * world.w + x as usize]);
            if matches!(
                e,
                Elem::Virus | Elem::Bacteria | Elem::Acid | Elem::Poison | Elem::Tar | Elem::Chlorine
            ) {
                filth += 1;
                if y < surf - 2 {
                    on_the_surface += 1;
                }
            }
        }
    }
    println!(
        "  {} cells of the world are corrupted, {} of them up in the open air",
        filth, on_the_surface
    );
    if filth == 0 {
        println!("FAIL: the ZORGs left nothing behind them");
        failures += 1;
    }
    if on_the_surface == 0 {

        println!("NOTE: the corruption has not reached the surface yet");
    }
    failures
}

#[must_use]
pub fn run(_args: &[String]) -> i32 {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0001);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    println!("=== zorg cave check ===");
    let mut failures = 0u32;

    let Some((cx, cy)) = world.zorg_cave_center() else {
        println!("FAIL: the world has no ZORG cave");
        std::process::exit(1);
    };
    let surface = world.surface_y(cx);
    let depth = cy - surface;
    println!(
        "  the cave is at ({}, {}), {} below the surface",
        cx, cy, depth
    );

    let cave = scan_cave(&world, cx, cy);
    println!(
        "  it is {} x {} cells, {} of them open, {} of them with a floor",
        cave.width, cave.height, cave.open, cave.floor_ok
    );
    println!("  {} cells of it are ZORG food", cave.food);

    failures += check_cave(&world, &cave, depth, cy);
    failures += check_spawns(&world, &creatures);

    if failures == 0 {
        println!("\nOK: the ZORG cave is sound");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }

    println!("\n--- the deep over time ---");
    let (creatures, peak) = run_deep(&mut world, seed, cx, cy);

    failures += check_deep_population(&creatures, peak);
    report_players(&world, &creatures);
    report_sanctification(&world, &creatures);
    failures += check_sickness(&creatures);
    failures += check_filth(&world);

    if failures == 0 {
        println!("\nOK: the deep is a home for the ZORGs");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
    0
}
