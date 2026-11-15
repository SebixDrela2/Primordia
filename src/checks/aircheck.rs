
use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::{Landmark, World};

const W: usize = 1280;
const H: usize = 720;

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
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

        let a = world.fresh_air(x, surf - 3, 3);
        surface_air += a;
        surface_samples += 1;

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
    0
}

fn fire_lifetime(with_air: bool) -> u32 {

    let mut w = World::new(64, 100, 77);
    w.seed();

    let (_x0, _x1) = (30i32, 33i32);
    let (x0, x1) = (24i32, 40i32);
    let (y0, y1) = (70i32, 78i32);

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
    for y in (y0 - 1)..=y1 {
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

    for x in x0 + 4..x0 + 12 {
        w.set_xy(x, y1 - 2, Elem::Fire);
    }

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

fn stale_cost(fresh: bool) -> f32 {

    let mut w = World::new(64, 100, 99);
    w.seed();

    let (x0, x1) = (30i32, 33i32);
    let (y0, y1) = (70i32, 73i32);

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
    for y in (y0 - 1)..=y1 {
        w.set_xy(x0 - 1, y, Elem::Stone);
        w.set_xy(x1, y, Elem::Stone);
    }
    for x in x0 - 1..=x1 {
        w.set_xy(x, y0 - 1, Elem::Stone);
        w.set_xy(x, y1, Elem::Stone);
    }
    if fresh {

        for x in x0..x1 {
            w.set_xy(x, y0, Elem::Oxygen);
        }
    }

    w.mine_budget = 0;
    w.daylight = 1.0;

    let mut creatures = CreatureSystem::new(4242);
    creatures.populate(&w, 8);

    for c in &mut creatures.list {
        c.tool = crate::creature::Tool::None;
        c.tool_durability = 0;
        c.known_spells = 0;
        c.active_spell = crate::creature::Spell::None;
    }

    for c in &mut creatures.list {
        c.x = 30.0;
        c.y = (y1 - 1) as f32;
    }

    for _ in 0..20 {
        creatures.step(&mut w);
    }

    for c in &mut creatures.list {
        c.energy = c.max_energy * 0.6;
        c.stamina = 0.0;
    }
    let before: f32 = creatures.list.iter().map(|c| c.stamina).sum();
    for _ in 0..60 {

        for c in &mut creatures.list {
            c.x = 30.0;
            c.y = (y1 - 1) as f32;

            c.health = c.max_health();
            c.energy = c.max_energy * 0.6;
            c.disease = crate::creature::Disease::None;

            for i in 0..c.immunity.len() {
                c.immunity[i] = 2;
            }
            c.resting = true;
        }
        if fresh {
            for x in x0..x1 {
                if w.get(x, y0) == Elem::Empty {
                    w.set_xy(x, y0, Elem::Oxygen);
                }
            }
        }
        creatures.step(&mut w);

        for c in &mut creatures.list {
            c.stamina = c.stamina.min(c.max_stamina * 0.5);
        }
    }
    let after: f32 = creatures.list.iter().map(|c| c.stamina).sum();

    ((after - before) / creatures.list.len().max(1) as f32 / 60.0).max(0.0)
}
