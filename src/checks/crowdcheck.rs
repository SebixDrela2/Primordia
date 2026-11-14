
use crate::creature::CreatureSystem;
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let steps: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(12345);
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed ^ 0xDEADBEEF);
    creatures.populate(&world, 64);

    for _ in 1..=steps {
        world.step();
        creatures.step(&mut world);
    }

    let mut neighbours = 0f64;
    let mut in_crowd = 0usize;
    let mut near_food = 0usize;
    let mut counted = 0usize;
    for a in &creatures.list {
        let mut n = 0;
        for b in &creatures.list {
            if a.id == b.id {
                continue;
            }
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            if dx * dx + dy * dy <= 16.0 {
                n += 1;
            }
        }
        neighbours += n as f64;
        if n >= 8 {
            in_crowd += 1;
        }

        let mut best = false;
        for (dx, dy) in [
            (0i32, 0i32),
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let x = a.x.round() as i32 + dx;
            let y = a.y.round() as i32 + dy;
            let e = world.get(x, y);
            if e.is_edible() && e.nutrition() > 0.0 {
                best = true;
            }
        }
        if best {
            near_food += 1;
        }
        counted += 1;
    }
    let pop = counted.max(1);
    println!(
        "seed {} steps {} | pop {} | avg neighbours within 4 cells {:.2} | creatures in a crowd (>=8) {} ({:.0}%) | standing on food {} ({:.0}%) | plants {} | avg energy {:.0} | deaths {}",
        seed,
        steps,
        pop,
        neighbours / pop as f64,
        in_crowd,
        in_crowd as f64 * 100.0 / pop as f64,
        near_food,
        near_food as f64 * 100.0 / pop as f64,
        world.census.plants,
        creatures.stats.avg_energy,
        creatures.stats.deaths,
    );
    0
}
