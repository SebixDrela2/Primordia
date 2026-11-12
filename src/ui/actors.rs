use crate::creature::{CreatureSystem, Role, Stage};
use crate::world::World;
use macroquad::prelude::*;
use super::knowledge::HoverTarget;
use super::minimap::draw_weather;

/// Draw the creatures, their shots and the world overlays.
pub fn draw_actors(
    creatures: &CreatureSystem,
    world: &mut World,
    ui_hover: &HoverTarget,
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
    zoom: f32,
    now: f32,
) {

        // Draw creature bodies and names (only those in view)
        let show_all_names = zoom >= 2.5;
        let hovered_creature: Option<u32> = match ui_hover {
            HoverTarget::Creature { id } => Some(*id),
            _ => None,
        };
        let scr_w = screen_width();
        let scr_h = screen_height();

        // Colony campfires: every species that still has a queen burns a light
        // at its nest, so settlements are visible from across the world.
        {
            let mut fires: Vec<(f32, f32, [u8; 3])> = Vec::new();
            for c in &creatures.list {
                if c.role == crate::creature::Role::Queen {
                    if !fires.iter().any(|(_, _, col)| col == &c.team_color) {
                        fires.push((c.base_x, c.base_y, c.team_color));
                    }
                }
            }
            let scale_x = scr_w / view_w;
            let scale_y = scr_h / view_h;
            for (fx, fy, col) in fires {
                if fx < view_x - 20.0 || fx > view_x + view_w + 20.0 {
                    continue;
                }
                let sx = (fx - view_x) * scale_x;
                let sy = (fy - view_y) * scale_y;
                let flicker = 0.75 + 0.25 * ((now * 6.0 + fx) * 1.7).sin();
                // A wide, very faint wash of the species colour: the map
                // slowly becomes a picture of who holds what.
                for (radius, alpha) in [(120.0f32, 0.018f32), (70.0, 0.026)] {
                    draw_circle(
                        sx,
                        sy,
                        radius * scale_x.min(1.0),
                        Color::new(
                            col[0] as f32 / 255.0,
                            col[1] as f32 / 255.0,
                            col[2] as f32 / 255.0,
                            alpha,
                        ),
                    );
                }
                for (radius, alpha) in [(26.0f32, 0.05f32), (15.0, 0.08), (7.0, 0.12)] {
                    draw_circle(
                        sx,
                        sy,
                        radius * scale_x.min(1.2) * flicker,
                        Color::new(
                            1.0,
                            0.65 + col[0] as f32 / 900.0,
                            0.25,
                            alpha * flicker,
                        ),
                    );
                }
                draw_circle(
                    sx,
                    sy,
                    3.0 * scale_x.min(1.5),
                    Color::new(1.0, 0.85, 0.45, 0.9),
                );
            }
        }

        // Fireflies: little lights drifting over the ground at night.
        {
            let night = (creatures.day_night - 0.55).abs() > 0.45;
            if night && zoom < 3.0 {
                let scale_x = scr_w / view_w;
                let scale_y = scr_h / view_h;
                for i in 0..70 {
                    let fi = i as f32;
                    // A slow, looping drift so the motes never sit still.
                    let px = view_x
                        + ((fi * 37.1 + now * 6.0 + (fi * 13.0).sin() * 9.0)
                            .rem_euclid(view_w.max(1.0)));
                    let py = view_y
                        + ((fi * 91.7 + now * 4.0 + (fi * 7.0).cos() * 5.0)
                            .rem_euclid(view_h.max(1.0)));
                    let blink = ((now * 1.6 + fi * 2.3).sin() * 0.5 + 0.5).powi(3);
                    if blink < 0.15 {
                        continue;
                    }
                    let sx = (px - view_x) * scale_x;
                    let sy = (py - view_y) * scale_y;
                    draw_circle(sx, sy, 1.6, Color::new(0.95, 1.0, 0.6, 0.5 * blink));
                    draw_circle(sx, sy, 4.0, Color::new(0.9, 1.0, 0.5, 0.07 * blink));
                }
            }
        }

        // The bonds: a thin line between two animals that are partners.
        for c in &creatures.list {
            let Some(partner_id) = c.bond else {
                continue;
            };
            let Some(p) = creatures.list.iter().find(|o| o.id == partner_id) else {
                continue;
            };
            if p.health <= 0.0 || p.id <= c.id {
                continue;
            }
            let in_view = (c.x) >= view_x
                && (c.x) < (view_x + view_w)
                && (c.y) >= view_y
                && (c.y) < (view_y + view_h);
            if !in_view {
                continue;
            }
            let scale_x = scr_w / view_w;
            let scale_y = scr_h / view_h;
            let sx = (c.x - view_x) * scale_x;
            let sy = (c.y - view_y) * scale_y;
            let px = (p.x - view_x) * scale_x;
            let py = (p.y - view_y) * scale_y;
            let col = Color::new(
                c.team_color[0] as f32 / 255.0,
                c.team_color[1] as f32 / 255.0,
                c.team_color[2] as f32 / 255.0,
                0.35,
            );
            draw_line(sx, sy, px, py, 1.2, col);
        }

        for c in &creatures.list {
            let in_view = (c.x) >= view_x
                && (c.x) < (view_x + view_w)
                && (c.y) >= view_y
                && (c.y) < (view_y + view_h);
            if !in_view {
                continue;
            }

            let scale_x = scr_w / view_w;
            let scale_y = scr_h / view_h;
            let s = scale_x.min(scale_y);
            let sx = (c.x - view_x) * scale_x;
            let sy = (c.y - view_y) * scale_y;
            // A juvenile is drawn small and an elder slightly large, so a
            // colony shows its ages at a glance.
            let r = (c.body_size() * s).max(2.0);
            let base = Color::new(
                c.team_color[0] as f32 / 255.0,
                c.team_color[1] as f32 / 255.0,
                c.team_color[2] as f32 / 255.0,
                0.92,
            );
            let core = Color::new(1.0, 1.0, 0.95, 0.92);

            // Soft drop shadow + species glow make the colony pop out of the
            // terrain.
            draw_circle(
                sx,
                sy + r * 0.85,
                r * 0.95,
                Color::new(0.0, 0.0, 0.0, 0.22),
            );
            if c.role == Role::Queen || zoom >= 1.5 {
                draw_circle(
                    sx,
                    sy,
                    r * 2.1,
                    Color::new(base.r, base.g, base.b, 0.10),
                );
            }

            // Role silhouettes make the colony readable at a glance.
            match c.role {
                Role::Soldier => {
                    let angle = c.vy.atan2(c.vx) - std::f32::consts::FRAC_PI_2;
                    draw_poly(sx, sy, 3, r * 1.3, angle, base);
                    draw_circle(sx, sy, (r * 0.32).max(1.0), core);
                }
                Role::Queen => {
                    draw_circle(sx, sy, r * 1.15, base);
                    draw_circle_lines(sx, sy, r * 1.65, 1.6, base);
                    draw_circle(sx, sy, (r * 0.4).max(1.2), Color::new(1.0, 0.93, 0.55, 0.95));
                }
                Role::Priest | Role::Shaman => {
                    draw_poly(sx, sy, 4, r * 1.25, std::f32::consts::FRAC_PI_4, base);
                    draw_circle(sx, sy, (r * 0.3).max(1.0), core);
                }
                Role::Builder | Role::Merchant => {
                    draw_poly(sx, sy, 6, r * 1.18, 0.0, base);
                    draw_circle(sx, sy, (r * 0.28).max(1.0), core);
                }
                Role::Explorer => {
                    draw_poly(sx, sy, 3, r * 1.2, std::f32::consts::PI, base);
                    draw_circle(sx, sy, (r * 0.28).max(1.0), core);
                }
                Role::Envoy => {
                    // A diplomat: a diamond with a bright core, and a little
                    // banner so envoys stand out in a crowd.
                    draw_poly(sx, sy, 4, r * 1.3, 0.0, base);
                    draw_circle(sx, sy, (r * 0.26).max(1.0), core);
                    draw_line(
                        sx + r * 0.9,
                        sy - r * 0.9,
                        sx + r * 0.9,
                        sy - r * 2.4,
                        1.4,
                        base,
                    );
                    draw_rectangle(
                        sx + r * 0.9,
                        sy - r * 2.4,
                        r * 1.1,
                        r * 0.7,
                        Color::new(1.0, 0.86, 0.35, 0.85),
                    );
                }
                Role::Worker => {
                    draw_circle(sx, sy, r, base);
                    draw_circle(sx, sy, (r * 0.3).max(1.0), core);
                }
            }

            // The life stage is drawn as a ring around the animal: a thin
            // ring for a juvenile, a plain one for an adult and a broken
            // one for an elder.
            match c.stage {
                Stage::Juvenile => {
                    draw_circle_lines(
                        sx,
                        sy,
                        r * 1.5,
                        1.0,
                        Color::new(base.r, base.g, base.b, 0.55),
                    );
                }
                Stage::Elder => {
                    // A second, fainter ring marks the old ones.
                    draw_circle_lines(
                        sx,
                        sy,
                        r * 1.45,
                        1.2,
                        Color::new(1.0, 0.85, 0.55, 0.6),
                    );
                }
                Stage::Adult => {}
            }

            // After dark the animals catch the light with their eyes.
            if (creatures.day_night - 0.55).abs() > 0.45 {
                let eye = (r * 0.16).max(0.9);
                let shine = 0.55 + 0.45 * ((now * 3.0 + c.id as f32) * 0.7).sin();
                for side in [-1.0f32, 1.0] {
                    draw_circle(
                        sx + side * r * 0.32,
                        sy - r * 0.22,
                        eye,
                        Color::new(1.0, 0.95, 0.65, 0.35 + 0.45 * shine),
                    );
                }
            }

            // A creature that has left its colony is marked with a pale
            // outline: it does not belong to the colony any more.
            if c.migrating {
                draw_circle_lines(
                    sx,
                    sy,
                    r * 1.7,
                    1.4,
                    Color::new(1.0, 0.95, 0.7, 0.7),
                );
            }

            // A creature that is courting shows a small heart above it, so
            // the player can watch the courtship of a colony.
            if c.mate.is_some() && c.sired > 0 {
                draw_circle(sx, sy - r * 1.9, (r * 0.22).max(1.0), Color::new(1.0, 0.45, 0.6, 0.85));
            }

            // Damage bar (only when hurt).
            let hp = (c.health / 100.0).clamp(0.0, 1.0);
            if hp < 0.999 {
                let bw = (r * 2.6).max(6.0);
                let by = sy - r - 6.0;
                draw_rectangle(sx - bw / 2.0, by, bw, 2.0, Color::new(0.0, 0.0, 0.0, 0.55));
                let col = if hp > 0.6 {
                    Color::new(0.32, 0.85, 0.42, 0.9)
                } else if hp > 0.3 {
                    Color::new(0.95, 0.75, 0.22, 0.9)
                } else {
                    Color::new(0.92, 0.26, 0.2, 0.9)
                };
                draw_rectangle(sx - bw / 2.0, by, bw * hp, 2.0, col);
            }

            // Hover ring.
            if hovered_creature == Some(c.id) {
                draw_circle_lines(
                    sx,
                    sy,
                    (r * 2.5).max(8.0),
                    2.0,
                    Color::new(1.0, 1.0, 1.0, 0.8),
                );
            }

            // Names only for queens, the hovered creature, tiny populations,
            // or when zoomed in. Keeps the view clean and avoids text churn.
            let show_name = show_all_names
                || c.role == Role::Queen
                || hovered_creature == Some(c.id)
                || creatures.list.len() <= 12;
            if show_name {
                let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
                let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");
                if !name_str.is_empty() {
                    let font = (9.0 * s).clamp(9.0, 16.0);
                    let dims = measure_text(name_str, None, font as u16, 1.0);
                    draw_text(
                        name_str,
                        sx - dims.width / 2.0,
                        sy - r - 7.0,
                        font,
                        Color::new(base.r, base.g, base.b, 0.95),
                    );
                }
            }
        }

        // ---- Shots in flight ----
        let vscale_x = scr_w / view_w;
        let vscale_y = scr_h / view_h;
        for p in &creatures.projectiles {
            let sx = (p.x - view_x) * vscale_x;
            let sy = (p.y - view_y) * vscale_y;
            let px = (p.px - view_x) * vscale_x;
            let py = (p.py - view_y) * vscale_y;
            let c = p.elem.color();
            let glow = Color::new(
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
                1.0,
            );
            draw_line(
                px,
                py,
                sx,
                sy,
                2.0,
                Color::new(
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    0.4,
                ),
            );
            draw_circle(
                sx,
                sy,
                5.0,
                Color::new(
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    0.22,
                ),
            );
            draw_circle(sx, sy, 2.2, glow);
        }

        // ---- Turret markers ----
        for s in &creatures.structures {
            if !s.armed {
                continue;
            }
            let sx = (s.x - view_x) * vscale_x;
            let sy = (s.y - view_y) * vscale_y;
            draw_circle_lines(
                sx,
                sy,
                8.0,
                1.5,
                Color::new(
                    s.color[0] as f32 / 255.0,
                    s.color[1] as f32 / 255.0,
                    s.color[2] as f32 / 255.0,
                    0.7,
                ),
            );
        }

        // ---- Bombs in flight ----
        for b in &creatures.bombs {
            let sx = (b.x - view_x) * vscale_x;
            let sy = (b.y - view_y) * vscale_y;
            draw_circle(sx, sy, 4.0, Color::new(0.12, 0.12, 0.15, 0.95));
            draw_circle(sx, sy, 2.0, Color::new(0.9, 0.55, 0.15, 0.95));
            draw_circle(
                sx,
                sy,
                7.0,
                Color::new(1.0, 0.6, 0.2, 0.18),
            );
        }

        // ---- Weather overlay ----
        draw_weather(world.weather, now);

        // ---- Meteor flashes ----
        for (mx, my) in &world.meteor_events {
            let sx = (mx - view_x) * vscale_x;
            let sy = (my - view_y) * vscale_y;
            draw_circle(sx, sy, 26.0, Color::new(1.0, 0.75, 0.35, 0.22));
            draw_circle(sx, sy, 12.0, Color::new(1.0, 0.9, 0.6, 0.5));
        }
        world.meteor_events.clear();

        // ---- Interaction animations ----
}
