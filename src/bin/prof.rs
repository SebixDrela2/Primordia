//! Where the time goes.
//!
//! cargo run --release --bin prof
//!
//! Prints the per-step cost of the world, the creatures and the renderer.
//! The renderer is measured on the region that is actually on screen, at
//! the zoom the game usually runs at, because that is the number that has
//! to fit inside a frame.

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
use std::time::Instant;
use world::World;

fn main() {
    let seed = 4242u64;
    let mut w = World::new(1280, 720, seed);
    w.seed();
    let mut cs = CreatureSystem::new(seed);
    cs.populate(&w, 64);
    for _ in 0..900 {
        w.step();
        cs.step(&mut w);
    }
    let pop = cs.list.len();

    let n = 1200;
    let t = Instant::now();
    for _ in 0..n {
        w.step();
    }
    let world_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t = Instant::now();
    for _ in 0..n {
        cs.step(&mut w);
    }
    let creature_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;

    // ---- The renderer ----
    //
    // The renderer only ever paints the part of the world that is on
    // screen, so that is what is measured: the region of a 480x270 view
    // at the zoom the game runs at, and the whole world for comparison.
    let mut renderer = render::Renderer::new();
    let mut buf = vec![0u8; 480 * 270 * 4];
    let frames = 400;
    let t = Instant::now();
    for i in 0..frames {
        renderer.render_region(
            &w,
            &cs,
            &mut buf,
            400,
            300,
            480,
            270,
            i as f32 * 0.016,
            0.3,
            0,
            world::Weather::Clear,
        );
    }
    let region_ms = t.elapsed().as_secs_f64() * 1000.0 / frames as f64;

    let mut full = vec![0u8; 1280 * 720 * 4];
    let t = Instant::now();
    for i in 0..40 {
        renderer.render_world(&w, &cs, &mut full, i as f32 * 0.016, 0.3, 0, world::Weather::Clear);
    }
    let world_render_ms = t.elapsed().as_secs_f64() * 1000.0 / 40 as f64;

    println!(
        "pop {} | world {:.3} ms | creature {:.3} ms | render region {:.3} ms | render world {:.3} ms",
        pop, world_ms, creature_ms, region_ms, world_render_ms
    );
    let frame_ms = world_ms + creature_ms + region_ms;
    println!(
        "a frame at 60 fps has {:.2} ms: the simulation and the region take {:.3} ms of it",
        1000.0 / 60.0,
        frame_ms
    );
}
