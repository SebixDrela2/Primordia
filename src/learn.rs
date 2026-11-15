//! The learner: the whole game, with no picture at all.
//!
//! This is the game itself, worked in the console. There is no window
//! and the card is never touched, so all the time goes into the
//! simulation: the world turns, the colony lives, the creatures learn
//! from what happens to them, the colony breeds, and when a species is
//! the last one standing the run ends and the **revival** begins - a
//! new world, and a new colony that carries the champion of the last
//! run, its mind and its genes and what it had learnt.
//!
//! Every revival is a generation of learning. The mind of the champion
//! is written to a file, so that the learner can be stopped at any
//! moment and the work is still there, and so that the game can be run
//! on it afterwards.

use crate::creature::Model;
use crate::session::{Outcome, Session};
use crate::rng::Rng;

/// The name of the file the model is written to, when nothing else is
/// asked for.
const DEFAULT_FILE: &str = "model.bin";

/// What the learner was asked to do.
struct Ask {
    file: String,
    /// How many revivals to work through. Zero means for ever.
    generations: u32,
    pop: usize,
    seed: u64,
    quiet: bool,
}

/// Read the ask out of the arguments.
fn read_ask(args: &[String]) -> Ask {
    let mut ask = Ask {
        file: DEFAULT_FILE.to_owned(),
        // For ever, unless the ask says otherwise.
        generations: 0,
        pop: 64,
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

/// Write the best mind of the run out, if it is better than the one
/// Write the lineage that would survive out of the world: the best of
/// the colony, which is what the revival carries on with. It is written
/// out when that lineage becomes another one, so the file always holds
/// the mind that is alive right now.
fn keep_the_best(session: &Session, ask: &Ask, seen: &mut u64) {
    if session.champion_version == *seen {
        return;
    }
    *seen = session.champion_version;

    let genome = session
        .champion
        .as_ref()
        .or(session.fallback_champion.as_ref());
    let Some(genome) = genome else {
        return;
    };

    let mut model = Model::from_net(
        &genome.net,
        genome.genes,
        crate::creature::decode_traits(&genome.genes),
        genome.culture,
        genome.size,
        genome.color,
    );
    model.fitness = genome.fitness;
    model.revival = session.revival;
    model.generation = genome.generation;
    if let Err(err) = model.save(&ask.file) {
        eprintln!("cannot write {}: {err}", ask.file);
        return;
    }
    if !ask.quiet {
        println!(
            "  the mind that survives fitted {:.3} - written to {}",
            model.fitness, ask.file
        );
    }
}
/// The learner. It ends the process when it is done.
#[must_use]
pub fn run(args: &[String]) -> i32 {
    let ask = read_ask(args);

    // The mind the learning starts from: the one that is in the file,
    // if there is one.
    if let Ok(model) = Model::load(&ask.file) {
        if !ask.quiet {
            println!(
                "starting from the model in {} ({} weights, fitness {:.3})",
                ask.file,
                model.weight_count(),
                model.fitness
            );
        }
    }

    let mut session = Session::new(ask.seed, ask.pop);
    let mut rng = Rng::new(ask.seed);
    let mut generation: u64 = 0;
    let mut last_beat: u64 = 0;
    // How many times the mind that would survive has changed.
    let mut seen: u64 = 0;

    // The learner works for ever unless it is told how many
    // generations to do.
    loop {
        // The world turns and the colony lives. This is the whole
        // game, with nothing drawn.
        session.step();

        // A word now and then, so that it is clear the learner is
        // working and not stuck. And the best mind so far is written
        // out, so that the learner can be stopped at any moment.
        if !ask.quiet && session.creatures.tick - last_beat >= 1200 {
            last_beat = session.creatures.tick;
            println!(
                "  tick {}: {} alive, the best {:.3}",
                session.creatures.tick,
                session.creatures.list.len(),
                session
                    .creatures
                    .list
                    .iter()
                    .map(|c| c.fitness)
                    .fold(0.0f32, f32::max)
            );
        }
        keep_the_best(&session, &ask, &mut seen);

        let Some(outcome) = session.check_victory() else {
            continue;
        };

        // The run is over. The lineage that is carried into the next
        // one is the mind that is written out: the mind that survives.
        let carried = session.carried();
        let mut saved = false;
        if let Some(genome) = &carried {
            let mut model = Model::from_net(
                &genome.net,
                genome.genes,
                crate::creature::decode_traits(&genome.genes),
                genome.culture,
                genome.size,
                genome.color,
            );
            model.fitness = genome.fitness;
            model.revival = session.revival;
            model.generation = generation as u32 + 1;
            if let Err(err) = model.save(&ask.file) {
                eprintln!("cannot write {}: {err}", ask.file);
                return 1;
            }
            saved = true;
        }
        generation += 1;
        if !ask.quiet {
            let what = match outcome {
                Outcome::Won { species, zorgs } => {
                    format!("{species} won with {zorgs} ZORGs left")
                }
                Outcome::Lost { zorgs } => format!("the colony was lost to {zorgs} ZORGs"),
                Outcome::Extinct => "nothing was left".to_owned(),
            };
            println!(
                "generation {generation:>3}: {what} - {} alive, the best fitted {:.3}{}",
                session.creatures.list.len(),
                carried.as_ref().map_or(0.0, |g| g.fitness),
                if saved {
                    " - written to the file"
                } else {
                    ""
                }
            );
        }

        // The next run: a new world, and a colony that carries the
        // lineage of the last one.
        let next_seed = rng.next_u32() as u64;
        let carried_ref = carried.as_ref();
        session.revive(next_seed, carried_ref);

        if ask.generations != 0 && generation >= ask.generations as u64 {
            break;
        }
    }

    println!(
        "the learner worked through {generation} generations; the mind that survives is in {}",
        ask.file
    );
    0
}