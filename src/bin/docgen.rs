//! Generates the element table section of `docs/SIMULATION.md` straight from
//! the element properties, so the documentation can never drift from the code.
//!
//! cargo run --release --bin docgen

#[path = "../creature/mod.rs"]
mod creature;
#[path = "../elements/mod.rs"]
mod elements;
#[path = "../rng.rs"]
mod rng;
#[path = "../world/mod.rs"]
mod world;

use elements::{Behavior, Elem};

fn behavior_name(b: Behavior) -> &'static str {
    match b {
        Behavior::Static => "Static",
        Behavior::Fall => "Fall",
        Behavior::Flow => "Flow",
        Behavior::Rise => "Rise",
        Behavior::Diffuse => "Diffuse",
        Behavior::Burn => "Burn",
        Behavior::Grow => "Grow",
        Behavior::Energy => "Energy",
        Behavior::Conduct => "Conduct",
        Behavior::Spread => "Spread",
        Behavior::Hatch => "Hatch",
        Behavior::Decay => "Decay",
    }
}

fn main() {
    println!("| # | Element | Category | Behaviour | Density | Nutrition | Toxicity | Lifetime | Solid | Edible | Emits light | Description |");
    println!("|---|---------|----------|-----------|---------|-----------|----------|----------|-------|--------|------------|-------------|");
    let mut n = 0;
    for v in 1..=Elem::MAX {
        let e = Elem::from_u8(v);
        let life = e.lifetime();
        let life_s = if life == u32::MAX {
            "permanent".to_string()
        } else {
            format!("{life} steps")
        };
        let emit = if e.emission()[0] > 0.0 { "yes" } else { "-" };
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            n,
            e.name(),
            e.category(),
            behavior_name(e.behavior()),
            e.density(),
            e.nutrition(),
            e.toxicity(),
            life_s,
            if e.is_solid() { "yes" } else { "-" },
            if e.is_edible() { "yes" } else { "-" },
            emit,
            e.description(),
        );
        n += 1;
    }
}
