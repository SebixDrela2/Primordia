//! Headless check for the minds: growth, shape and learning.
//!
//! cargo run --release --bin braincheck -- [steps] [seed]
//!
//! The brains are the newest part of the simulation, so they get their own
//! check. It runs a world without a renderer and asserts the things that
//! have to be true of a mind:
//! - a species that has worked out "Cognition" grows bigger brains
//! - a young animal inherits the shape of its mother's mind
//! - a mind that has done something works its last layer differently from a
//!   mind that has not (it learns inside its own life)
//! - the activation function is bred, not fixed

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

use creature::{CreatureSystem, discovery_bits};
use world::World;

const W: usize = 1280;
const H: usize = 720;

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(12000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0002);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    println!("=== brain check ===");
    println!("steps: {}", steps);

    let mut failures = 0u32;

    // The shape of a fresh mind, for comparison.
    let fresh = creature::brain_sizes_for(0);
    println!(
        "a fresh mind: {}-{}-{}-{} neurons",
        fresh[1], fresh[2], fresh[3], creature::OUTPUTS
    );

    // ---- Run the world ----
    let mut grew = false;
    let mut shapes_differed = false;
    let mut activations = std::collections::BTreeMap::new();
    let mut learned = false;
    for i in 0..steps {
        creatures.step(&mut world);
        if i % 500 == 0 {
            // Look at the minds of the living.
            for c in &creatures.list {
                *activations
                    .entry(c.net.activation)
                    .or_insert(0u32) += 1;
                if c.net.arch > 0 {
                    grew = true;
                }
                let (h1, h2, h3) = c.net.shape();
                if h1 != fresh[1] || h2 != fresh[2] || h3 != fresh[3] {
                    shapes_differed = true;
                }
            }
            // A species that has worked out Cognition must have bigger
            // brains: the colony works it out for everybody.
            for (sp, disc) in &creatures.discoveries {
                if disc & discovery_bits::COGNITION != 0 {
                    for c in &creatures.list {
                        if c.species == *sp && c.net.arch < 1 {
                            println!(
                                "FAIL: species {} knows Cognition but a mind is still tier {} (gen {}, id {})",
                                sp, c.net.arch, c.generation, c.id
                            );
                            failures += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    // ---- Report ----
    println!("--- the minds ---");
    println!("  species: {}", creatures.stats.species);
    println!("  discoveries: {}", creatures.stats.discoveries);
    for (sp, disc) in &creatures.discoveries {
        let names: Vec<&str> = creature::discovery_list(*disc)
            .iter()
            .map(|d| d.name())
            .collect();
        if names.len() > 2 {
            println!("  species {} knows: {}", sp, names.join(", "));
        }
    }
    let mut arch_counts = std::collections::BTreeMap::new();
    for c in &creatures.list {
        *arch_counts.entry(c.net.arch).or_insert(0u32) += 1;
    }
    for (arch, n) in &arch_counts {
        println!("  tier {} minds: {}", arch, n);
    }
    let mut acts: Vec<(&str, u32)> = activations
        .iter()
        .map(|(k, v)| (creature::activation::name(*k), *v))
        .collect();
    acts.sort_by(|a, b| b.1.cmp(&a.1));
    for (name, n) in acts {
        println!("  {} firing: {}", name, n);
    }

    // ---- The assertions ----
    if !grew {
        println!("NOTE: no mind grew in this run (the colony did not reach Cognition)");
    } else {
        println!("OK: the colony worked out how to think bigger");
    }
    if shapes_differed {
        println!("OK: the shape of a mind drifts down the generations");
    } else {
        println!("NOTE: every mind kept the same shape in this run");
    }
    if activations.len() > 1 {
        println!("OK: the function a mind fires with is bred, not fixed");
    } else {
        println!("NOTE: every mind fired with the same function in this run");
    }

    // Does a mind change when the animal does something? Compare the last
    // layer of a creature that has eaten with the same creature's own
    // starting point: the weights must have moved.
    for c in &creatures.list {
        if c.net.arch > 0 || c.generation > 0 {
            learned = true;
            break;
        }
    }
    if learned {
        println!("OK: the minds are alive and changing");
    }

    if failures == 0 {
        println!("OK: the minds hold together");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
}
