use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::World;

#[inline]
fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

#[inline]
fn noise(x: i32, y: i32) -> f32 {
    // Stable 0..1 value for per-cell colour variation.
    (hash2(x, y) >> 8) as f32 / 16_777_216.0
}

#[inline]
fn clamp_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0) as u8
}

/// Paint the whole world into an RGBA buffer.
///
/// `time` drives the animated shimmer/glow effects, `day_night` is 0.0 at
/// midnight and 0.5 at noon, and `season` is 0..=3.
pub fn render_world(
    world: &World,
    _creatures: &CreatureSystem,
    buf: &mut [u8],
    time: f32,
    day_night: f32,
    season: u8,
) {
    let w = world.w;
    let h = world.h;
    debug_assert_eq!(buf.len(), w * h * 4);

    // Ground level per column: the topmost *supported* solid cell. Using the
    // real terrain (instead of the analytic surface) keeps eroded gaps and
    // crater floors from being painted as black voids.
    let mut ground = vec![0i32; w];
    for x in 0..w {
        let mut g = h as i32 - 1;
        let mut found = false;
        for y in 0..h {
            let e = Elem::from_u8(world.kind[y * w + x]);
            if e.is_solid() {
                let supported = y + 1 < h
                    && Elem::from_u8(world.kind[(y + 1) * w + x]).is_solid();
                if supported || y + 1 >= h {
                    g = y as i32;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            for y in 0..h {
                if Elem::from_u8(world.kind[y * w + x]).is_solid() {
                    g = y as i32;
                    break;
                }
            }
        }
        ground[x] = g;
    }

    // 0.0 = midnight, 1.0 = noon.
    let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
    let light = 0.34 + 0.66 * day;

    // Seasonal atmosphere tint (subtle, so materials keep their identity).
    let (sr, sg, sb) = match season {
        0 => (0.94f32, 1.0f32, 0.95f32),
        1 => (1.0, 0.99, 0.92),
        2 => (1.0, 0.93, 0.82),
        _ => (0.85, 0.92, 1.0),
    };

    for y in 0..h {
        for x in 0..w {
            let cell = y * w + x;
            let o = cell * 4;
            let e = world.kind[cell];

            if e == Elem::Empty as u8 {
                let surf = ground[x] as f32;
                let n = noise(x as i32, y as i32);
                if (y as f32) < surf {
                    // ---- Sky ----
                    let t = (y as f32 / surf.max(1.0)).clamp(0.0, 1.0);
                    let mut r = 11.0 + t * 30.0;
                    let mut g = 18.0 + t * 44.0;
                    let mut b = 32.0 + t * 58.0;

                    let dim = 0.45 + 0.55 * day;
                    r *= dim;
                    g *= dim;
                    b *= dim;
                    r *= sr;
                    g *= sg;
                    b *= sb;

                    // A sparse field of stars appears as night falls.
                    if day < 0.38 && n > 0.9968 {
                        let star = 90.0 + 110.0 * n;
                        r = r.max(star * 0.85);
                        g = g.max(star * 0.9);
                        b = b.max(star);
                    }

                    buf[o] = clamp_u8(r);
                    buf[o + 1] = clamp_u8(g);
                    buf[o + 2] = clamp_u8(b);
                } else {
                    // ---- Cave / underground void ----
                    let depth = ((y as f32 - surf) / (h as f32 - surf).max(1.0)).clamp(0.0, 1.0);
                    let base = 9.0 - depth * 4.0;
                    let warmth = n * 1.5;
                    buf[o] = clamp_u8(base * 0.7 + warmth);
                    buf[o + 1] = clamp_u8(base * 0.9 + warmth);
                    buf[o + 2] = clamp_u8(base * 1.2 + warmth);
                }
                buf[o + 3] = 255;
                continue;
            }

            let elem = Elem::from_u8(e);
            let c = world.col[cell];
            let n = noise(x as i32, y as i32);
            let heat = world.heat[cell] as f32;

            let mut r = c[0] as f32;
            let mut g = c[1] as f32;
            let mut b = c[2] as f32;

            // Depth falloff: rock near the surface reads brighter, deep stone
            // fades into shadow.
            let depth = (y as f32 - ground[x] as f32).max(0.0);
            let depth_fade = (1.0 - depth * 0.0032).clamp(0.62, 1.0);
            let shade = depth_fade * (0.62 + 0.38 * light) * (0.94 + n * 0.12);
            r *= shade;
            g *= shade;
            b *= shade;

            // Seasonal tint.
            r *= 0.96 + 0.04 * sr;
            g *= 0.96 + 0.04 * sg;
            b *= 0.96 + 0.04 * sb;

            // ---- Element-specific life and light ----
            match elem {
                Elem::Water => {
                    let wave = ((x as f32 * 0.17)
                        + (y as f32 * 0.09)
                        + time * 1.6)
                        .sin()
                        * 0.5
                        + 0.5;
                    r += wave * 7.0;
                    g += wave * 13.0;
                    b += wave * 22.0;
                }
                Elem::Fire => {
                    let flicker =
                        0.7 + 0.3 * ((time * 11.0 + n * std::f32::consts::TAU).sin() * 0.5 + 0.5);
                    r = r * 0.9 + 90.0 * flicker;
                    g = g * 0.9 + 46.0 * flicker;
                    b = b * 0.9 + 12.0 * flicker;
                }
                Elem::Lava | Elem::Magma => {
                    let pulse =
                        0.5 + 0.5 * ((time * 2.1 + n * std::f32::consts::TAU).sin());
                    let hot = 40.0 + heat * 0.35 + pulse * 38.0;
                    r += hot;
                    g += hot * 0.36;
                    b += hot * 0.12;
                }
                Elem::Plasma | Elem::Energy => {
                    let pulse =
                        0.5 + 0.5 * ((time * 6.0 + n * std::f32::consts::TAU).sin());
                    r += 16.0 * pulse;
                    g += 32.0 * pulse;
                    b += 64.0 * pulse;
                }
                Elem::Mana | Elem::Soul | Elem::Portal | Elem::Rune | Elem::Ward => {
                    let pulse =
                        0.5 + 0.5 * ((time * 2.6 + n * std::f32::consts::TAU).sin());
                    r += 24.0 * pulse;
                    g += 18.0 * pulse;
                    b += 46.0 * pulse;
                }
                Elem::Crystal | Elem::Gem | Elem::Diamond => {
                    let sparkle = ((time * 3.0 + n * std::f32::consts::TAU).sin() * 0.5 + 0.5)
                        * 26.0
                        * n;
                    r += sparkle * 0.7;
                    g += sparkle * 0.9;
                    b += sparkle;
                }
                Elem::Plant | Elem::Moss | Elem::Fungus | Elem::Spore | Elem::Roots => {
                    // Individual plants vary in hue so forests stop looking flat.
                    let v = (n - 0.5) * 26.0;
                    r = (r - v * 0.5).max(0.0);
                    g = (g + v).max(0.0);
                    b = (b - v * 0.2).max(0.0);
                }
                Elem::Ice => {
                    b += 10.0;
                }
                _ => {}
            }

            buf[o] = clamp_u8(r);
            buf[o + 1] = clamp_u8(g);
            buf[o + 2] = clamp_u8(b);
            buf[o + 3] = 255;
        }
    }
}
