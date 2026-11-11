//! Diagnostic: find elements that end up in the sky when they should be
//! underground, and elements that visibly float with nothing beneath them.
//!
//! cargo run --release --bin spawncheck -- [frames]

#[path = "../creature/mod.rs"]
mod creature;
#[path = "../elements/mod.rs"]
mod elements;
#[path = "../rng.rs"]
mod rng;
#[path = "../world/mod.rs"]
mod world;

use elements::Elem;
use world::World;

/// Elements that must never appear above the terrain surface.
const UNDERGROUND_ONLY: &[Elem] = &[
    Elem::Lava,
    Elem::Magma,
    Elem::Acid,
    Elem::Mercury,
    Elem::Blood,
    Elem::Poison,
    Elem::Alcohol,
    Elem::Honey,
    Elem::Tar,
    Elem::Stone,
    Elem::Metal,
    Elem::Copper,
    Elem::Iron,
    Elem::Steel,
    Elem::Gold,
    Elem::Silver,
    Elem::Glass,
    Elem::Crystal,
    Elem::Gem,
    Elem::Diamond,
    Elem::Granite,
    Elem::Rust,
    Elem::Sulfur,
    Elem::Gunpowder,
    Elem::Brick,
    Elem::Concrete,
    Elem::Iron,
];

fn main() {
    let frames: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2400);
    let seed = 12345u64;
    let mut w = World::new(1280, 720, seed);
    w.seed();

    let mut reported: std::collections::HashMap<&'static str, (u32, u64, i32)> =
        std::collections::HashMap::new();

    for frame in 1..=frames {
        w.step();
        if frame % 15 != 0 {
            continue;
        }
        for x in 0..w.w as i32 {
            // The analytic terrain height is the reference: everything above
            // it is open sky.
            let surf = w.surface_y(x);
            // Give a little slack for foliage and tree canopies.
            let top = (surf - 8).max(0);
            for y in 0..top {
                let e = w.get(x, y);
                if UNDERGROUND_ONLY.contains(&e) {
                    let entry = reported.entry(e.name()).or_insert((0, frame, i32::MAX));
                    entry.0 += 1;
                    if frame < entry.1 {
                        entry.1 = frame;
                    }
                    if y < entry.2 {
                        entry.2 = y;
                    }
                }
            }
        }
    }

    if reported.is_empty() {
        println!("OK: no underground-only elements above the terrain after {frames} frames");
    } else {
        let mut v: Vec<_> = reported.iter().collect();
        v.sort_by_key(|(_, c)| std::cmp::Reverse(c.0));
        println!("UNDERGROUND ELEMENTS FOUND ABOVE THE TERRAIN:");
        for (name, (count, first, top)) in v {
            println!(
                "  {name:>12}: {count:>7} sightings (first at frame {first}, topmost row {top})"
            );
        }
    }
}
