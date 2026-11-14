
use crate::creature::{CreatureSystem, Culture};
use crate::world::World;

const W: usize = 640;
const H: usize = 400;

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
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let Some(&(species, _)) = counts.first() else {
        println!("FAIL: no player species is left to carry anything");
        std::process::exit(1);
    };
    println!("  the lineage that carries on: species {}", species);

    let culture = Culture::Mining.bit() | Culture::Craft.bit();
    let discoveries = crate::creature::discovery_bits::COGNITION
        | crate::creature::discovery_bits::HUSBANDRY
        | crate::creature::discovery_bits::POTTERY;
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

    let mut world2 = World::new(W, H, 0x5EED_1234_ABCD_0002);
    world2.seed();
    let mut next = CreatureSystem::new(0x5EED_1234_ABCD_0002 ^ 0x0F0F_0F0F);
    next.populate_revival(&world2, 16, &genome);

    let carried_culture = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
        .fold(0u16, |acc, c| acc | c.culture);
    let carried_knowledge: u32 = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
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

    if carried_knowledge != 0 {
        println!("FAIL: the next world starts with knowledge it did not earn");
        failures += 1;
    }

    let revived_gen = next
        .list
        .iter()
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
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
        .filter(|c| c.faction != crate::creature::Faction::Zorg)
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
    0
}
