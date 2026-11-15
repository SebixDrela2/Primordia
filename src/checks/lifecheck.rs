
use crate::creature::{Affinity, CreatureSystem, Stage, culture_list, trait_list};
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

/// The life stages the colony was seen to pass through. A colony is
/// seen at more than one stage over the run, so the stages are held as
/// a set of flags rather than as one state.
#[derive(Clone, Copy, Default)]
struct StagesSeen(u8);

impl StagesSeen {
    #[must_use]
    fn saw(self, stage: Stage) -> bool {
        self.0 & (1u8 << (stage as u8)) != 0
    }

    fn mark(&mut self, stage: Stage) {
        self.0 |= 1u8 << (stage as u8);
    }
}

/// What the colony was seen to go through.
#[derive(Default)]
struct Seen {
    stages: StagesSeen,
    affinity: bool,
    culture: bool,
    max_trait_count: u32,
}

/// The ZORG cave has to exist, be deep, and hold the founding ZORGs.
fn check_zorg_cave(world: &World, creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    println!("--- the ZORG cave ---");
    let cave = world.zorg_cave_center();
    if let Some((cx, cy)) = cave {
        let surface = world.surface_y(cx);
        println!(
            "  the cave is at ({}, {}), {} below the surface",
            cx, cy,
            cy - surface
        );
        if cy <= surface + 8 {
            println!("FAIL: the ZORG cave is not below the surface");
            failures += 1;
        }
        if cy >= world.h as i32 - 12 {
            println!("FAIL: the ZORG cave is in the bedrock");
            failures += 1;
        }

        let mut outside = 0;
        for c in &creatures.list {
            if c.faction != crate::creature::Faction::Zorg {
                continue;
            }
            if !world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32) {
                outside += 1;
            }
        }
        if outside > 0 {
            println!("FAIL: {} ZORGs did not spawn in the cave", outside);
            failures += 1;
        } else {
            println!("  all founding ZORGs spawned in it");
        }
    } else {
        println!("FAIL: the world has no ZORG cave");
        failures += 1;
    }
    failures
}

/// A queen claims the landmark her colony sits beside.
fn claim_landmarks(creatures: &CreatureSystem, landmark_claims: &mut Vec<(f32, f32, u32)>) {
    for c in &creatures.list {
        if c.role != crate::creature::Role::Queen {
            continue;
        }
        let claimed = landmark_claims
            .iter()
            .any(|(x, _y, sp)| *sp == c.species && (*x - c.base_x).abs() < 1.0);
        if !claimed {
            landmark_claims.push((c.base_x, c.base_y, c.species));
        }
    }
}

/// Look at every creature: its stage, its practices and its temperament.
fn scan_creatures(creatures: &CreatureSystem, seen: &mut Seen) -> u32 {
    let mut failures = 0;
    for c in &creatures.list {
        match c.stage {
            Stage::Juvenile | Stage::Adult | Stage::Elder => seen.stages.mark(c.stage),
        }
        if c.affinity != Affinity::None {
            seen.affinity = true;
        }
        if c.culture != 0 {
            seen.culture = true;
        }
        let tc = trait_list(c.traits).len() as u32;
        if tc > seen.max_trait_count {
            seen.max_trait_count = tc;
        }

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

        if c.stage == Stage::Juvenile && c.energy > c.max_energy {
            println!("FAIL: juvenile has more than its maximum energy");
            failures += 1;
        }
    }
    failures
}

fn report_step(creatures: &CreatureSystem, max_trait_count: u32, i: u64) {
    let s = &creatures.stats;

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
        .filter(|c| c.disease != crate::creature::Disease::None)
        .count();
    println!(
        "  step {:5}  pop {:3}  species {:2}  zorgs {:3}  biggest {:3}  over-cap {:2}  young {:3}  old {:2}  pairs {:4}  splits {:2}  started {:2}  far {:5.0}  sick {:3}  plagues {:2}  practices {}  traits/species {}",
        i,
        creatures.list.len(),
        s.species,
        creatures.zorg_count(),
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

/// Step the world and the creatures, and watch the colony grow.
fn run_life(
    world: &mut World,
    creatures: &mut CreatureSystem,
    steps: u64,
) -> (Seen, Vec<(f32, f32, u32)>, u32) {
    let mut seen = Seen::default();
    let mut failures = 0;
    let mut landmark_claims: Vec<(f32, f32, u32)> = Vec::new();

    for i in 0..steps {
        world.step();
        creatures.step(world);

        claim_landmarks(creatures, &mut landmark_claims);
        failures += scan_creatures(creatures, &mut seen);

        if i.is_multiple_of(1500) && i > 0 {
            report_step(creatures, seen.max_trait_count, i);
        }
    }
    (seen, landmark_claims, failures)
}

fn print_species(creatures: &CreatureSystem) {
    println!("\n--- species ---");
    let mut printed: Vec<u32> = Vec::new();
    for c in &creatures.list {
        if printed.contains(&c.species) {
            continue;
        }
        printed.push(c.species);
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
}

/// The deep: the ZORGs are one species, they ally with nobody, and they
/// are not a player species.
fn check_deep(creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    println!("\n--- the deep ---");
    let zorgs = creatures.list.iter().filter(|c| c.faction == crate::creature::Faction::Zorg).count();
    println!("  ZORGs alive: {}", zorgs);
    let zorg_species: Vec<u32> = creatures
        .list
        .iter()
        .filter(|c| c.faction == crate::creature::Faction::Zorg)
        .map(|c| c.species)
        .collect();
    if !zorg_species.is_empty() {
        let first = zorg_species[0];
        if zorg_species.iter().any(|s| *s != first) {
            println!("FAIL: the ZORGs are more than one species");
            failures += 1;
        }

        for r in &creatures.diplomacy.relations {
            if (r.a == first || r.b == first)
                && r.has(crate::creature::Treaty::Alliance) {
                    println!("FAIL: a ZORG is allied with a species");
                    failures += 1;
                }
        }
    }

    let player_has_zorg = creatures
        .list
        .iter()
        .any(|c| c.faction == crate::creature::Faction::Player && c.species == crate::creature::ZORG_SPECIES);
    if player_has_zorg {
        println!("FAIL: a ZORG is counted as a player species");
        failures += 1;
    }

    for c in &creatures.list {
        if c.faction != crate::creature::Faction::Player {
            continue;
        }
        let [r, g, b] = c.team_color;

        if r > 90 && b > 90 && g + 40 < r.min(b) {
            println!("FAIL: a player species is purple: {:?} {}", c.team_color, r);
            failures += 1;
            break;
        }
    }
    failures
}

fn print_succession(creatures: &CreatureSystem) {
    println!("\n--- succession ---");
    let in_crisis = creatures.species_in_crisis();
    println!("  colonies in crisis: {} of {}", in_crisis.len(), creatures.stats.species);
    for sp in in_crisis.iter().take(8) {
        println!("    the {} has no queen", crate::creature::species_name_for(*sp));
    }
}

fn print_predation(creatures: &CreatureSystem) {
    println!("\n--- predation ---");
    let mut fear_rows: Vec<(u32, u32, f32)> = creatures.predation.entries().to_vec();
    fear_rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    for (prey, predator, fear) in fear_rows.iter().take(12) {
        println!(
            "  the {} fears the {}: {:.0}",
            crate::creature::species_name_for(*prey),
            crate::creature::species_name_for(*predator),
            fear
        );
    }
    if fear_rows.is_empty() {
        println!("  (nothing has been hunted)");
    }
}

fn print_colony(creatures: &CreatureSystem, max_trait_count: u32) {
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
    for (sp, bits) in &creatures.discoveries {
        let names: Vec<&str> = crate::creature::discovery_list(*bits)
            .iter()
            .map(|d| d.name())
            .collect();
        println!(
            "    {:<16} {}",
            crate::creature::species_name_for(*sp),
            names.join(", ")
        );
    }
}

fn print_life_histories(creatures: &CreatureSystem) {
    println!("\n--- life histories ---");
    let mut printed: Vec<u32> = Vec::new();
    for c in &creatures.list {
        if printed.contains(&c.species) {
            continue;
        }
        printed.push(c.species);
        let strategy = if c.traits & crate::creature::trait_bits::PROLIFIC != 0 {
            "prolific"
        } else if c.traits & crate::creature::trait_bits::PROLONGED != 0 {
            "prolonged"
        } else {
            "balanced"
        };
        println!(
            "  {:<16} strategy {:<10} litter {}  cost {:.0}  lives {:.0} ticks  matures at {:.0}%",
            crate::creature::species_name_for(c.species),
            strategy,
            c.litter_size(),
            c.brood_cost(),
            c.lifespan(),
            c.maturity_fraction() * 100.0,
        );
    }
}

/// Who settled beside each landmark.
fn print_landmarks(world: &World, landmark_claims: &[(f32, f32, u32)]) -> u32 {
    let mut failures = 0;
    println!("\n--- landmarks ---");
    let mut owners: Vec<Vec<String>> = Vec::new();
    for l in &world.landmarks {
        let names: Vec<String> = landmark_claims
            .iter()
            .filter(|(lx, ly, _)| {
                ((lx - l.x).powi(2) + (ly - l.y).powi(2)).sqrt() <= l.kind.radius() * 1.6
            })
            .map(|(_, _, sp)| crate::creature::species_name_for(*sp))
            .collect();

        let mut uniq: Vec<String> = Vec::new();
        for n in &names {
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

    for (i, o) in owners.iter().enumerate() {
        if o.len() > 1 {
            println!(
                "NOTE: landmark {} is contested by {:?}",
                i, o
            );
        }
    }
    failures
}

/// Every stage has to have been lived, and a practice has to have been
/// learned.
fn check_stages(seen: &Seen, creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    if !seen.stages.saw(Stage::Juvenile) {
        println!("FAIL: no creature was ever a juvenile");
        failures += 1;
    }
    if !seen.stages.saw(Stage::Adult) {
        println!("FAIL: no creature was ever an adult");
        failures += 1;
    }
    if !seen.stages.saw(Stage::Elder) {
        println!("FAIL: no creature ever reached old age");
        failures += 1;
    }
    if !seen.affinity {
        println!("FAIL: no species ever became specialised");
        failures += 1;
    }
    if !seen.culture {
        println!("FAIL: no species ever learned a practice");
        failures += 1;
    }
    if creatures.stats.matings == 0 {
        println!("FAIL: no pair ever mated");
        failures += 1;
    }
    if creatures.two_parent_births == 0 {
        println!("FAIL: no young was born to two parents");
        failures += 1;
    }
    failures
}

/// Fear stays in range and never points at the species itself.
fn check_fear(creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    for (prey, predator, fear) in creatures.predation.entries() {
        if !(0.0..=100.0).contains(fear) {
            println!("FAIL: fear out of range: {}", fear);
            failures += 1;
        }
        if prey == predator {
            println!("FAIL: a species fears itself");
            failures += 1;
        }
    }
    failures
}

/// A colony with a queen is never in crisis.
fn check_crises(creatures: &CreatureSystem) -> u32 {
    let mut failures = 0;
    let queens: Vec<u32> = creatures
        .list
        .iter()
        .filter(|c| c.role == crate::creature::Role::Queen)
        .map(|c| c.species)
        .collect();
    for sp in creatures.species_in_crisis() {
        if queens.contains(&sp) {
            println!("FAIL: a colony with a queen is in crisis");
            failures += 1;
        }
    }
    failures
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
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

    failures += check_zorg_cave(&world, &creatures);

    let (life_seen, landmark_claims, creature_faults) =
        run_life(&mut world, &mut creatures, steps);
    failures += creature_faults;

    print_species(&creatures);
    failures += check_deep(&creatures);
    print_succession(&creatures);
    print_predation(&creatures);
    print_colony(&creatures, life_seen.max_trait_count);
    print_life_histories(&creatures);
    failures += print_landmarks(&world, &landmark_claims);
    failures += check_stages(&life_seen, &creatures);
    failures += check_fear(&creatures);
    failures += check_crises(&creatures);

    if failures == 0 {
        println!("\nOK: all life invariants hold");
    } else {
        println!("\n{} FAILURES", failures);
        std::process::exit(1);
    }
    0
}
