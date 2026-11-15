use crate::creature::{CreatureSystem, trait_list};
use crate::ui::diplomacy::diplomacy_summary;
use crate::world::World;
use crate::W;
use macroquad::prelude::*;

pub struct Telemetry {
    pop: Vec<f32>,
    species: Vec<f32>,
    events: Vec<(String, f32)>,
    last_sample: f32,
    known: Vec<(u32, String)>,
    last_season: u8,
    initialized: bool,
}

impl Telemetry {
    #[must_use]
    pub fn new() -> Self {
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

    pub fn reset(&mut self, revival: u32) {
        self.pop.clear();
        self.species.clear();
        self.known.clear();
        self.events.clear();
        self.last_sample = -1.0;
        self.last_season = 0;
        self.initialized = false;
        self.push(format!("REVIVAL {}  -  new lineage", revival));
    }

    pub fn push(&mut self, msg: String) {
        self.events.push((msg, 0.0));
        if self.events.len() > 8 {
            self.events.remove(0);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for e in &mut self.events {
            e.1 += dt;
        }
    }

    pub fn observe(&mut self, now: f32, creatures: &CreatureSystem) {
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

pub fn draw_telemetry(telemetry: &Telemetry, panel_x: f32, panel_y: f32) {
    let panel_w = 314.0f32;
    let panel_h = 226.0f32;

    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.03, 0.05, 0.08, 0.84));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.5, Color::new(0.13, 0.83, 0.93, 0.35));

    let pop_now = telemetry.pop.last().copied().unwrap_or(0.0);
    let sp_now = telemetry.species.last().copied().unwrap_or(0.0);
    draw_text("ECOLOGY", panel_x + 12.0, panel_y + 20.0, 14.0, Color::new(0.13, 0.83, 0.93, 0.9));
    draw_text(
        format!("pop {:.0}", pop_now),
        panel_x + 96.0,
        panel_y + 20.0,
        12.0,
        Color::new(0.35, 0.9, 1.0, 0.9),
    );
    draw_text(
        format!("species {:.0}", sp_now),
        panel_x + 184.0,
        panel_y + 20.0,
        12.0,
        Color::new(0.95, 0.78, 0.35, 0.9),
    );

    let gx = panel_x + 12.0;
    let gy = panel_y + 30.0;
    let gw = panel_w - 24.0;
    let gh = 78.0;
    draw_rectangle(gx, gy, gw, gh, Color::new(0.02, 0.03, 0.05, 0.9));
    draw_rectangle_lines(gx, gy, gw, gh, 1.0, Color::new(0.2, 0.3, 0.4, 0.45));

    let samples = telemetry.pop.len().min(160);
    if samples >= 2 {
        let start = telemetry.pop.len() - samples;
        let pop_max = telemetry.pop[start..]
            .iter()
            .copied()
            .fold(1.0f32, f32::max)
            .max(10.0);
        let step = gw / (samples - 1) as f32;
        let pop_color = Color::new(0.25, 0.85, 0.98, 0.85);
        let sp_color = Color::new(0.98, 0.78, 0.32, 0.8);

        let mut prev_pop: Option<(f32, f32)> = None;
        let mut prev_sp: Option<(f32, f32)> = None;
        for sample in 0..samples {
            let px = gx + sample as f32 * step;
            let pv = telemetry.pop[start + sample];
            let sv = telemetry.species[start + sample];
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
    draw_text("EVENTS", panel_x + 12.0, ey, 11.0, Color::new(0.55, 0.62, 0.7, 0.85));
    let mut yy = ey + 15.0;
    let mut any = false;
    for (msg, age) in telemetry.events.iter().rev() {
        let alpha = (1.0 - age / 16.0).clamp(0.25, 1.0);
        draw_text(
            msg,
            panel_x + 12.0,
            yy,
            11.0,
            Color::new(0.8, 0.86, 0.92, alpha),
        );
        yy += 14.0;
        any = true;
        if yy > panel_y + panel_h - 8.0 {
            break;
        }
    }
    if !any {
        draw_text(
            "hover to inspect - K for knowledge",
            panel_x + 12.0,
            yy,
            11.0,
            Color::new(0.5, 0.58, 0.66, 0.7),
        );
    }
}

#[must_use]
pub fn dominant_lineage(creatures: &CreatureSystem) -> (Option<u32>, Vec<&'static str>) {
    let mut counts: Vec<(u32, u32)> = Vec::new();
    for c in &creatures.list {
        match counts.iter_mut().find(|(id, _)| *id == c.species) {
            Some(e) => e.1 += 1,
            None => counts.push((c.species, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let Some((sp, _)) = counts.first().copied() else {
        return (None, Vec::new());
    };
    let traits = creatures
        .list
        .iter()
        .find(|c| c.species == sp)
        .map(|c| trait_list(c.traits).into_iter().map(crate::creature::Trait::name).collect())
        .unwrap_or_default();
    (Some(sp), traits)
}

fn mind_summary(creatures: &CreatureSystem, species: u32) -> String {
    let mut net: Option<&crate::creature::Net> = None;
    for c in &creatures.list {
        if c.species == species {
            net = Some(&c.net);
            break;
        }
    }
    match net {
        Some(n) => {
            let (h1, h2, h3) = n.shape();
            format!(
                "tier {} ({}-{}-{} neurons, {} firing)",
                n.arch,
                h1,
                h2,
                h3,
                crate::creature::activation::name(n.activation)
            )
        }
        None => "no mind yet".to_string(),
    }
}

/// What the heads-up display needs to know about the run: where the
/// camera is, how fast the world runs, whether it is paused and how the
/// run itself is going. These belong together, so they travel in one
/// struct rather than as a list of arguments.
pub struct HudState {
    pub cam_x: f32,
    pub cam_y: f32,
    pub seed: u64,
    pub sim_speed: f32,
    pub paused: bool,
    pub zoom: f32,
    pub revival: u32,
}

pub fn draw_hud(world: &World, creatures: &CreatureSystem, hud: &HudState) {
    let cam_x = hud.cam_x;
    let cam_y = hud.cam_y;

    let cam_col = (cam_x as i32).clamp(0, W as i32 - 1);
    let cam_depth = (cam_y as i32) - world.surface_y(cam_col);
    let place = if cam_depth > 14 {
        let zone = crate::world::DepthZone::of(cam_depth);
        format!("{} ({})", zone.name(), zone.blurb())
    } else {
        world.biome_at(cam_col).name().to_string()
    };

    let (leader, leader_traits) = dominant_lineage(creatures);

    draw_hud_title();

    let season_name = match creatures.season { 0=>"Spring", 1=>"Summer", 2=>"Autumn", _=>"Winter" };
    let time_of_day = if creatures.day_night < 0.25 { "Dawn" } else if creatures.day_night < 0.5 { "Morning" } else if creatures.day_night < 0.75 { "Dusk" } else { "Night" };

    let cam_air = world.fresh_air(cam_col, (cam_y as i32).clamp(0, world.h as i32 - 1), 4);
    let air_name = if cam_air > 0.55 {
        "fresh air"
    } else if cam_air > 0.2 {
        "stale air"
    } else {
        "no air"
    };

    let ctx = HudContext {
        world,
        creatures,
        hud,
        place,
        season_name,
        time_of_day,
        air_name,
        leader,
        leader_traits,
    };
    let lines = hud_lines(&ctx);
    draw_hud_lines(&lines);

    draw_volcano_banner(world);
    draw_help_line();
    draw_paused_banner(hud.paused);
}

/// The name of the game and the rule under it.
fn draw_hud_title() {
    draw_text(
        "PRIMORDIA",
        18.0,
        34.0,
        26.0,
        Color::new(0.13, 0.83, 0.93, 0.9),
    );
    draw_rectangle(18.0, 44.0, 64.0, 2.0, Color::new(0.13, 0.83, 0.93, 0.55));
}

/// The ten lines of numbers down the left of the screen, each with the
/// colour and the place it is drawn in.
struct HudLine {
    text: String,
    y: f32,
    size: f32,
    color: Color,
}

/// Everything the lines of numbers need, gathered in one place.
struct HudContext<'a> {
    world: &'a World,
    creatures: &'a CreatureSystem,
    hud: &'a HudState,
    place: String,
    season_name: &'static str,
    time_of_day: &'static str,
    air_name: &'static str,
    leader: Option<u32>,
    leader_traits: Vec<&'static str>,
}

/// The text of the ten lines, made here so that the drawing is one
/// short loop.
fn hud_lines(ctx: &HudContext) -> Vec<HudLine> {
    let mut lines = hud_head_lines(ctx);
    lines.extend(hud_tail_lines(ctx));
    lines
}

/// The first six lines: the run, the world, the place and the lineage.
fn hud_head_lines(ctx: &HudContext) -> Vec<HudLine> {
    let world = ctx.world;
    let creatures = ctx.creatures;
    let hud = ctx.hud;
    let stats = &creatures.stats;

    let line1 = format!(
        "seed {}   REVIVAL {}   {} organisms   {} species   gen {}",
        hud.seed, hud.revival, stats.population, stats.species, stats.max_generation
    );

    let zorgs = creatures.zorg_count();
    let deep = if world.zorg_cave_sealed {
        format!("{} ZORGs, THE DEEP IS SEALED", zorgs)
    } else {
        format!("{} ZORGs, the deep is open", zorgs)
    };
    let line2 = format!(
        "{} particles   avg energy {:.0}   fitness {:.2}   {}",
        world.census.particles, stats.avg_energy, stats.avg_fitness, deep
    );

    let line3 = format!(
        "{}  {}  {}  temp {:.0}C  mana {:.0}  spells {}  trades {}  SIMx{:.1}  zoom {:.1}x  {}",
        ctx.place,
        ctx.season_name,
        ctx.time_of_day,
        creatures.temperature,
        stats.avg_mana,
        stats.spells_cast,
        stats.trades,
        hud.sim_speed,
        hud.zoom,
        ctx.air_name
    );
    let line4 = format!(
        "camera: ({:.0},{:.0})   fps {}   births {}   deaths {}  structs {}  towers {}  shots {}  {}",
        hud.cam_x,
        hud.cam_y,
        get_fps(),
        stats.births,
        stats.deaths,
        stats.structures,
        stats.towers,
        stats.shots,
        world.weather.name()
    );
    let line5 = format!(
        "hunger {:.0}  thirst {:.0}  stamina {:.0}  diseases {}",
        stats.avg_hunger, stats.avg_thirst, stats.avg_stamina, stats.diseases
    );
    let line6 = match ctx.leader {
        Some(sp) => format!(
            "lineage {} evolved: {}   mind: {}",
            sp,
            if ctx.leader_traits.is_empty() {
                "no special traits yet".to_string()
            } else {
                ctx.leader_traits.join(", ")
            },
            mind_summary(creatures, sp)
        ),
        None => "lineage: none".to_string(),
    };

    vec![
        HudLine { text: line1, y: 66.0, size: 17.0, color: Color::new(0.85, 0.9, 0.95, 0.8) },
        HudLine { text: line2, y: 88.0, size: 17.0, color: Color::new(0.6, 0.66, 0.74, 0.8) },
        HudLine { text: line3, y: 110.0, size: 17.0, color: Color::new(0.7, 0.8, 0.6, 0.85) },
        HudLine { text: line4, y: 132.0, size: 17.0, color: Color::new(0.45, 0.5, 0.58, 0.8) },
        HudLine { text: line5, y: 154.0, size: 17.0, color: Color::new(0.6, 0.5, 0.58, 0.8) },
        HudLine { text: line6, y: 176.0, size: 17.0, color: Color::new(0.55, 0.85, 0.7, 0.85) },
    ]
}

/// The last four lines: the pressure, the diplomacy, the ages and the
/// succession.
fn hud_tail_lines(ctx: &HudContext) -> Vec<HudLine> {
    let world = ctx.world;
    let creatures = ctx.creatures;
    let stats = &creatures.stats;

    let p = &world.pressure;
    let line7 = format!(
        "pressure: scarcity {:.0}%  depletion {:.0}%  harshness {:.0}%  extremes {:.0}%  volcanic {:.0}%   difficulty {:.0}%",
        p.scarcity * 100.0,
        p.depletion * 100.0,
        p.harshness * 100.0,
        p.extremes * 100.0,
        p.volcanic * 100.0,
        world.hardship() * 100.0
    );

    let line8 = diplomacy_summary(creatures);

    let line9 = format!(
        "ages: {} young  {} old   pairs {}   splits {}   practices {}   discoveries {}",
        stats.juveniles,
        stats.elders,
        stats.matings,
        stats.splits,
        stats.practices,
        stats.discoveries
    );

    let crises = creatures.species_in_crisis();
    let line10 = if crises.is_empty() {
        String::new()
    } else {
        let names: Vec<String> = crises
            .iter()
            .map(|sp| crate::creature::species_name_for(*sp))
            .collect();
        format!("SUCCESSION CRISIS: {}", names.join(", "))
    };

    vec![
        HudLine { text: line7, y: 198.0, size: 15.0, color: Color::new(0.85, 0.55, 0.45, 0.8) },
        HudLine { text: line8, y: 218.0, size: 15.0, color: Color::new(0.45, 0.7, 1.0, 0.82) },
        HudLine { text: line9, y: 238.0, size: 15.0, color: Color::new(0.8, 0.65, 0.85, 0.82) },
        HudLine { text: line10, y: 258.0, size: 15.0, color: Color::new(0.95, 0.55, 0.35, 0.9) },
    ]
}

/// The ten lines of numbers, drawn one under the other.
fn draw_hud_lines(lines: &[HudLine]) {
    for line in lines {
        if line.text.is_empty() {
            continue;
        }
        draw_text(&line.text, 18.0, line.y, line.size, line.color);
    }
}

/// The banner over the top of the screen when the mountain is awake.
fn draw_volcano_banner(world: &World) {
    match world.volcano_state() {
        crate::world::Volcano::Erupting => {
            let flash = 0.6 + 0.4 * (get_time() as f32 * 6.0).sin();
            let w = 360.0;
            let x = (screen_width() - w) / 2.0;
            let y = 92.0;
            draw_rectangle(x, y, w, 40.0, Color::new(0.18, 0.03, 0.02, 0.82));
            draw_rectangle_lines(x, y, w, 40.0, 2.0, Color::new(1.0, 0.45, 0.15, 0.55 + 0.4 * flash));
            draw_text(
                "THE VOLCANO IS ERUPTING",
                x + 20.0,
                y + 27.0,
                22.0,
                Color::new(1.0, 0.65, 0.3, 0.6 + 0.4 * flash),
            );
        }
        crate::world::Volcano::Grumbling => {
            let w = 300.0;
            let x = (screen_width() - w) / 2.0;
            draw_rectangle(x, 92.0, w, 30.0, Color::new(0.12, 0.06, 0.03, 0.7));
            draw_rectangle_lines(x, 92.0, w, 30.0, 1.5, Color::new(1.0, 0.55, 0.2, 0.5));
            draw_text(
                "the mountain is grumbling",
                x + 20.0,
                92.0 + 21.0,
                16.0,
                Color::new(1.0, 0.6, 0.3, 0.8),
            );
        }
        crate::world::Volcano::Sleeping => {}
    }
}

/// The keys, along the bottom right.
fn draw_help_line() {
    let help = "WASD/Arrows: move  |  scroll: zoom  |  P: pause  |  R: reroll  |  K: knowledge  |  G: diplomacy  |  M: minds  |  +/-: sim speed  |  hover: inspect  |  ESC: quit";
    let dims = measure_text(help, None, 15, 1.0);
    draw_text(
        help,
        screen_width() - dims.width - 18.0,
        screen_height() - 16.0,
        15.0,
        Color::new(0.4, 0.45, 0.52, 0.7),
    );
}

/// The word in the middle of the screen when the run is held still.
fn draw_paused_banner(paused: bool) {
    if !paused {
        return;
    }
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

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}
