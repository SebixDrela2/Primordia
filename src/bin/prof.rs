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

    println!(
        "pop {} | world {:.3} ms | creature {:.3} ms",
        pop, world_ms, creature_ms
    );
}
