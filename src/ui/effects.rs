use crate::creature::SimEvent;
use macroquad::prelude::*;

/// Short-lived visual feedback for everything that happens in the sim.
pub enum EffectKind {
    Ring { color: [u8; 3], max_r: f32, width: f32 },
    Spark { color: [u8; 3], vx: f32, vy: f32, size: f32 },
    Text { color: [u8; 3], text: String },
}

pub struct Effect {
    kind: EffectKind,
    x: f32,
    y: f32,
    t: f32,
    life: f32,
}

pub struct Effects {
    items: Vec<Effect>,
}

impl Effects {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn ring(&mut self, x: f32, y: f32, color: [u8; 3], max_r: f32, life: f32) {
        self.items.push(Effect {
            kind: EffectKind::Ring {
                color,
                max_r,
                width: 2.0,
            },
            x,
            y,
            t: 0.0,
            life,
        });
    }

    pub fn spark(&mut self, x: f32, y: f32, color: [u8; 3], vx: f32, vy: f32, size: f32, life: f32) {
        self.items.push(Effect {
            kind: EffectKind::Spark {
                color,
                vx,
                vy,
                size,
            },
            x,
            y,
            t: 0.0,
            life,
        });
    }

    pub fn text(&mut self, x: f32, y: f32, color: [u8; 3], text: String) {
        self.items.push(Effect {
            kind: EffectKind::Text { color, text },
            x,
            y,
            t: 0.0,
            life: 1.3,
        });
    }

    /// Radial spray of sparks (used for deaths and impacts).
    pub fn burst(&mut self, x: f32, y: f32, color: [u8; 3], count: usize, speed: f32) {
        for i in 0..count {
            let a = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU;
            let jitter = ((i * 37) % 11) as f32 / 11.0 - 0.5;
            let s = speed * (0.6 + jitter);
            self.spark(
                x,
                y,
                color,
                a.cos() * s,
                a.sin() * s,
                2.0,
                0.55 + jitter.abs() * 0.3,
            );
        }
    }

    pub fn update(&mut self, dt: f32) {
        for e in &mut self.items {
            e.t += dt;
        }
        self.items.retain(|e| e.t < e.life);
        if self.items.len() > 600 {
            let excess = self.items.len() - 600;
            self.items.drain(0..excess);
        }
    }

    pub fn draw(&self, cam_x: f32, cam_y: f32, scale_x: f32, scale_y: f32) {
        for e in &self.items {
            let p = (e.t / e.life).clamp(0.0, 1.0);
            let fade = (1.0 - p).clamp(0.0, 1.0);
            let sx = (e.x - cam_x) * scale_x;
            let sy = (e.y - cam_y) * scale_y;
            match &e.kind {
                EffectKind::Ring {
                    color,
                    max_r,
                    width,
                } => {
                    // Ease-out expansion.
                    let r = (max_r * scale_x).max(1.0) * (1.0 - (1.0 - p) * (1.0 - p));
                    draw_circle_lines(
                        sx,
                        sy,
                        r,
                        *width,
                        Color::new(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            fade * 0.9,
                        ),
                    );
                }
                EffectKind::Spark {
                    color,
                    vx,
                    vy,
                    size,
                } => {
                    let px = (e.x + vx * e.t - cam_x) * scale_x;
                    let py = (e.y + vy * e.t - cam_y) * scale_y;
                    let s = (size * scale_x).max(1.0) * (1.0 - p * 0.5);
                    draw_circle(
                        px,
                        py,
                        s,
                        Color::new(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            fade,
                        ),
                    );
                }
                EffectKind::Text { color, text } => {
                    let dims = measure_text(text, None, 12, 1.0);
                    draw_text(
                        text,
                        sx - dims.width / 2.0,
                        sy - p * 16.0,
                        12.0,
                        Color::new(
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                            fade,
                        ),
                    );
                }
            }
        }
    }
}

/// Full-screen state shown when one species has wiped out all others (or the

/// Turn a simulation event into an animation.
pub fn apply_event(effects: &mut Effects, ev: SimEvent) {
            match ev {
                SimEvent::Hit {
                    x,
                    y,
                    color,
                    damage,
                } => {
                    effects.ring(x, y, color, 7.0, 0.35);
                    if damage >= 20.0 {
                        effects.text(x, y, [255, 210, 120], format!("-{}", damage as i32));
                    }
                }
                SimEvent::Shoot { x, y, color } => {
                    effects.ring(x, y, color, 6.0, 0.25);
                }
                SimEvent::Craft { x, y, color } => {
                    effects.ring(x, y, color, 12.0, 0.7);
                    effects.text(x, y, color, "CRAFT".to_string());
                }
                SimEvent::Build { x, y, color } => {
                    effects.ring(x, y, color, 16.0, 0.8);
                    effects.text(x, y, color, "BUILT".to_string());
                }
                SimEvent::Trade { x, y, color } => {
                    effects.ring(x, y, color, 9.0, 0.5);
                    effects.text(x, y, color, "TRADE".to_string());
                }
                SimEvent::Birth { x, y, color } => {
                    effects.ring(x, y, color, 10.0, 0.6);
                    effects.text(x, y, color, "BORN".to_string());
                }
                SimEvent::Death {
                    x,
                    y,
                    color,
                    size,
                } => {
                    effects.ring(x, y, color, 22.0 + size * 8.0, 0.9);
                    effects.ring(x, y, [255, 255, 255], 12.0 + size * 4.0, 0.5);
                    effects.burst(x, y, color, 14, 7.0);
                    effects.text(x, y, [255, 130, 130], "KO".to_string());
                }
                SimEvent::Heal { x, y, color } => {
                    effects.ring(x, y, [120, 255, 160], 12.0, 0.7);
                    effects.text(x, y, [150, 255, 190], "+HP".to_string());
                }
                SimEvent::Explosion { x, y, color } => {
                    effects.ring(x, y, [255, 200, 80], 34.0, 0.8);
                    effects.ring(x, y, [255, 120, 40], 20.0, 0.5);
                    effects.burst(x, y, [255, 190, 90], 20, 11.0);
                    effects.text(x, y, [255, 210, 120], "BOOM".to_string());
                }
                SimEvent::Treaty { x, y, color, label } => {
                    // A treaty is a formal, golden moment: two rings that
                    // close in on each other.
                    effects.ring(x, y, [255, 214, 120], 18.0, 0.9);
                    effects.ring(x, y, color, 11.0, 0.7);
                    effects.text(x, y, [255, 226, 150], label.clone());
                }
                SimEvent::War { x, y, color } => {
                    // War is a red, jagged break, ringed in the colour of
                    // the species that started it.
                    effects.ring(x, y, [255, 70, 70], 30.0, 0.9);
                    effects.ring(x, y, color, 22.0, 0.7);
                    effects.burst(x, y, [255, 90, 70], 18, 10.0);
                    effects.text(x, y, [255, 120, 110], "WAR".to_string());
                }
                SimEvent::Speciation { x, y, color } => {
                    // A new species: two rings opening outward from the
                    // place it was born.
                    effects.ring(x, y, [255, 255, 220], 26.0, 1.1);
                    effects.ring(x, y, color, 14.0, 0.8);
                    effects.burst(x, y, [255, 250, 210], 22, 9.0);
                    effects.text(x, y, [255, 255, 230], "NEW SPECIES".to_string());
                }
                SimEvent::Plague { x, y, color } => {
                    // A plague: a sickly green cloud.
                    effects.ring(x, y, [120, 200, 90], 18.0, 0.9);
                    effects.burst(x, y, [140, 220, 110], 14, 6.0);
                    effects.text(x, y, [170, 240, 140], "PLAGUE".to_string());
                }
                SimEvent::Discovery { x, y, color } => {
                    // A discovery: a bright golden spark.
                    effects.ring(x, y, [255, 230, 150], 20.0, 1.0);
                    effects.burst(x, y, [255, 235, 170], 16, 7.0);
                    effects.text(x, y, [255, 240, 190], "DISCOVERY".to_string());
                }
            }
}
