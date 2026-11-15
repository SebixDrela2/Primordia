#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::uninlined_format_args
)]
//! PRIMORDIA - the game itself: the window, the camera and the loop.

use primordia::checks;
use primordia::creature::{CreatureSystem, RevivalGenome, SimEvent};
use primordia::elements::Elem;
use primordia::render;
use primordia::ui::{
    apply_event, dominant_player_species, draw_actors, draw_atmosphere, draw_diplomacy_panel,
    draw_hud, draw_knowledge_window, draw_minds, draw_minimap, draw_tech_panel, draw_tech_tree,
    draw_telemetry, draw_tooltip, draw_victory, draw_vignette, tech_tree_under_pointer, Effects,
    HoverTarget, Outcome, Panel, Telemetry, UiState, VictoryScreen,
};
use primordia::world::World;
use primordia::{H, W};

use macroquad::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

mod rendertest;

const VIEW_W: usize = 480;
const VIEW_H: usize = 270;

const MINI_W: usize = 160;
const MINI_H: usize = 90;
const MINI_STEP: usize = 8;

const MIN_ZOOM: f32 = VIEW_W as f32 / W as f32;
const MAX_ZOOM: f32 = 6.0;

/// The time left over from the last frame, so that the simulation runs
/// at a fixed step however fast the picture is drawn.
static mut ACCUM: f64 = 0.0;

fn now_snapshot() -> std::time::Instant {
    std::time::Instant::now()
}

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
            eprintln!("screenshot: cannot create {path}: {e}");
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
            eprintln!("screenshot: cannot write header: {e}");
            return;
        }
    };

    let stride = w * 4;
    let mut flipped: Vec<u8> = vec![0u8; stride * h];
    for y in 0..h {
        let src = (h - 1 - y) * stride;
        let dst = y * stride;
        flipped[dst..dst + stride].copy_from_slice(&screen.bytes[src..src + stride]);
    }
    if let Err(e) = png_writer.write_image_data(&flipped) {
        eprintln!("screenshot: cannot write pixels: {e}");
    }
    eprintln!("screenshot written to {path}");
}

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

fn get_random_seed() -> u64 {
    // A seed can be forced from the outside, so that the same run can be
    // repeated: for a picture that is compared with another, or for a
    // profile that is compared with another.
    if let Ok(seed) = std::env::var("PRIMORDIA_SEED") {
        if let Ok(seed) = seed.parse::<u64>() {
            eprintln!("SEED: {seed}");
            return seed;
        }
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = now.as_nanos() as u64;
    let mut seed = nanos;
    seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(std::process::id() as u64);
    seed = seed.wrapping_mul(0xBF58_476D_1CE4_E5B9).wrapping_add(now.as_secs());
    eprintln!("SEED: {seed}");
    seed
}

/// Should the picture be painted on the graphics card?
///
/// The card is the one that paints the picture in the game. The
/// renderer on the processor is the reference it is checked against,
/// so it can be asked for instead with `PRIMORDIA_BACKEND=cpu`.
fn gpu_enabled() -> bool {
    std::env::var("PRIMORDIA_BACKEND").map_or(true, |v| !v.eq_ignore_ascii_case("cpu"))
}

/// Where the camera starts, from `PRIMORDIA_CAM`. It is either a pair
/// of numbers, or the word `zorg`, which is dealt with later, when the
/// world knows where its cave is.
fn camera_from_env(value: Option<&str>) -> (f32, f32) {
    let mut cam_x: f32 = 0.0;
    let mut cam_y: f32 = 0.0;
    if let Some(v) = value {
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
    (cam_x, cam_y)
}

fn run_check(name: &str, args: &[String]) -> Option<i32> {
    match name {
        "aircheck" => Some(checks::aircheck::run(args)),
        "bench" => Some(checks::bench::run(args)),
        "braincheck" => Some(checks::braincheck::run(args)),
        "crowdcheck" => Some(checks::crowdcheck::run(args)),
        "diplcheck" => Some(checks::diplcheck::run(args)),
        "docgen" => Some(checks::docgen::run(args)),
        "foodcheck" => Some(checks::foodcheck::run(args)),
        "lifecheck" => Some(checks::lifecheck::run(args)),
        "prof" => Some(checks::prof::run(args)),
        "revivalcheck" => Some(checks::revivalcheck::run(args)),
        "skycheck" => Some(checks::skycheck::run(args)),
        "spawncheck" => Some(checks::spawncheck::run(args)),
        "zorgcheck" => Some(checks::zorgcheck::run(args)),
        _ => None,
    }
}

/// Everything the game needs while it runs.
struct Game {
    seed: u64,
    world: World,
    creatures: CreatureSystem,
    renderer: render::CpuRenderer,
    /// The renderer on the graphics card. It is only there when the
    /// game was asked to paint the picture on the card.
    gpu: Option<render::GpuRenderer>,
    /// The window of the world that is on the screen.
    region: render::Region,

    cam_x: f32,
    cam_y: f32,
    cam_speed: f32,
    view_x: f32,
    view_y: f32,
    zoom: f32,
    sim_speed: f32,
    paused: bool,

    ui: UiState,
    telemetry: Telemetry,
    effects: Effects,

    revival: u32,
    champion: Option<RevivalGenome>,
    fallback_champion: Option<RevivalGenome>,
    victory: Option<VictoryScreen>,
    last_species: Option<u32>,
    last_diplo_tick: u64,
    force_render: bool,

    region_pixels: Vec<u8>,
    region_image: Image,
    region_texture: Texture2D,
    region_w: i32,
    region_h: i32,

    mini_image: Image,
    mini_texture: Texture2D,
    mini_dirty: bool,
    frame_counter: u32,

    snapshot: Option<String>,
    snapshot_after: f32,
    snapshot_at: Option<std::time::Instant>,
}

impl Game {
    fn new(seed: u64) -> Self {
        let mut world = World::new(W, H, seed);
        world.seed();

        let mut creatures = CreatureSystem::new(seed.wrapping_add(0x1234_5678_9ABC_DEF0));
        creatures.populate(&world, 64);

        let cam_env = std::env::var("PRIMORDIA_CAM").ok();
        let (mut cam_x, mut cam_y) = camera_from_env(cam_env.as_deref());

        let wanted = std::env::var("PRIMORDIA_PANEL").ok();
        let mut ui = UiState::new();
        for (panel, name) in [
            (Panel::Diplomacy, "diplomacy"),
            (Panel::Tech, "tech"),
            (Panel::Minds, "minds"),
        ] {
            if wanted.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(name)) {
                ui.panels.open(panel);
            }
        }

        let snapshot = std::env::var("PRIMORDIA_SNAPSHOT").ok();
        let snapshot_after = std::env::var("PRIMORDIA_SNAPSHOT_AFTER")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(30.0);
        let snapshot_at = snapshot.as_ref().map(|_| now_snapshot());

        let sim_speed: f32 = std::env::var("PRIMORDIA_SPEED")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.25, 48.0);

        let zoom: f32 = std::env::var("PRIMORDIA_ZOOM")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(MIN_ZOOM)
            .clamp(MIN_ZOOM, MAX_ZOOM);

        if cam_env
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case("zorg"))
        {
            if let Some((cx, cy)) = world.zorg_cave_center() {
                cam_x = cx as f32 - (VIEW_W as f32 / zoom) * 0.5;
                cam_y = cy as f32 - (VIEW_H as f32 / zoom) * 0.5;
            }
        }

        let mut telemetry = Telemetry::new();
        telemetry.reset(1);

        let mini_image = Image::gen_image_color(MINI_W as u16, MINI_H as u16, WHITE);
        let mini_texture = Texture2D::from_image(&mini_image);
        mini_texture.set_filter(FilterMode::Linear);

        let region_image = Image::gen_image_color(1, 1, WHITE);
        let region_texture = Texture2D::from_image(&region_image);
        region_texture.set_filter(FilterMode::Nearest);

        Game {
            seed,
            world,
            creatures,
            renderer: render::CpuRenderer::new(),
            gpu: if gpu_enabled() {
                Some(render::GpuRenderer::new())
            } else {
                None
            },
            region: render::Region {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            cam_x,
            cam_y,
            cam_speed: 300.0,
            view_x: cam_x,
            view_y: cam_y,
            zoom,
            sim_speed,
            paused: false,
            ui,
            telemetry,
            effects: Effects::new(),
            revival: 1,
            champion: None,
            fallback_champion: None,
            victory: None,
            last_species: None,
            last_diplo_tick: 0,
            force_render: true,
            region_pixels: Vec::new(),
            region_image,
            region_texture,
            region_w: 0,
            region_h: 0,
            mini_image,
            mini_texture,
            mini_dirty: true,
            frame_counter: 0,
            snapshot,
            snapshot_after,
            snapshot_at,
        }
    }

    /// One turn of the game. It returns false when the run is over.
    async fn frame(&mut self) -> bool {
        let now = get_time() as f32;

        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) {
            std::process::exit(0);
        }

        let dt = self.handle_input();
        let steps_taken = self.step_simulation();
        self.observe(now, dt);
        self.check_victory();
        self.advance_victory(dt);

        let view_w = VIEW_W as f32 / self.zoom;
        let view_h = VIEW_H as f32 / self.zoom;

        let (mx, my) = self.update_hover(view_w, view_h);

        let corner = self.render_region(now, steps_taken, view_w, view_h);
        self.draw_minimap();
        self.draw_scene(now, mx, my, view_w, view_h, corner);

        if self.snapshot_due() {
            return false;
        }
        next_frame().await;
        true
    }
    fn handle_input(&mut self) -> f32 {
        if is_key_pressed(KeyCode::R) && self.victory.is_none() {
            let new_seed = get_random_seed();
            self.world = World::new(W, H, new_seed);
            self.world.seed();
            self.creatures = CreatureSystem::new(new_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
            self.creatures.populate(&self.world, 64);
            self.cam_x = 0.0;
            self.cam_y = 0.0;
            self.view_x = 0.0;
            self.view_y = 0.0;
            self.zoom = MIN_ZOOM;

            self.revival = 1;
            self.champion = None;
            self.force_render = true;
            self.telemetry.reset(1);
        }

        if is_key_pressed(KeyCode::K) {
            self.ui.panels.toggle(Panel::Knowledge);
        }

        if is_key_pressed(KeyCode::T) {
            self.ui.panels.toggle(Panel::Tech);
        }

        if is_key_pressed(KeyCode::G) {
            self.ui.panels.toggle(Panel::Diplomacy);
        }

        if is_key_pressed(KeyCode::M) {
            self.ui.panels.toggle(Panel::Minds);
        }

        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
            self.sim_speed = (self.sim_speed * 2.0).min(48.0);
        }
        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
            self.sim_speed = (self.sim_speed / 2.0).max(0.25);
        }

        if is_key_pressed(KeyCode::P) && self.victory.is_none() {
            self.paused = !self.paused;
        }

        let mut view_w = VIEW_W as f32 / self.zoom;
        let mut view_h = VIEW_H as f32 / self.zoom;

        let (_, wheel_y) = mouse_wheel();
        let (mx, my) = mouse_position();
        let over_tech = self.ui.panels.has(Panel::Tech) && tech_tree_under_pointer(mx, my);

        if self.ui.panels.has(Panel::Minds) && wheel_y != 0.0 {
            self.ui.minds_index = (self.ui.minds_index - wheel_y.signum()).max(0.0);
        } else if wheel_y != 0.0 && !over_tech {
            let direction = wheel_y.signum();
            let old_wide = screen_width() / view_w;
            let old_tall = screen_height() / view_h;
            let anchor_x = self.cam_x + mx / old_wide;
            let anchor_y = self.cam_y + my / old_tall;

            self.zoom = (self.zoom * 1.12f32.powf(direction)).clamp(MIN_ZOOM, MAX_ZOOM);
            view_w = VIEW_W as f32 / self.zoom;
            view_h = VIEW_H as f32 / self.zoom;

            let new_wide = screen_width() / view_w;
            let new_tall = screen_height() / view_h;
            self.cam_x = anchor_x - mx / new_wide;
            self.cam_y = anchor_y - my / new_tall;
        }

let dt = get_frame_time();
        let cam_speed = self.cam_speed * (view_w / VIEW_W as f32);
        let max_cam_x = (W as f32 - view_w).max(0.0);
        let max_cam_y = (H as f32 - view_h).max(0.0);
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.cam_x -= cam_speed * dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.cam_x += cam_speed * dt;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            self.cam_y -= cam_speed * dt;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            self.cam_y += cam_speed * dt;
        }
        self.cam_x = self.cam_x.clamp(0.0, max_cam_x);
        self.cam_y = self.cam_y.clamp(0.0, max_cam_y);

        let ease = 1.0 - (-9.0 * dt).exp();
        self.view_x += (self.cam_x - self.view_x) * ease;
        self.view_y += (self.cam_y - self.view_y) * ease;
        if (self.cam_x - self.view_x).abs() < 0.05 {
            self.view_x = self.cam_x;
        }
        if (self.cam_y - self.view_y).abs() < 0.05 {
            self.view_y = self.cam_y;
        }

        dt
    }

    fn step_simulation(&mut self) -> u32 {
        let sim_dt = (get_frame_time().min(0.1) as f64) * (self.sim_speed as f64);
        let fixed_step = 1.0 / 60.0;

        let max_steps = (4.0 * self.sim_speed).min(48.0) as u32;
        let mut steps_taken = 0u32;
        if self.paused {
            unsafe { ACCUM = 0.0; }
        } else {
            unsafe {
                ACCUM += sim_dt;
                let mut steps = 0;
                while ACCUM >= fixed_step && steps < max_steps {
                    self.world.step();
                    self.creatures.step(&mut self.world);
                    ACCUM -= fixed_step;
                    steps += 1;
                }
                steps_taken = steps;
                if steps == max_steps {
                    ACCUM = 0.0;
                }
            }
        }

        steps_taken
    }

    fn observe(&mut self, now: f32, dt: f32) {
        if self.victory.is_none() {
            self.telemetry.observe(now, &self.creatures);

            for e in &self.creatures.diplomacy.log {
                if e.tick > self.last_diplo_tick {
                    self.telemetry.push(e.text.clone());
                }
            }
            self.last_diplo_tick = self.creatures.tick;
        }
        self.telemetry.tick(dt);

        for ev in self.creatures.events.drain(..) {
            if matches!(ev, SimEvent::Speciation { .. }) {
                self.telemetry.push("A NEW SPECIES HAS ARISEN".to_string());
            }
            apply_event(&mut self.effects, ev);
        }
        self.effects.update(dt);
    }

    fn check_victory(&mut self) {
if self.victory.is_some() {
    return;
}


if let Some(sp) = dominant_player_species(&self.creatures) {
if self.last_species != Some(sp) || self.fallback_champion.is_none() {
    self.last_species = Some(sp);
    self.fallback_champion = self.creatures.champion_overall();
}
}

let mut players: Vec<u32> = self.creatures
.list
.iter()
.filter(|c| c.faction == primordia::creature::Faction::Player)
.map(|c| c.species)
.collect();
players.sort_unstable();
players.dedup();
let zorgs = self.creatures.zorg_count();

if self.creatures.list.is_empty() {
    self.extinction_screen();
} else if players.is_empty() {
    self.lost_screen(zorgs);
} else if players.len() == 1 && (zorgs == 0 || self.world.zorg_cave_sealed) {
    self.won_screen(players[0], zorgs);
}
    }

    fn extinction_screen(&mut self) {
self.victory = Some(VictoryScreen {
winner: "NO SURVIVORS".to_string(),
color: [90, 100, 110],
survivors: 0,
generation: self.creatures.stats.max_generation,
revival: self.revival,
timer: 1.0,
outcome: Outcome::Extinct,
zorgs: 0,
alliances: 0,
wars: 0,
treaties: 0,
truces: 0,
traits: Vec::new(),
discoveries: Vec::new(),
deep_sealed: self.world.zorg_cave_sealed,

lineage_carried: self.fallback_champion.is_some() || self.champion.is_some(),
});
self.paused = true;
    }

    fn lost_screen(&mut self, zorgs: u32) {
self.victory = Some(VictoryScreen {
winner: "LOST".to_string(),
color: primordia::creature::ZORG_COLOR,
survivors: 0,
generation: self.creatures.stats.max_generation,
revival: self.revival,
timer: 1.0,
outcome: Outcome::Lost,
zorgs,
alliances: 0,
wars: 0,
treaties: 0,
truces: 0,
traits: Vec::new(),
discoveries: Vec::new(),
deep_sealed: self.world.zorg_cave_sealed,

lineage_carried: self.fallback_champion.is_some() || self.champion.is_some(),
});
self.paused = true;
    }

    fn won_screen(&mut self, winner: u32, zorgs: u32) {
self.champion = self.creatures.champion_from(winner);
let rep = self.creatures
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

let mut alliances = 0u32;
let mut wars = 0u32;
let mut treaties = 0u32;
for r in &self.creatures.diplomacy.relations {
if r.a == winner || r.b == winner {
if r.state == primordia::creature::RelationState::Allied {
alliances += 1;
}
if r.state == primordia::creature::RelationState::War {
wars += 1;
}
treaties += r.treaties.len() as u32;
}
}

let traits: Vec<&'static str> = primordia::creature::trait_list(rep.traits)
.iter()
.map(|t| t.name())
.collect();
let discoveries: Vec<&'static str> =
primordia::creature::discovery_list(self.creatures.species_discoveries(winner))
.iter()
.map(|d| d.name())
.collect();
self.victory = Some(VictoryScreen {
winner: name,
color: rep.team_color,
survivors: self.creatures.list.len() as u32,
generation: self.creatures.stats.max_generation,
revival: self.revival,
timer: 1.0,
outcome: Outcome::Won,
zorgs,
alliances,
wars,
treaties,
truces: self.creatures.diplomacy.stats.truces,
traits,
discoveries,
deep_sealed: self.world.zorg_cave_sealed,

lineage_carried: true,
});
self.paused = true;
    }

    fn advance_victory(&mut self, dt: f32) {
        if let Some(v) = self.victory.as_mut() {
            v.timer -= dt;
            if v.timer <= 0.0 {

                let won = v.outcome.is_won();

                let carried: Option<RevivalGenome> = if won {

                    self.champion.take()
                } else {

                    self.fallback_champion.take().or_else(|| self.champion.take())
                };

                let next_seed = get_random_seed();
                self.world = World::new(W, H, next_seed);
                self.world.seed();
                self.creatures = CreatureSystem::new(next_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
                match carried.as_ref() {
                    Some(genome) => self.creatures.populate_revival(&self.world, 36, genome),
                    None => self.creatures.populate(&self.world, 64),
                }

                self.revival += 1;

                self.champion = carried;
                self.last_species = None;
                self.fallback_champion = None;
                self.cam_x = 0.0;
                self.cam_y = 0.0;
                self.view_x = 0.0;
                self.view_y = 0.0;
                self.zoom = MIN_ZOOM;
                self.paused = false;
                self.force_render = true;
                self.telemetry.reset(self.revival);
                unsafe {
                    ACCUM = 0.0;
                }
                self.victory = None;
            }
        }
    }

    fn update_hover(&mut self, view_w: f32, view_h: f32) -> (f32, f32) {
        let (mx, my) = mouse_position();
        let scale_x = screen_width() / view_w;
        let scale_y = screen_height() / view_h;
        let world_x = self.view_x + mx / scale_x;
        let world_y = self.view_y + my / scale_y;
        let cell_x = world_x as i32;
        let cell_y = world_y as i32;

        self.ui.hover = HoverTarget::None;
        if cell_x >= 0 && cell_y >= 0 && cell_x < W as i32 && cell_y < H as i32 {
            let elem = self.world.get(cell_x, cell_y);
            if elem != Elem::Empty {
                self.ui.discover_element(elem);
                self.ui.hover = HoverTarget::Element { x: cell_x, y: cell_y, elem };
            }
        }

        for c in &self.creatures.list {
            let dx = world_x - c.x;
            let dy = world_y - c.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < c.size.max(3.0) {
                self.ui.discover_creature(c.id);
                self.ui.hover = HoverTarget::Creature { id: c.id };
                break;
            }
        }

        (mx, my)
    }

    fn render_region(
        &mut self,
        now: f32,
        steps_taken: u32,
        view_w: f32,
        view_h: f32,
    ) -> (i32, i32) {
        let vis_left = (self.view_x.floor() as i32).max(0);
        let vis_top = (self.view_y.floor() as i32).max(0);
        let vis_right = ((self.view_x + view_w).ceil() as i32).min(W as i32);
        let vis_bottom = ((self.view_y + view_h).ceil() as i32).min(H as i32);
        let vis_width = (vis_right - vis_left).max(1);
        let vis_height = (vis_bottom - vis_top).max(1);

        if self.region_w != vis_width || self.region_h != vis_height {
            self.region_w = vis_width;
            self.region_h = vis_height;
            self.region_pixels = vec![0u8; (self.region_w * self.region_h * 4) as usize];

            self.region_image = Image::gen_image_color(self.region_w as u16, self.region_h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
            self.region_texture = Texture2D::from_image(&self.region_image);
            self.region_texture.set_filter(FilterMode::Nearest);

            self.force_render = true;
        }

        if steps_taken > 0 || self.force_render {
            self.region = crate::render::Region {
                x: vis_left,
                y: vis_top,
                w: vis_width,
                h: vis_height,
            };
            if self.gpu.is_none() {
                self.renderer.render_region(
                    &self.world,
                    &self.creatures,
                    &mut self.region_pixels,
                    self.region,
                    crate::render::Clock {
                        time: now,
                        day_night: self.creatures.day_night,
                        season: self.creatures.season,
                        weather: self.world.weather,
                    },
                );
                self.region_image
                    .bytes
                    .copy_from_slice(&self.region_pixels);
                self.region_texture.update(&self.region_image);
            }
            self.force_render = false;
            self.mini_dirty = true;
        }

(vis_left, vis_top)
    }

    fn draw_minimap(&mut self) {
        self.frame_counter += 1;
        if self.mini_dirty && self.frame_counter.is_multiple_of(4) {
            for my in 0..MINI_H {
                for mx in 0..MINI_W {
                    let sx = (mx * MINI_STEP).min(W - 1);
                    let sy = (my * MINI_STEP).min(H - 1);
                    let cell = sy * W + sx;
                    let d = (my * MINI_W + mx) * 4;
                    let k = self.world.kind[cell];
                    if k == Elem::Empty as u8 {
                        self.mini_image.bytes[d] = 12;
                        self.mini_image.bytes[d + 1] = 18;
                        self.mini_image.bytes[d + 2] = 30;
                    } else {
                        let c = self.world.col[cell];
                        self.mini_image.bytes[d] = (c[0] as u32 * 3 / 4) as u8;
                        self.mini_image.bytes[d + 1] = (c[1] as u32 * 3 / 4) as u8;
                        self.mini_image.bytes[d + 2] = (c[2] as u32 * 3 / 4) as u8;
                    }
                    self.mini_image.bytes[d + 3] = 255;
                }
            }
            self.mini_texture.update(&self.mini_image);
            self.mini_dirty = false;
        }
    }

    fn draw_scene(
        &mut self,
        now: f32,
        mx: f32,
        my: f32,
        view_w: f32,
        view_h: f32,
        corner: (i32, i32),
    ) {
        let reg_scale_x = screen_width() / view_w;
        let reg_scale_y = screen_height() / view_h;
        let place = render::Place {
            x: (corner.0 as f32 - self.view_x) * reg_scale_x,
            y: (corner.1 as f32 - self.view_y) * reg_scale_y,
            w: self.region_w as f32 * reg_scale_x,
            h: self.region_h as f32 * reg_scale_y,
        };
        match self.gpu.as_mut() {
            // The renderer on the card paints the picture straight onto
            // the screen, so it is asked to do it where the picture goes.
            Some(gpu) => {
                gpu.render_region(
                    &self.world,
                    &self.creatures,
                    self.region,
                    crate::render::Clock {
                        time: now,
                        day_night: self.creatures.day_night,
                        season: self.creatures.season,
                        weather: self.world.weather,
                    },
                    place,
                );
            }
            None => {
                draw_texture_ex(
                    &self.region_texture,
                    place.x,
                    place.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(place.w, place.h)),
                        ..Default::default()
                    },
                );
            }
        }

        draw_vignette();

        draw_atmosphere(&self.world, self.view_x, self.view_y, view_w, view_h, now, self.creatures.day_night);
        draw_actors(
            &self.creatures,
            &mut self.world,
            &self.ui.hover,
            &primordia::ui::actors::ActorView {
                x: self.view_x,
                y: self.view_y,
                w: view_w,
                h: view_h,
                zoom: self.zoom,
            },
            now,
        );
        self.effects.draw(self.view_x, self.view_y, screen_width() / view_w, screen_height() / view_h);

        draw_minimap(
            &self.creatures,
            &self.world,
            self.view_x,
            self.view_y,
            view_w,
            view_h,
            &self.mini_texture,
        );
        draw_hud(
            &self.world,
            &self.creatures,
            &primordia::ui::hud::HudState {
                cam_x: self.view_x,
                cam_y: self.view_y,
                seed: self.seed,
                sim_speed: self.sim_speed,
                paused: self.paused,
                zoom: self.zoom,
                revival: self.revival,
            },
        );
        if screen_width() >= 1080.0 {
            draw_telemetry(&self.telemetry, screen_width() - 332.0, 18.0);
        }

        if self.ui.panels.has(Panel::Diplomacy) {
            draw_diplomacy_panel(&self.creatures, 18.0, 248.0);
        }

        draw_tooltip(&self.world, &self.creatures, &self.ui, mx, my);

        if self.ui.panels.has(Panel::Knowledge) {
            draw_knowledge_window(&self.world, &self.creatures, &self.ui);
        }

        if self.ui.panels.has(Panel::Tech) {
            draw_tech_tree(&self.creatures, &mut self.ui);
        }
        draw_tech_panel(&self.creatures);

        if self.ui.panels.has(Panel::Minds) {
            draw_minds(&self.creatures, self.ui.minds_index);
        }

        if let Some(v) = &self.victory {
            draw_victory(v);
        }
    }

    /// Is the picture due? It is when the run has been going for as
    /// long as it was asked to.
    fn snapshot_due(&mut self) -> bool {
        if let (Some(path), Some(at)) = (&self.snapshot, self.snapshot_at) {
            let elapsed = now_snapshot().duration_since(at).as_secs_f32();
            if elapsed >= self.snapshot_after {
                eprintln!("screenshot after {elapsed:.1}s");
                save_screenshot(path);
                return true;
            }
        }
        false
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() > 1 {
        let name = argv[1].clone();
        if name == "rendertest" {
            rendertest::run(&argv[2..]);
            return;
        }
        if let Some(code) = run_check(&name, &argv[2..]) {
            std::process::exit(code);
        }
    }
    let seed = get_random_seed();
    let mut game = Game::new(seed);

    loop {
        if !game.frame().await {
            break;
        }
    }
}
