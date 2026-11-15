
use crate::creature::{Creature, CreatureSystem, Role};
use crate::elements::Elem;
use std::io::BufWriter;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use std::process;
use crate::world::World;

const W: usize = 1280;
const H: usize = 720;

const ELEM_SLOTS: usize = Elem::MAX as usize + 1;

fn get_random_seed() -> u64 {
    // A seed can be forced from the outside, so that the same run can be
    // repeated and compared with another.
    if let Ok(seed) = std::env::var("PRIMORDIA_SEED") {
        if let Ok(seed) = seed.parse::<u64>() {
            eprintln!("SEED: {}", seed);
            return seed;
        }
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = now.as_nanos() as u64;
    let mut seed = nanos;
    seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(process::id() as u64);
    seed = seed.wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(now.as_secs());
    eprintln!("SEED: {}", seed);
    seed
}

/// What the colony exercised while the bench ran.
struct Usage {
    tools: [bool; 15],
    spells: [bool; 10],
    mounts: [bool; 5],
    tech_max: u8,
    gen_max: u32,
}

impl Usage {
    fn new() -> Self {
        Self {
            tools: [false; 15],
            spells: [false; 10],
            mounts: [false; 5],
            tech_max: 0,
            gen_max: 0,
        }
    }

    fn note(&mut self, c: &Creature) {
        let t = c.tool as usize;
        if t < self.tools.len() {
            self.tools[t] = true;
        }
        let s = c.active_spell as usize;
        if s < self.spells.len() {
            self.spells[s] = true;
        }
        let m = c.mount_kind as usize;
        if m < self.mounts.len() {
            self.mounts[m] = true;
        }
        self.tech_max = self.tech_max.max(c.tech_level);
        self.gen_max = self.gen_max.max(c.generation);
    }
}

fn count_elements(world: &World) -> [u32; ELEM_SLOTS] {
    let mut counts = [0u32; ELEM_SLOTS];
    for &k in &world.kind {
        if k == Elem::Empty as u8 {
            continue;
        }
        let idx = k as usize;
        if idx < counts.len() {
            counts[idx] += 1;
        }
    }
    counts
}

/// The solid cells that hang in the air above the terrain.
fn count_sky_resources(world: &World) -> u32 {
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
    sky_resources
}

fn report_seed(world: &World, seed_counts: &[u32; ELEM_SLOTS]) {
    let seed_present: usize = (1..=Elem::MAX).filter(|&e| seed_counts[e as usize] > 0).count();
    println!("seed-time elements present: {}/{}", seed_present, Elem::MAX as usize);
    println!("  Water cells at seed: {}", seed_counts[Elem::Water as usize]);
    println!(
        "  Floating solid resources above terrain: {}",
        count_sky_resources(world)
    );
}

fn run_bench(world: &mut World, creatures: &mut CreatureSystem, steps: u64) -> Usage {
    let mut usage = Usage::new();
    let start = Instant::now();
    let mut report_at = 0u64;
    for step in 1..=steps {
        world.step();
        creatures.step(world);

        for c in &creatures.list {
            usage.note(c);
        }

        if step >= report_at {
            report_at = step + steps / 10;
            let ms = start.elapsed().as_secs_f64() * 1000.0 / step as f64;
            report_step(world, creatures, step, ms);
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
    usage
}

fn report_step(world: &World, creatures: &CreatureSystem, step: u64, ms: f64) {
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

    let p = world.pressure;
    println!(
        "          pressure: scarcity {:.2}  depletion {:.2}  harshness {:.2}  extremes {:.2}  volcanic {:.2}",
        p.scarcity, p.depletion, p.harshness, p.extremes, p.volcanic
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
    report_tech_and_tools(creatures);
}

fn report_tech_and_tools(creatures: &CreatureSystem) {
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

fn report_usage(creatures: &CreatureSystem, usage: &Usage) {
    let tool_names = [
        "None", "Pickaxe", "Sword", "Hammer", "Drill", "Scanner", "Bow", "Spear",
        "Shield", "Staff", "Amulet", "Backpack", "Cart", "Boat", "Glider",
    ];
    let spell_names = [
        "None", "Fireball", "Heal", "Teleport", "Wall", "Light", "Summon", "Curse", "Bless", "Lightning",
    ];
    let mount_names = ["None", "Beast", "Cart", "Boat", "Glider"];

    let tools_used: usize = usage.tools.iter().skip(1).filter(|&&b| b).count();
    let spells_used: usize = usage.spells.iter().skip(1).filter(|&&b| b).count();
    let mounts_used: usize = usage.mounts.iter().skip(1).filter(|&&b| b).count();

    println!();
    println!("=== SYSTEM USAGE (exercised during bench) ===");
    println!("tools used: {}/14", tools_used);
    for (i, name) in tool_names.iter().enumerate().skip(1) {
        if usage.tools[i] {
            println!("  [x] {}", name);
        }
    }
    println!("spells used: {}/9", spells_used);
    for (i, name) in spell_names.iter().enumerate().skip(1) {
        if usage.spells[i] {
            println!("  [x] {}", name);
        }
    }
    println!("mounts used: {}/4", mounts_used);
    for (i, name) in mount_names.iter().enumerate().skip(1) {
        if usage.mounts[i] {
            println!("  [x] {}", name);
        }
    }
    println!("max tech level reached: {}", usage.tech_max);
    println!("max generation reached: {}", usage.gen_max);
    println!("structures built: {}", creatures.stats.structures);
    println!("trades completed: {}", creatures.stats.trades);
    println!("spells cast: {}", creatures.stats.spells_cast);
    println!("mounts tamed: {}", creatures.stats.mounts);
}

fn report_element_deltas(seed_counts: &[u32; ELEM_SLOTS], element_counts: &[u32; ELEM_SLOTS]) {
    let mut deltas: Vec<(&str, i64)> = Vec::new();
    for e in 1..=Elem::MAX {
        let s = seed_counts[e as usize] as i64;
        let c = element_counts[e as usize] as i64;
        if (c - s).abs() > 500 {
            deltas.push((Elem::from_u8(e).name(), c - s));
        }
    }
    deltas.sort_by_key(|d| d.1);
    println!();
    println!("=== BIGGEST ELEMENT DELTAS (end - seed) ===");
    for (name, d) in deltas.iter().take(12) {
        println!("  {:>12}: {:+}", name, d);
    }
}

fn report_elements_present(element_counts: &[u32; ELEM_SLOTS]) {
    println!();
    println!("=== ELEMENTS PRESENT IN WORLD (end of bench) ===");
    let mut present: Vec<(&str, u32)> = Vec::new();
    for e in 1..=Elem::MAX {
        let elem = Elem::from_u8(e);
        let cnt = element_counts[e as usize];
        if cnt > 0 {
            present.push((elem.name(), cnt));
        }
    }
    present.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (name, cnt) in present.iter().take(40) {
        println!("  {:>12}: {}", name, cnt);
    }

    let _total_present: u32 = present.iter().map(|(_, c)| c).sum();
    println!("total distinct elements present: {}/{}", present.len(), Elem::MAX as usize);

    let mut missing: Vec<&str> = Vec::new();
    for e in 1..=Elem::MAX {
        if element_counts[e as usize] == 0 {
            missing.push(Elem::from_u8(e).name());
        }
    }
    if !missing.is_empty() {
        println!("missing elements: {:?}", missing);
    }
}

fn write_png(world: &World, creatures: &CreatureSystem, out: &str) -> i32 {
    let mut buf = vec![0u8; W * H * 4];
    let mut renderer = crate::render::CpuRenderer::new();
    renderer.render_world(
        world,
        creatures,
        &mut buf,
        crate::render::Clock {
            time: 0.0,
            day_night: 0.25,
            season: 0,
            weather: world.weather,
        },
    );

    let Ok(file) = std::fs::File::create(out) else {
        println!("FAIL: cannot write {}", out);
        return 1;
    };
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, W as u32, H as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let Ok(mut png_writer) = encoder.write_header() else {
        println!("FAIL: cannot write the png header");
        return 1;
    };
    if png_writer.write_image_data(&buf).is_err() {
        println!("FAIL: cannot write the png data");
        return 1;
    }
    println!("wrote {}", out);
    0
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let mut args = args.iter();
    let steps: u64 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let out = args
        .next()
        .map_or("snapshot.png", |s| s.as_str())
        .to_string();

    let seed = get_random_seed();
    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    let seed_counts = count_elements(&world);
    report_seed(&world, &seed_counts);

    println!(
        "seed={}  start: particles={} plants={} water={} pop={} species={}",
        seed,
        world.census.particles,
        world.census.plants,
        world.census.water,
        creatures.list.len(),
        creatures.stats.species
    );

    let usage = run_bench(&mut world, &mut creatures, steps);

    report_usage(&creatures, &usage);

    let element_counts = count_elements(&world);
    report_element_deltas(&seed_counts, &element_counts);
    report_elements_present(&element_counts);

    write_png(&world, &creatures, &out)
}
