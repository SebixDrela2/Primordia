use crate::creature::{CreatureSystem, RelationState, Treaty};
use macroquad::prelude::*;

/// The diplomacy panel.
///
/// This is where the player reads what the species have done to each other:
/// who is allied, who is paying tribute, who is at war and how much trust
/// each pair has built up. Without it the whole sector would happen
/// invisibly.
pub fn draw_diplomacy_panel(c: &CreatureSystem, x: f32, y: f32) {
    let w = 336.0f32;
    let h = 400.0f32;
    let d = &c.diplomacy;

    draw_rectangle(x, y, w, h, Color::new(0.03, 0.05, 0.08, 0.88));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.35, 0.55, 0.85, 0.45));

    draw_text(
        "DIPLOMACY",
        x + 12.0,
        y + 20.0,
        14.0,
        Color::new(0.45, 0.72, 1.0, 0.95),
    );

    // The headline counters.
    let s = &d.stats;
    draw_text(
        &format!(
            "alliances {}   wars {}   treaties {}",
            s.alliances, s.wars, s.treaties
        ),
        x + 104.0,
        y + 20.0,
        12.0,
        Color::new(0.8, 0.86, 0.92, 0.85),
    );
    draw_text(
        &format!(
            "envoys {}   talks {}   truces {}",
            s.envoys, s.negotiations, s.truces
        ),
        x + 104.0,
        y + 35.0,
        11.0,
        Color::new(0.62, 0.7, 0.78, 0.8),
    );

    let mut yy = y + 54.0;

    // ---- The relations ----
    draw_text(
        "RELATIONS",
        x + 12.0,
        yy,
        11.0,
        Color::new(0.55, 0.62, 0.7, 0.85),
    );
    yy += 15.0;

    // Sort the relations so the interesting ones (wars, alliances) come
    // first: a player should not have to hunt for the crisis.
    let mut rows: Vec<(i32, &crate::creature::Relation)> = d
        .relations
        .iter()
        .map(|r| {
            let score = match r.state {
                RelationState::War => 0,
                RelationState::Allied => 1,
                RelationState::Overlord | RelationState::Vassal => 2,
                RelationState::Hostile => 3,
                RelationState::Cordial => 4,
                RelationState::Neutral => 5,
            };
            (score, r)
        })
        .collect();    rows.sort_by_key(|(score, r)| (*score, -(r.trust.abs() as i32)));

    if rows.is_empty() {
        draw_text(
            "no species has met another yet",
            x + 12.0,
            yy,
            11.0,
            Color::new(0.45, 0.52, 0.6, 0.75),
        );
        yy += 16.0;
    }

    let mut shown = 0;
    for (_, r) in rows.iter() {
        if shown >= 5 || yy > y + h - 150.0 {
            break;
        }
        shown += 1;
        let an = c.lineage_name(r.a);
        let bn = c.lineage_name(r.b);
        let col = r.state.color();
        let tint = Color::new(col[0] as f32 / 255.0, col[1] as f32 / 255.0, col[2] as f32 / 255.0, 0.95);

        // A colour chip per species, so the pair is recognisable.
        let ca = c.species_color(r.a);
        let cb = c.species_color(r.b);
        draw_rectangle(
            x + 12.0,
            yy - 1.0,
            6.0,
            10.0,
            Color::new(ca[0] as f32 / 255.0, ca[1] as f32 / 255.0, ca[2] as f32 / 255.0, 0.95),
        );
        draw_rectangle(
            x + 19.0,
            yy - 1.0,
            6.0,
            10.0,
            Color::new(cb[0] as f32 / 255.0, cb[1] as f32 / 255.0, cb[2] as f32 / 255.0, 0.95),
        );

        let label = format!("{} <-> {}", an, bn);
        draw_text(&label, x + 30.0, yy, 12.0, Color::new(0.86, 0.9, 0.95, 0.92));

        // How big the two colonies are, so the player can see who is
        // winning the arms race.
        let pa = c.species_population(r.a);
        let pb = c.species_population(r.b);
        let sizes = format!("{} vs {}", pa, pb);
        let sw = measure_text(&sizes, None, 10, 1.0).width;
        draw_text(
            &sizes,
            x + w - sw - 66.0,
            yy + 2.0,
            10.0,
            Color::new(0.55, 0.62, 0.7, 0.8),
        );

        let state_w = measure_text(r.state.name(), None, 11, 1.0).width;
        draw_text(
            r.state.name(),
            x + w - state_w - 12.0,
            yy + 1.0,
            11.0,
            tint,
        );

        // The treaties in force, or what the state means.
        let treaties: Vec<&str> = r.treaties.iter().map(|t| t.name()).collect();
        let tlabel = if treaties.is_empty() {
            r.state.blurb().to_string()
        } else {
            treaties.join(", ")
        };
        draw_text(
            &tlabel,
            x + 30.0,
            yy + 16.0,
            10.0,
            Color::new(0.62, 0.7, 0.78, 0.8),
        );

        // The trust bar sits below the text: red for hatred, green for
        // warmth. It is drawn last so it can never cut through a label.
        let bar_x = x + 30.0;
        let bar_y = yy + 32.0;
        let bar_w = w - 42.0;
        draw_rectangle(bar_x, bar_y, bar_w, 5.0, Color::new(0.10, 0.12, 0.16, 0.9));
        let mid = bar_x + bar_w * 0.5;
        let t = (r.trust / 100.0).clamp(-1.0, 1.0);
        let trust_col = if t >= 0.0 {
            Color::new(0.35, 0.85, 0.55, 0.9)
        } else {
            Color::new(0.9, 0.35, 0.32, 0.9)
        };
        let len = (t.abs() as f32) * bar_w * 0.5;
        if t >= 0.0 {
            draw_rectangle(mid, bar_y, len, 5.0, trust_col);
        } else {
            draw_rectangle(mid - len, bar_y, len, 5.0, trust_col);
        }
        draw_rectangle_lines(bar_x, bar_y, bar_w, 5.0, 1.0, Color::new(0.2, 0.24, 0.3, 0.6));

        yy += 44.0;
    }

    // ---- The event feed ----
    if yy < y + h - 60.0 {
        draw_line(
            x + 12.0,
            yy - 6.0,
            x + w - 12.0,
            yy - 6.0,
            1.0,
            Color::new(0.2, 0.26, 0.34, 0.6),
        );
        draw_text(
            "DIPLOMATIC RECORD",
            x + 12.0,
            yy + 4.0,
            11.0,
            Color::new(0.55, 0.62, 0.7, 0.85),
        );
        yy += 18.0;
        for e in d.log.iter().rev().take(5) {
            if yy > y + h - 12.0 {
                break;
            }
            draw_text(
                &e.text,
                x + 12.0,
                yy,
                10.5,
                Color::new(e.color[0] as f32 / 255.0, e.color[1] as f32 / 255.0, e.color[2] as f32 / 255.0, 0.88),
            );
            yy += 13.0;
        }
    }
}

/// A one-line summary of the diplomatic situation, for the HUD.
pub fn diplomacy_summary(c: &CreatureSystem) -> String {
    let d = &c.diplomacy;
    let s = &d.stats;
    let wars = d
        .relations
        .iter()
        .filter(|r| r.state == RelationState::War)
        .count();
    let allies = d
        .relations
        .iter()
        .filter(|r| r.state == RelationState::Allied)
        .count();
    format!(
        "diplo: {} allied  {} wars  {} treaties  {} envoys  {} talks  {} tributes",
        allies, wars, s.treaties, s.envoys, s.negotiations, s.tributes
    )
}

/// The treaties a species currently holds, as a short list.
pub fn treaties_of(c: &CreatureSystem, species: u32) -> Vec<Treaty> {
    let mut v = Vec::new();
    for r in c.diplomacy.relations.iter() {
        if r.a == species || r.b == species {
            for t in r.treaties.iter() {
                v.push(*t);
            }
        }
    }
    v
}

/// The treaties a species currently holds, for the creature tooltip.
pub fn treaty_blurbs(c: &CreatureSystem, species: u32) -> Vec<String> {
    treaties_of(c, species)
        .iter()
        .map(|t| format!("{}: {}", t.name(), t.blurb()))
        .collect()
}
