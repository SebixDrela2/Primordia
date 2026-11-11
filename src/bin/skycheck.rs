#[path = "../creature/mod.rs"]
mod creature;
#[path = "../elements/mod.rs"]
mod elements;
#[path = "../rng.rs"]
mod rng;
#[path = "../world/mod.rs"]
mod world;

use elements::Elem;
use std::collections::HashMap;
use world::World;

fn main() {
    let seed = 12345u64;
    let mut w = World::new(1280, 720, seed);
    w.seed();

    let mut above: HashMap<String, u32> = HashMap::new();
    let mut lava_ys: Vec<i32> = Vec::new();
    for x in 0..w.w as i32 {
        let surf = w.surface_y(x);
        for y in 0..(surf - 2).max(0) {
            let e = w.get(x, y);
            if e != Elem::Empty {
                *above.entry(e.name().to_string()).or_insert(0) += 1;
                if e == Elem::Lava {
                    lava_ys.push(y);
                }
            }
        }
    }
    lava_ys.sort_unstable();
    if !lava_ys.is_empty() {
        println!(
            "lava in the sky: {} cells, y range {}..{} (world height {}, surface ~{})",
            lava_ys.len(),
            lava_ys[0],
            lava_ys[lava_ys.len() - 1],
            w.h,
            w.surface_y(640)
        );
    }
    let mut v: Vec<_> = above.into_iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("elements above the terrain: {:?}", &v[..v.len().min(20)]);

    // And after a short simulation.
    let mut cs = creature::CreatureSystem::new(seed);
    cs.populate(&w, 64);
    for _ in 0..600 {
        w.step();
        cs.step(&mut w);
    }
    let mut above2: HashMap<String, u32> = HashMap::new();
    for x in 0..w.w as i32 {
        let surf = w.surface_y(x);
        for y in 0..(surf - 2).max(0) {
            let e = w.get(x, y);
            if e != Elem::Empty {
                *above2.entry(e.name().to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut v2: Vec<_> = above2.into_iter().collect();
    v2.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("after 600 steps:        {:?}", &v2[..v2.len().min(20)]);
}
