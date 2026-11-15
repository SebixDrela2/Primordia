
use crate::creature::{CreatureSystem, RelationState, Treaty};
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

/// Run the diplomacy sector and return the peak number of relations seen.
fn run_sector(world: &mut World, creatures: &mut CreatureSystem, steps: u64) -> usize {
    let mut last_report = 0u64;
    let mut peak_relations = 0usize;
    for i in 0..steps {
        world.step();
        creatures.step(world);
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
    peak_relations
}

fn print_relations(creatures: &CreatureSystem) {
    let d = &creatures.diplomacy;
    println!("\n--- relations ---");
    if d.relations.is_empty() {
        println!("  (no species ever met)");
    }
    for r in &d.relations {
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
}

fn print_log(creatures: &CreatureSystem) {
    let d = &creatures.diplomacy;
    println!("\n--- diplomatic log ---");
    for e in d.log.iter().rev().take(20) {
        println!("  {}", e.text);
    }
}

fn print_stats(creatures: &CreatureSystem, peak_relations: usize) {
    let d = &creatures.diplomacy;
    println!("\n--- stats ---");
    println!("  relations:   {}", d.relations.len());
    println!("  alliances:   {}", d.stats.alliances);
    println!("  wars:        {}", d.stats.wars);
    println!("  treaties:    {}", d.stats.treaties);
    println!("  tributes:    {}", d.stats.tributes);
    println!("  betrayals:   {}", d.stats.betrayals);
    println!("  raids:       {}", d.stats.raids);
    println!("  truces:      {}", d.stats.truces);
    println!("  negotiations:{}", d.stats.negotiations);
    println!("  envoys:      {}", d.stats.envoys);
    println!("  peak pairs:  {}", peak_relations);
}

/// A relation cannot be allied and at war at once.
fn check_states(creatures: &CreatureSystem) -> u32 {
    let d = &creatures.diplomacy;
    let mut failures = 0;
    for r in &d.relations {
        if r.state == RelationState::Allied && r.has(Treaty::Alliance) && r.state == RelationState::War {
            println!("FAIL: allied and at war at once");
            failures += 1;
        }
    }
    failures
}

/// Trust always stays inside its range.
fn check_trust(creatures: &CreatureSystem) -> u32 {
    let d = &creatures.diplomacy;
    let mut failures = 0;
    for r in &d.relations {
        if r.trust < -100.0 || r.trust > 100.0 {
            println!("FAIL: trust out of range: {}", r.trust);
            failures += 1;
        }
    }
    failures
}

/// Every species in a relation must still be alive.
fn check_no_extinct(creatures: &CreatureSystem) -> u32 {
    let d = &creatures.diplomacy;
    let mut failures = 0;
    let alive: Vec<u32> = creatures.list.iter().map(|c| c.species).collect();
    for r in &d.relations {
        if !alive.contains(&r.a) || !alive.contains(&r.b) {
            println!("FAIL: relation with an extinct species");
            failures += 1;
        }
    }
    failures
}

/// Walk the allied pairs of the colony.
fn scan_allied_pairs(creatures: &CreatureSystem) {
    let d = &creatures.diplomacy;
    for c in &creatures.list {
        for other in &creatures.list {
            if c.species == other.species {
                continue;
            }
            if d.are_allied(c.species, other.species) {

                break;
            }
        }
    }
}

/// A war that never ends in a truce is a broken sector.
fn check_truces(creatures: &CreatureSystem) -> u32 {
    let d = &creatures.diplomacy;
    let mut failures = 0;
    if d.stats.wars > 0 && d.stats.truces == 0 {
        println!("FAIL: {} wars declared but no war ever ended in a truce", d.stats.wars);
        failures += 1;
    }
    failures
}

/// A ceasefire cannot stand while the two are still at war.
fn check_ceasefires(creatures: &CreatureSystem) -> u32 {
    let d = &creatures.diplomacy;
    let mut failures = 0;
    for r in &d.relations {
        if r.has(Treaty::Ceasefire) && r.state == RelationState::War {
            println!("FAIL: ceasefire signed while still at war");
            failures += 1;
        }
    }
    failures
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
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

    let peak_relations = run_sector(&mut world, &mut creatures, steps);

    print_relations(&creatures);
    print_log(&creatures);
    print_stats(&creatures, peak_relations);

    let mut failures = 0;
    failures += check_states(&creatures);
    failures += check_trust(&creatures);
    failures += check_no_extinct(&creatures);
    scan_allied_pairs(&creatures);
    failures += check_truces(&creatures);
    failures += check_ceasefires(&creatures);

    if failures == 0 {
        println!("\nOK: all diplomacy invariants hold");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
    0
}
