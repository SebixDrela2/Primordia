mod creature;
mod elements;
mod render;
mod rng;
mod world;

use creature::{CreatureSystem, RevivalGenome, Role};
use elements::Elem;
use macroquad::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use world::World;

const W: usize = 960;
const H: usize = 540;
const VIEW_W: usize = 480;
const VIEW_H: usize = 270;

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

// ---------------------------------------------------------------------------
// Hover / knowledge state
// ---------------------------------------------------------------------------

enum HoverTarget {
    None,
    Element { x: i32, y: i32, elem: Elem },
    Creature { id: u32 },
}

struct UiState {
    hover: HoverTarget,
    knowledge_open: bool,
    discovered_elements: Vec<u8>,
    discovered_creatures: Vec<u32>,
}

impl UiState {
    fn new() -> Self {
        Self {
            hover: HoverTarget::None,
            knowledge_open: false,
            discovered_elements: Vec::new(),
            discovered_creatures: Vec::new(),
        }
    }

    fn discover_element(&mut self, e: Elem) {
        let id = e as u8;
        if !self.discovered_elements.contains(&id) {
            self.discovered_elements.push(id);
        }
    }

    fn discover_creature(&mut self, id: u32) {
        if !self.discovered_creatures.contains(&id) {
            self.discovered_creatures.push(id);
        }
    }
}

/// Rolling ecology history for the sparkline + event feed.
struct Telemetry {
    pop: Vec<f32>,
    species: Vec<f32>,
    events: Vec<(String, f32)>,
    last_sample: f32,
    known: Vec<(u32, String)>,
    last_season: u8,
    initialized: bool,
}

impl Telemetry {
    fn new() -> Self {
        Self {
            pop: Vec::new(),
            species: Vec::new(),
            events: Vec::new(),
            last_sample: -1.0,
            known: Vec::new(),
            last_season: 0,
            initialized: false,
        }
    }

    fn reset(&mut self, revival: u32) {
        self.pop.clear();
        self.species.clear();
        self.known.clear();
        self.events.clear();
        self.last_sample = -1.0;
        self.last_season = 0;
        self.initialized = false;
        self.push(format!("REVIVAL {}  -  new lineage", revival));
    }

    fn push(&mut self, msg: String) {
        self.events.push((msg, 0.0));
        if self.events.len() > 8 {
            self.events.remove(0);
        }
    }

    fn tick(&mut self, dt: f32) {
        for e in &mut self.events {
            e.1 += dt;
        }
    }

    fn observe(&mut self, now: f32, creatures: &CreatureSystem) {
        if self.last_sample < 0.0 || now - self.last_sample >= 0.4 {
            self.last_sample = now;

            self.pop.push(creatures.stats.population as f32);
            self.species.push(creatures.stats.species as f32);
            if self.pop.len() > 240 {
                self.pop.remove(0);
                self.species.remove(0);
            }

            let mut current: Vec<(u32, String)> = Vec::new();
            for c in &creatures.list {
                if !current.iter().any(|(id, _)| *id == c.species) {
                    let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
                    current.push((
                        c.species,
                        String::from_utf8_lossy(&c.name[..end]).to_string(),
                    ));
                }
            }

            if self.initialized {
                let extinct: Vec<String> = self
                    .known
                    .iter()
                    .filter(|(id, _)| !current.iter().any(|(cid, _)| cid == id))
                    .map(|(id, name)| format!("SPECIES {} EXTINCT  -  {}", id, name))
                    .collect();
                for msg in extinct {
                    self.push(msg);
                }
            }
            self.known = current;
            self.initialized = true;

            if creatures.season != self.last_season {
                self.last_season = creatures.season;
                let s = match creatures.season {
                    0 => "SPRING",
                    1 => "SUMMER",
                    2 => "AUTUMN",
                    _ => "WINTER",
                };
                self.push(format!("{} BEGINS", s));
            }
        }
    }
}

/// Full-screen state shown when one species has wiped out all others (or the
/// whole population died out).
struct VictoryScreen {
    winner: String,
    color: [u8; 3],
    survivors: u32,
    generation: u32,
    revival: u32,
    timer: f32,
    extinction: bool,
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = get_random_seed();

    let mut world = World::new(W, H, seed);
    world.seed();

    let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
    creatures.populate(&world, 36);

    let mut pixels = vec![0u8; W * H * 4];
    let mut image = Image::gen_image_color(W as u16, H as u16, WHITE);
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);

    // Camera position (top-left of view in world coordinates)
    let mut cam_x: f32 = 0.0;
    let mut cam_y: f32 = 0.0;
    let cam_speed = 300.0;

    static mut ACCUM: f64 = 0.0;

    let mut ui = UiState::new();

    // Simulation speed multiplier (1.0 = normal, press + / - to change)
    let mut sim_speed: f32 = 1.0;
    // P toggles pause
    let mut paused = false;
    // Scroll wheel zoom (1.0 = default view)
    let mut zoom: f32 = 1.0;
    // Endless "revival" lineage: run 1 is the original population, then each
    // victory seeds a new world from the winning species' genome.
    let mut revival: u32 = 1;
    let mut champion: Option<RevivalGenome> = None;
    let mut victory: Option<VictoryScreen> = None;
    let mut telemetry = Telemetry::new();
    telemetry.reset(1);
    let mut force_render = true;

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
            creatures.populate(&world, 36);
            cam_x = 0.0;
            cam_y = 0.0;
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

        // + / - : adjust simulation speed
        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
            sim_speed = (sim_speed * 2.0).min(16.0);
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
        let (_, wheel_y) = mouse_wheel();
        if wheel_y != 0.0 {
            let direction = wheel_y.signum();
            let (mx, my) = mouse_position();
            let old_sx = screen_width() / view_w;
            let old_sy = screen_height() / view_h;
            let anchor_x = cam_x + mx / old_sx;
            let anchor_y = cam_y + my / old_sy;

            zoom = (zoom * 1.12f32.powf(direction)).clamp(0.5, 6.0);
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

        let sim_dt = (get_frame_time().min(0.1) as f64) * (sim_speed as f64);
        let fixed_step = 1.0 / 60.0;
        let max_steps = (4.0 * sim_speed) as u32;
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
        }
        telemetry.tick(dt);

        // ---- Species domination / extinction -> victory screen ----
        if victory.is_none() {
            if creatures.list.is_empty() {
                victory = Some(VictoryScreen {
                    winner: "NO SURVIVORS".to_string(),
                    color: [90, 100, 110],
                    survivors: 0,
                    generation: creatures.stats.max_generation,
                    revival,
                    timer: 8.0,
                    extinction: true,
                });
                paused = true;
            } else {
                let mut species: Vec<u32> = creatures.list.iter().map(|c| c.species).collect();
                species.sort_unstable();
                species.dedup();
                if species.len() == 1 {
                    let winner = species[0];
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
                    victory = Some(VictoryScreen {
                        winner: name,
                        color: rep.team_color,
                        survivors: creatures.list.len() as u32,
                        generation: creatures.stats.max_generation,
                        revival,
                        timer: 8.0,
                        extinction: false,
                    });
                    paused = true;
                }
            }
        }

        // Count down, then restart the world from the winning genome.
        if let Some(v) = victory.as_mut() {
            v.timer -= dt;
            if v.timer <= 0.0 {
                let won = !v.extinction;
                let next_seed = get_random_seed();
                world = World::new(W, H, next_seed);
                world.seed();
                creatures = CreatureSystem::new(next_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
                match champion.as_ref() {
                    Some(genome) => creatures.populate_revival(&world, 36, genome),
                    None => creatures.populate(&world, 36),
                }
                if won {
                    revival += 1;
                }
                cam_x = 0.0;
                cam_y = 0.0;
                zoom = 1.0;
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
        let world_x = cam_x + mx / scale_x;
        let world_y = cam_y + my / scale_y;
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

        // Re-render the world only when the simulation actually advanced (or
        // after a reroll / revival). Pausing and panning then costs nothing.
        if steps_taken > 0 || force_render {
            render::render_world(
                &world,
                &creatures,
                &mut pixels,
                now,
                creatures.day_night,
                creatures.season,
            );
            image.bytes.copy_from_slice(&pixels);
            texture.update(&image);
            force_render = false;
        }

        // Draw only the camera view portion
        let src_x = cam_x;
        let src_y = cam_y;
        let src_w = view_w;
        let src_h = view_h;

        draw_texture_ex(
            &texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(src_x, src_y, src_w, src_h)),
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );

        // Draw creature bodies and names (only those in view)
        let show_all_names = zoom >= 2.5;
        let hovered_creature = match ui.hover {
            HoverTarget::Creature { id } => Some(id),
            _ => None,
        };
        let scr_w = screen_width();
        let scr_h = screen_height();

        for c in &creatures.list {
            let in_view = (c.x) >= cam_x
                && (c.x) < (cam_x + view_w)
                && (c.y) >= cam_y
                && (c.y) < (cam_y + view_h);
            if !in_view {
                continue;
            }

            let scale_x = scr_w / view_w;
            let scale_y = scr_h / view_h;
            let s = scale_x.min(scale_y);
            let sx = (c.x - cam_x) * scale_x;
            let sy = (c.y - cam_y) * scale_y;
            let r = (c.size * s).max(2.5);
            let base = Color::new(
                c.team_color[0] as f32 / 255.0,
                c.team_color[1] as f32 / 255.0,
                c.team_color[2] as f32 / 255.0,
                0.92,
            );
            let core = Color::new(1.0, 1.0, 0.95, 0.92);

            // Role silhouettes make the colony readable at a glance.
            match c.role {
                Role::Soldier => {
                    let angle = c.vy.atan2(c.vx) - std::f32::consts::FRAC_PI_2;
                    draw_poly(sx, sy, 3, r * 1.3, angle, base);
                    draw_circle(sx, sy, (r * 0.32).max(1.0), core);
                }
                Role::Queen => {
                    draw_circle(sx, sy, r * 1.15, base);
                    draw_circle_lines(sx, sy, r * 1.65, 1.6, base);
                    draw_circle(sx, sy, (r * 0.4).max(1.2), Color::new(1.0, 0.93, 0.55, 0.95));
                }
                Role::Priest | Role::Shaman => {
                    draw_poly(sx, sy, 4, r * 1.25, std::f32::consts::FRAC_PI_4, base);
                    draw_circle(sx, sy, (r * 0.3).max(1.0), core);
                }
                Role::Builder | Role::Merchant => {
                    draw_poly(sx, sy, 6, r * 1.18, 0.0, base);
                    draw_circle(sx, sy, (r * 0.28).max(1.0), core);
                }
                Role::Explorer => {
                    draw_poly(sx, sy, 3, r * 1.2, std::f32::consts::PI, base);
                    draw_circle(sx, sy, (r * 0.28).max(1.0), core);
                }
                Role::Worker => {
                    draw_circle(sx, sy, r, base);
                    draw_circle(sx, sy, (r * 0.3).max(1.0), core);
                }
            }

            // Damage bar (only when hurt).
            let hp = (c.health / 100.0).clamp(0.0, 1.0);
            if hp < 0.999 {
                let bw = (r * 2.6).max(6.0);
                let by = sy - r - 6.0;
                draw_rectangle(sx - bw / 2.0, by, bw, 2.0, Color::new(0.0, 0.0, 0.0, 0.55));
                let col = if hp > 0.6 {
                    Color::new(0.32, 0.85, 0.42, 0.9)
                } else if hp > 0.3 {
                    Color::new(0.95, 0.75, 0.22, 0.9)
                } else {
                    Color::new(0.92, 0.26, 0.2, 0.9)
                };
                draw_rectangle(sx - bw / 2.0, by, bw * hp, 2.0, col);
            }

            // Hover ring.
            if hovered_creature == Some(c.id) {
                draw_circle_lines(
                    sx,
                    sy,
                    (r * 2.5).max(8.0),
                    2.0,
                    Color::new(1.0, 1.0, 1.0, 0.8),
                );
            }

            // Names only for queens, the hovered creature, tiny populations,
            // or when zoomed in. Keeps the view clean and avoids text churn.
            let show_name = show_all_names
                || c.role == Role::Queen
                || hovered_creature == Some(c.id)
                || creatures.list.len() <= 12;
            if show_name {
                let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
                let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");
                if !name_str.is_empty() {
                    let font = (9.0 * s).clamp(9.0, 16.0);
                    let dims = measure_text(name_str, None, font as u16, 1.0);
                    draw_text(
                        name_str,
                        sx - dims.width / 2.0,
                        sy - r - 7.0,
                        font,
                        Color::new(base.r, base.g, base.b, 0.95),
                    );
                }
            }
        }

        draw_minimap(&creatures, cam_x, cam_y, view_w, view_h);
        draw_hud(&world, &creatures, cam_x, cam_y, seed, sim_speed, paused, zoom, revival);
        if screen_width() >= 1080.0 {
            draw_telemetry(&telemetry, screen_width() - 332.0, 18.0);
        }

        // Draw hover tooltip
        draw_tooltip(&world, &creatures, &ui, mx, my);

        // Draw knowledge window
        if ui.knowledge_open {
            draw_knowledge_window(&world, &creatures, &ui);
        }

        // Victory / extinction overlay sits on top of everything.
        if let Some(v) = &victory {
            draw_victory(v);
        }

        next_frame().await;
    }
}

/// Bottom-left minimap: species dots plus the current camera viewport.
fn draw_minimap(
    creatures: &CreatureSystem,
    cam_x: f32,
    cam_y: f32,
    view_w: f32,
    view_h: f32,
) {
    let mw = 180.0f32;
    let mh = mw * (H as f32 / W as f32);
    let x = 18.0;
    let y = screen_height() - mh - 44.0;
    if y < 180.0 {
        return; // too little vertical space; don't cover the HUD
    }

    draw_rectangle(
        x - 3.0,
        y - 3.0,
        mw + 6.0,
        mh + 6.0,
        Color::new(0.02, 0.03, 0.05, 0.82),
    );
    draw_rectangle(x, y, mw, mh, Color::new(0.04, 0.06, 0.09, 0.9));
    draw_rectangle_lines(x, y, mw, mh, 1.0, Color::new(0.3, 0.5, 0.7, 0.55));

    for c in &creatures.list {
        let px = x + (c.x / W as f32).clamp(0.0, 1.0) * mw;
        let py = y + (c.y / H as f32).clamp(0.0, 1.0) * mh;
        let color = Color::new(
            c.team_color[0] as f32 / 255.0,
            c.team_color[1] as f32 / 255.0,
            c.team_color[2] as f32 / 255.0,
            0.95,
        );
        draw_rectangle(px - 1.0, py - 1.0, 2.0, 2.0, color);
    }

    let vx = x + (cam_x / W as f32) * mw;
    let vy = y + (cam_y / H as f32) * mh;
    draw_rectangle_lines(
        vx,
        vy,
        (view_w / W as f32) * mw,
        (view_h / H as f32) * mh,
        1.5,
        Color::new(0.13, 0.83, 0.93, 0.85),
    );

    draw_text("MAP", x + 5.0, y + 13.0, 11.0, Color::new(0.6, 0.7, 0.8, 0.6));
}

/// Top-right ecology panel: population/species sparkline plus recent events.
fn draw_telemetry(t: &Telemetry, x: f32, y: f32) {
    let w = 314.0f32;
    let h = 226.0f32;

    draw_rectangle(x, y, w, h, Color::new(0.03, 0.05, 0.08, 0.84));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.13, 0.83, 0.93, 0.35));

    let pop_now = t.pop.last().copied().unwrap_or(0.0);
    let sp_now = t.species.last().copied().unwrap_or(0.0);
    draw_text("ECOLOGY", x + 12.0, y + 20.0, 14.0, Color::new(0.13, 0.83, 0.93, 0.9));
    draw_text(
        &format!("pop {:.0}", pop_now),
        x + 96.0,
        y + 20.0,
        12.0,
        Color::new(0.35, 0.9, 1.0, 0.9),
    );
    draw_text(
        &format!("species {:.0}", sp_now),
        x + 184.0,
        y + 20.0,
        12.0,
        Color::new(0.95, 0.78, 0.35, 0.9),
    );

    let gx = x + 12.0;
    let gy = y + 30.0;
    let gw = w - 24.0;
    let gh = 78.0;
    draw_rectangle(gx, gy, gw, gh, Color::new(0.02, 0.03, 0.05, 0.9));
    draw_rectangle_lines(gx, gy, gw, gh, 1.0, Color::new(0.2, 0.3, 0.4, 0.45));

    let n = t.pop.len().min(160);
    if n >= 2 {
        let start = t.pop.len() - n;
        let pop_max = t.pop[start..]
            .iter()
            .copied()
            .fold(1.0f32, f32::max)
            .max(10.0);
        let step = gw / (n - 1) as f32;
        let pop_color = Color::new(0.25, 0.85, 0.98, 0.85);
        let sp_color = Color::new(0.98, 0.78, 0.32, 0.8);

        let mut prev_pop: Option<(f32, f32)> = None;
        let mut prev_sp: Option<(f32, f32)> = None;
        for i in 0..n {
            let px = gx + i as f32 * step;
            let pv = t.pop[start + i];
            let sv = t.species[start + i];
            let py = gy + gh - (pv / pop_max).clamp(0.0, 1.0) * (gh - 6.0) - 3.0;
            let sy = gy + gh - (sv / 36.0).clamp(0.0, 1.0) * (gh - 6.0) - 3.0;
            if let Some((ox, oy)) = prev_pop {
                draw_line(ox, oy, px, py, 1.5, pop_color);
            }
            if let Some((ox, oy)) = prev_sp {
                draw_line(ox, oy, px, sy, 1.5, sp_color);
            }
            prev_pop = Some((px, py));
            prev_sp = Some((px, sy));
        }
    } else {
        draw_text(
            "collecting samples...",
            gx + 8.0,
            gy + 20.0,
            11.0,
            Color::new(0.5, 0.58, 0.66, 0.8),
        );
    }

    let ey = gy + gh + 18.0;
    draw_text("EVENTS", x + 12.0, ey, 11.0, Color::new(0.55, 0.62, 0.7, 0.85));
    let mut yy = ey + 15.0;
    let mut any = false;
    for (msg, age) in t.events.iter().rev() {
        let alpha = (1.0 - age / 16.0).clamp(0.25, 1.0);
        draw_text(
            msg,
            x + 12.0,
            yy,
            11.0,
            Color::new(0.8, 0.86, 0.92, alpha),
        );
        yy += 14.0;
        any = true;
        if yy > y + h - 8.0 {
            break;
        }
    }
    if !any {
        draw_text(
            "hover to inspect - K for knowledge",
            x + 12.0,
            yy,
            11.0,
            Color::new(0.5, 0.58, 0.66, 0.7),
        );
    }
}

fn draw_hud(world: &World, creatures: &CreatureSystem, cam_x: f32, cam_y: f32, seed: u64, sim_speed: f32, paused: bool, zoom: f32, revival: u32) {
    let stats = &creatures.stats;

    draw_text(
        "PRIMORDIA",
        18.0,
        34.0,
        26.0,
        Color::new(0.13, 0.83, 0.93, 0.9),
    );
    draw_rectangle(18.0, 44.0, 64.0, 2.0, Color::new(0.13, 0.83, 0.93, 0.55));

    let season_name = match creatures.season { 0=>"Spring", 1=>"Summer", 2=>"Autumn", _=>"Winter" };
    let time_of_day = if creatures.day_night < 0.25 { "Dawn" } else if creatures.day_night < 0.5 { "Morning" } else if creatures.day_night < 0.75 { "Dusk" } else { "Night" };

    let line1 = format!(
        "seed {}   REVIVAL {}   {} organisms   {} species   gen {}",
        seed, revival, stats.population, stats.species, stats.max_generation
    );
    let line2 = format!(
        "{} particles   avg energy {:.0}   fitness {:.2}",
        world.census.particles, stats.avg_energy, stats.avg_fitness
    );
    let line3 = format!(
        "{}  {}  temp {:.0}C  mana {:.0}  spells {}  trades {}  SIMx{:.1}  zoom {:.1}x",
        season_name, time_of_day, creatures.temperature, stats.avg_mana, stats.spells_cast, stats.trades, sim_speed, zoom
    );
    let line4 = format!(
        "camera: ({:.0},{:.0})   fps {}   births {}   deaths {}  structs {}  mounts {}",
        cam_x, cam_y, get_fps(), stats.births, stats.deaths, stats.structures, stats.mounts
    );
    let line5 = format!(
        "hunger {:.0}  thirst {:.0}  stamina {:.0}  diseases {}",
        stats.avg_hunger, stats.avg_thirst, stats.avg_stamina, stats.diseases
    );

    draw_text(&line1, 18.0, 66.0, 17.0, Color::new(0.85, 0.9, 0.95, 0.8));
    draw_text(&line2, 18.0, 88.0, 17.0, Color::new(0.6, 0.66, 0.74, 0.8));
    draw_text(&line3, 18.0, 110.0, 17.0, Color::new(0.7, 0.8, 0.6, 0.85));
    draw_text(&line4, 18.0, 132.0, 17.0, Color::new(0.45, 0.5, 0.58, 0.8));
    draw_text(&line5, 18.0, 154.0, 17.0, Color::new(0.6, 0.5, 0.58, 0.8));

    let help = "WASD/Arrows: move  |  scroll: zoom  |  P: pause  |  R: reroll  |  K: knowledge  |  +/-: sim speed  |  hover: inspect  |  ESC: quit";
    let dims = measure_text(help, None, 15, 1.0);
    draw_text(
        help,
        screen_width() - dims.width - 18.0,
        screen_height() - 16.0,
        15.0,
        Color::new(0.4, 0.45, 0.52, 0.7),
    );

    if paused {
        let label = "PAUSED";
        let dims = measure_text(label, None, 34, 1.0);
        let x = (screen_width() - dims.width) / 2.0;
        let y = screen_height() / 2.0 + 12.0;
        draw_rectangle(x - 18.0, y - 34.0, dims.width + 36.0, 48.0, Color::new(0.03, 0.04, 0.07, 0.72));
        draw_rectangle_lines(x - 18.0, y - 34.0, dims.width + 36.0, 48.0, 2.0, Color::new(0.13, 0.83, 0.93, 0.9));
        draw_text(label, x, y, 34.0, Color::new(0.13, 0.83, 0.93, 0.95));
        let hint = "(press P to resume)";
        let hd = measure_text(hint, None, 15, 1.0);
        draw_text(hint, (screen_width() - hd.width) / 2.0, y + 22.0, 15.0, Color::new(0.7, 0.78, 0.85, 0.85));
    }
}

/// Draw a tooltip at the mouse position showing what's under the cursor.
fn draw_tooltip(_world: &World, creatures: &CreatureSystem, ui: &UiState, mx: f32, my: f32) {
    match &ui.hover {
        HoverTarget::None => return,
        HoverTarget::Element { x, y, elem } => {
            let e = *elem;
            let name = e.name();
            let desc = e.description();
            let category = e.category();
            let density = e.density();
            let tox = e.toxicity();
            let nut = e.nutrition();

            let mut lines: Vec<String> = Vec::new();
            lines.push(format!("{}", name));
            lines.push(format!("[{}]  pos=({},{})", category, x, y));
            lines.push(format!("density={}  toxicity={:.0}  nutrition={:.1}", density, tox, nut));
            if e.is_hazardous() {
                lines.push("HAZARDOUS".to_string());
            }
            if e.is_solid() {
                lines.push("solid".to_string());
            }
            if e.flammable() {
                lines.push("flammable".to_string());
            }
            lines.push(desc.to_string());

            draw_tooltip_box(lines, mx, my);
        }
        HoverTarget::Creature { id } => {
            let c = creatures.list.iter().find(|c| c.id == *id);
            let Some(c) = c else { return };

            let role_str = match c.role {
                creature::Role::Worker => "Worker",
                creature::Role::Soldier => "Soldier",
                creature::Role::Queen => "Queen",
                creature::Role::Priest => "Priest",
                creature::Role::Merchant => "Merchant",
                creature::Role::Builder => "Builder",
                creature::Role::Explorer => "Explorer",
                creature::Role::Shaman => "Shaman",
            };

            let tool_str = match c.tool {
                creature::Tool::None => "none".to_string(),
                creature::Tool::Pickaxe => "pickaxe".to_string(),
                creature::Tool::Sword => "sword".to_string(),
                creature::Tool::Hammer => "hammer".to_string(),
                creature::Tool::Drill => "drill".to_string(),
                creature::Tool::Scanner => "scanner".to_string(),
                creature::Tool::Bow => "bow".to_string(),
                creature::Tool::Spear => "spear".to_string(),
                creature::Tool::Shield => "shield".to_string(),
                creature::Tool::Staff => "staff".to_string(),
                creature::Tool::Amulet => "amulet".to_string(),
                creature::Tool::Backpack => "backpack".to_string(),
                creature::Tool::Cart => "cart".to_string(),
                creature::Tool::Boat => "boat".to_string(),
                creature::Tool::Glider => "glider".to_string(),
            };

            let mut end = 10;
            for i in 0..10 {
                if c.name[i] == 0 {
                    end = i;
                    break;
                }
            }
            let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");

            let mut lines: Vec<String> = Vec::new();
            lines.push(format!("{}  (id {})", name_str, c.id));
            lines.push(format!("species {}  gen {}  role {}", c.species, c.generation, role_str));
            lines.push(format!("energy {:.0}/{:.0}  health {:.0}  fitness {:.2}",
                c.energy, c.max_energy, c.health, c.fitness));
            lines.push(format!("age {}  size {:.1}  tech {}", c.age, c.size, c.tech_level));
            lines.push(format!("tool {}  carried {:.0}", tool_str, c.carried));
            lines.push(format!("kills {}  offspring {}", c.kills, c.offspring));
            if c.disease != creature::Disease::None {
                lines.push(format!("disease: {:?}", c.disease));
            }

            draw_tooltip_box(lines, mx, my);
        }
    }
}

fn draw_tooltip_box(lines: Vec<String>, mx: f32, my: f32) {
    if lines.is_empty() {
        return;
    }

    // Measure the widest line
    let mut max_w: f32 = 0.0;
    let mut total_h = 0.0;
    for line in &lines {
        let dims = measure_text(line, None, 14, 1.0);
        max_w = max_w.max(dims.width);
        total_h += 16.0;
    }
    total_h += 8.0;

    let pad = 6.0;
    let box_w = max_w + pad * 2.0;
    let box_h = total_h + pad * 2.0;

    // Position below-right of cursor, clamped to screen
    let mut bx = mx + 14.0;
    let mut by = my + 14.0;
    if bx + box_w > screen_width() {
        bx = mx - box_w - 14.0;
    }
    if by + box_h > screen_height() {
        by = my - box_h - 14.0;
    }
    bx = bx.clamp(4.0, screen_width() - box_w - 4.0);
    by = by.clamp(4.0, screen_height() - box_h - 4.0);

    // Background
    draw_rectangle(bx, by, box_w, box_h, Color::new(0.05, 0.05, 0.08, 0.85));
    draw_rectangle_lines(bx, by, box_w, box_h, 1.5, Color::new(0.3, 0.5, 0.7, 0.9));

    // Title (first line)
    let title_color = Color::new(0.7, 0.85, 1.0, 1.0);
    draw_text(&lines[0], bx + pad, by + pad + 12.0, 14.0, title_color);

    // Remaining lines
    let mut yy = by + pad + 12.0 + 16.0;
    for line in &lines[1..] {
        draw_text(line, bx + pad, yy, 12.0, Color::new(0.85, 0.9, 0.95, 0.85));
        yy += 15.0;
    }
}

/// Draw the knowledge window: an encyclopedia of all discovered elements and creatures.
fn draw_knowledge_window(_world: &World, creatures: &CreatureSystem, ui: &UiState) {
    let w = 520.0;
    let h = screen_height() * 0.85;
    let x = screen_width() - w - 20.0;
    let y = 40.0;

    // Background
    draw_rectangle(x, y, w, h, Color::new(0.06, 0.06, 0.09, 0.93));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.3, 0.5, 0.7, 0.9));

    draw_text("KNOWLEDGE", x + 16.0, y + 28.0, 20.0, Color::new(0.7, 0.85, 1.0, 1.0));
    draw_rectangle_lines(x + 16.0, y + 34.0, 100.0, 2.0, 1.0, Color::new(0.3, 0.5, 0.7, 0.6));
    draw_text("press K to close", x + 16.0, y + 52.0, 14.0, Color::new(0.5, 0.55, 0.6, 0.7));

    let mut yy = y + 80.0;

    // ---- Elements section ----
    if !ui.discovered_elements.is_empty() {
        draw_text("ELEMENTS DISCOVERED", x + 16.0, yy, 16.0, Color::new(0.85, 0.9, 0.95, 0.9));
        yy += 22.0;

        // Group by category
        let mut cats: Vec<(&str, Vec<u8>)> = Vec::new();
        for &id in &ui.discovered_elements {
            let e = Elem::from_u8(id);
            let cat = e.category();
            if let Some(pos) = cats.iter_mut().find(|(c, _)| *c == cat) {
                pos.1.push(id);
            } else {
                cats.push((cat, vec![id]));
            }
        }

        for (cat, ids) in cats {
            draw_text(cat, x + 24.0, yy, 13.0, Color::new(0.6, 0.7, 0.8, 0.8));
            yy += 18.0;
            for &id in &ids {
                let e = Elem::from_u8(id);
                let name = e.name();
                let desc = e.description();
                let color = e.color();
                // color swatch
                draw_rectangle(x + 24.0, yy - 2.0, 10.0, 10.0, Color::new(
                    color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0, 0.9,
                ));
                draw_text(name, x + 40.0, yy + 8.0, 13.0, Color::new(0.9, 0.95, 1.0, 0.95));
                // short desc
                let short = if desc.len() > 60 { format!("{}...", &desc[..60]) } else { desc.to_string() };
                draw_text(&short, x + 40.0, yy + 20.0, 11.0, Color::new(0.65, 0.7, 0.75, 0.8));
                yy += 34.0;
            }
            yy += 8.0;
        }
        yy += 16.0;
    }

    // ---- Creatures section ----
    if !ui.discovered_creatures.is_empty() {
        draw_text("CREATURES OBSERVED", x + 16.0, yy, 16.0, Color::new(0.85, 0.9, 0.95, 0.9));
        yy += 22.0;

        for &id in &ui.discovered_creatures {
            let c = creatures.list.iter().find(|c| c.id == id);
            let Some(c) = c else { continue };

            let role_str = match c.role {
                creature::Role::Worker => "Worker",
                creature::Role::Soldier => "Soldier",
                creature::Role::Queen => "Queen",
                creature::Role::Priest => "Priest",
                creature::Role::Merchant => "Merchant",
                creature::Role::Builder => "Builder",
                creature::Role::Explorer => "Explorer",
                creature::Role::Shaman => "Shaman",
            };

            let mut end = 10;
            for i in 0..10 {
                if c.name[i] == 0 {
                    end = i;
                    break;
                }
            }
            let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");

            let swatch_color = c.team_color;
            draw_rectangle(x + 24.0, yy - 2.0, 10.0, 10.0, Color::new(
                swatch_color[0] as f32 / 255.0, swatch_color[1] as f32 / 255.0, swatch_color[2] as f32 / 255.0, 0.9,
            ));
            draw_text(name_str, x + 40.0, yy + 8.0, 13.0, Color::new(0.9, 0.95, 1.0, 0.95));
            draw_text(&format!("species {}  gen {}  role {}  energy {:.0}", c.species, c.generation, role_str, c.energy),
                x + 40.0, yy + 20.0, 11.0, Color::new(0.65, 0.7, 0.75, 0.8));
            yy += 34.0;
        }
    }

    if ui.discovered_elements.is_empty() && ui.discovered_creatures.is_empty() {
        draw_text("Hover over anything in the world to record it here.", x + 24.0, yy, 14.0, Color::new(0.6, 0.65, 0.7, 0.7));
    }
}
fn draw_victory(v: &VictoryScreen) {
    let sw = screen_width();
    let sh = screen_height();

    // Dim the whole simulation.
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.01, 0.02, 0.04, 0.66));

    let w = 600.0f32.min(sw - 60.0);
    let h = 360.0f32.min(sh - 60.0);
    let x = (sw - w) / 2.0;
    let y = (sh - h) / 2.0;
    draw_rectangle(x, y, w, h, Color::new(0.03, 0.05, 0.09, 0.94));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.13, 0.83, 0.93, 0.9));

    let (title, title_color) = if v.extinction {
        ("EXTINCTION", Color::new(0.92, 0.36, 0.3, 1.0))
    } else {
        ("VICTORY", Color::new(0.13, 0.83, 0.93, 1.0))
    };
    let td = measure_text(title, None, 48, 1.0);
    draw_text(title, (sw - td.width) / 2.0, y + 74.0, 48.0, title_color);

    let subtitle = if v.extinction {
        "the last lineage has died out"
    } else {
        "one species stands alone"
    };
    let sd = measure_text(subtitle, None, 17, 1.0);
    draw_text(
        subtitle,
        (sw - sd.width) / 2.0,
        y + 100.0,
        17.0,
        Color::new(0.6, 0.68, 0.76, 0.9),
    );

    // Winning species colour swatch + name.
    let color = Color::new(
        v.color[0] as f32 / 255.0,
        v.color[1] as f32 / 255.0,
        v.color[2] as f32 / 255.0,
        1.0,
    );
    let name = if v.extinction {
        v.winner.clone()
    } else {
        format!("\"{}\"", v.winner)
    };
    let nd = measure_text(&name, None, 30, 1.0);
    let name_x = (sw - nd.width) / 2.0;
    draw_circle(name_x - 20.0, y + 148.0 - 9.0, 8.0, color);
    draw_text(&name, name_x, y + 148.0, 30.0, color);

    let stats = if v.extinction {
        format!("generation {}", v.generation)
    } else {
        format!("{} survivors   generation {}", v.survivors, v.generation)
    };
    let std = measure_text(&stats, None, 16, 1.0);
    draw_text(
        &stats,
        (sw - std.width) / 2.0,
        y + 180.0,
        16.0,
        Color::new(0.78, 0.84, 0.9, 0.9),
    );

    let (l1, l2) = if v.extinction {
        (
            format!("Revival {} ends here. The lineage is lost.", v.revival),
            "A fresh random population will emerge.".to_string(),
        )
    } else {
        (
            format!("Revival {} conquered. Revival {} begins.", v.revival, v.revival + 1),
            "Descendants inherit the winning genome.".to_string(),
        )
    };
    let l1d = measure_text(&l1, None, 16, 1.0);
    draw_text(&l1, (sw - l1d.width) / 2.0, y + 216.0, 16.0, Color::new(0.85, 0.9, 0.95, 0.92));
    let l2d = measure_text(&l2, None, 14, 1.0);
    draw_text(&l2, (sw - l2d.width) / 2.0, y + 238.0, 14.0, Color::new(0.6, 0.66, 0.72, 0.85));

    let countdown = format!("restarting in {:.1}s", v.timer.max(0.0));
    let cd = measure_text(&countdown, None, 14, 1.0);
    draw_text(
        &countdown,
        (sw - cd.width) / 2.0,
        y + h - 26.0,
        14.0,
        Color::new(0.13, 0.83, 0.93, 0.85),
    );
}
