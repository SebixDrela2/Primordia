use macroquad::prelude::*;

/// Full-screen state shown when one species has wiped out all others (or the
/// whole population died out).
pub struct VictoryScreen {
    pub     winner: String,
    pub     color: [u8; 3],
    pub     survivors: u32,
    pub     generation: u32,
    pub     revival: u32,
    pub     timer: f32,
    pub     extinction: bool,
    /// How the winner got there: alliances it held, wars it fought and the
    /// treaties it signed.
    pub     alliances: u32,
    pub     wars: u32,
    pub     treaties: u32,
}

pub fn draw_victory(v: &VictoryScreen) {
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

    // The diplomatic record of the run: it says how the winner got here.
    let diplo = format!(
        "{} alliances   {} wars fought   {} treaties signed",
        v.alliances, v.wars, v.treaties
    );
    let dd = measure_text(&diplo, None, 14, 1.0);
    draw_text(
        &diplo,
        (sw - dd.width) / 2.0,
        y + 260.0,
        14.0,
        Color::new(0.45, 0.72, 1.0, 0.88),
    );

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
