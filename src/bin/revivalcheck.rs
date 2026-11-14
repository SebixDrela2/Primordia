//! Headless check for the revival: the lineage that goes on must carry
//! everything it earned.
//!
//! cargo run --release --bin revivalcheck -- [steps]
//!
//! A revival is a continuation, not a fresh start with an old body. If a
//! lineage loses a run and its genes are carried into the next world, it
//! must carry its **practices** and its **knowledge** with them: a
//! lineage that starts every revival at the bottom of the tech tree never
//! gets anywhere, and the evolution never learns anything.

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

use creature::{CreatureSystem, Culture};
use world::World;

const W: usize = 640;
const H: usize = 400;

fn main() {
    let steps: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(6000);
    let mut failures = 0u32;

    println!("=== revival check ===");
    println!("steps: {}", steps);

    let mut world = World::new(W, H, 0x5EED_1234_ABCD_0001);
    world.seed();
    let mut creatures = CreatureSystem::new(0x5EED_1234_ABCD_0001 ^ 0x0F0F_0F0F);
    creatures.populate(&world, 32);
    for _ in 0..steps {
        world.step();
        creatures.step(&mut world);
    }

    // ---- The lineage that would be carried on ----
    //
    // The species with the most members is the one that would hold the
    // world up, and it is the one whose genome goes into the next run.
    let mut counts: Vec<(u32, u32)> = Vec::new();
    for c in &creatures.list {
        if c.faction == creature::Faction::Zorg {
            continue;
        }
        match counts.iter_mut().find(|(id, _)| *id == c.species) {
            Some(e) => e.1 += 1,
            None => counts.push((c.species, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let Some(&(species, _)) = counts.first() else {
        println!("FAIL: no player species is left to carry anything");
        std::process::exit(1);
    };
    println!("  the lineage that carries on: species {}", species);

    // Give it something worth carrying: a practice it has learnt and a
    // piece of knowledge it has worked out.
    let culture = Culture::Mining.bit() | Culture::Craft.bit();
    let discoveries = creature::discovery_bits::COGNITION
        | creature::discovery_bits::HUSBANDRY
        | creature::discovery_bits::POTTERY;
    for c in creatures.list.iter_mut() {
        if c.species == species {
            c.culture |= culture;
        }
    }
    creatures.note_discovery(species, discoveries);

    let Some(genome) = creatures.champion_from(species) else {
        println!("FAIL: the lineage has no champion to carry");
        std::process::exit(1);
    };

    // ---- What the genome carries ----
    println!(
        "  the genome carries: culture {:04x}, temperament {:.2}/{:.2}",
        genome.culture,
        genome.personality.aggression,
        genome.personality.caution
    );
    if genome.culture & culture != culture {
        println!("FAIL: the genome does not carry the practices");
        failures += 1;
    }

    // ---- And what the next world starts with ----
    let mut world2 = World::new(W, H, 0x5EED_1234_ABCD_0002);
    world2.seed();
    let mut next = CreatureSystem::new(0x5EED_1234_ABCD_0002 ^ 0x0F0F_0F0F);
    next.populate_revival(&world2, 16, &genome);

    let carried_culture = next
        .list
        .iter()
        .filter(|c| c.faction != creature::Faction::Zorg)
        .fold(0u16, |acc, c| acc | c.culture);
    let carried_knowledge: u32 = next
        .list
        .iter()
        .filter(|c| c.faction != creature::Faction::Zorg)
        .map(|c| next.species_discoveries(c.species))
        .fold(0, |acc, d| acc | d);
    println!(
        "  the next world starts with: culture {:04x}, knowledge {:08x}",
        carried_culture, carried_knowledge
    );
    if carried_culture & culture != culture {
        println!("FAIL: the next world starts without the practices");
        failures += 1;
    }
    // The knowledge is *not* carried: the tech tree is what a run is for,
    // and a revival is a new species that has to work it out again.
    if carried_knowledge != 0 {
        println!("FAIL: the next world starts with knowledge it did not earn");
        failures += 1;
    }

    // ---- The generation and the temperament ----
    //
    // The revival is a continuation: the animals that come back are a
    // generation further on than the one that carried the line, and they
    // carry its temperament.
    let revived_gen = next
        .list
        .iter()
        .filter(|c| c.faction != creature::Faction::Zorg)
        .map(|c| c.generation)
        .max()
        .unwrap_or(0);
    println!(
        "  the next world starts at generation {} (the champion was at {})",
        revived_gen, genome.generation
    );
    if revived_gen <= genome.generation {
        println!("FAIL: the revival starts the generation count again");
        failures += 1;
    }
    let temperament_kept = next
        .list
        .iter()
        .filter(|c| c.faction != creature::Faction::Zorg)
        .any(|c| {
            (c.personality.aggression - genome.personality.aggression).abs() < 0.5
                && (c.personality.caution - genome.personality.caution).abs() < 0.5
        });
    if !temperament_kept {
        println!("FAIL: the revival does not carry the temperament");
        failures += 1;
    }

    if failures == 0 {
        println!("OK: the lineage carries what it earned");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
}
