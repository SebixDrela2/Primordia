//! The learner: the work of the network, with no window and no card.
//!
//! This is the headless half of the game. It makes a world, puts a
//! colony in it, and lets the minds work: every creature learns from
//! what happens to it, the colony breeds, and the best of the colony
//! is kept. Generation after generation the colony is given the mind
//! of the best creature of the generation before, so that the network
//! gets better and better, and at the end the best mind is written to
//! a file.
//!
//! Nothing is drawn and nothing is painted on the card: the learner
//! is only the work of the network.

use crate::creature::{CreatureSystem, Faction, Model};
use crate::world::World;
use crate::{H, W};

/// The name of the file the model is written to, when nothing else is
/// asked for.
const DEFAULT_FILE: &str = "model.bin";

/// What the learner was asked to do.
struct Ask {
    file: String,
    generations: u32,
    ticks: u32,
    pop: usize,
    seed: u64,
    quiet: bool,
}

/// Read the ask out of the arguments.
fn read_ask(args: &[String]) -> Ask {
    let mut ask = Ask {
        file: DEFAULT_FILE.to_owned(),
        generations: 8,
        ticks: 2400,
        pop: 48,
        seed: 20_240_501,
        quiet: false,
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let value = |i: &mut usize| -> Option<String> {
            *i += 1;
            args.get(*i).cloned()
        };
        match arg {
            "--generations" => {
                if let Some(v) = value(&mut i) {
                    ask.generations = v.parse().unwrap_or(ask.generations);
                }
            }
            "--ticks" => {
                if let Some(v) = value(&mut i) {
                    ask.ticks = v.parse().unwrap_or(ask.ticks);
                }
            }
            "--pop" => {
                if let Some(v) = value(&mut i) {
                    ask.pop = v.parse().unwrap_or(ask.pop);
                }
            }
            "--seed" => {
                if let Some(v) = value(&mut i) {
                    ask.seed = v.parse().unwrap_or(ask.seed);
                }
            }
            "--quiet" => ask.quiet = true,
            "--file" => {
                if let Some(v) = value(&mut i) {
                    ask.file = v;
                }
            }
            _ => {}
        }
        i += 1;
    }
    ask
}

/// The learner. It ends the process when it is done.
#[must_use]
pub fn run(args: &[String]) -> i32 {
    let ask = read_ask(args);

    // The mind the learning starts from: the one that is in the file,
    // if there is one, or a fresh one.
    let best: Option<Model> = match Model::load(&ask.file) {
        Ok(model) => {
            if !ask.quiet {
                println!(
                    "starting from the model in {} ({} weights, fitness {:.3})",
                    ask.file,
                    model.weight_count(),
                    model.fitness
                );
            }
            Some(model)
        }
        Err(_) => None,
    };

    let mut best_fitness = f32::MIN;
    let mut best_model: Option<Model> = None;
    // The mind the next generation starts from: the best one so far.
    let mut carried = best;

    for generation in 0..ask.generations {
        let seed = ask.seed.wrapping_add(generation as u64 * 0x9E37_79B9);
        let (model, fitness, pop) = one_generation(seed, ask.ticks, ask.pop, carried.as_ref());
        if fitness > best_fitness {
            best_fitness = fitness;
            best_model = Some(model.clone());
        }
        // The best mind so far is the one the next generation starts
        // from, so that the work is never thrown away.
        carried.clone_from(&best_model);
        if !ask.quiet {
            println!(
                "generation {generation:>3}: the best of {pop} fitted {fitness:.3} (the best so far {best_fitness:.3})",
            );
        }
    }

    let Some(model) = best_model else {
        eprintln!("the learner found nothing worth keeping");
        return 1;
    };

    if let Err(err) = model.save(&ask.file) {
        eprintln!("cannot write {}: {err}", ask.file);
        return 1;
    }
    println!(
        "wrote {} ({} weights, fitness {:.3})",
        ask.file,
        model.weight_count(),
        best_fitness
    );
    0
}

/// One generation: a world, a colony, and the ticks they live through.
///
/// The colony starts from the mind that was given, so that the
/// generation after this one starts where this one ended.
fn one_generation(
    seed: u64,
    ticks: u32,
    pop: usize,
    start: Option<&Model>,
) -> (Model, f32, usize) {
    let mut world = World::new(W, H, seed);
    world.seed();

    let mut creatures = CreatureSystem::new(seed);
    creatures.populate(&world, pop);

    // Give the colony the mind it is to work with.
    if let Some(model) = start {
        for c in &mut creatures.list {
            if c.faction != Faction::Player {
                continue;
            }
            c.net = model.to_net();
            c.genes = model.genes;
            c.traits = model.traits;
            c.culture = model.culture;
            c.size = model.size;
        }
    }

    for _ in 0..ticks {
        world.step();
        creatures.step(&mut world);
    }

    // The best of the colony is the mind that is kept.
    let best = creatures
        .list
        .iter()
        .filter(|c| c.faction == Faction::Player)
        .max_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let pop = creatures
        .list
        .iter()
        .filter(|c| c.faction == Faction::Player)
        .count();

    let Some(best) = best else {
        // Nothing was left alive: keep what we had.
        let model = if let Some(model) = start {
            model.clone()
        } else {
            let mut rng = crate::rng::Rng::new(seed);
            Model::from_net(
                &crate::creature::Net::random(&crate::creature::brain_sizes(), &mut rng),
                [0.0; 32],
                0,
                0,
                1.0,
                [200, 200, 200],
            )
        };
        return (model, 0.0, pop);
    };

    let mut model = Model::from_net(
        &best.net,
        best.genes,
        best.traits,
        best.culture,
        best.size,
        best.color,
    );
    model.generation = best.generation;
    model.fitness = best.fitness;
    (model, best.fitness, pop)
}
