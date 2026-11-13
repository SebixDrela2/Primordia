//! Headless check for the ZORG cave.
//!
//! cargo run --release --bin zorgcheck -- [seed]
//!
//! The ZORGs have a cave of their own in the deep, and every one of them
//! is born inside it. This checks that:
//! - the world really has a cave, and that it is far below the surface
//! - the cave is hollow, with a floor to stand on
//! - the cave holds the things a ZORG eats
//! - every founding ZORG starts inside it, and none is buried in rock
//! - and that the ZORGs survive and breed over a run, instead of starving
//!   in the hole they made

#[path = "../creature/mod.rs"]
mod creature;
#[path = "../elements/mod.rs"]
mod elements;
#[path = "../render/mod.rs"]
mod render;
#[path = "../rng.rs"]
mod rng;
#[path = "../world/mod.rs"]
mod world;

use creature::{CreatureSystem, Faction, ZORG_START};
use elements::Elem;
use world::World;

const W: usize = 1280;
const H: usize = 720;
/// How many steps the run at the end is watched for.
const RUN: u64 = 6000;

fn main() {
    let seed: u64 = std::env::args()
        .skip(1)
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0001);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    println!("=== zorg cave check ===");
    let mut failures = 0u32;

    // ---- The cave itself ----
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

    // How big is it, and how hollow?
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut open = 0u32;
    let mut floor_ok = 0u32;
    let mut food = 0u32;
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
            if creature::zorg_food(world.get(x, y)) {
                food += 1;
            }
        }
    }
    println!(
        "  it is {} x {} cells, {} of them open, {} of them with a floor",
        max_x - min_x + 1,
        max_y - min_y + 1,
        open,
        floor_ok
    );
    println!("  {} cells of it are ZORG food", food);

    if open < 200 {
        println!("FAIL: the cave is not hollow enough ({} open cells)", open);
        failures += 1;
    }
    if floor_ok < 30 {
        println!("FAIL: the cave has hardly any floor to stand on");
        failures += 1;
    }
    if food < 50 {
        println!("FAIL: the cave has no larder for the ZORGs");
        failures += 1;
    }

    // It has to be deep: this is the bottom of the world, not a hole a
    // colony stumbles into under the grass.
    if depth < 120 {
        println!("FAIL: the cave is only {} below the surface", depth);
        failures += 1;
    }
    let floor_y = cy + world::ZORG_CAVE_RY / 2 + 1;
    if floor_y >= world.h as i32 - 6 {
        println!("FAIL: the cave runs into the bedrock");
        failures += 1;
    }

    // ---- The founding ZORGs ----
    let mut outside = 0;
    let mut inside = 0;
    let mut buried = 0;
    for c in creatures.list.iter() {
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

    if failures == 0 {
        println!("\nOK: the ZORG cave is sound");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }

    // ---- And what happens to them over a run ----
    //
    // The deep has to be a home for them: they have to live off the rock
    // and the poison of their own cave, and they have to breed there. A
    // cave that is stripped bare in a few thousand ticks is no home at all.
    println!("\n--- the deep over time ---");
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);
    let mut peak = 0u32;
    for i in 0..RUN {
        world.step();
        creatures.step(&mut world);
        if i % 1000 != 0 {
            continue;
        }
        let zorgs: Vec<&creature::Creature> = creatures
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg)
            .collect();
        let n = zorgs.len().max(1) as f32;
        peak = peak.max(zorgs.len() as u32);
        let hunger: f32 = zorgs.iter().map(|c| c.hunger).sum::<f32>() / n;
        let energy: f32 = zorgs.iter().map(|c| c.energy).sum::<f32>() / n;
        let in_cave = zorgs
            .iter()
            .filter(|c| world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32))
            .count();
        // How much food is left in the cave?
        let mut left = 0u32;
        for y in (cy - 30)..(cy + 30) {
            for x in (cx - 45)..(cx + 45) {
                if world.in_zorg_cave(x, y) && creature::zorg_food(world.get(x, y)) {
                    left += 1;
                }
            }
        }
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
    // They have to have survived the run and grown.
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

    // The endgame: the run can only end against the deep if the deep can
    // be closed. A colony that has worked out sanctification can raise a
    // ward of sanctified fire, and a ward raised in the deep seals the
    // cave: the ZORGs stop breeding and the run can be won.
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

    // ---- The answer to the deep ----
    //
    // The ZORGs cannot be out-fought, so there has to be a way to close
    // their cave. The colony that works out sanctification can raise a
    // ward of sanctified fire, and a ward in the deep seals the cave.
    let sanctified: Vec<u32> = creatures
        .discoveries
        .iter()
        .filter(|(_, d)| d & creature::discovery_bits::SANCTIFY != 0)
        .map(|(sp, _)| *sp)
        .collect();
    let wards = creatures
        .structures
        .iter()
        .filter(|s| s.kind == creature::structure_kind::WARD)
        .count();
    let sunfires = creatures
        .list
        .iter()
        .filter(|c| c.tool == creature::Tool::Sunfire)
        .count();
    println!(
        "  {} species know sanctification, {} wards stand, {} carry a sunfire",
        sanctified.len(),
        wards,
        sunfires
    );
    for sp in sanctified.iter() {
        let members = creatures.list.iter().filter(|c| c.species == *sp).count();
        let queen = creatures
            .list
            .iter()
            .any(|c| c.species == *sp && c.role == creature::Role::Queen);
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

    // ---- The filth of the deep ----
    //
    // The ZORGs are a dirty faction: they carry their sickness and they
    // leave their poison behind them. The world has to show it.
    let carriers = creatures
        .list
        .iter()
        .filter(|c| c.faction == Faction::Zorg && c.disease != creature::Disease::None)
        .count();
    let sick_players = creatures
        .list
        .iter()
        .filter(|c| c.faction != Faction::Zorg && c.disease != creature::Disease::None)
        .count();
    println!(
        "  {} ZORGs carry a sickness, {} other creatures are sick",
        carriers, sick_players
    );
    if carriers == 0 {
        println!("FAIL: not one ZORG is carrying a sickness");
        failures += 1;
    }
    // The corruption the ZORGs leave behind.
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
        // Not a failure: the corruption only reaches the surface when the
        // ZORGs come up out of the deep, which takes a while.
        println!("NOTE: the corruption has not reached the surface yet");
    }

    if failures == 0 {
        println!("\nOK: the deep is a home for the ZORGs");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
}
