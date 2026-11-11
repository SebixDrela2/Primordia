//! Headless simulation harness: mirrors the real interactive simulation so
//! bench numbers reflect what the player actually sees.
//!
//! cargo run --release --bin bench -- [steps] [png-out]

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

use creature::{CreatureSystem, Role};
use elements::Elem;
use std::io::BufWriter;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use std::process;
use world::World;

const W: usize = 1280;
const H: usize = 720;

/// Same seed logic as main.rs: time + pid, no persistent counter.
fn get_random_seed() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = now.as_nanos() as u64;
    let mut seed = nanos;
    seed = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(process::id() as u64);
    seed = seed.wrapping_mul(0xBF58476D1CE4E5B9).wrapping_add(now.as_secs());
    eprintln!("SEED: {}", seed);
    seed
}

fn main() {
    let mut args = std::env::args().skip(1);
    let steps: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let out = args.next().unwrap_or_else(|| "snapshot.png".to_string());

    let seed = get_random_seed();
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    // Seed-time census: all elements present right after world generation
    let mut seed_counts = [0u32; Elem::MAX as usize + 1];
    for &k in &world.kind {
        if k == Elem::Empty as u8 { continue; }
        let idx = k as usize;
        if idx < seed_counts.len() { seed_counts[idx] += 1; }
    }
    let seed_present: usize = (1..=Elem::MAX).filter(|&e| seed_counts[e as usize] > 0).count();
    println!("seed-time elements present: {}/{}", seed_present, Elem::MAX as usize);
    let water_idx = Elem::Water as usize;
    println!("  Water cells at seed: {}", seed_counts[water_idx]);

    // Sanity check: no solid resources should be floating above the terrain.
    let mut sky_resources = 0u32;
    for x in 0..W as i32 {
        let surf = world.surface_y(x);
        for y in 0..(surf - 3).max(0) {
            let e = world.get(x, y);
            if e.is_solid() && !matches!(e, Elem::Plant | Elem::Wood | Elem::Moss | Elem::Spore | Elem::Roots) {
                sky_resources += 1;
            }
        }
    }
    println!("  Floating solid resources above terrain: {}", sky_resources);

    println!(
        "seed={}  start: particles={} plants={} water={} pop={} species={}",
        seed,
        world.census.particles,
        world.census.plants,
        world.census.water,
        creatures.list.len(),
        creatures.stats.species
    );

    // Track which systems get exercised during the bench run.
    let mut tools_seen: [bool; 15] = [false; 15];
    let mut spells_seen: [bool; 10] = [false; 10];
    let mut mounts_seen: [bool; 5] = [false; 5];
    let mut tech_max: u8 = 0;
    let mut gen_max: u32 = 0;

    let start = Instant::now();
    let mut report_at = 0u64;
    for step in 1..=steps {
        world.step();
        creatures.step(&mut world);

        // Sample system usage every frame.
        for c in &creatures.list {
            let t = c.tool as usize;
            if t < tools_seen.len() {
                tools_seen[t] = true;
            }
            let s = c.active_spell as usize;
            if s < spells_seen.len() {
                spells_seen[s] = true;
            }
            let m = c.mount_kind as usize;
            if m < mounts_seen.len() {
                mounts_seen[m] = true;
            }
            tech_max = tech_max.max(c.tech_level);
            gen_max = gen_max.max(c.generation);
        }

        if step >= report_at {
            report_at = step + steps / 10;
            let ms = start.elapsed().as_secs_f64() * 1000.0 / step as f64;
            println!(
                "step {:>6}  {:.3} ms/step  {:>6.0} steps/s | particles {:>6} plants {:>5} water {:>6} | pop {:>3} species {:>3} gen {:>3} energy {:>7.1} fitness {:.2} births {} deaths {}",
                step,
                ms,
                1000.0 / ms,
                world.census.particles,
                world.census.plants,
                world.census.water,
                creatures.list.len(),
                creatures.stats.species,
                creatures.stats.max_generation,
                creatures.stats.avg_energy,
                creatures.stats.avg_fitness,
                creatures.stats.births,
                creatures.stats.deaths
            );
            println!(
                "          eats {}  builds {}  trades {}  spells {}  mounts {}  structs {} | workers {} soldiers {} queens {} priests {} merchants {} builders {} explorers {} shamans {}",
                creatures.eats_total,
                creatures.builds_total,
                creatures.stats.trades,
                creatures.stats.spells_cast,
                creatures.stats.mounts,
                creatures.stats.structures,
                creatures.list.iter().filter(|c| c.role == Role::Worker).count(),
                creatures.list.iter().filter(|c| c.role == Role::Soldier).count(),
                creatures.list.iter().filter(|c| c.role == Role::Queen).count(),
                creatures.list.iter().filter(|c| c.role == Role::Priest).count(),
                creatures.list.iter().filter(|c| c.role == Role::Merchant).count(),
                creatures.list.iter().filter(|c| c.role == Role::Builder).count(),
                creatures.list.iter().filter(|c| c.role == Role::Explorer).count(),
                creatures.list.iter().filter(|c| c.role == Role::Shaman).count(),
            );
            println!(
                "          tech: t0={} t1={} t2={} t3={} t4={} | hunger={:.0} thirst={:.0} stamina={:.0} diseases={} season={} temp={:.0}",
                creatures.stats.tech_stone,
                creatures.stats.tech_copper,
                creatures.stats.tech_iron,
                creatures.stats.tech_gold,
                creatures.stats.tech_crystal,
                creatures.stats.avg_hunger,
                creatures.stats.avg_thirst,
                creatures.stats.avg_stamina,
                creatures.stats.diseases,
                creatures.season,
                creatures.temperature
            );
            println!(
                "          tools: pk={} sw={} hm={} dr={} sc={} bo={} sp={} sh={} st={} am={} bp={} ca={} bo2={} gl={}",
                creatures.stats.tools_pickaxe,
                creatures.stats.tools_sword,
                creatures.stats.tools_hammer,
                creatures.stats.tools_drill,
                creatures.stats.tools_scanner,
                creatures.stats.tools_bow,
                creatures.stats.tools_spear,
                creatures.stats.tools_shield,
                creatures.stats.tools_staff,
                creatures.stats.tools_amulet,
                creatures.stats.tools_backpack,
                creatures.stats.tools_cart,
                creatures.stats.tools_boat,
                creatures.stats.tools_glider,
            );
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "total: {} steps in {:.2}s = {:.0} steps/s ({:.3} ms/step)",
        steps,
        elapsed,
        steps as f64 / elapsed,
        elapsed * 1000.0 / steps as f64
    );

    // ---- System usage verification ----
    let tool_names = [
        "None", "Pickaxe", "Sword", "Hammer", "Drill", "Scanner", "Bow", "Spear",
        "Shield", "Staff", "Amulet", "Backpack", "Cart", "Boat", "Glider",
    ];
    let spell_names = [
        "None", "Fireball", "Heal", "Teleport", "Wall", "Light", "Summon", "Curse", "Bless", "Lightning",
    ];
    let mount_names = ["None", "Beast", "Cart", "Boat", "Glider"];

    let tools_used: usize = tools_seen.iter().skip(1).filter(|&&b| b).count();
    let spells_used: usize = spells_seen.iter().skip(1).filter(|&&b| b).count();
    let mounts_used: usize = mounts_seen.iter().skip(1).filter(|&&b| b).count();

    println!("");
    println!("=== SYSTEM USAGE (exercised during bench) ===");
    println!("tools used: {}/14", tools_used);
    for (i, name) in tool_names.iter().enumerate().skip(1) {
        if tools_seen[i] {
            println!("  [x] {}", name);
        }
    }
    println!("spells used: {}/9", spells_used);
    for (i, name) in spell_names.iter().enumerate().skip(1) {
        if spells_seen[i] {
            println!("  [x] {}", name);
        }
    }
    println!("mounts used: {}/4", mounts_used);
    for (i, name) in mount_names.iter().enumerate().skip(1) {
        if mounts_seen[i] {
            println!("  [x] {}", name);
        }
    }
    println!("max tech level reached: {}", tech_max);
    println!("max generation reached: {}", gen_max);
    println!("structures built: {}", creatures.stats.structures);
    println!("trades completed: {}", creatures.stats.trades);
    println!("spells cast: {}", creatures.stats.spells_cast);
    println!("mounts tamed: {}", creatures.stats.mounts);

    // ---- Element census: which elements ever existed in the world ----
    // We approximate by counting what's present at the end plus what was eaten/built.
    let mut element_counts = [0u32; Elem::MAX as usize + 1];
    // Count current world contents
    for &k in &world.kind {
        if k == Elem::Empty as u8 {
            continue;
        }
        let idx = k as usize;
        if idx < element_counts.len() {
            element_counts[idx] += 1;
        }
    }

    // Diagnostic: which elements lost/gained the most cells over the run.
    let mut deltas: Vec<(&str, i64)> = Vec::new();
    for e in 1..=Elem::MAX {
        let s = seed_counts[e as usize] as i64;
        let c = element_counts[e as usize] as i64;
        if (c - s).abs() > 500 {
            deltas.push((Elem::from_u8(e).name(), c - s));
        }
    }
    deltas.sort_by_key(|d| d.1);
    println!("");
    println!("=== BIGGEST ELEMENT DELTAS (end - seed) ===");
    for (name, d) in deltas.iter().take(12) {
        println!("  {:>12}: {:+}", name, d);
    }

    println!("");
    println!("=== ELEMENTS PRESENT IN WORLD (end of bench) ===");
    let mut present: Vec<(&str, u32)> = Vec::new();
    for e in 1..=Elem::MAX {
        let elem = Elem::from_u8(e);
        let cnt = element_counts[e as usize];
        if cnt > 0 {
            present.push((elem.name(), cnt));
        }
    }
    present.sort_by(|a, b| b.1.cmp(&a.1));
    for (name, cnt) in present.iter().take(40) {
        println!("  {:>12}: {}", name, cnt);
    }

    let total_present: u32 = present.iter().map(|(_, c)| c).sum();
    println!("total distinct elements present: {}/{}", present.len(), Elem::MAX as usize);

    // Print missing elements
    let mut missing: Vec<&str> = Vec::new();
    for e in 1..=Elem::MAX {
        if element_counts[e as usize] == 0 {
            missing.push(Elem::from_u8(e).name());
        }
    }
    if !missing.is_empty() {
        println!("missing elements: {:?}", missing);
    }

    // Write a PNG snapshot of the final frame
    let mut buf = vec![0u8; (W * H * 4) as usize];
    let mut renderer = render::Renderer::new();
    renderer.render_world(&world, &creatures, &mut buf, 0.0, 0.25, 0, world.weather);

    let file = std::fs::File::create(&out).expect("create png");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, W as u32, H as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().expect("png header");
    png_writer.write_image_data(&buf).expect("png data");
    println!("wrote {}", out);
}
