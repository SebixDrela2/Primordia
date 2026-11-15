use crate::creature::{Creature, CreatureSystem, Faction, Role, Stage};
use crate::render::sprites::{self, Effect};
use crate::world::World;
use macroquad::prelude::*;
use super::knowledge::HoverTarget;
use super::minimap::draw_weather;
use super::pictures::draw_sprite;

/// The window of the world the actors are drawn in: where it starts,
/// how big it is and how far it is zoomed in. These belong together, so
/// they travel in one struct rather than as a list of arguments.
pub struct ActorView {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub zoom: f32,
}

/// The world window together with the screen scale that goes with it.
/// Every helper needs the same four numbers, so they travel in one
/// struct rather than as a list of arguments.
struct ActorFrame {
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
    scale_x: f32,
    scale_y: f32,
    zoom: f32,
}

impl ActorFrame {
    fn new(view: &ActorView, scr_w: f32, scr_h: f32) -> Self {
        Self {
            view_x: view.x,
            view_y: view.y,
            view_w: view.w,
            view_h: view.h,
            scale_x: scr_w / view.w,
            scale_y: scr_h / view.h,
            zoom: view.zoom,
        }
    }

    /// The screen x of a world x.
    fn sx(&self, x: f32) -> f32 {
        (x - self.view_x) * self.scale_x
    }

    /// The screen y of a world y.
    fn sy(&self, y: f32) -> f32 {
        (y - self.view_y) * self.scale_y
    }

    /// Is this world point inside the window?
    fn in_view(&self, x: f32, y: f32) -> bool {
        x >= self.view_x
            && x < (self.view_x + self.view_w)
            && y >= self.view_y
            && y < (self.view_y + self.view_h)
    }
}

pub fn draw_actors(
    creatures: &CreatureSystem,
    world: &mut World,
    ui_hover: &HoverTarget,
    view: &ActorView,
    now: f32,
) {
    let show_all_names = view.zoom >= 2.5;
    let hovered_creature: Option<u32> = match ui_hover {
        HoverTarget::Creature { id } => Some(*id),
        _ => None,
    };
    let scr_w = screen_width();
    let scr_h = screen_height();
    let frame = ActorFrame::new(view, scr_w, scr_h);

    draw_campfire_glow(creatures, &frame, now);
    draw_night_fireflies(creatures, &frame, now);
    draw_claim_borders(creatures, &frame, scr_w, scr_h);
    draw_succession_claims(creatures, &frame, now);
    draw_bonds(creatures, &frame);
    draw_creatures(creatures, &frame, now, show_all_names, hovered_creature);
    draw_projectiles(creatures, &frame);
    draw_armed_structures(creatures, &frame);
    draw_bombs(creatures, &frame);
    draw_weather_particles(world, creatures, now, scr_w, scr_h);

    draw_weather(world.weather, now);

    draw_meteors(world, &frame);
}

/// The glow of every campfire of the colonies.
fn draw_campfire_glow(creatures: &CreatureSystem, frame: &ActorFrame, now: f32) {
    let mut fires: Vec<(f32, f32, [u8; 3])> = Vec::new();
    for c in &creatures.list {
        if c.role == crate::creature::Role::Queen {
            let lit = creatures
                .fire_lit
                .get(c.species as usize)
                .copied()
                .unwrap_or(false);
            if !lit {
                continue;
            }
            if !fires.iter().any(|(_, _, col)| col == &c.team_color) {
                fires.push((c.base_x, c.base_y, c.team_color));
            }
        }
    }
    for (fx, fy, col) in fires {
        if fx < frame.view_x - 20.0 || fx > frame.view_x + frame.view_w + 20.0 {
            continue;
        }
        let sx = frame.sx(fx);
        let sy = frame.sy(fy);
        let flicker = 0.75 + 0.25 * ((now * 6.0 + fx) * 1.7).sin();
        let fire = Color::new(
            col[0] as f32 / 255.0,
            col[1] as f32 / 255.0,
            col[2] as f32 / 255.0,
            1.0,
        );

        // The wide glow of the fire, and the narrower one inside it.
        for (radius, alpha) in [(120.0f32, 0.030f32), (70.0, 0.040)] {
            draw_sprite(
                sprites::effect_tile(Effect::Glow),
                sx,
                sy,
                radius * 2.0 * frame.scale_x.min(1.0),
                0.0,
                Color::new(fire.r, fire.g, fire.b, alpha),
            );
        }
        for (radius, alpha) in [(26.0f32, 0.07f32), (15.0, 0.10)] {
            draw_sprite(
                sprites::effect_tile(Effect::Glow),
                sx,
                sy,
                radius * 2.0 * frame.scale_x.min(1.2) * flicker,
                0.0,
                Color::new(1.0, 0.65 + col[0] as f32 / 900.0, 0.25, alpha * flicker),
            );
        }

        // The flame itself, which dances.
        let frame_index = ((now * 9.0).floor() as usize) % 4;
        let flame = match frame_index {
            0 => Effect::Flame0,
            1 => Effect::Flame1,
            2 => Effect::Flame2,
            _ => Effect::Flame3,
        };
        let flame_size = 13.0 * frame.scale_x.min(1.6) * (0.9 + 0.2 * flicker);
        draw_sprite(
            sprites::effect_tile(flame),
            sx,
            sy - flame_size * 0.18,
            flame_size,
            0.0,
            Color::new(1.0, 0.72 + col[0] as f32 / 1400.0, 0.32, 0.95),
        );
    }
}

/// The little lights that drift over the world at night.
fn draw_night_fireflies(creatures: &CreatureSystem, frame: &ActorFrame, now: f32) {
    let night = (creatures.day_night - 0.55).abs() > 0.45;
    if !night || frame.zoom >= 3.0 {
        return;
    }
    for i in 0..70 {
        let fi = i as f32;

        let px = frame.view_x
            + ((fi * 37.1 + now * 6.0 + (fi * 13.0).sin() * 9.0)
                .rem_euclid(frame.view_w.max(1.0)));
        let py = frame.view_y
            + ((fi * 91.7 + now * 4.0 + (fi * 7.0).cos() * 5.0)
                .rem_euclid(frame.view_h.max(1.0)));
        let blink = ((now * 1.6 + fi * 2.3).sin() * 0.5 + 0.5).powi(3);
        if blink < 0.15 {
            continue;
        }
        let sx = frame.sx(px);
        let sy = frame.sy(py);
        draw_sprite(
            sprites::effect_tile(Effect::Ember),
            sx,
            sy,
            6.0,
            0.0,
            Color::new(0.95, 1.0, 0.6, 0.55 * blink),
        );
        draw_sprite(
            sprites::effect_tile(Effect::Glow),
            sx,
            sy,
            9.0,
            0.0,
            Color::new(0.9, 1.0, 0.5, 0.10 * blink),
        );
    }
}

/// The dashed ring each colony holds around its home.
fn draw_claim_borders(
    creatures: &CreatureSystem,
    frame: &ActorFrame,
    scr_w: f32,
    scr_h: f32,
) {
    let mut claims: Vec<(f32, f32, f32, [u8; 3])> = Vec::new();
    for c in &creatures.list {
        if c.role != Role::Queen {
            continue;
        }
        if claims.iter().any(|(_, _, _, col)| col == &c.team_color) {
            continue;
        }
        let radius = creatures.claim_of(c.species).map_or(18.0, |(_, _, r)| r);
        claims.push((c.base_x, c.base_y, radius, c.team_color));
    }
    for (cx, cy, radius, col) in claims {
        let sx = frame.sx(cx);
        let sy = frame.sy(cy);
        let r = radius * frame.scale_x.min(frame.scale_y);
        if sx + r < 0.0 || sx - r > scr_w || sy + r < 0.0 || sy - r > scr_h {
            continue;
        }

        let segments = 48;
        for i in 0..segments {
            if i % 2 == 1 {
                continue;
            }
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            draw_line(
                sx + a0.cos() * r,
                sy + a0.sin() * r,
                sx + a1.cos() * r,
                sy + a1.sin() * r,
                1.2,
                Color::new(
                    col[0] as f32 / 255.0,
                    col[1] as f32 / 255.0,
                    col[2] as f32 / 255.0,
                    0.28,
                ),
            );
        }
    }
}

/// The claim of a species that has died out, still drawn in red.
fn draw_succession_claims(creatures: &CreatureSystem, frame: &ActorFrame, now: f32) {
    for (species, _) in &creatures.succession {
        let Some((cx, cy, radius)) = creatures.claim_of(*species) else {
            continue;
        };
        let sx = frame.sx(cx);
        let sy = frame.sy(cy);
        let r = radius * frame.scale_x.min(frame.scale_y);
        let flick = 0.4 + 0.4 * (now * 5.0 + cx).sin();
        let segments = 36;
        for i in 0..segments {
            if i % 3 == 2 {
                continue;
            }
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            draw_line(
                sx + a0.cos() * r,
                sy + a0.sin() * r,
                sx + a1.cos() * r,
                sy + a1.sin() * r,
                1.6,
                Color::new(0.95, 0.35, 0.25, 0.35 * flick),
            );
        }
    }
}

/// The line between the two of a bonded pair.
fn draw_bonds(creatures: &CreatureSystem, frame: &ActorFrame) {
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
        if !frame.in_view(c.x, c.y) {
            continue;
        }
        let sx = frame.sx(c.x);
        let sy = frame.sy(c.y);
        let px = frame.sx(p.x);
        let py = frame.sy(p.y);
        let col = Color::new(
            c.team_color[0] as f32 / 255.0,
            c.team_color[1] as f32 / 255.0,
            c.team_color[2] as f32 / 255.0,
            0.35,
        );
        draw_line(sx, sy, px, py, 1.2, col);
    }
}

/// Every creature of the colony, with its markers.
fn draw_creatures(
    creatures: &CreatureSystem,
    frame: &ActorFrame,
    now: f32,
    show_all_names: bool,
    hovered_creature: Option<u32>,
) {
    let night = (creatures.day_night - 0.55).abs() > 0.45;
    for c in &creatures.list {
        if !frame.in_view(c.x, c.y) {
            continue;
        }

        let s = frame.scale_x.min(frame.scale_y);
        let sx = frame.sx(c.x);
        let sy = frame.sy(c.y);

        let r = (c.body_size() * s).max(2.0);
        let base = Color::new(
            c.team_color[0] as f32 / 255.0,
            c.team_color[1] as f32 / 255.0,
            c.team_color[2] as f32 / 255.0,
            0.92,
        );
        let core = Color::new(1.0, 1.0, 0.95, 0.92);

        draw_creature_body(c, sx, sy, r, base, core, frame.zoom);
        draw_creature_stage(c, sx, sy, r, base);
        draw_creature_marks(c, sx, sy, r, now, night);
        draw_creature_health(c, sx, sy, r);

        if hovered_creature == Some(c.id) {
            draw_circle_lines(
                sx,
                sy,
                (r * 2.5).max(8.0),
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
        }

        let show_name = show_all_names
            || c.role == Role::Queen
            || hovered_creature == Some(c.id)
            || creatures.list.len() <= 12;
        if show_name {
            draw_creature_name(c, sx, sy, r, base, s);
        }
    }
}

/// The shape of the creature: its shadow, its halo and its body.
fn draw_creature_body(
    c: &Creature,
    sx: f32,
    sy: f32,
    r: f32,
    base: Color,
    core: Color,
    zoom: f32,
) {
    draw_circle(sx, sy + r * 0.85, r * 0.95, Color::new(0.0, 0.0, 0.0, 0.22));
    if c.role == Role::Queen || zoom >= 1.5 {
        draw_circle(sx, sy, r * 2.1, Color::new(base.r, base.g, base.b, 0.10));
    }

    // The picture of the creature: the body with its fur on it and the
    // gear of its role. Every species takes a look of its own.
    let variant = (c.species as usize + c.id as usize) % sprites::VARIANTS;
    let tile = sprites::tile_of(c.role, c.faction, variant);
    let heading = creature_heading(c);
    let size = r * 3.4;
    draw_sprite(tile, sx, sy, size, heading, base);

    // The eyes, in front of the head, where the creature is looking.
    let speed = (c.vx * c.vx + c.vy * c.vy).sqrt();
    if speed > 0.01 {
        let fx = c.vx / speed;
        let fy = c.vy / speed;
        let px = -fy;
        let py = fx;
        let eye = (r * 0.20).max(0.8);
        let shine = if c.faction == Faction::Zorg {
            Color::new(0.55, 1.0, 0.45, 0.95)
        } else {
            Color::new(1.0, 0.97, 0.82, 0.85)
        };
        for side in [-1.0f32, 1.0] {
            draw_circle(
                sx + fx * r * 0.55 + px * side * r * 0.30,
                sy + fy * r * 0.55 + py * side * r * 0.30,
                eye,
                shine,
            );
        }
    }

    // The old core dot, for the creatures that are so small that the
    // picture of them is only a speck.
    if r < 2.6 {
        draw_circle(sx, sy, (r * 0.3).max(1.0), core);
    }
}

/// The way the creature is looking: the head turns to where it walks.
fn creature_heading(c: &Creature) -> f32 {
    if c.vx * c.vx + c.vy * c.vy < 0.0001 {
        return 0.0;
    }
    c.vx.atan2(-c.vy)
}

/// The ring that says how old the creature is.
fn draw_creature_stage(c: &Creature, sx: f32, sy: f32, r: f32, base: Color) {
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
}

/// The marks on the creature: its eyes, its sickness, its journey and its mate.
fn draw_creature_marks(c: &Creature, sx: f32, sy: f32, r: f32, now: f32, night: bool) {
    if night {
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

    if c.disease != crate::creature::Disease::None {
        let beat = 0.5 + 0.5 * ((now * 3.4 + c.id as f32 * 0.7).sin());
        draw_circle_lines(
            sx,
            sy,
            r * 1.55,
            1.2,
            Color::new(0.45, 0.95, 0.35, 0.35 + 0.5 * beat),
        );
    }

    if c.migrating {
        draw_circle_lines(sx, sy, r * 1.7, 1.4, Color::new(1.0, 0.95, 0.7, 0.7));
    }

    if c.mate.is_some() && c.sired > 0 {
        draw_circle(
            sx,
            sy - r * 1.9,
            (r * 0.22).max(1.0),
            Color::new(1.0, 0.45, 0.6, 0.85),
        );
    }
}

/// The bar under the creature that shows how much of it is left.
fn draw_creature_health(c: &Creature, sx: f32, sy: f32, r: f32) {
    let hp = (c.health / 100.0).clamp(0.0, 1.0);
    if hp >= 0.999 {
        return;
    }
    let bw = (r * 2.6).max(6.0);
    let by = sy - r - 6.0;
    draw_rectangle(
        sx - bw / 2.0,
        by,
        bw,
        2.0,
        Color::new(0.0, 0.0, 0.0, 0.55),
    );
    let col = if hp > 0.6 {
        Color::new(0.32, 0.85, 0.42, 0.9)
    } else if hp > 0.3 {
        Color::new(0.95, 0.75, 0.22, 0.9)
    } else {
        Color::new(0.92, 0.26, 0.2, 0.9)
    };
    draw_rectangle(sx - bw / 2.0, by, bw * hp, 2.0, col);
}

/// The name over the creature.
fn draw_creature_name(c: &Creature, sx: f32, sy: f32, r: f32, base: Color, s: f32) {
    let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
    let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");
    if name_str.is_empty() {
        return;
    }
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

/// The shots in the air.
fn draw_projectiles(creatures: &CreatureSystem, frame: &ActorFrame) {
    for p in &creatures.projectiles {
        let sx = frame.sx(p.x);
        let sy = frame.sy(p.y);
        let px = frame.sx(p.px);
        let py = frame.sy(p.py);
        let c = p.elem.color();
        let glow = Color::new(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0);
        draw_line(
            px,
            py,
            sx,
            sy,
            2.0,
            Color::new(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 0.4),
        );
        // The spark of the shot: a soft halo and the bright heart of
        // it.
        draw_sprite(
            sprites::effect_tile(Effect::Spark),
            sx,
            sy,
            16.0,
            0.0,
            Color::new(glow.r, glow.g, glow.b, 0.28),
        );
        draw_sprite(
            sprites::effect_tile(Effect::Spark),
            sx,
            sy,
            7.0,
            0.0,
            Color::new(glow.r, glow.g, glow.b, 0.95),
        );
    }
}

/// The ring around every armed building.
fn draw_armed_structures(creatures: &CreatureSystem, frame: &ActorFrame) {
    for s in &creatures.structures {
        if !s.armed {
            continue;
        }
        let sx = frame.sx(s.x);
        let sy = frame.sy(s.y);
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
}

/// The bombs lying on the ground.
fn draw_bombs(creatures: &CreatureSystem, frame: &ActorFrame) {
    for b in &creatures.bombs {
        let sx = frame.sx(b.x);
        let sy = frame.sy(b.y);
        draw_sprite(
            sprites::effect_tile(Effect::Glow),
            sx,
            sy,
            14.0,
            0.0,
            Color::new(1.0, 0.6, 0.2, 0.18),
        );
        draw_sprite(
            sprites::effect_tile(Effect::Bomb),
            sx,
            sy,
            11.0,
            0.0,
            Color::new(1.0, 1.0, 1.0, 0.95),
        );
    }
}

/// The motes of dust, ash and pollen that hang in the air.
fn draw_weather_particles(
    world: &World,
    creatures: &CreatureSystem,
    now: f32,
    scr_w: f32,
    scr_h: f32,
) {
    let season = world.season;
    let volcano = world.volcano();

    let motes = 46;
    for i in 0..motes {
        let fi = i as f32;
        let seed = fi * 1.618;

        let px = (seed * 137.0 + now * (7.0 + (i % 5) as f32 * 2.5)) % scr_w;
        let py = (seed * 91.0 + now * (4.0 + (i % 3) as f32 * 1.7)) % scr_h;
        let sway = ((now * 1.3 + seed) * 2.0).sin() * 7.0;
        let alpha = 0.05 + 0.07 * ((now * 0.8 + seed).sin() * 0.5 + 0.5);
        draw_sprite(
            sprites::effect_tile(Effect::Ember),
            px + sway,
            py,
            4.0 + (i % 2) as f32 * 2.0,
            0.0,
            Color::new(1.0, 0.96, 0.85, alpha),
        );
    }

    if season == 2 || volcano.is_some() {
        let n = if season == 2 { 26 } else { 14 };
        for i in 0..n {
            let fi = i as f32;
            let seed = fi * 2.399;
            let px = (seed * 173.0 + now * (26.0 + (i % 6) as f32 * 9.0)) % scr_w;
            let py = (seed * 61.0 + now * (34.0 + (i % 4) as f32 * 11.0)) % scr_h;
            let sway = ((now * 2.1 + seed) * 3.0).sin() * 11.0;
            let (r, g, b) = if season == 2 {
                match i % 3 {
                    0 => (0.82, 0.48, 0.16),
                    1 => (0.90, 0.72, 0.24),
                    _ => (0.55, 0.62, 0.22),
                }
            } else {
                (0.55, 0.53, 0.52)
            };
            draw_line(
                px + sway,
                py,
                px + sway + 2.5,
                py + 4.0,
                1.4,
                Color::new(r, g, b, 0.30),
            );
        }
    }

    if season == 1 && (creatures.day_night - 0.55).abs() > 0.4 {
        for i in 0..22 {
            let fi = i as f32;
            let seed = fi * 3.71;
            let px = (seed * 211.0 + now * (10.0 + (i % 4) as f32 * 6.0)) % scr_w;
            let py = (seed * 83.0 + now * (7.0 + (i % 3) as f32 * 4.0)) % scr_h;
            let pulse = ((now * 2.6 + seed).sin() * 0.5 + 0.5).powi(3);
            draw_sprite(
                sprites::effect_tile(Effect::Ember),
                px,
                py,
                7.0,
                0.0,
                Color::new(0.75, 1.0, 0.45, pulse * 0.6),
            );
        }
    }
}

/// The meteors that fall now and then.
fn draw_meteors(world: &World, frame: &ActorFrame) {
    for (mx, my, life) in &world.meteor_events {
        let t = *life as f32 / 22.0;
        let sx = frame.sx(*mx);
        let sy = frame.sy(*my);
        let tail = 190.0 * frame.scale_y * (0.4 + t * 0.6);
        draw_line(
            sx - tail * 0.18,
            sy - tail,
            sx,
            sy,
            2.4,
            Color::new(1.0, 0.85, 0.55, 0.35 + 0.5 * t),
        );
        draw_circle(
            sx,
            sy,
            16.0 * t + 6.0,
            Color::new(1.0, 0.7, 0.3, 0.16 * t),
        );
        draw_circle(sx, sy, 7.0 * t + 2.0, Color::new(1.0, 0.9, 0.65, 0.7 * t));
    }
}
