use crate::creature::{CreatureSystem, trait_list};
use crate::ui::diplomacy::diplomacy_summary;
use crate::world::World;
use crate::W;
use macroquad::prelude::*;

/// Rolling ecology history for the sparkline + event feed.
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


/// Top-right ecology panel: population/species sparkline plus recent events.
pub fn draw_telemetry(t: &Telemetry, x: f32, y: f32) {
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


/// The species with the most members and the traits it has evolved.
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
        .map(|c| trait_list(c.traits).into_iter().map(|t| t.name()).collect())
        .unwrap_or_default();
    (Some(sp), traits)
}

pub fn draw_hud(world: &World, creatures: &CreatureSystem, cam_x: f32, cam_y: f32, seed: u64, sim_speed: f32, paused: bool, zoom: f32, revival: u32) {
    let stats = &creatures.stats;
    let biome_name = world.biome_at((cam_x as i32).clamp(0, W as i32 - 1)).name();
    let weather_name = world.weather.name();
    // The dominant lineage and the traits it evolved.
    let (leader, leader_traits) = dominant_lineage(creatures);

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
        "{}  {}  {}  temp {:.0}C  mana {:.0}  spells {}  trades {}  SIMx{:.1}  zoom {:.1}x",
        biome_name,
        season_name,
        time_of_day,
        creatures.temperature,
        stats.avg_mana,
        stats.spells_cast,
        stats.trades,
        sim_speed,
        zoom
    );
    let line4 = format!(
        "camera: ({:.0},{:.0})   fps {}   births {}   deaths {}  structs {}  towers {}  shots {}  {}",
        cam_x,
        cam_y,
        get_fps(),
        stats.births,
        stats.deaths,
        stats.structures,
        stats.towers,
        stats.shots,
        weather_name
    );
    let line5 = format!(
        "hunger {:.0}  thirst {:.0}  stamina {:.0}  diseases {}",
        stats.avg_hunger, stats.avg_thirst, stats.avg_stamina, stats.diseases
    );
    let line6 = match leader {
        Some(sp) => format!(
            "lineage {} evolved: {}",
            sp,
            if leader_traits.is_empty() {
                "no special traits yet".to_string()
            } else {
                leader_traits.join(", ")
            }
        ),
        None => "lineage: none".to_string(),
    };

    // The environmental pressure, shown as separate bars. Each one rises for
    // its own reason, so the player can see which part of the world is
    // turning against them.
    let p = &world.pressure;
    let line7 = format!(
        "pressure: scarcity {:.0}%  depletion {:.0}%  harshness {:.0}%  extremes {:.0}%  volcanic {:.0}%",
        p.scarcity * 100.0,
        p.depletion * 100.0,
        p.harshness * 100.0,
        p.extremes * 100.0,
        p.volcanic * 100.0
    );
    // The diplomatic state of the world, in one line.
    let line8 = diplomacy_summary(creatures);
    // The life of the world: how the colony is aged, and what it has
    // learned.
    let line9 = format!(
        "ages: {} young  {} old   pairs {}   splits {}   practices {}",
        stats.juveniles,
        stats.elders,
        stats.matings,
        stats.splits,
        stats.practices
    );

    draw_text(&line1, 18.0, 66.0, 17.0, Color::new(0.85, 0.9, 0.95, 0.8));
    draw_text(&line2, 18.0, 88.0, 17.0, Color::new(0.6, 0.66, 0.74, 0.8));
    draw_text(&line3, 18.0, 110.0, 17.0, Color::new(0.7, 0.8, 0.6, 0.85));
    draw_text(&line4, 18.0, 132.0, 17.0, Color::new(0.45, 0.5, 0.58, 0.8));
    draw_text(&line5, 18.0, 154.0, 17.0, Color::new(0.6, 0.5, 0.58, 0.8));
    draw_text(&line6, 18.0, 176.0, 17.0, Color::new(0.55, 0.85, 0.7, 0.85));
    draw_text(&line7, 18.0, 198.0, 15.0, Color::new(0.85, 0.55, 0.45, 0.8));
    draw_text(&line8, 18.0, 218.0, 15.0, Color::new(0.45, 0.7, 1.0, 0.82));
    draw_text(&line9, 18.0, 238.0, 15.0, Color::new(0.8, 0.65, 0.85, 0.82));

    let help = "WASD/Arrows: move  |  scroll: zoom  |  P: pause  |  R: reroll  |  K: knowledge  |  G: diplomacy  |  +/-: sim speed  |  hover: inspect  |  ESC: quit";
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

