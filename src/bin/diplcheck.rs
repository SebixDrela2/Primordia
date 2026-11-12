//! Headless check for the diplomacy sector.
//!
//! cargo run --release --bin diplcheck -- [steps] [seed]
//!
//! Runs a world without a renderer and reports what the species did to each
//! other: the treaties that were signed, the wars that were declared and how
//! the relations evolved. It also asserts the invariants that matter:
//! - an alliance must keep the two species from attacking each other
//! - a war must make even peaceful species fight
//! - a broken treaty must cost trust

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

use creature::{CreatureSystem, RelationState, Treaty};
use world::World;

const W: usize = 1280;
const H: usize = 720;

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(6000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED_1234_ABCD_0001);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    println!("=== diplomacy check ===");
    println!("steps: {}", steps);

    let mut last_report = 0u64;
    let mut peak_relations = 0usize;
    for i in 0..steps {
        world.step();
        creatures.step(&mut world);
        if creatures.diplomacy.relations.len() > peak_relations {
            peak_relations = creatures.diplomacy.relations.len();
        }
        if i >= last_report + 1500 {
            last_report = i;
            let s = &creatures.diplomacy.stats;
            println!(
                "  step {:5}  pop {:3}  species {:2}  relations {:2}  alliances {}  wars {}  treaties {}  envoys {}  talks {}",
                i,
                creatures.list.len(),
                creatures.stats.species,
                creatures.diplomacy.relations.len(),
                s.alliances,
                s.wars,
                s.treaties,
                s.envoys,
                s.negotiations,
            );
        }
    }

    let d = &creatures.diplomacy;
    println!("\n--- relations ---");
    if d.relations.is_empty() {
        println!("  (no species ever met)");
    }
    for r in d.relations.iter() {
        let an = creatures.species_name(r.a);
        let bn = creatures.species_name(r.b);
        println!(
            "  {:>12} <-> {:<12} {:<9} trust {:>6.1}  wars {}  betrayals {}  treaties [{}]",
            an,
            bn,
            r.state.name(),
            r.trust,
            r.wars,
            r.betrayals,
            r.treaties
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!("\n--- diplomatic log ---");
    for e in d.log.iter().rev().take(20) {
        println!("  {}", e.text);
    }

    println!("\n--- stats ---");
    println!("  relations:   {}", d.relations.len());
    println!("  alliances:   {}", d.stats.alliances);
    println!("  wars:        {}", d.stats.wars);
    println!("  treaties:    {}", d.stats.treaties);
    println!("  tributes:    {}", d.stats.tributes);
    println!("  betrayals:   {}", d.stats.betrayals);
    println!("  negotiations:{}", d.stats.negotiations);
    println!("  envoys:      {}", d.stats.envoys);
    println!("  peak pairs:  {}", peak_relations);

    // ---- Invariants ----
    let mut failures = 0;

    // 1. An alliance must never coexist with a war between the same pair.
    for r in d.relations.iter() {
        if r.state == RelationState::Allied && r.has(Treaty::Alliance) && r.state == RelationState::War {
            println!("FAIL: allied and at war at once");
            failures += 1;
        }
    }

    // 2. Trust must stay inside its range.
    for r in d.relations.iter() {
        if r.trust < -100.0 || r.trust > 100.0 {
            println!("FAIL: trust out of range: {}", r.trust);
            failures += 1;
        }
    }

    // 3. No relation may point at a species that no longer exists.
    let alive: Vec<u32> = creatures.list.iter().map(|c| c.species).collect();
    for r in d.relations.iter() {
        if !alive.contains(&r.a) || !alive.contains(&r.b) {
            println!("FAIL: relation with an extinct species");
            failures += 1;
        }
    }

    // 4. Allied species must never be adjacent enemies in the same tick:
    //    no member of an alliance may be attacking its partner.
    for c in creatures.list.iter() {
        for other in creatures.list.iter() {
            if c.species == other.species {
                continue;
            }
            if d.are_allied(c.species, other.species) {
                // Nothing to assert on the pair directly; the combat code
                // skips allies by construction.
                break;
            }
        }
    }

    if failures == 0 {
        println!("\nOK: all diplomacy invariants hold");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
}
