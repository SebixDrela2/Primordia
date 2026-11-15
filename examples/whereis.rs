//! Say where the colony of the world is, so that the camera can be put
//! on it and the creatures can be looked at.

// The numbers are the ones the game is run with, so the casts are the
// ones it does on purpose.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use primordia::creature::{CreatureSystem, Faction};
use primordia::world::World;
use primordia::{H, W};

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4242);
    let seconds: f32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(120.0);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    let steps = (seconds * 60.0) as u32;
    for _ in 0..steps {
        world.step();
        creatures.step(&mut world);
    }

    let mut best = None;
    for c in &creatures.list {
        if c.faction != Faction::Player {
            continue;
        }
        if c.role == primordia::creature::Role::Queen {
            best = Some((c.x, c.y));
            break;
        }
    }
    match best {
        Some((x, y)) => println!("the queen is at {x:.0},{y:.0}"),
        None => println!("no queen found"),
    }
}
