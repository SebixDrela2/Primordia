
use crate::creature::{Creature, CreatureSystem};
use crate::creature::senses::{DIR_CHANNELS, sight, smell};
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

/// The senses have to be primitive, but there have to be enough of them.
fn check_senses() -> u32 {
    println!("--- the senses ---");
    println!("  inputs: {}", crate::creature::INPUTS);
    println!("  outputs: {}", crate::creature::OUTPUTS);
    println!("  per direction: {} channels", DIR_CHANNELS);
    let mut failures = 0;
    if crate::creature::INPUTS < 100 {
        println!("FAIL: there are too few senses for a brain to work with");
        failures += 1;
    }
    failures
}

/// A newborn mind wants what it smells, and it can learn to want it.
fn check_learning() -> u32 {
    println!("--- the learning ---");
    let mut failures = 0;
    let instinct = test_instinct();
    if instinct {
        println!("OK: a newborn mind already wants what it smells");
    } else {
        println!("FAIL: a newborn mind does not want what it smells");
        failures += 1;
    }
    let learned = test_learning();
    if learned {
        println!("OK: a mind learns to want what it smells");
    } else {
        println!("FAIL: a mind cannot learn to want what it smells");
        failures += 1;
    }
    failures
}

fn avg_energy(creatures: &CreatureSystem) -> f32 {
    let p = creatures.list.len().max(1) as f32;
    creatures.list.iter().map(|c| c.energy).sum::<f32>() / p
}

fn avg_depth(world: &World, creatures: &CreatureSystem) -> f32 {
    let p = creatures.list.len().max(1) as f32;
    creatures
        .list
        .iter()
        .map(|c| (c.y.round() as i32 - world.surface_y(c.x.round() as i32)).max(0) as f32)
        .sum::<f32>()
        / p
}

/// Is there something worth eating within reach of this creature?
fn food_in_reach(world: &World, c: &Creature) -> bool {
    let cx = c.x.round() as i32;
    let cy = c.y.round() as i32;
    for (dx, dy) in crate::world::DIRS8.iter().copied().chain([(0, 0)]) {
        let e = world.get(cx + dx, cy + dy);
        if e.is_edible() && e.nutrition() > 3.0 {
            return true;
        }
    }
    false
}

fn count_food_in_reach(world: &World, creatures: &CreatureSystem) -> usize {
    let mut n = 0;
    for c in &creatures.list {
        if food_in_reach(world, c) {
            n += 1;
        }
    }
    n
}

fn report_world_step(world: &World, creatures: &CreatureSystem, i: u64, hunger: f32) {
    println!(
        "  step {} pop {} species {} hunger {:.0} energy {:.0} depth {:.0} food {} zorgs {} births {} deaths {}",
        i,
        creatures.list.len(),
        creatures.stats.species,
        hunger,
        avg_energy(creatures),
        avg_depth(world, creatures),
        count_food_in_reach(world, creatures),
        creatures.list.iter().filter(|c| c.faction == crate::creature::Faction::Zorg).count(),
        creatures.stats.births,
        creatures.stats.deaths
    );
}

/// Step the world and the creatures, and watch the hunger of the colony.
fn run_world(world: &mut World, creatures: &mut CreatureSystem, steps: u64) -> (f32, f32) {
    let mut hunger_first = 0.0f32;
    let mut hunger_second = 0.0f32;
    let mut samples_first = 0u32;
    let mut samples_second = 0u32;
    for i in 0..steps {

        world.step();
        creatures.step(world);
        if i.is_multiple_of(300) {
            let pop = creatures.list.len().max(1) as f32;
            let hunger: f32 = creatures.list.iter().map(|c| c.hunger).sum::<f32>() / pop;
            report_world_step(world, creatures, i, hunger);
            if i < steps / 2 {
                hunger_first += hunger;
                samples_first += 1;
            } else {
                hunger_second += hunger;
                samples_second += 1;
            }
        }
    }
    (
        hunger_first / samples_first.max(1) as f32,
        hunger_second / samples_second.max(1) as f32,
    )
}

fn print_meals(creatures: &CreatureSystem) {
    println!("  meals: {}", creatures.eats_total);

    let mut meals: Vec<(&str, u32)> = Vec::new();
    for (i, n) in creatures.eats_by_elem.iter().enumerate() {
        if *n > 0 {
            meals.push((crate::elements::Elem::from_u8(i as u8).name(), *n));
        }
    }
    meals.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (name, n) in meals.iter().take(8) {
        println!("    {} meals: {}", name, n);
    }
}

/// What the colony looks like at the end of the run.
fn print_colony(world: &World, creatures: &CreatureSystem) {
    let mut starved = 0u32;
    let mut hurt = 0u32;
    for c in &creatures.list {
        if c.hunger > 90.0 {
            starved += 1;
        }
        if c.health < 60.0 {
            hurt += 1;
        }
    }
    println!("  the living: {} starving, {} hurt", starved, hurt);

    let mut over = 0u32;
    let mut total = 0u32;
    for c in &creatures.list {
        total += 1;
        if c.out_cache[2] > 0.3 {
            over += 1;
        }
    }
    println!("  asking to eat: {}/{}", over, total);

    let mut with_food = 0u32;
    let mut resting = 0u32;
    let mut tired = 0u32;
    for c in &creatures.list {
        if food_in_reach(world, c) {
            with_food += 1;
        }
        if c.resting {
            resting += 1;
        }
        if c.energy < 400.0 {
            tired += 1;
        }
    }
    println!(
        "  food within reach: {}/{}   resting: {}   short of energy: {}",
        with_food, total, resting, tired
    );

    let mut depth = 0.0f32;
    for c in &creatures.list {
        let cx = c.x.round() as i32;
        depth += (c.y.round() as i32 - world.surface_y(cx)).max(0) as f32;
    }
    println!("  average depth below the surface: {:.1}", depth / total.max(1) as f32);
    println!("  population: {}", creatures.list.len());
    println!("  species: {}", creatures.stats.species);
}

/// The colony has to have fed itself and it has to still be alive.
fn check_colony(creatures: &CreatureSystem, hunger_first: f32, hunger_second: f32) -> u32 {
    let mut failures = 0;
    if creatures.eats_total == 0 {
        println!("FAIL: no creature ever ate");
        failures += 1;
    }
    if hunger_second < hunger_first {
        println!("OK: the colony learned to feed itself");
    } else {
        println!("NOTE: the colony did not get any less hungry by the end");
    }
    if creatures.list.is_empty() {
        println!("FAIL: everything died");
        failures += 1;
    }
    failures
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(12000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0004);

    let mut failures = 0u32;

    println!("=== brain check ===");
    println!("steps: {}", steps);

    failures += check_senses();
    failures += check_learning();

    println!("--- the world ---");
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    let (hunger_first, hunger_second) = run_world(&mut world, &mut creatures, steps);
    println!("  hunger in the first half:  {:.1}", hunger_first);
    println!("  hunger in the second half: {:.1}", hunger_second);

    print_meals(&creatures);
    print_colony(&world, &creatures);
    failures += check_colony(&creatures, hunger_first, hunger_second);

    if failures == 0 {
        println!("OK: the minds hold together");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
    0
}

fn test_instinct() -> bool {
    let mut rng = crate::rng::Rng::new(0x00C0_FFEE);
    let net = crate::creature::Net::random(&crate::creature::brain_sizes_for(0), &mut rng);
    let mut net = net;
    let mut act = Vec::new();
    let mut scratch = Vec::new();

    let want = 3usize;
    let mut input = vec![0.0f32; crate::creature::INPUTS];
    let base = want * DIR_CHANNELS;
    input[base + sight::NEAR] = 0.5;
    input[base + sight::GREEN] = 0.8;
    input[base + sight::SOFT] = 0.9;

    input[base + sight::CHANNELS + smell::SWEET] = 1.0;
    net.forward_into(&input, &mut act, &mut scratch);

    let mut best = 0usize;
    let mut best_val = f32::MIN;
    for d in 0..crate::creature::outputs::DESIRE_COUNT {
        let v = act[crate::creature::outputs::DESIRE + d];
        if v > best_val {
            best_val = v;
            best = d;
        }
    }
    println!("  a fresh mind wants direction {} (it smells in {})", best, want);
    best == want
}

fn test_learning() -> bool {
    let mut rng = crate::rng::Rng::new(0xBEEF_1234);
    let mut net = crate::creature::Net::random(&crate::creature::brain_sizes_for(0), &mut rng);

    net.skip.iter_mut().for_each(|v| *v = 0.0);
    net.skip_up = 0.0;
    let mut act = Vec::new();
    let mut scratch = Vec::new();

    let want = 3usize;

    let mut input = vec![0.0f32; crate::creature::INPUTS];

    let mut desire_before = 0.0f32;
    let mut desire_after = 0.0f32;

    for round in 0..600 {

        input.fill(0.0);
        let base = want * DIR_CHANNELS;
        input[base + sight::NEAR] = 0.5;
        input[base + sight::GREEN] = 0.8;
        input[base + sight::SOFT] = 0.9;
        input[base + sight::CHANNELS + smell::SWEET] = 1.0;
        net.forward_into(&input, &mut act, &mut scratch);
        let d = net.out_cache_or(&act, crate::creature::outputs::DESIRE + want);
        if round == 0 {
            desire_before = d;
        }
        if round == 599 {
            desire_after = d;
        }

        net.credit_desire(want, 0.5, 0.05);
    }

    println!("  desire for the smelt direction: {:.3} -> {:.3}", desire_before, desire_after);
    desire_after > desire_before + 0.1
}

trait NetExt {
    fn out_cache_or(&self, act: &[f32], idx: usize) -> f32;
}

impl NetExt for crate::creature::Net {
    fn out_cache_or(&self, act: &[f32], idx: usize) -> f32 {
        act.get(idx).copied().unwrap_or(0.0)
    }
}
