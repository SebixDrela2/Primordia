//! Headless check for the air of the world.
//!
//! cargo run --release --bin aircheck -- [steps] [seed]
//!
//! The air is one of the newest parts of the simulation, so it gets its
//! own check. It runs a world without a renderer and asserts the things
//! that have to be true of the air:
//! - the surface has fresh air in it, because the plants breathe
//! - the deep does not, because nothing down there makes any
//! - an undergrove does, and it is well below the surface
//! - fire in a sealed cave goes out, and fire in the open does not
//! - a creature in stale air tires faster than one in fresh air

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

use creature::CreatureSystem;
use elements::Elem;
use world::{Landmark, World};

const W: usize = 1280;
const H: usize = 720;

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0003);

    let mut world = World::new(W, H, seed);
    world.seed();

    println!("=== air check ===");
    println!("steps: {}", steps);

    let mut failures = 0u32;

    // ---- The undergroves ----
    //
    // They are the one landmark in the deep, and they have to be deep.
    let groves: Vec<_> = world
        .landmarks
        .iter()
        .filter(|l| l.kind == Landmark::Undergrove)
        .collect();
    println!("--- the undergroves ---");
    println!("  found: {}", groves.len());
    for g in &groves {
        let col = g.x as i32;
        let surface = world.surface_y(col);
        let depth = g.y as i32 - surface;
        println!(
            "  a grove at ({}, {}), {} below the surface",
            g.x, g.y, depth
        );
        if depth < 20 {
            println!("FAIL: an undergrove is not in the deep");
            failures += 1;
        }
        // The air of the grove has to be there.
        let air = world.fresh_air(g.x as i32, g.y as i32, 6);
        println!("    the air in it: {:.0}%", air * 100.0);
        if air < 0.2 {
            println!("FAIL: an undergrove has no air in it");
            failures += 1;
        }
    }
    if groves.is_empty() {
        println!("NOTE: this world has no undergroves");
    }

    // ---- The surface has air, the deep does not ----
    //
    // The plants breathe in the daylight, so the open air is fresh.
    world.daylight = 1.0;
    for _ in 0..steps {
        world.step();
    }
    println!("--- the air of the world ---");
    let mut surface_air = 0.0f32;
    let mut surface_samples = 0u32;
    let mut deep_air = 0.0f32;
    let mut deep_samples = 0u32;
    for x in (60..W as i32 - 60).step_by(17) {
        let surf = world.surface_y(x);
        // Just above the ground: this is the open air.
        let a = world.fresh_air(x, surf - 3, 3);
        surface_air += a;
        surface_samples += 1;
        // Well below: this is the deep.
        let b = world.fresh_air(x, surf + 60, 3);
        deep_air += b;
        deep_samples += 1;
    }
    let surface_air = surface_air / surface_samples.max(1) as f32;
    let deep_air = deep_air / deep_samples.max(1) as f32;
    println!("  the open air: {:.0}% fresh", surface_air * 100.0);
    println!("  the deep:     {:.0}% fresh", deep_air * 100.0);
    if surface_air < 0.15 {
        println!("FAIL: the open air is not fresh");
        failures += 1;
    }
    if deep_air > surface_air {
        println!("FAIL: the deep is fresher than the open air");
        failures += 1;
    }

    // ---- Fire needs air ----
    //
    // A fire with oxygen around it burns on; a fire in a sealed cave
    // eats its own air and goes out. The two are measured directly: a
    // chamber is built, a fire is lit in it, and the fire is watched.
    println!("--- fire ---");
    let open = fire_lifetime(true);
    let sealed = fire_lifetime(false);
    println!("  a fire in the open air lives {} steps", open);
    println!("  a fire in a sealed cave lives {} steps", sealed);
    if open == 0 {
        println!("FAIL: no fire burns in the open air");
        failures += 1;
    }
    if sealed >= open {
        println!("FAIL: a fire in a sealed cave lasts as long as one in the open");
        failures += 1;
    }

    // ---- A creature in stale air tires faster ----
    println!("--- the cost of stale air ---");
    let stale = stale_cost(false);
    let fresh = stale_cost(true);
    println!("  the wind recovered in stale air: {:.3}", stale);
    println!("  the wind recovered in fresh air:  {:.3}", fresh);
    if fresh <= stale {
        println!("FAIL: stale air costs nothing");
        failures += 1;
    }

    if failures == 0 {
        println!("OK: the air holds together");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
}

/// How long a fire lives in a chamber, with air in it or without.
///
/// A chamber is built in the rock, a fire is lit in it, and the world is
/// run until the fire goes out. A fire with oxygen around it has a supply
/// of air; a fire in a sealed chamber has only the air it brings with it.
fn fire_lifetime(with_air: bool) -> u32 {
    // A tall world, so the chamber can be well below the surface: nothing
    // down there makes any oxygen of its own.
    let mut w = World::new(64, 100, 77);
    w.seed();
    // A tiny chamber, so the creature cannot walk about: the measurement
    // has to be of the air and not of how much walking costs.
    let (x0, x1) = (30i32, 33i32);
    let (x0, x1) = (24i32, 40i32);
    let (y0, y1) = (70i32, 78i32);
    // A chamber in the rock, sealed on every side. Everything in it and
    // around it is cleared, so the world cannot have left any oxygen in it.
    for y in y0 - 3..y1 + 3 {
        for x in x0 - 3..x1 + 3 {
            w.set_xy(x, y, Elem::Stone);
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            w.set_xy(x, y, Elem::Empty);
        }
    }
    for y in y0 - 1..y1 + 1 {
        w.set_xy(x0 - 1, y, Elem::Stone);
        w.set_xy(x1, y, Elem::Stone);
    }
    for x in x0 - 1..=x1 {
        w.set_xy(x, y0 - 1, Elem::Stone);
        w.set_xy(x, y1, Elem::Stone);
    }
    if with_air {
        for y in y0..y1 - 1 {
            for x in x0..x1 {
                w.set_xy(x, y, Elem::Oxygen);
            }
        }
    }
    // Light the fire on the floor of the chamber. There is no fuel around
    // it, so the only thing that can keep it alive is the air.
    for x in x0 + 4..x0 + 12 {
        w.set_xy(x, y1 - 2, Elem::Fire);
    }
    // Watch it burn.
    let mut steps = 0u32;
    while steps < 900 {
        w.step();
        steps += 1;
        let mut any = false;
        for y in y0..y1 {
            for x in x0..x1 {
                if w.get(x, y) == Elem::Fire {
                    any = true;
                }
            }
        }
        if !any {
            break;
        }
    }
    steps
}

/// How much energy a creature loses in a tick, in stale air and in fresh.
///
/// A little world is built with a chamber in it, and the chamber is either
/// full of oxygen or full of nothing at all. A creature is put in the
/// chamber and the energy it burns in a hundred ticks is measured.
fn stale_cost(fresh: bool) -> f32 {
    // A tall world, so the chamber can be well below the surface: nothing
    // down there makes any oxygen of its own.
    let mut w = World::new(64, 100, 99);
    w.seed();
    // A tiny chamber, so the creature cannot walk about: the measurement
    // has to be of the air and not of how much walking costs.
    let (x0, x1) = (30i32, 33i32);
    let (y0, y1) = (70i32, 73i32);
    // A chamber in the deep, sealed on every side. Everything around it is
    // cleared, so the world cannot have left any oxygen in it.
    for y in y0 - 3..y1 + 3 {
        for x in x0 - 3..x1 + 3 {
            w.set_xy(x, y, Elem::Stone);
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            w.set_xy(x, y, Elem::Empty);
        }
    }
    for y in y0 - 1..y1 + 1 {
        w.set_xy(x0 - 1, y, Elem::Stone);
        w.set_xy(x1, y, Elem::Stone);
    }
    for x in x0 - 1..=x1 {
        w.set_xy(x, y0 - 1, Elem::Stone);
        w.set_xy(x, y1, Elem::Stone);
    }
    if fresh {
        for y in y0..y1 - 1 {
            for x in x0..x1 {
                w.set_xy(x, y, Elem::Oxygen);
            }
        }
    }
    // No mining and no tools: the creature must not dig its way out of
    // the chamber, or the measurement would be of the rock outside and
    // not of the air inside.
    w.mine_budget = 0;
    w.daylight = 1.0;

    let mut creatures = CreatureSystem::new(4242);
    creatures.populate(&w, 8);
    // No pick and no drill: the walls of the chamber stay walls.
    for c in creatures.list.iter_mut() {
        c.tool = creature::Tool::None;
        c.tool_durability = 0;
    }
    // Put every creature in the chamber.
    for c in creatures.list.iter_mut() {
        c.x = 30.0;
        c.y = (y1 - 1) as f32;
    }
    // Let them settle.
    for _ in 0..20 {
        creatures.step(&mut w);
    }
    // Make them all tired, so they lie down and recover their wind. The
    // measurement is then of the wind they get back, which is what the
    // air actually changes.
    for c in creatures.list.iter_mut() {
        c.energy = c.max_energy * 0.6;
        c.stamina = 0.0;
    }
    let before: f32 = creatures.list.iter().map(|c| c.stamina).sum();
    for _ in 0..60 {
        // Put them back in the chamber and fill it with air again, so the
        // measurement is of the air and not of what the creatures do to
        // the room they are in: a creature that digs a tunnel, or a fire
        // that eats the floor, would otherwise take the whole chamber
        // away and the two cases would measure the same stale rock.
        for c in creatures.list.iter_mut() {
            c.x = 30.0;
            c.y = (y1 - 1) as f32;
        }
        if fresh {
            for y in y0..y1 - 1 {
                for x in x0..x1 {
                    if w.get(x, y) == Elem::Empty {
                        w.set_xy(x, y, Elem::Oxygen);
                    }
                }
            }
        }
        creatures.step(&mut w);
        // Keep them lying down, so the measurement is of the recovery and
        // not of a creature that got up.
        for c in creatures.list.iter_mut() {
            c.stamina = c.stamina.min(c.max_stamina * 0.5);
        }
    }
    let after: f32 = creatures.list.iter().map(|c| c.stamina).sum();
    // The wind they got back, per creature per tick.
    ((after - before) / creatures.list.len().max(1) as f32 / 60.0).max(0.0)
}
