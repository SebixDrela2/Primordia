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

/// Smooth 2D value noise (used for the cloud layer).
#[inline]
fn vnoise(x: f32, y: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = x - xi;
    let yf = y - yi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let ix = xi as i32;
    let iy = yi as i32;
    let a = noise(ix, iy);
    let b = noise(ix + 1, iy);
    let c = noise(ix, iy + 1);
    let d = noise(ix + 1, iy + 1);
    (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
}

/// Cloud density: domain-warped fractal noise. The warping breaks up the
/// round blobs so the result reads as torn cloud banks instead of ellipses.
#[inline]
fn cloud_density(x: f32, y: f32) -> f32 {
    let wx = vnoise(x * 0.8 + 3.1, y * 0.8 + 1.7) - 0.5;
    let wy = vnoise(x * 0.8 + 11.7, y * 0.8 + 5.2) - 0.5;
    let px = x + wx * 2.4;
    let py = y + wy * 1.1;
    vnoise(px, py) * 0.55
        + vnoise(px * 2.2 + 1.7, py * 2.2) * 0.3
        + vnoise(px * 4.6 + 8.2, py * 4.6) * 0.15
}

#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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
    let solid = Elem::solid_table();
    let mut ground = vec![0i32; w];
    for x in 0..w {
        let mut g = h as i32 - 1;
        let mut found = false;
        for y in 0..h {
            let k = world.kind[y * w + x];
            if k != Elem::Empty as u8 && solid[k as usize] {
                let supported = y + 1 < h
                    && world.kind[(y + 1) * w + x] != Elem::Empty as u8
                    && solid[world.kind[(y + 1) * w + x] as usize];
                if supported || y + 1 >= h {
                    g = y as i32;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            for y in 0..h {
                let k = world.kind[y * w + x];
                if k != Elem::Empty as u8 && solid[k as usize] {
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

    // Sun / moon position across the sky (day_night 0 = midnight, 0.5 = noon).
    let sun_x = (day_night * w as f32) as i32;
    let sun_height = (1.0 - (day_night - 0.5).abs() * 2.0).max(0.0);
    let sun_y = (h as f32 * (0.62 - 0.55 * sun_height)) as i32;
    let is_day = day > 0.25;

    // Cloud layer: domain-warped fractal noise sampled from a coarse grid.
    // The grid is stretched horizontally so the clouds come out as wide,
    // flat banks instead of round blobs.
    const GSX: f32 = 30.0;
    const GSY: f32 = 13.0;
    let gw = (w as f32 / GSX) as usize + 2;
    let gh = (h as f32 / GSY) as usize + 2;
    let mut clouds = vec![0.0f32; gw * gh];
    for gy in 0..gh {
        for gx in 0..gw {
            let fx = gx as f32 + time * 0.06;
            let fy = gy as f32;
            clouds[gy * gw + gx] = cloud_density(fx, fy);
        }
    }

    // Seasonal atmosphere tint (subtle, so materials keep their identity).
    let (sr, sg, sb) = match season {
        0 => (0.94f32, 1.0f32, 0.95f32),
        1 => (1.0, 0.99, 0.92),
        2 => (1.0, 0.93, 0.82),
        _ => (0.85, 0.92, 1.0),
    };

    for y in 0..h {
        let mut cv_row = 0.0f32;
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

                    // Sun or moon, with a soft halo.
                    let sdx = x as i32 - sun_x;
                    let sdy = y as i32 - sun_y;
                    let sd = (sdx * sdx + sdy * sdy) as f32;
                    if sd < 900.0 {
                        let halo = (1.0 - (sd / 900.0).sqrt()).max(0.0);
                        let core = if sd < 36.0 { 1.0 } else { 0.0 };
                        let (cr, cg, cb) = if is_day {
                            (255.0, 238.0, 190.0)
                        } else {
                            (200.0, 215.0, 245.0)
                        };
                        let amt = halo * halo * 0.55 + core;
                        r += (cr - r) * amt;
                        g += (cg - g) * amt;
                        b += (cb - b) * amt;
                    }

                    // Cloud banks, sampled from the warped noise grid. Only the
                    // upper sky carries clouds, and the sample is reused for
                    // every second column to keep the cost down.
                    if (y as f32) < surf * 0.6 {
                        if x & 1 == 0 {
                            let fx = x as f32 / GSX;
                            let fy = y as f32 / GSY;
                            let gx0 = (fx.floor() as usize).min(gw - 2);
                            let gy0 = (fy.floor() as usize).min(gh - 2);
                            let tx = fx - gx0 as f32;
                            let ty = fy - gy0 as f32;
                            let c00 = clouds[gy0 * gw + gx0];
                            let c10 = clouds[gy0 * gw + gx0 + 1];
                            let c01 = clouds[(gy0 + 1) * gw + gx0];
                            let c11 = clouds[(gy0 + 1) * gw + gx0 + 1];
                            cv_row = (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty)
                                + (c01 * (1.0 - tx) + c11 * tx) * ty;
                        }
                        // Soft edges, and only the denser parts become clouds.
                        let amt = smoothstep(0.56, 0.78, cv_row) * (0.20 + 0.32 * day);
                        if amt > 0.002 {
                            r += (206.0 - r) * amt;
                            g += (214.0 - g) * amt;
                            b += (232.0 - b) * amt;
                        }
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
