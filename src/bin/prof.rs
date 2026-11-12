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

    let mut r = render::Renderer::new();
    let mut buf = vec![0u8; 1280 * 720 * 4];

    let t = Instant::now();
    for i in 0..20 {
        r.render_region(
            &w, &cs, &mut buf, 0, 0, 1280, 470, i as f32 * 0.016, 0.3, 0, world::Weather::Clear,
        );
    }
    let sky = t.elapsed().as_secs_f64() * 1000.0 / 20.0;

    let t = Instant::now();
    for i in 0..20 {
        r.render_region(
            &w, &cs, &mut buf, 0, 470, 1280, 250, i as f32 * 0.016, 0.3, 0, world::Weather::Clear,
        );
    }
    let ground = t.elapsed().as_secs_f64() * 1000.0 / 20.0;

    println!("sky {:.3} ms   ground {:.3} ms", sky, ground);
}
