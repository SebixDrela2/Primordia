//! The mind inspector: a look inside the brains of the cleverest animals.
//!
//! Press **M** and this window opens. It shows the cleverest creatures of
//! the cleverest *colony* — never the ZORGs, who are the deep's own
//! ecology and not a people, and who have no diplomacy worth reading.
//!
//! One animal is shown at a time, and it is shown big: the whole network
//! fills the window. The **scroll wheel** moves from one mind to the next,
//! so the reader can walk through the cleverest animals of the colony
//! and watch how their minds differ.
//!
//! Each animal is drawn as the network it actually is: the senses on the
//! left, the hidden layers in the middle, the decisions on the right, and
//! *every* connection between them drawn as a line. A line that is
//! pushing a unit the way it is already going is green; one that is
//! pushing it the other way is red; and one that carries nothing is grey.
//!
//! The nodes are sized by how hard they are firing, so you can see the
//! shape of what the brain is doing at a glance.

use crate::creature::senses::{self, DIR_CHANNELS, DIRS};
use crate::creature::{CreatureSystem, Faction, MindView};
use macroquad::prelude::*;

/// How many creatures are listed.
const SHOWN: usize = 5;

/// The radius of a node.
const R: f32 = 3.0;

/// One animal, as the inspector sees it.
struct Subject {
    name: String,
    species: u32,
    generation: u32,
    fitness: f32,
    colour: [u8; 3],
    mind: MindView,
}

/// Find the cleverest animals of the cleverest colony.
fn gather(creatures: &CreatureSystem) -> Vec<Subject> {
    // The cleverest colony: the one with the best total fitness.
    let mut colony_fitness: Vec<(u32, f32)> = Vec::new();
    for c in &creatures.list {
        // The deep is not a people, and its members have no diplomacy
        // worth reading. They are left out of the picture entirely.
        if c.faction == Faction::Zorg {
            continue;
        }
        match colony_fitness.iter_mut().find(|(sp, _)| *sp == c.species) {
            Some(e) => e.1 += c.fitness,
            None => colony_fitness.push((c.species, c.fitness)),
        }
    }
    colony_fitness.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(&(best_species, _)) = colony_fitness.first() else {
        return Vec::new();
    };

    // Within that colony, the cleverest.
    let mut best: Vec<&crate::creature::Creature> = creatures
        .list
        .iter()
        .filter(|c| c.species == best_species && c.faction != Faction::Zorg)
        .collect();
    best.sort_by(|a, b| {
        b.fitness
            .partial_cmp(&a.fitness)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    best.truncate(SHOWN);

    best.iter()
        .map(|c| Subject {
            name: crate::creature::species_name_for(c.species),
            species: c.species,
            generation: c.generation,
            fitness: c.fitness,
            colour: c.team_color,
            mind: c.net.inspect(),
        })
        .collect()
}

/// Draw the whole window.
pub fn draw_minds(creatures: &CreatureSystem, index: f32) {
    let subjects = gather(creatures);
    let sw = screen_width();
    let sh = screen_height();
    let w = (sw * 0.97).min(1900.0);
    let h = (sh * 0.95).min(1150.0);
    let x = (sw - w) * 0.5;
    let y = (sh - h) * 0.5;

    draw_rectangle(x, y, w, h, Color::new(0.03, 0.05, 0.08, 0.98));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.25, 0.55, 0.7, 0.9));
    draw_text(
        "THE MINDS OF THE CLEVEREST",
        x + 18.0,
        y + 30.0,
        24.0,
        Color::new(0.4, 0.85, 0.95, 1.0),
    );
    draw_text(
        "the cleverest animals of the cleverest colony - the deep is not a people and is not shown",
        x + 18.0,
        y + 52.0,
        13.0,
        Color::new(0.6, 0.68, 0.75, 0.9),
    );
    draw_text(
        "scroll: another mind     close: M or Escape",
        x + w - 300.0,
        y + 30.0,
        13.0,
        Color::new(0.5, 0.58, 0.66, 0.9),
    );

    if subjects.is_empty() {
        draw_text(
            "no colony has risen yet",
            x + 18.0,
            y + 110.0,
            18.0,
            Color::new(0.7, 0.7, 0.7, 0.9),
        );
        return;
    }

    // Which mind is on show. The scroll wheel walks through them.
    let n = subjects.len();
    let idx = (index.round().max(0.0) as usize).min(n - 1);

    // The row of minds along the top: the reader can see them all and
    // know which one is open.
    let chips_y = y + 72.0;
    let mut cx = x + 18.0;
    for (i, s) in subjects.iter().enumerate() {
        let label = format!("{}  gen {}  fit {:.2}", s.name, s.generation, s.fitness);
        let tw = label.len() as f32 * 6.6 + 26.0;
        let sel = i == idx;
        let base = Color::new(
            s.colour[0] as f32 / 255.0,
            s.colour[1] as f32 / 255.0,
            s.colour[2] as f32 / 255.0,
            1.0,
        );
        draw_rectangle(
            cx,
            chips_y - 15.0,
            tw,
            24.0,
            if sel {
                Color::new(base.r * 0.45 + 0.08, base.g * 0.45 + 0.08, base.b * 0.45 + 0.08, 1.0)
            } else {
                Color::new(0.07, 0.09, 0.13, 1.0)
            },
        );
        draw_rectangle_lines(
            cx,
            chips_y - 15.0,
            tw,
            24.0,
            if sel { 1.6 } else { 1.0 },
            if sel {
                base
            } else {
                Color::new(0.2, 0.26, 0.34, 0.9)
            },
        );
        draw_text(
            &label,
            cx + 13.0,
            chips_y + 1.0,
            12.0,
            if sel {
                Color::new(0.92, 0.96, 1.0, 1.0)
            } else {
                Color::new(0.52, 0.58, 0.66, 0.95)
            },
        );
        cx += tw + 8.0;
    }

    // The network itself: the whole of the window below the chips.
    let top = chips_y + 26.0;
    let legend_h = 56.0;
    let area_h = (y + h - legend_h) - top;
    let s = &subjects[idx];
    draw_subject(s, x + 14.0, top, w - 28.0, area_h);

    // The legend, at the foot of the window: one legend for the lot.
    let legend_y = y + h - legend_h;
    draw_rectangle(
        x,
        legend_y,
        w,
        legend_h,
        Color::new(0.03, 0.05, 0.08, 0.99),
    );
    draw_legend(x + 18.0, legend_y);
}

/// One column of the input side: a direction, or the body, or the kin.
struct InputColumn {
    /// Where the column sits.
    x: f32,
    /// What the column is.
    label: &'static str,
    /// The first input of the column in the input vector.
    first: usize,
    /// How many inputs it has.
    count: usize,
    /// The node of each input.
    ys: Vec<f32>,
}

/// Where each column of the diagram sits.
struct Layout {
    /// The input columns, left to right.
    inputs: Vec<InputColumn>,
    /// The x of each hidden column.
    hidden: Vec<f32>,
    /// The x of the output column.
    output: f32,
    /// The y of the first node.
    top: f32,
    /// The height of the drawing area.
    height: f32,
    /// The distance between two nodes of a hidden layer.
    step: f32,
}

/// Work out where everything sits.
fn layout(x: f32, y: f32, w: f32, h: f32, mind: &MindView) -> Layout {
    // The header sits above the diagram, so the diagram starts well below
    // the top of its area.
    let top = y + 74.0;
    let height = (h - 96.0).max(120.0);
    // The tallest hidden column decides the spacing of the hidden layers
    // and of the decisions.
    let tallest = mind
        .hidden_layers
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(mind.outputs);
    let step = (height / tallest as f32).min(22.0).max(4.0);

    // ---- The input side ----
    //
    // Ten columns: the eight directions, then the body and the kin.
    // Each column is spread over the whole height, so a short column
    // (a direction, with seventeen channels) is still readable.
    let dir_ch = DIR_CHANNELS;
    let mut inputs: Vec<InputColumn> = Vec::new();
    let mut cols: Vec<(usize, usize, &'static str)> = Vec::new();
    for d in 0..DIRS {
        cols.push((d * dir_ch, dir_ch, DIR_SHORT[d]));
    }
    cols.push((
        senses::BODY_BASE,
        senses::BODY_CHANNELS,
        "body",
    ));
    cols.push((senses::KIN_BASE, senses::KIN_CHANNELS, "kin"));
    let in_w = (w * 0.24).max(220.0);
    let col_w = in_w / cols.len() as f32;
    for (i, (first, count, label)) in cols.iter().enumerate() {
        let cx = x + 40.0 + i as f32 * col_w;
        let step_i = (height / *count as f32).max(2.0);
        let ys = (0..*count).map(|k| top + k as f32 * step_i).collect();
        inputs.push(InputColumn {
            x: cx,
            label,
            first: *first,
            count: *count,
            ys,
        });
    }

    // ---- The hidden side ----
    let out_w = (w * 0.20).max(190.0);
    let ncols = mind.hidden_layers.len().max(1);
    let hid_x = x + in_w + 60.0;
    let hid_w = (w - in_w - out_w - 120.0).max(200.0);
    let hcol = hid_w / ncols as f32;
    let hidden: Vec<f32> = (0..ncols).map(|c| hid_x + c as f32 * hcol).collect();

    Layout {
        inputs,
        hidden,
        output: x + w - out_w,
        top,
        height,
        step,
    }
}

/// The eight directions, for the tight column headings.
///
/// Short names: the columns stand close together and a long name would
/// run into its neighbour.
const DIR_SHORT: [&str; 8] = ["S", "SE", "E", "NE", "N", "NW", "W", "SW"];

/// Draw one animal's mind.
fn draw_subject(s: &Subject, x: f32, y: f32, w: f32, h: f32) {
    // The header: who it is and how it is doing.
    let col = Color::new(
        s.colour[0] as f32 / 255.0,
        s.colour[1] as f32 / 255.0,
        s.colour[2] as f32 / 255.0,
        1.0,
    );
    draw_text(
        &format!(
            "{}   species {}   generation {}   fitness {:.2}",
            s.name, s.species, s.generation, s.fitness
        ),
        x + 4.0,
        y + 14.0,
        17.0,
        col,
    );
    draw_text(
        &format!(
            "tier {} mind   {} hidden units   {} decisions   {} firing with   {} senses in",
            s.mind.arch,
            s.mind.hidden_layers.iter().sum::<usize>(),
            s.mind.outputs,
            crate::creature::activation::name(s.mind.activation),
            s.mind.channels.len()
        ),
        x + 470.0,
        y + 14.0,
        12.0,
        Color::new(0.58, 0.66, 0.74, 0.9),
    );

    let lay = layout(x, y, w, h, &s.mind);
    draw_connections(s, &lay);
    draw_inputs(s, &lay);
    draw_hidden(s, &lay);
    draw_outputs(s, &lay);
}

/// The colour of a sensory channel, so the legend and the picture agree.
fn channel_colour(kind: usize, index: usize) -> Color {
    match kind {
        // Sight.
        0 => match index {
            senses::sight::NEAR => Color::new(0.95, 0.95, 1.0, 1.0),
            senses::sight::LIGHT => Color::new(1.0, 0.93, 0.65, 1.0),
            senses::sight::RED => Color::new(0.95, 0.35, 0.3, 1.0),
            senses::sight::GREEN => Color::new(0.35, 0.9, 0.4, 1.0),
            senses::sight::BLUE => Color::new(0.35, 0.55, 1.0, 1.0),
            senses::sight::WARM => Color::new(1.0, 0.5, 0.2, 1.0),
            senses::sight::SOFT => Color::new(0.78, 0.65, 0.95, 1.0),
            senses::sight::WET => Color::new(0.25, 0.8, 0.9, 1.0),
            senses::sight::SHEEN => Color::new(0.9, 0.85, 0.45, 1.0),
            _ => Color::new(0.55, 0.95, 0.85, 1.0),
        },
        // Smell.
        _ => match index {
            senses::smell::SWEET => Color::new(0.98, 0.72, 0.25, 1.0),
            senses::smell::ROTTEN => Color::new(0.5, 0.75, 0.3, 1.0),
            senses::smell::ACID => Color::new(0.8, 0.95, 0.25, 1.0),
            senses::smell::MINERAL => Color::new(0.6, 0.68, 0.78, 1.0),
            senses::smell::SMOKE => Color::new(0.75, 0.5, 0.5, 1.0),
            _ => Color::new(0.45, 0.8, 0.95, 1.0),
        },
    }
}

/// The name of a sensory channel, for the legend.
fn channel_name(kind: usize, index: usize) -> &'static str {
    if kind == 0 {
        match index {
            senses::sight::NEAR => "near",
            senses::sight::LIGHT => "light",
            senses::sight::RED => "red",
            senses::sight::GREEN => "green",
            senses::sight::BLUE => "blue",
            senses::sight::WARM => "warm",
            senses::sight::SOFT => "soft",
            senses::sight::WET => "wet",
            senses::sight::SHEEN => "gloss",
            _ => "alive",
        }
    } else {
        match index {
            senses::smell::SWEET => "sweet",
            senses::smell::ROTTEN => "rotten",
            senses::smell::ACID => "acid",
            senses::smell::MINERAL => "mineral",
            senses::smell::SMOKE => "smoke",
            _ => "own kind",
        }
    }
}

/// The name of a decision.
fn decision_name(o: usize) -> &'static str {
    match o {
        0 => "move x",
        1 => "move y",
        2 => "eat",
        3 => "reproduce",
        4 => "attack",
        5 => "build",
        6 => "craft",
        7 => "cast",
        8 => "mount",
        9 => "trade",
        10 => "rest",
        11 => "swap spell",
        12 => "mine",
        13 => "drink",
        14 => "shoot",
        15 => "flee",
        16 => "seek",
        17 => "forage",
        18 => "rally",
        19 => "defend",
        20 => "negotiate",
        21 => "hostile",
        22 => "ally",
        23 => "tribute",
        24 => "court",
        25 => "go south",
        26 => "go south-east",
        27 => "go east",
        28 => "go north-east",
        29 => "go north",
        30 => "go north-west",
        31 => "go west",
        32 => "go south-west",
        _ => "?",
    }
}

/// The legend at the foot of the window.
fn draw_legend(x: f32, y: f32) {
    draw_text(
        "sight:",
        x,
        y + 11.0,
        11.0,
        Color::new(0.6, 0.68, 0.75, 0.95),
    );
    let mut lx = x + 38.0;
    for c in 0..senses::sight::CHANNELS {
        let col = channel_colour(0, c);
        draw_rectangle(lx, y + 4.0, 9.0, 9.0, col);
        let name = channel_name(0, c);
        draw_text(
            name,
            lx + 12.0,
            y + 11.0,
            11.0,
            Color::new(0.68, 0.75, 0.82, 0.95),
        );
        lx += 20.0 + name.len() as f32 * 6.0;
    }
    draw_text(
        "smell:",
        x,
        y + 30.0,
        11.0,
        Color::new(0.6, 0.68, 0.75, 0.95),
    );
    let mut lx2 = x + 38.0;
    for c in 0..senses::smell::CHANNELS {
        let col = channel_colour(1, c);
        draw_rectangle(lx2, y + 23.0, 9.0, 9.0, col);
        let name = channel_name(1, c);
        draw_text(
            name,
            lx2 + 12.0,
            y + 30.0,
            11.0,
            Color::new(0.68, 0.75, 0.82, 0.95),
        );
        lx2 += 20.0 + name.len() as f32 * 6.0;
    }
    draw_text(
        "green line: it pushes the unit the way it is already going    red line: it pushes the other way    grey line: it carries nothing",
        x + 620.0,
        y + 30.0,
        11.0,
        Color::new(0.6, 0.68, 0.75, 0.95),
    );
}

/// The input layer: the eight directions, the body and the kin.
fn draw_inputs(s: &Subject, lay: &Layout) {
    for col in &lay.inputs {
        draw_text(
            col.label,
            col.x - 22.0,
            lay.top - 10.0,
            9.0,
            Color::new(0.55, 0.62, 0.7, 0.95),
        );
        for k in 0..col.count {
            // The direction columns are coloured by their channel; the
            // body and the kin are drawn in their own colour.
            let v = s
                .mind
                .channels
                .get(col.first + k)
                .copied()
                .unwrap_or(0.0);
            let base = if col.count == DIR_CHANNELS {
                let kind = if k < senses::sight::CHANNELS { 0 } else { 1 };
                let ch = if kind == 0 {
                    k
                } else {
                    k - senses::sight::CHANNELS
                };
                channel_colour(kind, ch)
            } else {
                Color::new(0.55, 0.72, 0.85, 1.0)
            };
            let a = 0.16 + 0.84 * v.abs().min(1.0);
            draw_circle(
                col.x,
                col.ys[k],
                R,
                Color::new(base.r, base.g, base.b, a),
            );
        }
    }
    draw_text(
        "INPUT LAYER",
        lay.inputs[0].x - 30.0,
        lay.top - 28.0,
        13.0,
        Color::new(0.5, 0.75, 0.85, 0.95),
    );
    draw_text(
        "what it senses: eight directions, then its own body, then its own kind",
        lay.inputs[0].x - 30.0,
        lay.top + lay.height + 12.0,
        9.0,
        Color::new(0.5, 0.58, 0.66, 0.9),
    );
}

/// The hidden layers: a column of nodes for each of them.
fn draw_hidden(s: &Subject, lay: &Layout) {
    for (c, count) in s.mind.hidden_layers.iter().enumerate() {
        let cx = lay.hidden[c];
        draw_text(
            &format!("layer {}", c + 1),
            cx - 20.0,
            lay.top - 10.0,
            9.0,
            Color::new(0.55, 0.62, 0.7, 0.95),
        );
        let peak = s.mind.layer_peak(c);
        for i in 0..*count {
            let v = s.mind.layer_fire(c, i);
            let a = (v.abs() / peak.max(0.001)).min(1.0);
            let col = if v.abs() > 0.01 {
                Color::new(0.3 + 0.6 * a, 0.85, 0.98, 0.45 + 0.55 * a)
            } else {
                Color::new(0.3, 0.37, 0.45, 0.7)
            };
            draw_circle(cx, lay.top + i as f32 * lay.step, R, col);
        }
    }
    draw_text(
        "HIDDEN LAYERS",
        lay.hidden[0] - 40.0,
        lay.top - 28.0,
        13.0,
        Color::new(0.5, 0.75, 0.85, 0.95),
    );
}

/// The output layer: one node per decision, labelled.
fn draw_outputs(s: &Subject, lay: &Layout) {
    let cx = lay.output;
    draw_text(
        "OUTPUT LAYER",
        cx - 34.0,
        lay.top - 28.0,
        13.0,
        Color::new(0.5, 0.75, 0.85, 0.95),
    );
    for o in 0..s.mind.outputs {
        let d = s.mind.drive(o);
        let a = d.abs().min(1.0);
        let col = if d >= 0.0 {
            Color::new(0.45, 0.95, 0.65, 0.35 + 0.6 * a)
        } else {
            Color::new(0.95, 0.45, 0.4, 0.35 + 0.6 * a)
        };
        draw_circle(cx, lay.top + o as f32 * lay.step, R, col);
        // Only the decisions that are actually pushing get a name: there
        // are thirty-three of them and they are a hand's width apart.
        if d.abs() > 0.15 {
            draw_text(
                decision_name(o),
                cx + 9.0,
                lay.top + o as f32 * lay.step + 3.0,
                10.0,
                Color::new(0.8, 0.88, 0.95, 0.95),
            );
            draw_text(
                &format!("{:+.2}", d),
                cx + 104.0,
                lay.top + o as f32 * lay.step + 3.0,
                10.0,
                Color::new(0.65, 0.9, 0.78, 0.95),
            );
        }
    }
    draw_text(
        "what it decides",
        cx - 34.0,
        lay.top + lay.height + 12.0,
        9.0,
        Color::new(0.5, 0.58, 0.66, 0.9),
    );
}

/// The colour of a connection, given the pull it applies and what the
/// unit it feeds is doing.
///
/// A connection that pushes a unit the way it is already going is green:
/// it is helping. One that pushes it the other way is red: it is
/// arguing against it.
fn pull_colour(pull: f32, drive: f32, strength: f32) -> Color {
    let agrees = pull.signum() == drive.signum() && drive.abs() > 0.01;
    let a = (0.05 + 0.42 * strength.min(1.0)).min(0.47);
    if agrees {
        Color::new(0.35, 0.95, 0.62, a)
    } else {
        Color::new(0.95, 0.32, 0.32, a)
    }
}

/// How strong a pull has to be before it is worth colouring.
///
/// There are thousands of connections in a mind and they are all drawn, but
/// only the ones that are actually doing something are picked out in
/// colour. The rest are the fabric of the network.
const STRONG: f32 = 0.22;

/// The connections between the layers.
///
/// **Every** connection is drawn: the grey ones are the fabric of the
/// network, and the coloured ones are the ones that are doing something
/// now. They are coloured the same way at every layer, not only at the
/// last one: a line from one hidden layer to the next is green when it
/// pushes the unit it feeds the way that unit is already going, and red
/// when it pushes it the other way.
fn draw_connections(s: &Subject, lay: &Layout) {
    // The fabric: every connection, very faint, so the shape of the network
    // reads as a web rather than as a solid block.
    let grey = Color::new(0.5, 0.58, 0.68, 0.045);

    // ---- The input columns to the first hidden layer ----
    if let Some(&first) = lay.hidden.first() {
        let n0 = s.mind.hidden_layers.first().copied().unwrap_or(0);
        let mut peak = 0.0f32;
        for col in &lay.inputs {
            for i in 0..n0 {
                let p = s.mind.column_pull(0, col.first, col.count, i);
                if p.abs() > peak {
                    peak = p.abs();
                }
            }
        }
        for col in &lay.inputs {
            let sy = col.ys.iter().sum::<f32>() / col.ys.len().max(1) as f32;
            for i in 0..n0 {
                let pull = s.mind.column_pull(0, col.first, col.count, i);
                let hy = lay.top + i as f32 * lay.step;
                if pull.abs() < peak * STRONG {
                    // The fabric: every connection is there, faint.
                    draw_line(col.x, sy, first, hy, 0.4, grey);
                    continue;
                }
                let drive = s.mind.unit_drive(0, i);
                let c = pull_colour(pull, drive, pull.abs() / peak.max(0.001));
                draw_line(col.x, sy, first, hy, 0.8, c);
            }
        }
    }

    // ---- Hidden layer to hidden layer ----
    for c in 0..s.mind.hidden_layers.len().saturating_sub(1) {
        let n0 = s.mind.hidden_layers[c];
        let n1 = s.mind.hidden_layers[c + 1];
        let (x0, x1) = (lay.hidden[c], lay.hidden[c + 1]);
        let mut peak = 0.0f32;
        for i in 0..n0 {
            for j in 0..n1 {
                let p = s.mind.unit_pull(c, i, j);
                if p.abs() > peak {
                    peak = p.abs();
                }
            }
        }
        for i in 0..n0 {
            for j in 0..n1 {
                let pull = s.mind.unit_pull(c, i, j);
                let (sx, sy) = (x0, lay.top + i as f32 * lay.step);
                let (hx, hy) = (x1, lay.top + j as f32 * lay.step);
                if pull.abs() < peak * STRONG {
                    draw_line(sx, sy, hx, hy, 0.35, grey);
                    continue;
                }
                let drive = s.mind.unit_drive(c, j);
                let col = pull_colour(pull, drive, pull.abs() / peak.max(0.001));
                draw_line(sx, sy, hx, hy, 0.8, col);
            }
        }
    }

    // ---- The last hidden layer to the decisions ----
    let last = s.mind.hidden_layers.len().saturating_sub(1);
    if let Some(&lh) = lay.hidden.last() {
        let n = s.mind.hidden_layers.last().copied().unwrap_or(0);
        let mut peak = 0.0f32;
        for i in 0..n {
            for o in 0..s.mind.outputs {
                let p = s.mind.unit_pull(last, i, o);
                if p.abs() > peak {
                    peak = p.abs();
                }
            }
        }
        for i in 0..n {
            for o in 0..s.mind.outputs {
                let pull = s.mind.unit_pull(last, i, o);
                let (sx, sy) = (lh, lay.top + i as f32 * lay.step);
                let (hx, hy) = (lay.output, lay.top + o as f32 * lay.step);
                if pull.abs() < peak * STRONG {
                    draw_line(sx, sy, hx, hy, 0.35, grey);
                    continue;
                }
                let drive = s.mind.drive(o);
                let col = pull_colour(pull, drive, pull.abs() / peak.max(0.001));
                draw_line(sx, sy, hx, hy, 0.9, col);
            }
        }
    }
}
