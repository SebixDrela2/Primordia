mod creature;
mod elements;
mod render;
mod rng;
mod world;

use creature::{CreatureSystem, RevivalGenome, SimEvent};
use elements::Elem;
use macroquad::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use world::World;

const W: usize = 1280;
const H: usize = 720;
const VIEW_W: usize = 480;
const VIEW_H: usize = 270;
/// Zoom that fits the whole world on screen.
const MIN_ZOOM: f32 = VIEW_W as f32 / W as f32;
const MAX_ZOOM: f32 = 6.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "PRIMORDIA".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

/// Generate a random seed from time + process id. No persistent counter, so the
/// seed is fully determined by the moment you start (and shown on screen).
fn get_random_seed() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = now.as_nanos() as u64;
    let mut seed = nanos;
    seed = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(std::process::id() as u64);
    seed = seed.wrapping_mul(0xBF58476D1CE4E5B9).wrapping_add(now.as_secs());
    eprintln!("SEED: {}", seed);
    seed
}

mod ui;
use ui::*;

/// A monotonic clock for measuring how long the game has been running.
///
/// The system clock can jump (a time sync, a manual change), which would
/// make a timed screenshot fire at the wrong moment or not at all.
fn now_snapshot() -> std::time::Instant {
    std::time::Instant::now()
}

/// Save the current frame to a PNG file.
///
/// This is used by the screenshot tooling: `PRIMORDIA_SNAPSHOT=out.png`
/// makes the game write a picture of itself and exit after a while.
fn save_screenshot(path: &str) {
    let screen = get_screen_data();
    let w = screen.width as usize;
    let h = screen.height as usize;
    if w == 0 || h == 0 {
        return;
    }
    let file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("screenshot: cannot create {}: {}", path, e);
            return;
        }
    };
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = match encoder.write_header() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("screenshot: cannot write header: {}", e);
            return;
        }
    };
    // The framebuffer comes back bottom-up, so the rows are flipped before
    // they are written out.
    let stride = w * 4;
    let mut flipped: Vec<u8> = vec![0u8; stride * h];
    for y in 0..h {
        let src = (h - 1 - y) * stride;
        let dst = y * stride;
        flipped[dst..dst + stride].copy_from_slice(&screen.bytes[src..src + stride]);
    }
    if let Err(e) = png_writer.write_image_data(&flipped) {
        eprintln!("screenshot: cannot write pixels: {}", e);
    }
    eprintln!("screenshot written to {}", path);
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = get_random_seed();

    let mut world = World::new(W, H, seed);
    world.seed();

    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 64);

    // Camera position (top-left of view in world coordinates)
    let mut cam_x: f32 = 0.0;
    let mut cam_y: f32 = 0.0;
    let cam_speed = 300.0;

    // PRIMORDIA_CAM=x,y starts the camera at a point in the world. It is
    // handy for looking at a particular part of the map - the ZORG cave in
    // the deep, for instance - without having to pan there by hand.
    let cam_env = std::env::var("PRIMORDIA_CAM").ok();
    if let Some(v) = cam_env.as_deref() {
        if !v.eq_ignore_ascii_case("zorg") {
            let mut parts = v.split(',');
            if let (Some(x), Some(y)) = (parts.next(), parts.next()) {
                if let (Ok(x), Ok(y)) = (x.trim().parse::<f32>(), y.trim().parse::<f32>()) {
                    cam_x = x;
                    cam_y = y;
                }
            }
        }
    }

    static mut ACCUM: f64 = 0.0;

    let mut ui = UiState::new();
    // PRIMORDIA_PANEL=diplomacy opens a window straight away (handy for
    // screenshots and for checking the panel without hunting for the key).
    if std::env::var("PRIMORDIA_PANEL")
        .ok()
        .as_deref()
        .map_or(false, |p| p.eq_ignore_ascii_case("diplomacy"))
    {
        ui.diplomacy_open = true;
    }
    // PRIMORDIA_PANEL=tech opens the tech tree straight away.
    if std::env::var("PRIMORDIA_PANEL")
        .ok()
        .as_deref()
        .map_or(false, |p| p.eq_ignore_ascii_case("tech"))
    {
        ui.tech_open = true;
    }
    // PRIMORDIA_PANEL=minds opens the mind inspector straight away.
    if std::env::var("PRIMORDIA_PANEL")
        .ok()
        .as_deref()
        .map_or(false, |p| p.eq_ignore_ascii_case("minds"))
    {
        ui.minds_open = true;
    }
    // PRIMORDIA_SNAPSHOT=out.png writes a picture of the game after
    // PRIMORDIA_SNAPSHOT_AFTER seconds and closes the window.
    let snapshot = std::env::var("PRIMORDIA_SNAPSHOT").ok();
    let snapshot_after = std::env::var("PRIMORDIA_SNAPSHOT_AFTER")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(30.0);
    let snapshot_at = snapshot.as_ref().map(|_| now_snapshot());

    // The renderer owns all its scratch buffers, so rendering never
    // allocates once the world exists.
    let mut renderer = render::Renderer::new();

    // Simulation speed multiplier (1.0 = normal, press + / - to change).
    // PRIMORDIA_SPEED can be set to start at a different speed.
    let mut sim_speed: f32 = std::env::var("PRIMORDIA_SPEED")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1.0)
        .clamp(0.25, 48.0);
    // P toggles pause
    let mut paused = false;
    // Scroll wheel zoom (0.5 = whole world visible, 6.0 = close-up).
    // Every run starts fully zoomed out.
    // PRIMORDIA_ZOOM can be set to start at a different zoom.
    let mut zoom: f32 = std::env::var("PRIMORDIA_ZOOM")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(MIN_ZOOM)
        .clamp(MIN_ZOOM, MAX_ZOOM);
    // The ZORG cave is the one part of the world that belongs to the deep.
    // PRIMORDIA_CAM=zorg opens the run looking at it, in the middle of the
    // screen.
    if cam_env
        .as_deref()
        .map_or(false, |c| c.eq_ignore_ascii_case("zorg"))
    {
        if let Some((cx, cy)) = world.zorg_cave_center() {
            cam_x = cx as f32 - (VIEW_W as f32 / zoom) * 0.5;
            cam_y = cy as f32 - (VIEW_H as f32 / zoom) * 0.5;
        }
    }
    // Endless "revival" lineage: run 1 is the original population, then each
    // victory seeds a new world from the winning species' genome.
    let mut revival: u32 = 1;
    let mut champion: Option<RevivalGenome> = None;
    let mut victory: Option<VictoryScreen> = None;
    // Snapshot of the most recent species' genome, used if the whole
    // population dies out (the last lineage still gets a revival).
    let mut fallback_champion: Option<RevivalGenome> = None;
    let mut last_species: Option<u32> = None;
    let mut telemetry = Telemetry::new();
    telemetry.reset(1);
    // The last diplomatic tick that was mirrored into the ecology feed.
    let mut last_diplo_tick: u64 = 0;
    let mut effects = Effects::new();
    let mut force_render = true;
    // Smoothed camera position (the input drives `cam_x`/`cam_y`, the view
    // eases towards it). It starts where the camera starts, so a run that
    // opens on a given part of the world opens there.
    let mut view_x: f32 = cam_x;
    let mut view_y: f32 = cam_y;

    // Minimap terrain texture (downsampled world).
    const MINI_W: usize = 160;
    const MINI_H: usize = 90;
    const MINI_STEP: usize = 8;
    let mut mini_image = Image::gen_image_color(MINI_W as u16, MINI_H as u16, WHITE);
    let mini_texture = Texture2D::from_image(&mini_image);
    mini_texture.set_filter(FilterMode::Linear);
    let mut mini_dirty = true;
    let mut frame_counter: u32 = 0;

    // Region texture: only the visible slice of the world is rendered.
    let mut region_pixels: Vec<u8> = Vec::new();
    let mut region_image = Image::gen_image_color(1, 1, WHITE);
    let mut region_texture = Texture2D::from_image(&region_image);
    region_texture.set_filter(FilterMode::Nearest);
    let mut region_w: i32 = 0;
    let mut region_h: i32 = 0;

    loop {
        let now = get_time() as f32;

        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) {
            std::process::exit(0);
        }

        // R: reroll the world with a brand new seed
        if is_key_pressed(KeyCode::R) && victory.is_none() {
            let new_seed = get_random_seed();
            world = World::new(W, H, new_seed);
            world.seed();
            creatures = CreatureSystem::new(new_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
            creatures.populate(&world, 64);
            cam_x = 0.0;
            cam_y = 0.0;
            view_x = 0.0;
            view_y = 0.0;
            zoom = MIN_ZOOM;
            // A manual reroll starts a brand new lineage.
            revival = 1;
            champion = None;
            force_render = true;
            telemetry.reset(1);
        }

        // K: toggle knowledge window
        if is_key_pressed(KeyCode::K) {
            ui.knowledge_open = !ui.knowledge_open;
        }

        // T: toggle the tech tree window
        if is_key_pressed(KeyCode::T) {
            ui.tech_open = !ui.tech_open;
        }

        // G: toggle the diplomacy panel
        if is_key_pressed(KeyCode::G) {
            ui.diplomacy_open = !ui.diplomacy_open;
        }

        // M: toggle the mind inspector
        if is_key_pressed(KeyCode::M) {
            ui.minds_open = !ui.minds_open;
        }

        // + / - : adjust simulation speed
        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
            sim_speed = (sim_speed * 2.0).min(48.0);
        }
        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
            sim_speed = (sim_speed / 2.0).max(0.25);
        }

        // P: pause / resume (ignored while the victory screen is up)
        if is_key_pressed(KeyCode::P) && victory.is_none() {
            paused = !paused;
        }

        // View size in world cells for the current zoom level.
        let mut view_w = VIEW_W as f32 / zoom;
        let mut view_h = VIEW_H as f32 / zoom;

        // Scroll wheel: zoom in/out around the mouse cursor. Some platforms
        // report large/accumulated wheel deltas (e.g. ±120), so normalize each
        // frame to a single notch and apply a gentle step.
        //
        // The wheel over the tech tree scrolls the tree instead of zooming.
        let (_, wheel_y) = mouse_wheel();
        let (mx, my) = mouse_position();
        let over_tech = ui.tech_open && tech_tree_under_pointer(mx, my);
        // The wheel over the mind inspector walks from one of the cleverest
        // animals to the next instead of zooming.
        if ui.minds_open && wheel_y != 0.0 {
            ui.minds_index = (ui.minds_index - wheel_y.signum()).max(0.0);
        } else if wheel_y != 0.0 && !over_tech {
            let direction = wheel_y.signum();
            let old_sx = screen_width() / view_w;
            let old_sy = screen_height() / view_h;
            let anchor_x = cam_x + mx / old_sx;
            let anchor_y = cam_y + my / old_sy;

            zoom = (zoom * 1.12f32.powf(direction)).clamp(MIN_ZOOM, MAX_ZOOM);
            view_w = VIEW_W as f32 / zoom;
            view_h = VIEW_H as f32 / zoom;

            let new_sx = screen_width() / view_w;
            let new_sy = screen_height() / view_h;
            cam_x = anchor_x - mx / new_sx;
            cam_y = anchor_y - my / new_sy;
        }

        // Camera movement (speed scales with how much world is visible)
        let dt = get_frame_time();
        let cam_speed = cam_speed * (view_w / VIEW_W as f32);
        let max_cam_x = (W as f32 - view_w).max(0.0);
        let max_cam_y = (H as f32 - view_h).max(0.0);
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            cam_x -= cam_speed * dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            cam_x += cam_speed * dt;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            cam_y -= cam_speed * dt;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            cam_y += cam_speed * dt;
        }
        cam_x = cam_x.clamp(0.0, max_cam_x);
        cam_y = cam_y.clamp(0.0, max_cam_y);

        // Ease the actual view towards the camera target for smooth panning.
        let ease = 1.0 - (-9.0 * dt).exp();
        view_x += (cam_x - view_x) * ease;
        view_y += (cam_y - view_y) * ease;
        if (cam_x - view_x).abs() < 0.05 {
            view_x = cam_x;
        }
        if (cam_y - view_y).abs() < 0.05 {
            view_y = cam_y;
        }

        let sim_dt = (get_frame_time().min(0.1) as f64) * (sim_speed as f64);
        let fixed_step = 1.0 / 60.0;
        // Cap the catch-up work per frame so a fast sim speed never stalls
        // the renderer for a whole second.
        let max_steps = (4.0 * sim_speed).min(48.0) as u32;
        let mut steps_taken = 0u32;
        if paused {
            unsafe { ACCUM = 0.0; }
        } else {
            unsafe {
                ACCUM += sim_dt;
                let mut steps = 0;
                while ACCUM >= fixed_step && steps < max_steps {
                    world.step();
                    creatures.step(&mut world);
                    ACCUM -= fixed_step;
                    steps += 1;
                }
                steps_taken = steps as u32;
                if steps == max_steps {
                    ACCUM = 0.0;
                }
            }
        }

        // ---- Rolling ecology telemetry ----
        if victory.is_none() {
            telemetry.observe(now, &creatures);
            // Mirror the diplomatic record into the ecology feed, so a
            // treaty or a declaration of war shows up in the event list.
            for e in creatures.diplomacy.log.iter() {
                if e.tick > last_diplo_tick {
                    telemetry.push(e.text.clone());
                }
            }
            last_diplo_tick = creatures.tick;
        }
        telemetry.tick(dt);

        // ---- Turn simulation events into animations ----
        for ev in creatures.events.drain(..) {
            if matches!(ev, SimEvent::Speciation { .. }) {
                telemetry.push("A NEW SPECIES HAS ARISEN".to_string());
            }
            apply_event(&mut effects, ev);
        }
        effects.update(dt);

        // ---- Species domination / extinction -> victory screen ----
        if victory.is_none() {
            // Keep a running snapshot of the dominant *player* lineage so a
            // total extinction can still revive the last species to stand.
            // The ZORGs are never a player: they cannot win, and they never
            // carry a genome into the next revival.
            if let Some(sp) = dominant_player_species(&creatures) {
                if last_species != Some(sp) || fallback_champion.is_none() {
                    last_species = Some(sp);
                    fallback_champion = creatures.champion_from(sp);
                }
            }

            // The species that are left, split into the players and the
            // ZORGs.
            let mut players: Vec<u32> = creatures
                .list
                .iter()
                .filter(|c| c.faction == crate::creature::Faction::Player)
                .map(|c| c.species)
                .collect();
            players.sort_unstable();
            players.dedup();
            let zorgs = creatures.zorg_count();

            if creatures.list.is_empty() {
                // Total extinction: the last species to hold on still carries
                // its genome into the next revival.
                victory = Some(VictoryScreen {
                    winner: "NO SURVIVORS".to_string(),
                    color: [90, 100, 110],
                    survivors: 0,
                    generation: creatures.stats.max_generation,
                    revival,
                    timer: 1.0,
                    extinction: true,
                    lost: false,
                    zorgs: 0,
                    alliances: 0,
                    wars: 0,
                    treaties: 0,
                    truces: 0,
                    traits: Vec::new(),
                    discoveries: Vec::new(),
                    deep_sealed: world.zorg_cave_sealed,
                });
                paused = true;
            } else if players.is_empty() {
                // Only the ZORGs are left. They are not a player team, so
                // this is not a win: the run is lost. The genome carried
                // into the next revival is the last player species, not the
                // ZORGs.
                victory = Some(VictoryScreen {
                    winner: "LOST".to_string(),
                    color: crate::creature::ZORG_COLOR,
                    survivors: 0,
                    generation: creatures.stats.max_generation,
                    revival,
                    timer: 1.0,
                    extinction: true,
                    lost: true,
                    zorgs,
                    alliances: 0,
                    wars: 0,
                    treaties: 0,
                    truces: 0,
                    traits: Vec::new(),
                    discoveries: Vec::new(),
                    deep_sealed: world.zorg_cave_sealed,
                });
                paused = true;
            } else if players.len() == 1 && (zorgs == 0 || world.zorg_cave_sealed) {
                // One player species is left and the deep is closed: the
                // world belongs to that lineage.
                //
                // The ZORGs cannot be out-fought - the cave breeds more of
                // them for as long as it stands. The only way to finish
                // them off is to seal their cave with a ward of sanctified
                // fire, or to kill every last one of them.
                let winner = players[0];
                    champion = creatures.champion_from(winner);
                    let rep = creatures
                        .list
                        .iter()
                        .find(|c| c.species == winner)
                        .unwrap();
                    let end = rep
                        .name
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(rep.name.len());
                    let name = String::from_utf8_lossy(&rep.name[..end]).to_string();
                    // The diplomatic record of the winner: the alliances it
                    // held, the wars it fought and the treaties it signed.
                    let mut alliances = 0u32;
                    let mut wars = 0u32;
                    let mut treaties = 0u32;
                    for r in creatures.diplomacy.relations.iter() {
                        if r.a == winner || r.b == winner {
                            if r.state == crate::creature::RelationState::Allied {
                                alliances += 1;
                            }
                            if r.state == crate::creature::RelationState::War {
                                wars += 1;
                            }
                            treaties += r.treaties.len() as u32;
                        }
                    }
                    // What the winning lineage evolved and worked out.
                    let traits: Vec<&'static str> = crate::creature::trait_list(rep.traits)
                        .iter()
                        .map(|t| t.name())
                        .collect();
                    let discoveries: Vec<&'static str> =
                        crate::creature::discovery_list(creatures.species_discoveries(winner))
                            .iter()
                            .map(|d| d.name())
                            .collect();
                    victory = Some(VictoryScreen {
                        winner: name,
                        color: rep.team_color,
                        survivors: creatures.list.len() as u32,
                        generation: creatures.stats.max_generation,
                        revival,
                        timer: 1.0,
                        extinction: false,
                        lost: false,
                        zorgs,
                        alliances,
                        wars,
                        treaties,
                        truces: creatures.diplomacy.stats.truces,
                        traits,
                        discoveries,
                        deep_sealed: world.zorg_cave_sealed,
                    });
                    paused = true;
                }
        }

        // Count down, then restart the world from the winning genome.
        if let Some(v) = victory.as_mut() {
            v.timer -= dt;
            if v.timer <= 0.0 {
                // A run that was lost to the ZORGs is not a win, but the
                // next revival still starts: the last player lineage gets
                // another go at the deep.
                let won = !v.extinction && !v.lost;
                let next_seed = get_random_seed();
                world = World::new(W, H, next_seed);
                world.seed();
                creatures = CreatureSystem::new(next_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
                match champion.as_ref() {
                    Some(genome) => creatures.populate_revival(&world, 36, genome),
                    None => creatures.populate(&world, 64),
                }
                if won {
                    revival += 1;
                }
                // An extinct run still passes the torch to the last lineage.
                if champion.is_none() {
                    champion = fallback_champion.take();
                }
                last_species = None;
                fallback_champion = None;
                cam_x = 0.0;
                cam_y = 0.0;
                view_x = 0.0;
                view_y = 0.0;
                zoom = MIN_ZOOM;
                paused = false;
                force_render = true;
                telemetry.reset(revival);
                unsafe {
                    ACCUM = 0.0;
                }
                victory = None;
            }
        }


        let (mx, my) = mouse_position();
        let scale_x = screen_width() / view_w;
        let scale_y = screen_height() / view_h;
        let world_x = view_x + mx / scale_x;
        let world_y = view_y + my / scale_y;
        let cell_x = world_x as i32;
        let cell_y = world_y as i32;

        ui.hover = HoverTarget::None;
        if cell_x >= 0 && cell_y >= 0 && cell_x < W as i32 && cell_y < H as i32 {
            let elem = world.get(cell_x, cell_y);
            if elem != Elem::Empty {
                ui.discover_element(elem);
                ui.hover = HoverTarget::Element { x: cell_x, y: cell_y, elem };
            }
        }

        // Check creature hover (draw over elements)
        for c in &creatures.list {
            let dx = world_x - c.x;
            let dy = world_y - c.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < c.size.max(3.0) {
                ui.discover_creature(c.id);
                ui.hover = HoverTarget::Creature { id: c.id };
                break;
            }
        }

        // Render only the visible slice of the world into a region texture.
        // At high zoom this is a tiny fraction of the world, which keeps the
        // frame time low no matter how big the world gets.
        let vis_x0 = (view_x.floor() as i32).max(0);
        let vis_y0 = (view_y.floor() as i32).max(0);
        let vis_x1 = ((view_x + view_w).ceil() as i32).min(W as i32);
        let vis_y1 = ((view_y + view_h).ceil() as i32).min(H as i32);
        let vis_w = (vis_x1 - vis_x0).max(1);
        let vis_h = (vis_y1 - vis_y0).max(1);

        if region_w != vis_w || region_h != vis_h {
            region_w = vis_w;
            region_h = vis_h;
            region_pixels = vec![0u8; (region_w * region_h * 4) as usize];
            // Start from a dark colour, not white: the region is re-rendered
            // below, but a resized region is briefly shown before that
            // happens, and white made zooming flash a white screen.
            region_image = Image::gen_image_color(region_w as u16, region_h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
            region_texture = Texture2D::from_image(&region_image);
            region_texture.set_filter(FilterMode::Nearest);
            // The region has to be repainted straight away, whatever the
            // simulation is doing.
            force_render = true;
        }

        // Re-render the world only when the simulation actually advanced (or
        // after a reroll / revival). Pausing and panning then costs nothing.
        if steps_taken > 0 || force_render {
            renderer.render_region(
                &world,
                &creatures,
                &mut region_pixels,
                vis_x0,
                vis_y0,
                vis_w,
                vis_h,
                now,
                creatures.day_night,
                creatures.season,
                world.weather,
            );
            region_image
                .bytes
                .copy_from_slice(&region_pixels);
            region_texture.update(&region_image);
            force_render = false;
            mini_dirty = true;
        }

        // Refresh the minimap terrain a few times a second (sampled straight
        // from the world, so it works no matter what is on screen).
        frame_counter += 1;
        if mini_dirty && frame_counter % 4 == 0 {
            for my in 0..MINI_H {
                for mx in 0..MINI_W {
                    let sx = (mx * MINI_STEP).min(W - 1);
                    let sy = (my * MINI_STEP).min(H - 1);
                    let cell = sy * W + sx;
                    let d = (my * MINI_W + mx) * 4;
                    let k = world.kind[cell];
                    if k == Elem::Empty as u8 {
                        mini_image.bytes[d] = 12;
                        mini_image.bytes[d + 1] = 18;
                        mini_image.bytes[d + 2] = 30;
                    } else {
                        let c = world.col[cell];
                        mini_image.bytes[d] = (c[0] as u32 * 3 / 4) as u8;
                        mini_image.bytes[d + 1] = (c[1] as u32 * 3 / 4) as u8;
                        mini_image.bytes[d + 2] = (c[2] as u32 * 3 / 4) as u8;
                    }
                    mini_image.bytes[d + 3] = 255;
                }
            }
            mini_texture.update(&mini_image);
            mini_dirty = false;
        }

        // Draw the rendered region, positioned so it lines up with the camera.
        let reg_scale_x = screen_width() / view_w;
        let reg_scale_y = screen_height() / view_h;
        draw_texture_ex(
            &region_texture,
            (vis_x0 as f32 - view_x) * reg_scale_x,
            (vis_y0 as f32 - view_y) * reg_scale_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(
                    region_w as f32 * reg_scale_x,
                    region_h as f32 * reg_scale_y,
                )),
                ..Default::default()
            },
        );

        // Atmospheric vignette.
        draw_vignette();
        // The things that float in the air: dust, embers, spores, bubbles.
        draw_atmosphere(&world, view_x, view_y, view_w, view_h, now, creatures.day_night);
        draw_actors(&creatures, &mut world, &ui.hover, view_x, view_y, view_w, view_h, zoom, now);
        effects.draw(view_x, view_y, screen_width() / view_w, screen_height() / view_h);

        draw_minimap(
            &creatures,
            &world,
            view_x,
            view_y,
            view_w,
            view_h,
            &mini_texture,
        );
        draw_hud(&world, &creatures, view_x, view_y, seed, sim_speed, paused, zoom, revival);
        if screen_width() >= 1080.0 {
            draw_telemetry(&telemetry, screen_width() - 332.0, 18.0);
        }

        // The diplomacy panel: what the species have done to each other.
        // It is anchored under the HUD text rather than to the bottom of
        // the screen, so it stays in one place whatever the window size.
        if ui.diplomacy_open {
            draw_diplomacy_panel(&creatures, 18.0, 248.0);
        }

        // Draw hover tooltip
        draw_tooltip(&world, &creatures, &ui, mx, my);

        // Draw knowledge window
        if ui.knowledge_open {
            draw_knowledge_window(&world, &creatures, &ui);
        }

        // The tech tree: the whole tree, and where the world has got to.
        if ui.tech_open {
            draw_tech_tree(&creatures, &mut ui);
        }
        draw_tech_panel(&creatures);

        // The mind inspector: what the cleverest animals are thinking.
        if ui.minds_open {
            draw_minds(&creatures, ui.minds_index);
        }

        // Victory / extinction overlay sits on top of everything.
        if let Some(v) = &victory {
            draw_victory(v);
        }

        // The screenshot hook: write a picture of the game and stop.
        if let (Some(path), Some(at)) = (&snapshot, snapshot_at) {
            let elapsed = now_snapshot().duration_since(at).as_secs_f32();
            if elapsed >= snapshot_after {
                eprintln!("screenshot after {:.1}s", elapsed);
                save_screenshot(path);
                break;
            }
        }

        next_frame().await;
    }
}
