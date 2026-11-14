
use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let steps: u64 = args.first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let seed = 12345u64;
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed ^ 0xDEADBEEF);
    creatures.populate(&world, 64);

    for step in 1..=steps {
        world.step();
        creatures.step(&mut world);
        if step.is_multiple_of(500) {
            println!(
                "step {:>5} | pop {:>4} | plants {:>5} | avg_energy {:>6.1} | births {:>4} deaths {:>4} | food {:?} |",
                step,
                creatures.list.len(),
                world.census.plants,
                creatures.stats.avg_energy,
                creatures.stats.births,
                creatures.stats.deaths,
                creatures.stats.avg_fitness,
            );
        }
    }

    let mut rows: Vec<(String, u32, f32)> = Vec::new();
    for i in 0..256usize {
        let n = crate::world::counters::MEALS[i].load(std::sync::atomic::Ordering::Relaxed);
        if n == 0 {
            continue;
        }
        let e = Elem::from_u8(i as u8);
        rows.push((e.name().to_string(), n, e.nutrition()));
    }
    rows.sort_by_key(|(_, n, _)| std::cmp::Reverse(*n));
    let total: u32 = rows.iter().map(|(_, n, _)| *n).sum();
    println!("\n=== what the colony ate over {} steps ({} meals) ===", steps, total);
    for (name, n, nutr) in &rows {
        println!(
            "  {:<12} {:>7} meals  ({:>5.1}%)  nutrition {:>5.1}  energy each {:>6.0}",
            name,
            n,
            *n as f32 * 100.0 / total as f32,
            nutr,
            nutr * 4.0
        );
    }
    let refused = crate::world::counters::CORPSE_REFUSED.load(std::sync::atomic::Ordering::Relaxed);
    let eaten_corpse = crate::world::counters::CORPSE_EATEN.load(std::sync::atomic::Ordering::Relaxed);
    println!(
        "\ncannibalism rule: {} own-kind corpses refused, {} rival corpses eaten",
        refused, eaten_corpse
    );
    0
}
