//! The panel of the run: the story of how the colony is doing.
//!
//! When the game is run on a model that was learned, the panel shows
//! what the model is doing: how many of the colony there are, how well
//! the best of them is doing, how much has been eaten and how much has
//! been built. The charts fill as the run goes on.

use macroquad::prelude::*;

use crate::creature::{CreatureSystem, Faction, Model};

/// The story of the run, as it goes on.
pub struct Story {
    /// How many of the colony there were, again and again.
    population: Vec<f32>,
    /// The best of the colony, again and again.
    fitness: Vec<f32>,
    /// How many had been eaten.
    eaten: Vec<f32>,
    /// How many had been built.
    built: Vec<f32>,
    /// The last numbers.
    pub pop: usize,
    pub best: f32,
    pub eats: u64,
    pub builds: u64,
    pub births: u32,
    pub deaths: u32,
    pub ticks: u64,
}

impl Story {
    #[must_use]
    pub fn new() -> Self {
        Story {
            population: Vec::new(),
            fitness: Vec::new(),
            eaten: Vec::new(),
            built: Vec::new(),
            pop: 0,
            best: 0.0,
            eats: 0,
            builds: 0,
            births: 0,
            deaths: 0,
            ticks: 0,
        }
    }

    /// Take note of how the colony is doing.
    pub fn note(&mut self, creatures: &CreatureSystem) {
        let mut pop = 0usize;
        let mut best = f32::MIN;
        for c in &creatures.list {
            if c.faction != Faction::Player {
                continue;
            }
            pop += 1;
            best = best.max(c.fitness);
        }
        self.pop = pop;
        self.best = if pop == 0 { 0.0 } else { best };
        self.eats = creatures.eats_total;
        self.builds = creatures.builds_total;
        self.births = creatures.births_total;
        self.deaths = creatures.deaths_total;
        self.ticks = creatures.tick;
        self.population.push(pop as f32);
        self.fitness.push(self.best);
        self.eaten.push(self.eats as f32);
        self.built.push(self.builds as f32);
    }
}

impl Default for Story {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the panel of the run down the left of the screen, under the
/// words of the game.
pub fn draw_panel(story: &Story, model: &Model, _width: f32, height: f32) {
    let panel_w = 300.0;
    let x = 16.0;
    let y = 268.0;
    let panel_h = (height - y - 130.0).min(272.0);

    // The frame of the panel.
    draw_rectangle(x, y, panel_w, panel_h, Color::new(0.03, 0.05, 0.09, 0.82));
    draw_rectangle_lines(
        x,
        y,
        panel_w,
        panel_h,
        1.0,
        Color::new(0.3, 0.48, 0.66, 0.7),
    );

    // What the model is.
    draw_text(
        format!(
            "the model: generation {}, {} weights",
            model.generation,
            model.weight_count()
        ),
        x + 10.0,
        y + 10.0,
        12.0,
        Color::new(0.65, 0.8, 0.95, 0.95),
    );
    draw_text(
        format!(
            "the colony: {} alive, the best {:.3}, {} eaten, {} built",
            story.pop, story.best, story.eats, story.builds
        ),
        x + 10.0,
        y + 26.0,
        12.0,
        Color::new(0.65, 0.8, 0.95, 0.95),
    );

    // The charts.
    let chart_w = panel_w - 20.0;
    let top = y + 46.0;
    let chart_h = ((panel_h - 60.0) / 3.0 - 8.0).max(50.0);
    chart(
        x + 10.0,
        top,
        chart_w,
        chart_h,
        "the colony",
        &story.population,
        0.3,
        0.9,
        0.5,
    );
    chart(
        x + 10.0,
        top + chart_h + 8.0,
        chart_w,
        chart_h,
        "the best",
        &story.fitness,
        0.95,
        0.75,
        0.35,
    );
    chart(
        x + 10.0,
        top + (chart_h + 8.0) * 2.0,
        chart_w,
        chart_h,
        "what was eaten",
        &story.eaten,
        0.4,
        0.7,
        0.95,
    );
}

/// One chart: a line that grows as the run goes on.
#[allow(clippy::too_many_arguments)]
fn chart(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    title: &str,
    values: &[f32],
    r: f32,
    g: f32,
    b: f32,
) {
    draw_rectangle(
        left,
        top,
        width,
        height,
        Color::new(0.05, 0.07, 0.11, 0.9),
    );
    draw_rectangle_lines(
        left,
        top,
        width,
        height,
        1.0,
        Color::new(0.22, 0.3, 0.4, 0.8),
    );
    draw_text(
        title,
        left + 8.0,
        top + 6.0,
        12.0,
        Color::new(r, g, b, 0.95),
    );
    if values.len() < 2 {
        return;
    }
    // Only so many points are drawn: a line of two thousand points is
    // more than the picture can show.
    let stride = (values.len() / 90).max(1);
    let max = values.iter().copied().fold(1.0f32, f32::max);
    let step = width / (values.len() - 1) as f32;
    let mut last = (left, top + height - 4.0);
    let mut first = true;
    for (i, value) in values.iter().enumerate() {
        if i % stride != 0 && i + 1 != values.len() {
            continue;
        }
        let px = left + i as f32 * step;
        let py = top + height - 4.0 - (value / max) * (height - 24.0);
        if !first {
            draw_line(last.0, last.1, px, py, 1.4, Color::new(r, g, b, 0.85));
        }
        first = false;
        last = (px, py);
    }
}
