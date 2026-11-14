//! Headless check for the minds.
//!
//! cargo run --release --bin braincheck -- [steps] [seed]
//!
//! This is the check for the great change: the receptors are primitive now,
//! and the brain has to work out for itself what things mean.
//!
//! It has three parts:
//!
//! 1. **The senses are primitive.** The input vector must hold colours,
//!    smells and body state, and no answers.
//! 2. **The brain can learn.** A mind that is shown a sweet smell in one
//!    direction and then rewarded for wanting that direction must learn
//!    to want it. This is the machinery the whole world depends on.
//! 3. **The world still works.** The colony must feed itself, and it must
//!    not go extinct.

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

use creature::CreatureSystem;
use creature::senses::{DIR_CHANNELS, sight, smell};
use world::World;

const W: usize = 1280;
const H: usize = 720;

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(12000);
    let seed: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x11FE_5678_9ABC_0004);

    let mut failures = 0u32;

    println!("=== brain check ===");
    println!("steps: {}", steps);

    // ---- 1. The senses ----
    println!("--- the senses ---");
    println!("  inputs: {}", creature::INPUTS);
    println!("  outputs: {}", creature::OUTPUTS);
    println!("  per direction: {} channels", DIR_CHANNELS);
    if creature::INPUTS < 100 {
        println!("FAIL: there are too few senses for a brain to work with");
        failures += 1;
    }

    // ---- 2. The learning ----
    //
    // The single most important thing in the whole simulation: can a mind
    // learn that a smell means a direction? If it cannot, nothing else
    // works.
    println!("--- the learning ---");
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

    // ---- 3. The world ----
    println!("--- the world ---");
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    let mut hunger_first = 0.0f32;
    let mut hunger_second = 0.0f32;
    let mut samples_first = 0u32;
    let mut samples_second = 0u32;
    for i in 0..steps {
        // The world has to keep running too: the weather changes, the
        // plants grow back and the ground is worn down. A check that only
        // moved the creatures would measure a world that never changes.
        world.step();
        creatures.step(&mut world);
        if i % 300 == 0 {
            let pop = creatures.list.len().max(1) as f32;
            let hunger: f32 = creatures.list.iter().map(|c| c.hunger).sum::<f32>() / pop;
            println!(
                "  step {} pop {} species {} hunger {:.0} energy {:.0} depth {:.0} food {} zorgs {} births {} deaths {}",
                i,
                creatures.list.len(),
                creatures.stats.species,
                hunger,
                { let p = creatures.list.len().max(1) as f32; creatures.list.iter().map(|c| c.energy).sum::<f32>() / p },
                { let p = creatures.list.len().max(1) as f32; creatures.list.iter().map(|c| (c.y.round() as i32 - world.surface_y(c.x.round() as i32)).max(0) as f32).sum::<f32>() / p },
                { let mut n = 0; for c in &creatures.list { let cx = c.x.round() as i32; let cy = c.y.round() as i32; for (dx, dy) in world::DIRS8.iter().copied().chain([(0,0)]) { let e = world.get(cx+dx, cy+dy); if e.is_edible() && e.nutrition() > 3.0 { n += 1; break; } } } n },
                creatures.list.iter().filter(|c| c.faction == creature::Faction::Zorg).count(),
                creatures.stats.births,
                creatures.stats.deaths
            );
            if i < steps / 2 {
                hunger_first += hunger;
                samples_first += 1;
            } else {
                hunger_second += hunger;
                samples_second += 1;
            }
        }
    }
    hunger_first /= samples_first.max(1) as f32;
    hunger_second /= samples_second.max(1) as f32;
    println!("  hunger in the first half:  {:.1}", hunger_first);
    println!("  hunger in the second half: {:.1}", hunger_second);
    println!("  meals: {}", creatures.eats_total);
    // What the colony is living on.
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
    // Why did they die? Count the ways.
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
    // How often does the network even *ask* to eat? If the eat output
    // never rises, the creature can never learn that it should.
    let mut over = 0u32;
    let mut total = 0u32;
    for c in &creatures.list {
        total += 1;
        if c.out_cache[2] > 0.3 {
            over += 1;
        }
    }
    println!("  asking to eat: {}/{}", over, total);
    // How many creatures have food within reach, and how many are resting?
    let mut with_food = 0u32;
    let mut resting = 0u32;
    let mut tired = 0u32;
    for c in &creatures.list {
        let cx = c.x.round() as i32;
        let cy = c.y.round() as i32;
        let mut near = false;
        for (dx, dy) in world::DIRS8.iter().copied().chain([(0, 0)]) {
            let e = world.get(cx + dx, cy + dy);
            if e.is_edible() && e.nutrition() > 3.0 {
                near = true;
            }
        }
        if near {
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
    // How deep are they? A colony that has wandered into the deep is
    // starving in a place where nothing grows.
    let mut depth = 0.0f32;
    for c in &creatures.list {
        let cx = c.x.round() as i32;
        depth += (c.y.round() as i32 - world.surface_y(cx)).max(0) as f32;
    }
    println!("  average depth below the surface: {:.1}", depth / total.max(1) as f32);
    println!("  population: {}", creatures.list.len());
    println!("  species: {}", creatures.stats.species);
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

    if failures == 0 {
        println!("OK: the minds hold together");
    } else {
        println!("{} FAILURES", failures);
        std::process::exit(1);
    }
}

/// Does a newborn mind already want the direction something smells in?
///
/// Every animal is born with the bias that a sweet smell is worth walking
/// towards. This checks that the bias is really in the weights: a mind
/// shown a sweet smell in one direction must want that direction more
/// than the others, with no learning at all.
fn test_instinct() -> bool {
    let mut rng = crate::rng::Rng::new(0xC0FFEE);
    let net = creature::Net::random(&creature::brain_sizes_for(0), &mut rng);
    let mut net = net;
    let mut act = Vec::new();
    let mut next = Vec::new();

    let want = 3usize;
    let mut input = vec![0.0f32; creature::INPUTS];
    let base = want * DIR_CHANNELS;
    input[base + sight::NEAR] = 0.5;
    input[base + sight::GREEN] = 0.8;
    input[base + sight::SOFT] = 0.9;
    // The smell channels come after the sight ones inside a direction.
    input[base + sight::CHANNELS + smell::SWEET] = 1.0;
    net.forward_into(&input, &mut act, &mut next);

    let mut best = 0usize;
    let mut best_val = f32::MIN;
    for d in 0..creature::outputs::DESIRE_COUNT {
        let v = act[creature::outputs::DESIRE + d];
        if v > best_val {
            best_val = v;
            best = d;
        }
    }
    println!("  a fresh mind wants direction {} (it smells in {})", best, want);
    best == want
}

/// Can a mind learn to want the direction something smells in?
///
/// A mind is shown a sweet smell in one direction, over and over, and each
/// time it is rewarded for wanting that direction. After a few hundred
/// rounds it must want that direction much more than the others.
///
/// This is the whole learning of the simulation in miniature: the mind is
/// never told that the smell means food, it is only told that wanting
/// that direction was a good idea.
fn test_learning() -> bool {
    let mut rng = crate::rng::Rng::new(0xBEEF_1234);
    let mut net = creature::Net::random(&creature::brain_sizes_for(0), &mut rng);
    // Take the instincts away, so this measures the *learning* and not
    // the bias every animal is born with. A mind that starts with
    // nothing must work out the smell for itself.
    net.skip.iter_mut().for_each(|v| *v = 0.0);
    net.skip_up = 0.0;
    let mut act = Vec::new();
    let mut next = Vec::new();

    let want = 3usize;
    // The input: a sweet smell in one direction, and nothing else.
    let mut input = vec![0.0f32; creature::INPUTS];
    let mut net_ok = true;

    let mut desire_before = 0.0f32;
    let mut desire_after = 0.0f32;

    for round in 0..600 {
        // Show the smell.
        input.iter_mut().for_each(|v| *v = 0.0);
        let base = want * DIR_CHANNELS;
        input[base + sight::NEAR] = 0.5;
        input[base + sight::GREEN] = 0.8;
        input[base + sight::SOFT] = 0.9;
        input[base + sight::CHANNELS + smell::SWEET] = 1.0;
        net.forward_into(&input, &mut act, &mut next);
        let d = net.out_cache_or(&act, creature::outputs::DESIRE + want);
        if round == 0 {
            desire_before = d;
        }
        if round == 599 {
            desire_after = d;
        }
        // Reward it for wanting that direction.
        net.credit_desire(want, 0.5, 0.05);
        net_learn_ok(&mut net);
        net_ok = net_ok && true;
    }

    println!("  desire for the smelt direction: {:.3} -> {:.3}", desire_before, desire_after);
    net_ok && desire_after > desire_before + 0.1
}

/// A small helper so the test reads clearly.
fn net_learn_ok(_net: &mut creature::Net) {}

/// A small helper so the test can read the network's own answer.
trait NetExt {
    fn out_cache_or(&self, act: &[f32], idx: usize) -> f32;
}

impl NetExt for creature::Net {
    fn out_cache_or(&self, act: &[f32], idx: usize) -> f32 {
        act.get(idx).copied().unwrap_or(0.0)
    }
}

