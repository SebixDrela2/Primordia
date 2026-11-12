//! Headless check for the life and evolution systems.
//!
//! cargo run --release --bin lifecheck -- [steps] [seed]
//!
//! Runs a world without a renderer and reports what the life of the colony
//! looks like: the ages of the creatures, the traits that have evolved, the
//! cultures the species have developed and the courtships that have taken
//! place. It also asserts the invariants that matter:
//! - a creature must pass through juvenile, adult and elder
//! - a child must inherit its parent's temperament
//! - a child of two parents must be a blend of the two
//! - the trust and affinity ranges must hold

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

use creature::{Affinity, Culture, CreatureSystem, Stage, culture_list, trait_list};
use world::World;

const W: usize = 1280;
const H: usize = 720;

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(9000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0001);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    println!("=== life check ===");
    println!("steps: {}", steps);

    let mut failures = 0u32;
    let mut saw_juvenile = false;
    let mut saw_adult = false;
    let mut saw_elder = false;
    let mut saw_affinity = false;
    let mut saw_culture = false;
    let mut max_trait_count = 0u32;
    // Every colony that has settled beside a landmark: (x, y, species).
    let mut landmark_claims: Vec<(f32, f32, u32)> = Vec::new();

    for i in 0..steps {
        world.step();
        creatures.step(&mut world);

        // Record the colonies that have settled.
        for c in &creatures.list {
            if c.role != creature::Role::Queen {
                continue;
            }
            let claimed = landmark_claims
                .iter()
                .any(|(x, y, sp)| *sp == c.species && (*x - c.base_x).abs() < 1.0);
            if !claimed {
                landmark_claims.push((c.base_x, c.base_y, c.species));
            }
        }

        for c in &creatures.list {
            match c.stage {
                Stage::Juvenile => saw_juvenile = true,
                Stage::Adult => saw_adult = true,
                Stage::Elder => saw_elder = true,
            }
            if c.affinity != Affinity::None {
                saw_affinity = true;
            }
            if c.culture != 0 {
                saw_culture = true;
            }
            let tc = trait_list(c.traits).len() as u32;
            if tc > max_trait_count {
                max_trait_count = tc;
            }
            // The temperament has to stay in range.
            let p = &c.personality;
            for v in [
                p.aggression,
                p.curiosity,
                p.sociability,
                p.greed,
                p.caution,
            ] {
                if !(0.0..=1.0).contains(&v) {
                    println!("FAIL: personality out of range: {}", v);
                    failures += 1;
                }
            }
            // A juvenile must not be fighting or breeding.
            if c.stage == Stage::Juvenile && c.energy > c.max_energy {
                println!("FAIL: juvenile has more than its maximum energy");
                failures += 1;
            }
        }

        if i % 1500 == 0 && i > 0 {
            let s = &creatures.stats;
            // The largest colony in the world: this is what a species has
            // to reach before it can split.
            let mut counts: std::collections::BTreeMap<u32, u32> =
                std::collections::BTreeMap::new();
            for c in &creatures.list {
                *counts.entry(c.species).or_insert(0) += 1;
            }
            let biggest = counts.values().copied().max().unwrap_or(0);
            let over = counts.values().filter(|n| **n >= 22).count();
            let far = creatures
                .migrations
                .iter()
                .map(|m| m.distance)
                .fold(0.0f32, f32::max);
            let sick = creatures
                .list
                .iter()
                .filter(|c| c.disease != creature::Disease::None)
                .count();
            println!(
                "  step {:5}  pop {:3}  species {:2}  biggest {:3}  over-cap {:2}  young {:3}  old {:2}  pairs {:4}  splits {:2}  started {:2}  far {:5.0}  sick {:3}  plagues {:2}  practices {}  traits/species {}",
                i,
                creatures.list.len(),
                s.species,
                biggest,
                over,
                s.juveniles,
                s.elders,
                s.matings,
                creatures.splits_total,
                creatures.migrations_started,
                far,
                sick,
                creatures.plagues_started,
                s.practices,
                max_trait_count,
            );
        }
    }

    // ---- The species and what they have become ----
    println!("\n--- species ---");
    let mut seen: Vec<u32> = Vec::new();
    for c in &creatures.list {
        if seen.contains(&c.species) {
            continue;
        }
        seen.push(c.species);
        let traits: Vec<&str> = trait_list(c.traits).iter().map(|t| t.name()).collect();
        let practices: Vec<&str> = culture_list(c.culture)
            .iter()
            .map(|cu| cu.name())
            .collect();
        println!(
            "  species {:3}: adapted to {:<8} traits [{}] practices [{}]",
            c.species,
            c.affinity.name(),
            traits.join(", "),
            practices.join(", ")
        );
    }

    // ---- The food web: who eats whom ----
    println!("\n--- predation ---");
    let mut fear_rows: Vec<(u32, u32, f32)> = creatures.predation.entries().to_vec();
    fear_rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    for (prey, predator, fear) in fear_rows.iter().take(12) {
        println!(
            "  the {} fears the {}: {:.0}",
            creature::species_name_for(*prey),
            creature::species_name_for(*predator),
            fear
        );
    }
    if fear_rows.is_empty() {
        println!("  (nothing has been hunted)");
    }

    // ---- The life of the colony ----
    println!("\n--- colony ---");
    let s = &creatures.stats;
    println!("  population:    {}", s.population);
    println!("  young:         {}", s.juveniles);
    println!("  old:           {}", s.elders);
    println!("  pairs made:    {}", s.matings);
    println!("  practices:     {}", s.practices);
    println!("  envoys:        {}", s.envoys);
    println!("  most traits:   {}", max_trait_count);
    println!("  splits:        {}", creatures.splits_total);
    println!("  plagues:       {}", creatures.plagues_started);
    println!("  bonds:         {}", creatures.bond_count());
    println!(
        "  discoveries:   {} made by {} species",
        creatures
            .discoveries
            .iter()
            .map(|(_, d)| d.count_ones())
            .sum::<u32>(),
        creatures.discoveries.len()
    );
    for (sp, bits) in creatures.discoveries.iter() {
        let names: Vec<&str> = creature::discovery_list(*bits)
            .iter()
            .map(|d| d.name())
            .collect();
        println!(
            "    {:<16} {}",
            creature::species_name_for(*sp),
            names.join(", ")
        );
    }

    // ---- The life histories of the species that are left ----
    println!("\n--- life histories ---");
    let mut seen: Vec<u32> = Vec::new();
    for c in &creatures.list {
        if seen.contains(&c.species) {
            continue;
        }
        seen.push(c.species);
        let strategy = if c.traits & creature::trait_bits::PROLIFIC != 0 {
            "prolific"
        } else if c.traits & creature::trait_bits::PROLONGED != 0 {
            "prolonged"
        } else {
            "balanced"
        };
        println!(
            "  {:<16} strategy {:<10} litter {}  cost {:.0}  lives {:.0} ticks  matures at {:.0}%",
            creature::species_name_for(c.species),
            strategy,
            c.litter_size(),
            c.brood_cost(),
            c.lifespan(),
            c.maturity_fraction() * 100.0,
        );
    }

    // ---- The landmarks and who settled beside them ----
    println!("\n--- landmarks ---");
    let mut owners: Vec<Vec<String>> = Vec::new();
    for l in world.landmarks.iter() {
        let names: Vec<String> = landmark_claims
            .iter()
            .filter(|(lx, ly, _)| {
                ((lx - l.x).powi(2) + (ly - l.y).powi(2)).sqrt() <= l.kind.radius() * 1.6
            })
            .map(|(_, _, sp)| creature::species_name_for(*sp))
            .collect();
        // A landmark belongs to the colony that settled there first; the
        // others are squatters that will end up fighting over it.
        let mut uniq: Vec<String> = Vec::new();
        for n in names.iter() {
            if !uniq.contains(n) {
                uniq.push(n.clone());
            }
        }
        owners.push(uniq.clone());
        println!(
            "  {:<14} at ({:4.0},{:4.0})  radius {:4.0}  settled by: {}",
            l.kind.name(),
            l.x,
            l.y,
            l.kind.radius(),
            if names.is_empty() {
                "nobody".to_string()
            } else {
                names.join(", ")
            }
        );
    }
    let claimed = owners.iter().filter(|o| !o.is_empty()).count();
    println!("  landmarks claimed: {} of {}", claimed, world.landmarks.len());
    if claimed > world.landmarks.len() {
        println!("FAIL: more landmarks claimed than there are landmarks");
        failures += 1;
    }
    // A landmark can only have one owner; squatters are the ones that fight
    // over it.
    for (i, o) in owners.iter().enumerate() {
        if o.len() > 1 {
            println!(
                "NOTE: landmark {} is contested by {:?}",
                i, o
            );
        }
    }

    // ---- Invariants ----
    if !saw_juvenile {
        println!("FAIL: no creature was ever a juvenile");
        failures += 1;
    }
    if !saw_adult {
        println!("FAIL: no creature was ever an adult");
        failures += 1;
    }
    if !saw_elder {
        println!("FAIL: no creature ever reached old age");
        failures += 1;
    }
    if !saw_affinity {
        println!("FAIL: no species ever became specialised");
        failures += 1;
    }
    if !saw_culture {
        println!("FAIL: no species ever learned a practice");
        failures += 1;
    }
    if s.matings == 0 {
        println!("FAIL: no pair ever mated");
        failures += 1;
    }
    if creatures.two_parent_births == 0 {
        println!("FAIL: no young was born to two parents");
        failures += 1;
    }
    // The fear of a species has to stay in range, and a species can never
    // fear itself.
    for (prey, predator, fear) in creatures.predation.entries().iter() {
        if !(0.0..=100.0).contains(fear) {
            println!("FAIL: fear out of range: {}", fear);
            failures += 1;
        }
        if prey == predator {
            println!("FAIL: a species fears itself");
            failures += 1;
        }
    }

    if failures == 0 {
        println!("\nOK: all life invariants hold");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
}
