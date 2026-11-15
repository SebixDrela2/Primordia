use crate::elements::Elem;
use crate::render::sprites::{self, Effect};
use crate::world::World;
use macroquad::prelude::*;

use super::pictures::draw_sprite;

fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    (x >> 8) as f32 / 16_777_216.0
}

enum Mote {

    Dust,

    Ember,

    Spore,

    Bubble,
}

pub fn draw_atmosphere(
    world: &World,
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
    time: f32,
    day_night: f32,
) {
    let scr_w = screen_width();
    let scr_h = screen_height();
    let scale_x = scr_w / view_w;
    let scale_y = scr_h / view_h;

    let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
    let light = 0.25 + 0.75 * day;

    let zoom = scr_w / view_w.max(1.0);
    let count = (220.0 / zoom.max(0.6)) as u32;

    for i in 0..count {
        let hx = hash(i.wrapping_mul(3).wrapping_add(1));
        let hy = hash(i.wrapping_mul(7).wrapping_add(13));
        let hz = hash(i.wrapping_mul(11).wrapping_add(29));
        let hw = hash(i.wrapping_mul(13).wrapping_add(41));

        let speed = 3.0 + hw * 9.0;
        let drift = (time * speed + hx * view_w) % view_w.max(1.0);
        let wx = view_x + drift;
        if wx < view_x - 4.0 || wx > view_x + view_w + 4.0 {
            continue;
        }
        let cx = wx.round() as i32;
        if cx < 0 || cx >= world.w as i32 {
            continue;
        }
        let surface = world.surface_y(cx);

        let (kind, rise) = {
            let below = world.get(cx, surface - 2);
            let at = world.get(cx, surface - 6);
            match (at, below) {
                (Elem::Fire | Elem::Ember | Elem::Lava | Elem::Magma, _) => (Mote::Ember, 9.0),
                (Elem::Spore | Elem::Fungus | Elem::Moss, _) => (Mote::Spore, 3.0),
                (Elem::Water, _) => (Mote::Bubble, 6.0),
                _ => match below {
                    Elem::Fire | Elem::Ember | Elem::Lava | Elem::Magma => (Mote::Ember, 9.0),
                    Elem::Spore | Elem::Fungus | Elem::Moss => (Mote::Spore, 3.0),
                    Elem::Water => (Mote::Bubble, 6.0),
                    _ => (Mote::Dust, 0.0),
                },
            }
        };

        let height = 2.0 + hy * 12.0;
        let bob = (time * (0.6 + hz) + hy * 10.0).sin() * (0.8 + hz * 1.6);
        let wy = surface as f32 - height - bob - rise * ((time * 0.5 + hz * 8.0) % 12.0);
        if wy < view_y - 20.0 || wy > view_y + view_h + 20.0 {
            continue;
        }

        let sx = (wx - view_x) * scale_x;
        let sy = (wy - view_y) * scale_y;
        if sx < -8.0 || sx > scr_w + 8.0 || sy < -8.0 || sy > scr_h + 8.0 {
            continue;
        }

        let size = (0.9 + hz * 1.6) * scale_x.clamp(0.6, 2.0);
        let (r, g, b, a) = match kind {
            Mote::Dust => {
                let fade = 0.10 + 0.22 * hz;
                (0.85, 0.86, 0.90, fade * light)
            }
            Mote::Ember => {

                let t = (time * 0.7 + hz * 5.0) % 1.0;
                (1.0, 0.55 - t * 0.3, 0.18 - t * 0.15, (0.75 - t * 0.6) * (0.4 + hz * 0.6))
            }
            Mote::Spore => (0.62, 0.42, 0.85, 0.30 + 0.25 * hz),
            Mote::Bubble => (0.70, 0.85, 1.0, 0.16 + 0.18 * hz),
        };
        if a <= 0.01 {
            continue;
        }
        // The mote is a picture, not a dot: a soft glow with a bright
        // heart in it.
        draw_sprite(
            sprites::effect_tile(Effect::Glow),
            sx,
            sy,
            size * 6.0,
            0.0,
            Color::new(r, g, b, a * 0.35),
        );
        draw_sprite(
            sprites::effect_tile(Effect::Ember),
            sx,
            sy,
            size * 3.4,
            0.0,
            Color::new(r, g, b, a),
        );
    }
}
