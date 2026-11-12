use crate::creature::CreatureSystem;
use crate::world;
use crate::world::World;
use crate::{H, W};
use macroquad::prelude::*;

/// Soft darkening towards the screen edges.
pub fn draw_vignette() {
    let w = screen_width();
    let h = screen_height();
    let steps = 12;
    let band = (w.min(h) * 0.22).max(40.0);
    for i in 0..steps {
        let f = 1.0 - i as f32 / steps as f32;
        let a = 0.05 * f;
        let t = band * f;
        let c = Color::new(0.0, 0.0, 0.0, a);
        draw_rectangle(0.0, 0.0, w, t, c);
        draw_rectangle(0.0, h - t, w, t, c);
        draw_rectangle(0.0, 0.0, t, h, c);
        draw_rectangle(w - t, 0.0, t, h, c);
    }
}

/// Screen-space weather: rain streaks, snow and storm flashes.
pub fn draw_weather(weather: world::Weather, time: f32) {
    let w = screen_width();
    let h = screen_height();
    match weather {
        world::Weather::Rain | world::Weather::Storm => {
            let n = if weather == world::Weather::Storm { 150 } else { 70 };
            let slant = if weather == world::Weather::Storm { 14.0 } else { 7.0 };
            for i in 0..n {
                let fi = i as f32;
                let ii = i as u32;
                let x = ((fi * 97.3 + time * 260.0) % w as f32).max(0.0);
                let y = ((fi * 53.7 + time * (620.0 + fi * 3.0)) % h as f32).max(0.0);
                let a = 0.16 + (ii % 5) as f32 * 0.03;
                draw_line(
                    x,
                    y,
                    x - slant,
                    y + 18.0,
                    1.0,
                    Color::new(0.62, 0.72, 0.88, a),
                );
            }
        }
        world::Weather::Snow => {
            for i in 0..110 {
                let fi = i as f32;
                let ii = i as u32;
                let x = ((fi * 71.9 + time * 26.0 + (ii % 7) as f32 * 13.0) % w as f32).max(0.0);
                let y = ((fi * 41.3 + time * (55.0 + (ii % 9) as f32 * 4.0)) % h as f32).max(0.0);
                let s = 1.0 + (ii % 3) as f32 * 0.5;
                draw_circle(x, y, s, Color::new(0.95, 0.97, 1.0, 0.5));
            }
        }
        _ => {}
    }
}

/// The species with the most living members (ties broken by lowest id).
pub fn dominant_species(creatures: &CreatureSystem) -> Option<u32> {
    let mut counts: Vec<(u32, u32)> = Vec::new();
    for c in &creatures.list {
        match counts.iter_mut().find(|(id, _)| *id == c.species) {
            Some(e) => e.1 += 1,
            None => counts.push((c.species, 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    counts.first().map(|(id, _)| *id)
}

/// Bottom-left minimap: terrain, species dots and the current camera viewport.
pub fn draw_minimap(
    creatures: &CreatureSystem,
    world: &World,
    cam_x: f32,
    cam_y: f32,
    view_w: f32,
    view_h: f32,
    mini_texture: &Texture2D,
) {
    let mw = 180.0f32;
    let mh = mw * (H as f32 / W as f32);
    let x = 18.0;
    let y = screen_height() - mh - 44.0;
    if y < 180.0 {
        return; // too little vertical space; don't cover the HUD
    }

    draw_rectangle(
        x - 3.0,
        y - 3.0,
        mw + 6.0,
        mh + 6.0,
        Color::new(0.02, 0.03, 0.05, 0.82),
    );
    // Downsampled terrain.
    draw_texture_ex(
        mini_texture,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(mw, mh)),
            ..Default::default()
        },
    );
    draw_rectangle(x, y, mw, mh, Color::new(0.02, 0.04, 0.07, 0.25));
    draw_rectangle_lines(x, y, mw, mh, 1.0, Color::new(0.3, 0.5, 0.7, 0.55));

    // The landmarks: the places worth fighting over.
    for l in &world.landmarks {
        let px = x + (l.x / W as f32).clamp(0.0, 1.0) * mw;
        let py = y + (l.y / H as f32).clamp(0.0, 1.0) * mh;
        let col = match l.kind {
            crate::world::Landmark::CrystalGrove => Color::new(0.45, 0.75, 1.0, 0.9),
            crate::world::Landmark::HotSpring => Color::new(1.0, 0.55, 0.35, 0.9),
            crate::world::Landmark::BoneField => Color::new(0.85, 0.85, 0.75, 0.9),
            crate::world::Landmark::FertileSoil => Color::new(0.45, 0.9, 0.45, 0.9),
            crate::world::Landmark::OldRuin => Color::new(0.8, 0.5, 0.95, 0.9),
        };
        draw_circle(px, py, 3.0, col);
        draw_circle_lines(px, py, 4.5, 1.0, col);
    }

    for c in &creatures.list {
        let px = x + (c.x / W as f32).clamp(0.0, 1.0) * mw;
        let py = y + (c.y / H as f32).clamp(0.0, 1.0) * mh;
        let color = Color::new(
            c.team_color[0] as f32 / 255.0,
            c.team_color[1] as f32 / 255.0,
            c.team_color[2] as f32 / 255.0,
            0.95,
        );
        draw_rectangle(px - 1.0, py - 1.0, 2.0, 2.0, color);
    }

    let vx = x + (cam_x / W as f32) * mw;
    let vy = y + (cam_y / H as f32) * mh;
    draw_rectangle_lines(
        vx,
        vy,
        (view_w / W as f32) * mw,
        (view_h / H as f32) * mh,
        1.5,
        Color::new(0.13, 0.83, 0.93, 0.85),
    );

    draw_text("MAP", x + 5.0, y + 13.0, 11.0, Color::new(0.6, 0.7, 0.8, 0.6));
}

