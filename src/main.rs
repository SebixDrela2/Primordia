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
use primordia::creature::{CreatureSystem, Model, SimEvent};
use primordia::elements::Elem;
use primordia::render;
use primordia::session::{Outcome as RunOutcome, Session};
use primordia::ui::{
    apply_event, draw_actors, draw_atmosphere, draw_diplomacy_panel,
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

/// The model of a run, when the game is run on one that was learned.
fn model_from_env() -> Option<Model> {
    if !std::env::args().any(|a| a == "--run") {
        return None;
    }
    let path = model_path();
    match Model::load(&path) {
        Ok(model) => {
            eprintln!(
                "running the model in {path} (generation {}, {} weights, fitness {:.3})",
                model.generation,
                model.weight_count(),
                model.fitness
            );
            Some(model)
        }
        Err(err) => {
            eprintln!("cannot read {path}: {err}");
            eprintln!("learn one first: primordia --learn {path}");
            std::process::exit(1);
        }
    }
}

/// The file the model is read from and written to.
fn model_path() -> String {
    let mut path = "model.bin".to_owned();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--file" {
            if let Some(next) = args.get(i + 1) {
                path.clone_from(next);
            }
        }
        i += 1;
    }
    path
}

/// Give the colony the mind of the model, and stop the minds from
/// learning.
fn apply_model(creatures: &mut CreatureSystem, model: &Model) {
    creatures.learn = false;
    creatures.give_the_mind(model);
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
    /// The simulation: the world, the colony, the victory and the
    /// revival. The learner works the same one, with no drawing.
    session: Session,
    /// The model the colony is running on, when the game is run on one
    /// that was learned.
    model: Option<Model>,
    /// The renderer on the graphics card. It paints the picture.
    gpu: render::GpuRenderer,
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

    victory: Option<VictoryScreen>,
    last_diplo_tick: u64,
    force_render: bool,

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
        // The simulation: the world, the colony, the victory and the
        // revival. The learner works the same one, with no drawing.
        let mut session = Session::new(seed, 64);

        // The model of a run: the colony is given the mind that was
        // learned, and the minds do not learn anything new, so that
        // what is seen is the doing of the weights that were saved.
        let model = model_from_env();
        if let Some(model) = &model {
            apply_model(&mut session.creatures, model);
        }

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
            if let Some((cx, cy)) = session.world.zorg_cave_center() {
                cam_x = cx as f32 - (VIEW_W as f32 / zoom) * 0.5;
                cam_y = cy as f32 - (VIEW_H as f32 / zoom) * 0.5;
            }
        }

        let mut telemetry = Telemetry::new();
        telemetry.reset(1);

        let mini_image = Image::gen_image_color(MINI_W as u16, MINI_H as u16, WHITE);
        let mini_texture = Texture2D::from_image(&mini_image);
        mini_texture.set_filter(FilterMode::Linear);

        Game {
            seed,
            session,
            model,
            gpu: render::GpuRenderer::new(),
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
            victory: None,
            last_diplo_tick: 0,
            force_render: true,
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
            self.session.world = World::new(W, H, new_seed);
            self.session.world.seed();
            self.session.creatures = CreatureSystem::new(new_seed.wrapping_add(0x1234_5678_9ABC_DEF0));
            self.session.creatures.populate(&self.session.world, 64);
            self.cam_x = 0.0;
            self.cam_y = 0.0;
            self.view_x = 0.0;
            self.view_y = 0.0;
            self.zoom = MIN_ZOOM;

            self.session.revival = 1;
            self.session.champion = None;
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
                    // The session turns the world and the colony. The
                    // learner works the same one.
                    self.session.step();
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
            self.telemetry.observe(now, &self.session.creatures);

            for e in &self.session.creatures.diplomacy.log {
                if e.tick > self.last_diplo_tick {
                    self.telemetry.push(e.text.clone());
                }
            }
            self.last_diplo_tick = self.session.creatures.tick;
        }
        self.telemetry.tick(dt);

        for ev in self.session.creatures.events.drain(..) {
            if matches!(ev, SimEvent::Speciation { .. }) {
                self.telemetry.push("A NEW SPECIES HAS ARISEN".to_string());
            }
            apply_event(&mut self.effects, ev);
        }
        self.effects.update(dt);
    }

    fn check_victory(&mut self) {
        // The screen of the last run is still up: it is not made again.
        if self.victory.is_some() {
            return;
        }
        // The session says how the run ended: the learner works the
        // same one, so the two cannot drift apart.
        let Some(outcome) = self.session.check_victory() else {
            return;
        };
        match outcome {
            RunOutcome::Won { species, zorgs } => self.won_screen(species, zorgs),
            RunOutcome::Lost { zorgs } => self.lost_screen(zorgs),
            RunOutcome::Extinct => self.extinction_screen(),
        }
    }
    fn extinction_screen(&mut self) {
self.victory = Some(VictoryScreen {
winner: "NO SURVIVORS".to_string(),
color: [90, 100, 110],
survivors: 0,
generation: self.session.creatures.stats.max_generation,
revival: self.session.revival,
timer: 1.0,
outcome: Outcome::Extinct,
zorgs: 0,
alliances: 0,
wars: 0,
treaties: 0,
truces: 0,
traits: Vec::new(),
discoveries: Vec::new(),
deep_sealed: self.session.world.zorg_cave_sealed,

lineage_carried: self.session.fallback_champion.is_some() || self.session.champion.is_some(),
});
self.paused = true;
    }

    fn lost_screen(&mut self, zorgs: u32) {
self.victory = Some(VictoryScreen {
winner: "LOST".to_string(),
color: primordia::creature::ZORG_COLOR,
survivors: 0,
generation: self.session.creatures.stats.max_generation,
revival: self.session.revival,
timer: 1.0,
outcome: Outcome::Lost,
zorgs,
alliances: 0,
wars: 0,
treaties: 0,
truces: 0,
traits: Vec::new(),
discoveries: Vec::new(),
deep_sealed: self.session.world.zorg_cave_sealed,

lineage_carried: self.session.fallback_champion.is_some() || self.session.champion.is_some(),
});
self.paused = true;
    }

    fn won_screen(&mut self, winner: u32, zorgs: u32) {
self.session.champion = self.session.creatures.champion_from(winner);
let rep = self.session.creatures
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
for r in &self.session.creatures.diplomacy.relations {
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
primordia::creature::discovery_list(self.session.creatures.species_discoveries(winner))
.iter()
.map(|d| d.name())
.collect();
self.victory = Some(VictoryScreen {
winner: name,
color: rep.team_color,
survivors: self.session.creatures.list.len() as u32,
generation: self.session.creatures.stats.max_generation,
revival: self.session.revival,
timer: 1.0,
outcome: Outcome::Won,
zorgs,
alliances,
wars,
treaties,
truces: self.session.creatures.diplomacy.stats.truces,
traits,
discoveries,
deep_sealed: self.session.world.zorg_cave_sealed,

lineage_carried: true,
});
self.paused = true;
    }

    fn advance_victory(&mut self, dt: f32) {
        if let Some(v) = self.victory.as_mut() {
            v.timer -= dt;
            if v.timer <= 0.0 {
                // The session carries the lineage into the next run and
                // makes the new world. The learner does the same.
                let carried = self.session.carried();
                let next_seed = get_random_seed();
                self.session.revive(next_seed, carried.as_ref());

                self.cam_x = 0.0;
                self.cam_y = 0.0;
                self.view_x = 0.0;
                self.view_y = 0.0;
                self.zoom = MIN_ZOOM;
                self.paused = false;
                self.force_render = true;
                self.telemetry.reset(self.session.revival);
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
            let elem = self.session.world.get(cell_x, cell_y);
            if elem != Elem::Empty {
                self.ui.discover_element(elem);
                self.ui.hover = HoverTarget::Element { x: cell_x, y: cell_y, elem };
            }
        }

        for c in &self.session.creatures.list {
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
        _now: f32,
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
            self.force_render = true;
        }

        // The window of the world that is on the screen. It follows the
        // camera every frame, because the picture on the card is drawn
        // where the window says it is.
        self.region = crate::render::Region {
            x: vis_left,
            y: vis_top,
            w: vis_width,
            h: vis_height,
        };

        if steps_taken > 0 || self.force_render {
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
                    let k = self.session.world.kind[cell];
                    if k == Elem::Empty as u8 {
                        self.mini_image.bytes[d] = 12;
                        self.mini_image.bytes[d + 1] = 18;
                        self.mini_image.bytes[d + 2] = 30;
                    } else {
                        let c = self.session.world.col[cell];
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
        // The renderer on the card paints the picture straight onto
        // the screen, so it is asked to do it where the picture goes.
        self.gpu.render_region(
            &self.session.world,
            &self.session.creatures,
            self.region,
            crate::render::Clock {
                time: now,
                day_night: self.session.creatures.day_night,
                season: self.session.creatures.season,
                weather: self.session.world.weather,
            },
            place,
        );

        draw_vignette();

        draw_atmosphere(&self.session.world, self.view_x, self.view_y, view_w, view_h, now, self.session.creatures.day_night);
        draw_actors(
            &self.session.creatures,
            &mut self.session.world,
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
            &self.session.creatures,
            &self.session.world,
            self.view_x,
            self.view_y,
            view_w,
            view_h,
            &self.mini_texture,
        );
        draw_hud(
            &self.session.world,
            &self.session.creatures,
            &primordia::ui::hud::HudState {
                cam_x: self.view_x,
                cam_y: self.view_y,
                seed: self.seed,
                sim_speed: self.sim_speed,
                paused: self.paused,
                zoom: self.zoom,
                revival: self.session.revival,
                model: self.model.as_ref().map(|m| primordia::ui::hud::ModelInfo {
                    generation: m.generation,
                    weights: m.weight_count(),
                    ticks: m.ticks,
                }),
            },
        );
        if screen_width() >= 1080.0 {
            draw_telemetry(&self.telemetry, screen_width() - 332.0, 18.0);
        }

        if self.ui.panels.has(Panel::Diplomacy) {
            draw_diplomacy_panel(&self.session.creatures, 18.0, 248.0);
        }

        draw_tooltip(&self.session.world, &self.session.creatures, &self.ui, mx, my);

        if self.ui.panels.has(Panel::Knowledge) {
            draw_knowledge_window(&self.session.world, &self.session.creatures, &self.ui);
        }

        if self.ui.panels.has(Panel::Tech) {
            draw_tech_tree(&self.session.creatures, &mut self.ui);
        }
        draw_tech_panel(&self.session.creatures);

        if self.ui.panels.has(Panel::Minds) {
            draw_minds(&self.session.creatures, self.ui.minds_index);
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

/// The window of the game, and the two ways of working without it.
///
/// The game itself is `amain`, which the window runs. The learner is
/// the other half: it does the work of the network with no window and
/// no card at all, so it is started before the window is made.
fn main() {
    let argv: Vec<String> = std::env::args().collect();

    // The learner: no window, no card, only the work of the network.
    if argv.iter().any(|a| a == "--learn") {
        std::process::exit(primordia::learn::run(&argv));
    }

    // The run of a saved model, and the game itself, both want the
    // window.
    macroquad::Window::from_config(window_conf(), amain());
}

async fn amain() {
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
    eprintln!("backend: gpu");
    let mut game = Game::new(seed);

    loop {
        if !game.frame().await {
            break;
        }
    }
}