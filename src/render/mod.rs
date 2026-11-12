//! Rendering.
//!
//! Split by responsibility:
//! - mod.rs    the renderer, sky and terrain shading
//! - light.rs  the dynamic light field cast by emissive cells
//!
//! The hot loop lives in `paint`, a free function that takes plain slices.
//! Keeping it out of a `&mut self` method matters: the compiler cannot
//! prove that the pixel buffer does not alias the renderer's own fields, so
//! every pixel would reload them from memory.

pub mod fx;
pub mod light;

use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::{Biome, World, Weather};
use fx::{fx_table, Fx, FxEntry};
use light::LightField;

/// Reusable renderer state. All buffers are allocated once and reused, so
/// rendering never allocates in the hot path.
pub struct Renderer {
    /// The dynamic light field for the visible region.
    light: LightField,
    /// Cloud noise grid, refreshed every frame.
    cloud_row: Vec<f32>,
    /// The light field, copied out of the `LightField` so the pixel loop can
    /// read it through a plain shared slice. Sharing the field directly would
    /// make the compiler reload its pointer on every pixel write.
    light_rows: Vec<[f32; 3]>,
    /// Per-element visual effects, so the pixel loop never matches on the
    /// element enum.
    fx: Vec<FxEntry>,
    /// Per-column aurora band height. The band only depends on the column, so
    /// it is worked out once per frame instead of twice per pixel.
    aurora_col: Vec<f32>,
    /// Per-column biome tint delta (the tint minus one), so the pixel loop only
    /// has to add it.
    biome_delta: Vec<[f32; 3]>,
    /// Per-column sky gradient: the colour at the horizon and the slope over
    /// the height of the sky. The sky is the biggest part of the screen, so
    /// the gradient is worked out once per column instead of per pixel.
    sky_base: Vec<[f32; 3]>,
    sky_slope: Vec<[f32; 3]>,
    /// Per-column shadow cast by the clouds above.
    cloud_shadow: Vec<f32>,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            light: LightField::new(),
            cloud_row: Vec::new(),
            light_rows: Vec::new(),
            fx: fx_table(),
            aurora_col: Vec::new(),
            biome_delta: Vec::new(),
            sky_base: Vec::new(),
            sky_slope: Vec::new(),
            cloud_shadow: Vec::new(),
        }
    }

    /// Paint the whole world into an RGBA buffer.
    pub fn render_world(
        &mut self,
        world: &World,
        creatures: &CreatureSystem,
        buf: &mut [u8],
        time: f32,
        day_night: f32,
        season: u8,
        weather: Weather,
    ) {
        self.render_region(
            world,
            creatures,
            buf,
            0,
            0,
            world.w as i32,
            world.h as i32,
            time,
            day_night,
            season,
            weather,
        );
    }

    /// Paint only the visible part of the world. `buf` holds `rw * rh` pixels.
    ///
    /// `time` drives the animated shimmer/glow effects, `day_night` is 0.0 at
    /// midnight and 0.5 at noon, and `season` is 0..=3.
    #[allow(clippy::too_many_arguments)]
    pub fn render_region(
        &mut self,
        world: &World,
        _creatures: &CreatureSystem,
        buf: &mut [u8],
        x0: i32,
        y0: i32,
        rw: i32,
        rh: i32,
        time: f32,
        day_night: f32,
        season: u8,
        weather: Weather,
    ) {
        let w = world.w;
        let h = world.h;

        // ---- Dynamic light field -----------------------------------------
        // Emissive cells inside the region seed the field; the blur spreads
        // the light so a lava chamber lights the whole cave around it.
        //
        // The field is anchored to *world* coordinates, not to the region:
        // a light cell always covers the same four world cells whatever the
        // camera is doing. Anchoring it to the region instead made the
        // lighting slide every time the camera panned or zoomed.
        //
        // Only the underground part of the world needs lighting: above the
        // ground the sun already provides the illumination, so the sky rows
        // are skipped entirely. That keeps the extra pass small even when the
        // whole world is on screen.
        const SCALE: i32 = light::LIGHT_SCALE as i32;
        // The light cell that covers world column x is x / SCALE, so the
        // window starts on the cell boundary that contains x0. The margin is
        // the blur radius: with it, every cell in the middle of the region
        // gets exactly the same light it would get if the whole world were
        // in view.
        const MARGIN: i32 = 3;
        let gx0 = x0.div_euclid(SCALE) - MARGIN;
        let gy0 = y0.div_euclid(SCALE) - MARGIN;
        let gx1 = (x0 + rw).div_euclid(SCALE) + MARGIN;
        let gy1 = (y0 + rh).div_euclid(SCALE) + MARGIN;
        let lw = (gx1 - gx0 + 1) as usize;
        let lh = (gy1 - gy0 + 1) as usize;
        self.light.resize(lw, lh);
        self.light.clear();
        // Seed the field from the emissive cells. Whole light cells are
        // scanned, so a cell gathers exactly the same light no matter where
        // the camera is looking.
        let sx0 = (gx0 * SCALE).max(0);
        let sy0 = (gy0 * SCALE).max(0);
        let sx1 = ((gx1 + 1) * SCALE).min(w as i32);
        let sy1 = ((gy1 + 1) * SCALE).min(h as i32);
        for y in sy0..sy1 {
            let row = y as usize * w;
            let ly = (y.div_euclid(SCALE) - gy0) as usize;
            for x in sx0..sx1 {
                let k = world.kind[row + x as usize];
                if world.emissive[k as usize] {
                    let lx = (x.div_euclid(SCALE) - gx0) as usize;
                    self.light.add(lx, ly, world.emission[k as usize]);
                }
            }
        }
        // Spread the light outwards with a decaying falloff. This keeps the
        // brightness of the source instead of averaging it away, so a lava
        // chamber really lights the cave around it.
        let __t1 = std::time::Instant::now();
        self.light.blur(2);
        self.light.finalize();
        let __t2 = __t1.elapsed().as_secs_f64()*1000.0;

        // ---- Sky ---------------------------------------------------------
        // Cloud layer: domain-warped fractal noise sampled from a coarse grid.
        // The grid is stretched horizontally so the clouds come out as wide,
        // flat banks instead of round blobs.
        const GSX: f32 = 30.0;
        const GSY: f32 = 13.0;
        let gw = (w as f32 / GSX) as usize + 2;
        let gh = (h as f32 / GSY) as usize + 2;
        if self.cloud_row.len() != gw * gh {
            self.cloud_row.clear();
            self.cloud_row.resize(gw * gh, 0.0);
        }
        for gy in 0..gh {
            for gx in 0..gw {
                let fx = gx as f32 + time * 0.06;
                let fy = gy as f32;
                self.cloud_row[gy * gw + gx] = cloud_density(fx, fy);
            }
        }

        // Copy the light field into a flat buffer. The pixel loop then reads
        // it through a plain shared slice, which the compiler knows cannot
        // alias the pixel buffer.
        if self.light_rows.len() != self.light.buf.len() {
            self.light_rows.clear();
            self.light_rows.resize(self.light.buf.len(), [0.0; 3]);
        }
        self.light_rows.copy_from_slice(&self.light.buf);

        // The aurora band depends only on the column, so it is worked out
        // once per frame instead of twice for every pixel of that column.
        if self.aurora_col.len() != rw as usize {
            self.aurora_col.clear();
            self.aurora_col.resize(rw as usize, 0.0);
        }
        for i in 0..rw as usize {
            let x = x0 + i as i32;
            self.aurora_col[i] = ((x as f32 * 0.021 + time * 0.08).sin() * 34.0
                + (x as f32 * 0.007 - time * 0.03).sin() * 22.0
                + 90.0);
        }

        // The shadow the clouds cast on the ground. It is sampled from the
        // same noise grid as the clouds, a little further down, so the shade
        // drifts with the weather.
        if self.cloud_shadow.len() != rw as usize {
            self.cloud_shadow.clear();
            self.cloud_shadow.resize(rw as usize, 1.0);
        }
        const CSX: f32 = 30.0;
        const CSY: f32 = 13.0;
        let cgw = (w as f32 / CSX) as usize + 2;
        let cgh = (h as f32 / CSY) as usize + 2;
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1);
            let gy = (world.ground[x as usize] as f32 * 0.35) / CSY;
            let gx = x as f32 / CSX;
            let gx0 = (gx.floor() as usize).min(cgw - 2);
            let gy0 = (gy.floor() as usize).min(cgh - 2);
            let tx = gx - gx0 as f32;
            let ty = gy - gy0 as f32;
            let c00 = self.cloud_row[gy0 * cgw + gx0];
            let c10 = self.cloud_row[gy0 * cgw + gx0 + 1];
            let c01 = self.cloud_row[(gy0 + 1) * cgw + gx0];
            let c11 = self.cloud_row[(gy0 + 1) * cgw + gx0 + 1];
            let cv = (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty) + (c01 * (1.0 - tx) + c11 * tx) * ty;
            // A dense cloud darkens the ground a little.
            self.cloud_shadow[i] = 1.0 - smoothstep(0.58, 0.85, cv) * 0.16;
        }

        // The biome tint delta per column, so the pixel loop only adds it.
        if self.biome_delta.len() != rw as usize {
            self.biome_delta.clear();
            self.biome_delta.resize(rw as usize, [0.0; 3]);
        }
        // The sky gradient is separable: the colour depends on the column
        // (biome, weather, season, time of day) and the row (the height in
        // the sky). Working the column part out once turns the per-pixel sky
        // into a multiply and an add.
        if self.sky_base.len() != rw as usize {
            self.sky_base.clear();
            self.sky_base.resize(rw as usize, [0.0; 3]);
            self.sky_slope.clear();
            self.sky_slope.resize(rw as usize, [0.0; 3]);
        }
        let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
        let dim = 0.45 + 0.55 * day;
        let (sr, sg, sb) = match season {
            0 => (0.94f32, 1.0f32, 0.95f32),
            1 => (1.0, 0.99, 0.92),
            2 => (1.0, 0.93, 0.82),
            _ => (0.85, 0.92, 1.0),
        };
        let weather_sky = match weather {
            Weather::Clear => [1.0f32, 1.0, 1.0],
            Weather::Rain => [0.78, 0.82, 0.90],
            Weather::Storm => [0.62, 0.66, 0.76],
            Weather::Snow => [0.92, 0.95, 1.02],
            Weather::Meteor => [0.88, 0.80, 0.86],
        };
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1) as usize;
            let bt = world.biome_tint[x];
            self.biome_delta[i] = [bt[0] - 1.0, bt[1] - 1.0, bt[2] - 1.0];
            // Sky colour at the horizon and the slope over the sky height.
            let k = [
                dim * sr * weather_sky[0] * (1.0 + (bt[0] - 1.0) * 0.55),
                dim * sg * weather_sky[1] * (1.0 + (bt[1] - 1.0) * 0.55),
                dim * sb * weather_sky[2] * (1.0 + (bt[2] - 1.0) * 0.55),
            ];
            self.sky_base[i] = [11.0 * k[0], 18.0 * k[1], 32.0 * k[2]];
            self.sky_slope[i] = [30.0 * k[0], 44.0 * k[1], 58.0 * k[2]];
        }

        let __t3 = std::time::Instant::now();
        // The hot loop takes plain slices, so nothing has to be reloaded from
        // the renderer state for every pixel.
        paint(
            world,
            buf,
            &self.cloud_row,
            &self.fx,
            &self.light_rows,
            &self.aurora_col,
            &self.biome_delta,
            &self.cloud_shadow,
            &self.sky_base,
            &self.sky_slope,
            lw,
            lh,
            x0,
            y0,
            rw,
            rh,
            time,
            day_night,
            season,
            weather,
            gx0,
            gy0,
        );
        // The per-frame timing print is only useful while profiling, and
        // printing every frame costs more than the render itself.
        if std::env::var_os("PRIMORDIA_RENDER_TIMING").is_some() {
            eprintln!(
                "blur {:.3}  paint {:.3}",
                __t2,
                __t3.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}

/// The pixel loop. Everything it touches is a plain slice, which lets the
/// compiler keep the pointers in registers.
#[allow(clippy::too_many_arguments)]
fn paint(
    world: &World,
    buf: &mut [u8],
    cloud_row: &[f32],
    fx: &[FxEntry],
    light_rows: &[[f32; 3]],
    aurora_col: &[f32],
    biome_delta: &[[f32; 3]],
    cloud_shadow: &[f32],
    sky_base: &[[f32; 3]],
    sky_slope: &[[f32; 3]],
    lw: usize,
    lh: usize,
    x0: i32,
    y0: i32,
    rw: i32,
    rh: i32,
    time: f32,
    day_night: f32,
    season: u8,
    weather: Weather,
    gx0: i32,
    gy0: i32,
) {
    let w = world.w;
    let h = world.h;
    let empty = Elem::Empty as u8;

    // 0.0 = midnight, 1.0 = noon.
    let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
    let mut light_level = 0.34 + 0.66 * day;

    // Weather dims the world and mutes the sun.
    let (weather_dim, weather_sky, weather_snow) = match weather {
        Weather::Clear => (0.0, [1.0, 1.0, 1.0], 0.0),
        Weather::Rain => (0.14, [0.78, 0.82, 0.90], 0.0),
        Weather::Storm => (0.30, [0.62, 0.66, 0.76], 0.0),
        Weather::Snow => (0.06, [0.92, 0.95, 1.02], 0.35),
        Weather::Meteor => (0.10, [0.88, 0.80, 0.86], 0.0),
    };
    light_level = (light_level * (1.0 - weather_dim)).clamp(0.08, 1.0);

    // Seasonal atmosphere tint (subtle, so materials keep their identity).
    let (sr, sg, sb) = match season {
        0 => (0.94f32, 1.0f32, 0.95f32),
        1 => (1.0, 0.99, 0.92),
        2 => (1.0, 0.93, 0.82),
        _ => (0.85, 0.92, 1.0),
    };
    // The season tint and the weather mute, folded into one multiplier.
    let sky_mul = [
        sr * weather_sky[0],
        sg * weather_sky[1],
        sb * weather_sky[2],
    ];

    // The terrain shading uses the same factors for every pixel, so they are
    // worked out once here instead of once per pixel.
    let season_r = 0.96 + 0.04 * sr;
    let season_g = 0.96 + 0.04 * sg;
    let season_b = 0.96 + 0.04 * sb;
    let light_f = 0.62 + 0.38 * light_level;

    // The aurora only shows on cold, clear nights, so the check is hoisted
    // out of the pixel loop.
    let aurora = day < 0.32;
    // Sun / moon position across the sky (day_night 0 = midnight, 0.5 = noon).
    let sun_x = (day_night * w as f32) as i32;
    let sun_height = (1.0 - (day_night - 0.5).abs() * 2.0).max(0.0);
    let sun_y = (h as f32 * (0.62 - 0.55 * sun_height)) as i32;
    let is_day = day > 0.25;

    // Cloud grid dimensions, used for the sky sampling.
    const GSX: f32 = 30.0;
    const GSY: f32 = 13.0;
    let gw = (w as f32 / GSX) as usize + 2;
    let gh = (h as f32 / GSY) as usize + 2;

    // Hoist the world arrays so the inner loop touches them directly.
    let kind = &world.kind;
    let col = &world.col;
    let heat = &world.heat;
    let ground = &world.ground;    let ground_inv = &world.ground_inv;
    let biome_tint = &world.biome_tint;
    let biome = &world.biome;
    let _ = biome_tint;
    for ry in 0..rh {
        let y = y0 + ry;
        if y < 0 || y >= h as i32 {
            continue;
        }
        let mut cv_row = 0.0f32;
        let row_base = y as usize * w;
        // The row of the light field that this row of the region reads. It is
        // hoisted out of the pixel loop so the inner loop only shifts.
        // The light field is read through a shared reference, so the compiler
        // knows it cannot alias the pixel buffer. Reading the light field
        // directly would make every pixel write reload the field's pointer.
        let light_row: &[[f32; 3]] = {
            let ly = (y.div_euclid(4) - gy0).max(0) as usize;
            if ly < lh {
                &light_rows[ly * lw..ly * lw + lw]
            } else {
                &[]
            }
        };
        for rx in 0..rw {
            let x = x0 + rx;
            if x < 0 || x >= w as i32 {
                continue;
            }
            // The light column advances every four world cells. Tracking it
            // incrementally avoids a division per pixel.
            let light_col = if light_row.is_empty() {
                0
            } else {
                let c = (x >> 2) - gx0;
                if c < 0 {
                    0
                } else if c as usize >= lw {
                    lw - 1
                } else {
                    c as usize
                }
            };
            let cell = row_base + x as usize;
            let o = (ry as usize * rw as usize + rx as usize) * 4;
            let e = kind[cell];
            // The biome tint is a per-column value; the delta is what is
            // actually needed, so it is looked up directly.
            let bt = biome_delta[rx as usize];

            if e == empty {
                let surf = ground[x as usize] as f32;
                if (y as f32) < surf {
                    // ---- Sky ----
                    // The gradient was worked out per column, so a pixel only
                    // needs one multiply and one add per channel.
                    let t = (y as f32 * ground_inv[x as usize]).clamp(0.0, 1.0);
                    let base = sky_base[rx as usize];
                    let slope = sky_slope[rx as usize];
                    let mut r = base[0] + t * slope[0];
                    let mut g = base[1] + t * slope[1];
                    let mut b = base[2] + t * slope[2];

                    // A sparse field of stars appears as night falls.
                    if day < 0.38 {
                        let sn = noise(x, y);
                        if sn > 0.9968 {
                            let star = 90.0 + 110.0 * sn;
                            r = r.max(star * 0.85);
                            g = g.max(star * 0.9);
                            b = b.max(star);
                        }
                    }

                    // Sun or moon, with a soft halo. Only the cells around it
                    // need the expensive part.
                    let sdx = x - sun_x;
                    let sdy = y - sun_y;
                    if sdx.abs() < 32 && sdy.abs() < 32 {
                        let sd = (sdx * sdx + sdy * sdy) as f32;
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

                    // Aurora over the tundra on cold, clear nights.
                    if aurora && biome[x as usize] == Biome::Tundra {
                        let band = aurora_col[rx as usize] - y as f32;
                        if band.abs() < 30.0 {
                            let amt = (1.0 - band.abs() / 30.0) * 0.5 * (1.0 - day / 0.32);
                            let (ar, ag, ab) = if band > 0.0 {
                                (90.0, 255.0, 170.0)
                            } else {
                                (150.0, 110.0, 255.0)
                            };
                            r += (ar - r) * amt;
                            g += (ag - g) * amt;
                            b += (ab - b) * amt;
                        }
                    }

                    // Cloud banks, sampled from the warped noise grid. Only the
                    // upper sky carries clouds, and the sample is reused for
                    // every third column to keep the cost down.
                    if (y as f32) < surf * 0.6 {
                        if rx % 3 == 0 {
                            let fx = x as f32 / GSX;
                            let fy = y as f32 / GSY;
                            let gx0 = (fx.floor() as usize).min(gw - 2);
                            let gy0 = (fy.floor() as usize).min(gh - 2);
                            let tx = fx - gx0 as f32;
                            let ty = fy - gy0 as f32;
                            let c00 = cloud_row[gy0 * gw + gx0];
                            let c10 = cloud_row[gy0 * gw + gx0 + 1];
                            let c01 = cloud_row[(gy0 + 1) * gw + gx0];
                            let c11 = cloud_row[(gy0 + 1) * gw + gx0 + 1];
                            cv_row = (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty)
                                + (c01 * (1.0 - tx) + c11 * tx) * ty;
                        }
                        // Stormy skies carry heavier, darker cloud.
                        let cover = 0.20 + 0.32 * day + weather_dim * 0.9;
                        let amt = smoothstep(0.56, 0.78, cv_row) * cover;
                        if amt > 0.002 {
                            let cr = 206.0 - weather_dim * 90.0;
                            let cg = 214.0 - weather_dim * 90.0;
                            let cb = 232.0 - weather_dim * 60.0;
                            r += (cr - r) * amt;
                            g += (cg - g) * amt;
                            b += (cb - b) * amt;
                        }
                    }

                    // A thin haze of snow settles over the whole sky in a
                    // blizzard, which reads as reduced visibility.
                    if weather_snow > 0.0 {
                        let a = weather_snow * 0.35;
                        r += (235.0 - r) * a;
                        g += (240.0 - g) * a;
                        b += (250.0 - b) * a;
                    }

                    buf[o] = clamp_u8(r);
                    buf[o + 1] = clamp_u8(g);
                    buf[o + 2] = clamp_u8(b);
                } else {
                    // ---- Cave / underground void ----
                    let depth = ((y as f32 - surf) / (h as f32 - surf).max(1.0)).clamp(0.0, 1.0);
                    let base = 9.0 - depth * 4.0;
                    let warmth = noise(x, y) * 1.5;
                    buf[o] = clamp_u8(base * 0.7 + warmth);
                    buf[o + 1] = clamp_u8(base * 0.9 + warmth);
                    buf[o + 2] = clamp_u8(base * 1.2 + warmth);
                }
                buf[o + 3] = 255;
                continue;
            }

            let c = col[cell];
            let n = noise(x, y);
            let heat_v = heat[cell] as f32;

            let mut r = c[0] as f32;
            let mut g = c[1] as f32;
            let mut b = c[2] as f32;

            // Depth falloff: rock near the surface reads brighter, deep stone
            // fades into shadow.
            let depth = (y as f32 - ground[x as usize] as f32).max(0.0);
            let depth_fade = (1.0 - depth * 0.0032).clamp(0.62, 1.0);
            let shade = depth_fade * light_f * (0.94 + n * 0.12) * cloud_shadow[rx as usize];
            r *= shade;
            g *= shade;
            b *= shade;

            // Seasonal tint.
            r *= season_r;
            g *= season_g;
            b *= season_b;

            // In winter the first few cells under the sky are covered in snow.
            if season == 3 && depth < 3.0 {
                let s = (1.0 - depth * (1.0 / 3.0)) * 0.6;
                r += (232.0 - r) * s;
                g += (238.0 - g) * s;
                b += (248.0 - b) * s;
            }

            // Biome tint: deserts glow warm, tundra cold, forests green. It
            // fades out with depth so only the surface reads as "biome".
            let fade = (1.0 - depth * (1.0 / 60.0)).clamp(0.0, 1.0);
            r += bt[0] * fade;
            g += bt[1] * fade;
            b += bt[2] * fade;

            // ---- Element-specific life and light ----
            // The effect is described by a table entry, so the pixel loop
            // never matches on the element enum.
            let fx = &fx[e as usize];
            match fx.kind {
                Fx::None => {}
                Fx::Wave => {
                    let wave = ((x as f32 * 0.17) + (y as f32 * 0.09) + time * 1.6).sin() * 0.5 + 0.5;
                    r += wave * fx.color[0];
                    g += wave * fx.color[1];
                    b += wave * fx.color[2];
                    // Water catches a little of the sky above it.
                    let sky = sky_base[rx as usize];
                    r += sky[0] * 0.02;
                    g += sky[1] * 0.02;
                    b += sky[2] * 0.03;
                    // Foam where the water meets the land. The shore is
                    // checked against the four neighbours, which is cheap
                    // because only the water pixels pay for it.
                    if is_shore(kind, w, h, x, y, empty, Elem::Water as u8) {
                        let surf = ((x as f32 * 0.31) + (y as f32 * 0.19) - time * 2.4).sin() * 0.5
                            + 0.5;
                        let foam = (0.35 + 0.65 * surf) * 46.0;
                        r += foam * 0.9;
                        g += foam * 0.95;
                        b += foam;
                    }
                }
                Fx::Flicker => {
                    let flicker =
                        0.7 + 0.3 * ((time * 11.0 + n * std::f32::consts::TAU).sin() * 0.5 + 0.5);
                    r = r * 0.9 + fx.color[0] * flicker;
                    g = g * 0.9 + fx.color[1] * flicker;
                    b = b * 0.9 + fx.color[2] * flicker;
                }
                Fx::Heat => {
                    let pulse = 0.5 + 0.5 * ((time * 2.1 + n * std::f32::consts::TAU).sin());
                    let hot = 40.0 + heat_v * 0.35 + pulse * 38.0;
                    r += hot * fx.color[0];
                    g += hot * fx.color[1];
                    b += hot * fx.color[2];
                }
                Fx::Pulse | Fx::Mystic | Fx::Ember => {
                    let pulse = 0.5 + 0.5 * ((time * fx.speed + n * std::f32::consts::TAU).sin());
                    r += fx.color[0] * pulse;
                    g += fx.color[1] * pulse;
                    b += fx.color[2] * pulse;
                }
                Fx::Sparkle => {
                    let sparkle = ((time * 3.0 + n * std::f32::consts::TAU).sin() * 0.5 + 0.5) * 26.0 * n;
                    r += sparkle * fx.color[0];
                    g += sparkle * fx.color[1];
                    b += sparkle * fx.color[2];
                }
                Fx::Foliage => {
                    // Individual plants vary in hue so forests stop looking flat.
                    let v = (n - 0.5) * 26.0;
                    r = (r + fx.color[0] * v / 26.0).max(0.0);
                    g = (g + fx.color[1] * v / 26.0).max(0.0);
                    b = (b + fx.color[2] * v / 26.0).max(0.0);
                }
                Fx::Glint => {
                    b += 10.0;
                    // Ice and snow catch a cold specular highlight.
                    let glint = ((time * 0.9 + n * 6.0).sin() * 0.5 + 0.5) * 8.0;
                    r += glint * fx.color[0];
                    g += glint * fx.color[1];
                    b += glint * fx.color[2];
                }
                Fx::Grain => {
                    // Charcoal is matte black with a faint warm grain.
                    r += fx.color[0] * n;
                    g += fx.color[1] * n;
                }
                Fx::Sheen => {
                    // Tar is glossy: a slow-moving highlight on black.
                    let sheen =
                        ((time * 0.5 + x as f32 * 0.05 + y as f32 * 0.03).sin() * 0.5 + 0.5) * 10.0;
                    r += sheen * fx.color[0];
                    g += sheen * fx.color[1];
                    b += sheen * fx.color[2];
                }
            }

            // ---- Dynamic light ----
            // Light from nearby emissive cells warms and brightens the rock
            // around them. This is what makes caves readable.
            //
            // The light field is coarser than the render, so nearest
            // neighbour is enough: the field is already blurred over several
            // cells. The column is derived from the world coordinate, so the
            // lighting of a cell does not depend on the camera.
            if !light_row.is_empty() {
                let lf = light_row[light_col];
                r += lf[0];
                g += lf[1];
                b += lf[2];
            }

            buf[o] = clamp_u8(r);
            buf[o + 1] = clamp_u8(g);
            buf[o + 2] = clamp_u8(b);
            buf[o + 3] = 255;
        }
    }
}

/// Is this water cell touching the shore?
///
/// The foam of a breaking wave only shows on the cells that touch the
/// land, so the four neighbours are looked at. Only the water pixels pay
/// for the check.
#[inline]
fn is_shore(
    kind: &[u8],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    empty: u8,
    water: u8,
) -> bool {
    let xi = x as usize;
    let yi = y as usize;
    let row = yi * w;
    // The land is anything that is neither empty nor more water.
    let land = |k: u8| k != empty && k != water;
    let left = xi > 0 && land(kind[row + xi - 1]);
    let right = xi + 1 < w && land(kind[row + xi + 1]);
    let up = yi > 0 && land(kind[row - w + xi]);
    let down = yi + 1 < h && land(kind[row + w + xi]);
    left || right || up || down
}

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
