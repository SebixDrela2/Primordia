use macroquad::prelude::*;

/// How the run ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// A species stands alone and the deep is closed.
    Won,
    /// The last lineage died out.
    Extinct,
    /// The ZORGs took the world.
    Lost,
}

impl Outcome {
    /// Did a species win the world?
    #[must_use]
    pub fn is_won(self) -> bool {
        matches!(self, Outcome::Won)
    }

    /// Did the run end with the lineage gone rather than conquering?
    #[must_use]
    pub fn lineage_ended(self) -> bool {
        !self.is_won()
    }
}

/// The end-of-run summary. `outcome` says how the run ended; the two
/// flags are the separate facts the screen also reports - whether the
/// deep was sealed and whether the lineage carried over.
pub struct VictoryScreen {
    pub     winner: String,
    pub     color: [u8; 3],
    pub     survivors: u32,
    pub     generation: u32,
    pub     revival: u32,
    pub     timer: f32,
    pub     outcome: Outcome,

    pub     zorgs: u32,

    pub     alliances: u32,
    pub     wars: u32,
    pub     treaties: u32,

    pub     truces: u32,

    pub     traits: Vec<&'static str>,

    pub     discoveries: Vec<&'static str>,

    pub     deep_sealed: bool,

    pub     lineage_carried: bool,
}

pub fn draw_victory(screen: &VictoryScreen) {
    let sw = screen_width();
    let sh = screen_height();

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.01, 0.02, 0.04, 0.66));

    let panel_w = 600.0f32.min(sw - 60.0);
    let panel_h = 360.0f32.min(sh - 60.0);
    let panel_x = (sw - panel_w) / 2.0;
    let panel_y = (sh - panel_h) / 2.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.03, 0.05, 0.09, 0.94));
    let frame = if screen.outcome == Outcome::Lost {
        Color::new(
            crate::creature::ZORG_COLOR[0] as f32 / 255.0,
            crate::creature::ZORG_COLOR[1] as f32 / 255.0,
            crate::creature::ZORG_COLOR[2] as f32 / 255.0,
            0.9,
        )
    } else {
        Color::new(0.13, 0.83, 0.93, 0.9)
    };
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, frame);

    draw_victory_title(screen, sw, panel_y);
    draw_victory_winner(screen, sw, panel_y);
    draw_victory_stats(screen, sw, panel_y);
    draw_victory_lineage(screen, sw, panel_y);
    draw_victory_footer(screen, sw, panel_y, panel_h);
}

/// The big word at the top of the panel and the line under it.
fn draw_victory_title(screen: &VictoryScreen, sw: f32, panel_y: f32) {
    let (title, title_color) = if screen.outcome == Outcome::Lost {

        ("LOST", Color::new(
            crate::creature::ZORG_COLOR[0] as f32 / 255.0,
            crate::creature::ZORG_COLOR[1] as f32 / 255.0,
            crate::creature::ZORG_COLOR[2] as f32 / 255.0,
            1.0,
        ))
    } else if screen.outcome == Outcome::Extinct {
        ("EXTINCTION", Color::new(0.92, 0.36, 0.3, 1.0))
    } else {
        ("VICTORY", Color::new(0.13, 0.83, 0.93, 1.0))
    };
    let td = measure_text(title, None, 48, 1.0);
    draw_text(title, (sw - td.width) / 2.0, panel_y + 74.0, 48.0, title_color);

    let subtitle = if screen.outcome == Outcome::Lost {
        "the ZORGs have taken the world"
    } else if screen.outcome == Outcome::Extinct {
        "the last lineage has died out"
    } else if screen.deep_sealed {
        "one species stands alone, and the deep is sealed"
    } else {
        "one species stands alone"
    };
    let sd = measure_text(subtitle, None, 17, 1.0);
    draw_text(
        subtitle,
        (sw - sd.width) / 2.0,
        panel_y + 100.0,
        17.0,
        Color::new(0.6, 0.68, 0.76, 0.9),
    );
}

/// The name of the lineage, with its colour as a dot beside it.
fn draw_victory_winner(screen: &VictoryScreen, sw: f32, panel_y: f32) {
    let color = Color::new(
        screen.color[0] as f32 / 255.0,
        screen.color[1] as f32 / 255.0,
        screen.color[2] as f32 / 255.0,
        1.0,
    );
    let name = if screen.outcome.lineage_ended() {
        screen.winner.clone()
    } else {
        format!("\"{}\"", screen.winner)
    };
    let nd = measure_text(&name, None, 30, 1.0);
    let name_x = (sw - nd.width) / 2.0;
    draw_circle(name_x - 20.0, panel_y + 148.0 - 9.0, 8.0, color);
    draw_text(&name, name_x, panel_y + 148.0, 30.0, color);
}

/// The one line of counts under the name.
fn draw_victory_stats(screen: &VictoryScreen, sw: f32, panel_y: f32) {
    let stats = if screen.outcome == Outcome::Lost {
        format!(
            "{} ZORGs still standing   generation {}",
            screen.zorgs, screen.generation
        )
    } else if screen.outcome == Outcome::Extinct {
        format!("generation {}", screen.generation)
    } else if screen.deep_sealed {
        format!(
            "{} survivors   {} ZORGs left in a closed cave   generation {}",
            screen.survivors, screen.zorgs, screen.generation
        )
    } else {
        format!("{} survivors   generation {}", screen.survivors, screen.generation)
    };
    let std = measure_text(&stats, None, 16, 1.0);
    draw_text(
        &stats,
        (sw - std.width) / 2.0,
        panel_y + 180.0,
        16.0,
        Color::new(0.78, 0.84, 0.9, 0.9),
    );
}

/// What happens to the lineage now.
fn draw_victory_lineage(screen: &VictoryScreen, sw: f32, panel_y: f32) {
    let (l1, l2) = if screen.outcome.lineage_ended() {
        if screen.lineage_carried {
            (
                format!("Revival {} ends here. The lineage survives.", screen.revival),
                "Descendants inherit the last lineage's genome.".to_string(),
            )
        } else {
            (
                format!("Revival {} ends here. The lineage is lost.", screen.revival),
                "A fresh random population will emerge.".to_string(),
            )
        }
    } else {
        (
            format!("Revival {} conquered. Revival {} begins.", screen.revival, screen.revival + 1),
            "Descendants inherit the winning genome.".to_string(),
        )
    };
    let l1d = measure_text(&l1, None, 16, 1.0);
    draw_text(&l1, (sw - l1d.width) / 2.0, panel_y + 216.0, 16.0, Color::new(0.85, 0.9, 0.95, 0.92));
    let l2d = measure_text(&l2, None, 14, 1.0);
    draw_text(&l2, (sw - l2d.width) / 2.0, panel_y + 238.0, 14.0, Color::new(0.6, 0.66, 0.72, 0.85));
}

/// The tail of the panel: the diplomacy, what was evolved, what was
/// worked out and the countdown.
fn draw_victory_footer(screen: &VictoryScreen, sw: f32, panel_y: f32, panel_h: f32) {
    let diplo = format!(
        "{} alliances   {} wars fought   {} treaties signed   {} truces",
        screen.alliances, screen.wars, screen.treaties, screen.truces
    );
    let dd = measure_text(&diplo, None, 14, 1.0);
    draw_text(
        &diplo,
        (sw - dd.width) / 2.0,
        panel_y + 260.0,
        14.0,
        Color::new(0.45, 0.72, 1.0, 0.88),
    );

    if !screen.traits.is_empty() {
        let evolved = format!("evolved: {}", screen.traits.join(", "));
        let ed = measure_text(&evolved, None, 14, 1.0);
        draw_text(
            &evolved,
            (sw - ed.width) / 2.0,
            panel_y + 282.0,
            14.0,
            Color::new(0.55, 0.9, 0.7, 0.9),
        );
    }
    if !screen.discoveries.is_empty() {
        let found = format!("worked out: {}", screen.discoveries.join(", "));
        let fd = measure_text(&found, None, 13, 1.0);
        draw_text(
            &found,
            (sw - fd.width) / 2.0,
            panel_y + 302.0,
            13.0,
            Color::new(0.85, 0.75, 0.55, 0.9),
        );
    }

    let countdown = format!("restarting in {:.1}s", screen.timer.max(0.0));
    let cd = measure_text(&countdown, None, 14, 1.0);
    draw_text(
        &countdown,
        (sw - cd.width) / 2.0,
        panel_y + panel_h - 26.0,
        14.0,
        Color::new(0.13, 0.83, 0.93, 0.85),
    );
}
