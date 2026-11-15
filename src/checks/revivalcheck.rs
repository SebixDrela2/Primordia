
use crate::creature::{CreatureSystem, Culture, RevivalGenome};
use crate::world::World;

const W: usize = 640;
const H: usize = 400;

/// The player species with the most members is the one that carries on.
fn pick_lineage(creatures: &CreatureSystem) -> Option<u32> {
    let mut counts: Vec<(u32, u32)> = Vec::new();
    for c in &creatures.list {
        if c.faction == crate::creature::Faction::Zorg {
            continue;
        }
        match counts.iter_mut().find(|(id, _)| *id == c.species) {
            Some(e) => e.1 += 1,
            None => counts.push((c.species, 1)),
        }
    }
    counts.sort_by_key(|b| std::cmp::Reverse(b.1));
    counts.first().map(|&(id, _)| id)
}

/// Give the lineage a few practices and a discovery, then take its champion.
fn take_champion(creatures: &mut CreatureSystem, species: u32) -> Option<RevivalGenome> {
    let culture = Culture::Mining.bit() | Culture::Craft.bit();
    let discoveries = crate::creature::discovery_bits::COGNITION
        | crate::creature::discovery_bits::HUSBANDRY
        | crate::creature::discovery_bits::POTTERY;
    for c in &mut creatures.list {
        if c.species == species {
            c.culture |= culture;
        }
    }
    creatures.note_discovery(species, discoveries);
    creatures.champion_from(species)
}

/// A fresh world seeded from the champion.
fn start_next_world(genome: &RevivalGenome) -> CreatureSystem {
    let mut world2 = World::new(W, H, 0x5EED_1234_ABCD_0002);
    world2.seed();
    let mut next = CreatureSystem::new(0x5EED_1234_ABCD_0002 ^ 0x0F0F_0F0F);
    next.populate_revival(&world2, 16, genome);
    next
}

/// What the next world starts with.
struct Carried {
    culture: u16,
    knowledge: u32,
    generation: u32,
    temperament_kept: bool,
}

fn read_carried(next: &CreatureSystem, genome: &RevivalGenome) -> Carried {
    let culture = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
        .fold(0u16, |acc, c| acc | c.culture);
    let knowledge = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
        .map(|c| next.species_discoveries(c.species))
        .fold(0, |acc, d| acc | d);
    let generation = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
        .map(|c| c.generation)
        .max()
        .unwrap_or(0);
    let temperament_kept = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
        .any(|c| {
            (c.personality.aggression - genome.personality.aggression).abs() < 0.5
                && (c.personality.caution - genome.personality.caution).abs() < 0.5
        });
    Carried {
        culture,
        knowledge,
        generation,
        temperament_kept,
    }
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let steps: u64 = args.first()
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

    let Some(species) = pick_lineage(&creatures) else {
        println!("FAIL: no player species is left to carry anything");
        std::process::exit(1);
    };
    println!("  the lineage that carries on: species {}", species);

    let Some(genome) = take_champion(&mut creatures, species) else {
        println!("FAIL: the lineage has no champion to carry");
        std::process::exit(1);
    };

    println!(
        "  the genome carries: culture {:04x}, temperament {:.2}/{:.2}",
        genome.culture,
        genome.personality.aggression,
        genome.personality.caution
    );
    let culture = Culture::Mining.bit() | Culture::Craft.bit();
    if genome.culture & culture != culture {
        println!("FAIL: the genome does not carry the practices");
        failures += 1;
    }

    let next = start_next_world(&genome);
    let carried = read_carried(&next, &genome);
    println!(
        "  the next world starts with: culture {:04x}, knowledge {:08x}",
        carried.culture, carried.knowledge
    );
    if carried.culture & culture != culture {
        println!("FAIL: the next world starts without the practices");
        failures += 1;
    }

    if carried.knowledge != 0 {
        println!("FAIL: the next world starts with knowledge it did not earn");
        failures += 1;
    }

    println!(
        "  the next world starts at generation {} (the champion was at {})",
        carried.generation, genome.generation
    );
    if carried.generation <= genome.generation {
        println!("FAIL: the revival starts the generation count again");
        failures += 1;
    }
    if !carried.temperament_kept {
        println!("FAIL: the revival does not carry the temperament");
        failures += 1;
    }

    if failures == 0 {
        println!("OK: the lineage carries what it earned");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
    0
}
