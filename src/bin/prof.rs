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
    for _ in 0..900 { w.step(); cs.step(&mut w); }
    let pop = cs.list.len();
    let n = 1200;
    let t = Instant::now();
    for _ in 0..n { w.step(); }
    let world_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t = Instant::now();
    for _ in 0..n { cs.step(&mut w); }
    let creature_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let mut buf = vec![0u8; 1280 * 720 * 4];
    let mut renderer = render::Renderer::new();
    // Warm up so the measurement does not include the first-call costs.
    for i in 0..10 {
        renderer.render_world(&w, &cs, &mut buf, i as f32 * 0.016, 0.25, 0, w.weather);
    }
    let t = Instant::now();
    for i in 0..60 { renderer.render_world(&w, &cs, &mut buf, i as f32 * 0.016, 0.25, 0, w.weather); }
    let render_ms = t.elapsed().as_secs_f64() * 1000.0 / 60.0;
    println!(
        "pop {} | world {:.3} ms | creature {:.3} ms | render {:.3} ms",
        pop, world_ms, creature_ms, render_ms
    );
}
