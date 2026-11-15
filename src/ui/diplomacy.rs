use crate::creature::{CreatureSystem, Diplomacy, Relation, RelationState, Treaty};
use macroquad::prelude::*;

const PANEL_W: f32 = 336.0;
const PANEL_H: f32 = 400.0;

pub fn draw_diplomacy_panel(creatures: &CreatureSystem, panel_x: f32, panel_y: f32) {
    let diplomacy = &creatures.diplomacy;

    draw_rectangle(panel_x, panel_y, PANEL_W, PANEL_H, Color::new(0.03, 0.05, 0.08, 0.88));
    draw_rectangle_lines(panel_x, panel_y, PANEL_W, PANEL_H, 1.5, Color::new(0.35, 0.55, 0.85, 0.45));

    draw_diplomacy_header(diplomacy, panel_x, panel_y);

    let mut yy = panel_y + 54.0;

    draw_text(
        "RELATIONS",
        panel_x + 12.0,
        yy,
        11.0,
        Color::new(0.55, 0.62, 0.7, 0.85),
    );
    yy += 15.0;

    let rows = sorted_relations(diplomacy);

    if rows.is_empty() {
        draw_text(
            "no species has met another yet",
            panel_x + 12.0,
            yy,
            11.0,
            Color::new(0.45, 0.52, 0.6, 0.75),
        );
        yy += 16.0;
    }

    for (shown, (_, relation)) in rows.iter().enumerate() {
        if shown >= 5 || yy > panel_y + PANEL_H - 150.0 {
            break;
        }
        draw_relation_row(creatures, relation, panel_x, yy);
        yy += 44.0;
    }

    if yy < panel_y + PANEL_H - 60.0 {
        draw_diplomatic_record(diplomacy, panel_x, panel_y, yy);
    }
}

/// The title of the panel and the two lines of counts under it.
fn draw_diplomacy_header(diplomacy: &Diplomacy, panel_x: f32, panel_y: f32) {
    draw_text(
        "DIPLOMACY",
        panel_x + 12.0,
        panel_y + 20.0,
        14.0,
        Color::new(0.45, 0.72, 1.0, 0.95),
    );

    let stats = &diplomacy.stats;
    draw_text(
        format!(
            "alliances {}   wars {}   treaties {}",
            stats.alliances, stats.wars, stats.treaties
        ),
        panel_x + 104.0,
        panel_y + 20.0,
        12.0,
        Color::new(0.8, 0.86, 0.92, 0.85),
    );
    draw_text(
        format!(
            "envoys {}   talks {}   truces {}",
            stats.envoys, stats.negotiations, stats.truces
        ),
        panel_x + 104.0,
        panel_y + 35.0,
        11.0,
        Color::new(0.62, 0.7, 0.78, 0.8),
    );
}

/// The relations, worst news first: wars, then allies, then the rest.
fn sorted_relations(diplomacy: &Diplomacy) -> Vec<(i32, &Relation)> {
    let mut rows: Vec<(i32, &Relation)> = diplomacy
        .relations
        .iter()
        .map(|relation| {
            let score = match relation.state {
                RelationState::War => 0,
                RelationState::Allied => 1,
                RelationState::Overlord | RelationState::Vassal => 2,
                RelationState::Hostile => 3,
                RelationState::Cordial => 4,
                RelationState::Neutral => 5,
            };
            (score, relation)
        })
        .collect();
    rows.sort_by_key(|(score, relation)| (*score, -(relation.trust.abs() as i32)));
    rows
}

/// One line of the relation list: the two colours, the names, the
/// populations, the state, the treaties and the trust bar.
fn draw_relation_row(creatures: &CreatureSystem, relation: &Relation, panel_x: f32, yy: f32) {
    let an = creatures.lineage_name(relation.a);
    let bn = creatures.lineage_name(relation.b);
    let col = relation.state.color();
    let tint = Color::new(col[0] as f32 / 255.0, col[1] as f32 / 255.0, col[2] as f32 / 255.0, 0.95);

    let ca = creatures.species_color(relation.a);
    let cb = creatures.species_color(relation.b);
    draw_rectangle(
        panel_x + 12.0,
        yy - 1.0,
        6.0,
        10.0,
        Color::new(ca[0] as f32 / 255.0, ca[1] as f32 / 255.0, ca[2] as f32 / 255.0, 0.95),
    );
    draw_rectangle(
        panel_x + 19.0,
        yy - 1.0,
        6.0,
        10.0,
        Color::new(cb[0] as f32 / 255.0, cb[1] as f32 / 255.0, cb[2] as f32 / 255.0, 0.95),
    );

    let label = format!("{} <-> {}", an, bn);
    draw_text(&label, panel_x + 30.0, yy, 12.0, Color::new(0.86, 0.9, 0.95, 0.92));

    let pa = creatures.species_population(relation.a);
    let pb = creatures.species_population(relation.b);
    let sizes = format!("{} vs {}", pa, pb);
    let sw = measure_text(&sizes, None, 10, 1.0).width;
    draw_text(
        &sizes,
        panel_x + PANEL_W - sw - 66.0,
        yy + 2.0,
        10.0,
        Color::new(0.55, 0.62, 0.7, 0.8),
    );

    let state_w = measure_text(relation.state.name(), None, 11, 1.0).width;
    draw_text(
        relation.state.name(),
        panel_x + PANEL_W - state_w - 12.0,
        yy + 1.0,
        11.0,
        tint,
    );

    let treaties: Vec<&str> = relation.treaties.iter().map(|t| t.name()).collect();
    let tlabel = if treaties.is_empty() {
        relation.state.blurb().to_string()
    } else {
        treaties.join(", ")
    };
    draw_text(
        &tlabel,
        panel_x + 30.0,
        yy + 16.0,
        10.0,
        Color::new(0.62, 0.7, 0.78, 0.8),
    );

    draw_trust_bar(relation, panel_x, yy + 32.0);
}

/// The trust of a relation, drawn as a bar that grows from the middle.
fn draw_trust_bar(relation: &Relation, panel_x: f32, bar_y: f32) {
    let bar_x = panel_x + 30.0;
    let bar_w = PANEL_W - 42.0;
    draw_rectangle(bar_x, bar_y, bar_w, 5.0, Color::new(0.10, 0.12, 0.16, 0.9));
    let mid = bar_x + bar_w * 0.5;
    let trust_ratio = (relation.trust / 100.0).clamp(-1.0, 1.0);
    let trust_col = if trust_ratio >= 0.0 {
        Color::new(0.35, 0.85, 0.55, 0.9)
    } else {
        Color::new(0.9, 0.35, 0.32, 0.9)
    };
    let len = trust_ratio.abs() * bar_w * 0.5;
    if trust_ratio >= 0.0 {
        draw_rectangle(mid, bar_y, len, 5.0, trust_col);
    } else {
        draw_rectangle(mid - len, bar_y, len, 5.0, trust_col);
    }
    draw_rectangle_lines(bar_x, bar_y, bar_w, 5.0, 1.0, Color::new(0.2, 0.24, 0.3, 0.6));
}

/// The tail of the panel: the last few things that happened.
fn draw_diplomatic_record(diplomacy: &Diplomacy, panel_x: f32, panel_y: f32, yy: f32) {
    draw_line(
        panel_x + 12.0,
        yy - 6.0,
        panel_x + PANEL_W - 12.0,
        yy - 6.0,
        1.0,
        Color::new(0.2, 0.26, 0.34, 0.6),
    );
    draw_text(
        "DIPLOMATIC RECORD",
        panel_x + 12.0,
        yy + 4.0,
        11.0,
        Color::new(0.55, 0.62, 0.7, 0.85),
    );
    let mut yy = yy + 18.0;
    for entry in diplomacy.log.iter().rev().take(5) {
        if yy > panel_y + PANEL_H - 12.0 {
            break;
        }
        draw_text(
            &entry.text,
            panel_x + 12.0,
            yy,
            10.5,
            Color::new(entry.color[0] as f32 / 255.0, entry.color[1] as f32 / 255.0, entry.color[2] as f32 / 255.0, 0.88),
        );
        yy += 13.0;
    }
}

#[must_use]
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

#[must_use]
pub fn treaties_of(c: &CreatureSystem, species: u32) -> Vec<Treaty> {
    let mut v = Vec::new();
    for r in &c.diplomacy.relations {
        if r.a == species || r.b == species {
            for t in &r.treaties {
                v.push(*t);
            }
        }
    }
    v
}

#[must_use]
pub fn treaty_blurbs(c: &CreatureSystem, species: u32) -> Vec<String> {
    treaties_of(c, species)
        .iter()
        .map(|t| format!("{}: {}", t.name(), t.blurb()))
        .collect()
}
