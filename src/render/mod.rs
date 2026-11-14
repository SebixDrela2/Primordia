pub mod bloom;
pub mod fx;
pub mod light;

use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::{Biome, World, Weather, DEPTH_TINT};
use bloom::Bloom;
use fx::{fx_table, Fx, FxEntry};
use light::LightField;

pub struct Renderer {

    light: LightField,

    cloud_row: Vec<f32>,

    light_rows: Vec<[f32; 3]>,

    fx: [FxEntry; fx::FX_TABLE_LEN],

    aurora_col: Vec<f32>,

    biome_delta: Vec<[f32; 3]>,

    sky_base: Vec<[f32; 3]>,
    sky_slope: Vec<[f32; 3]>,

    cloud_shadow: Vec<f32>,

    sun_shaft: Vec<f32>,

    vig_col: Vec<f32>,

    bloom: Bloom,
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
            sun_shaft: Vec::new(),
            vig_col: Vec::new(),
            bloom: Bloom::new(),
        }
    }

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

        const SCALE: i32 = light::LIGHT_SCALE as i32;

        const MARGIN: i32 = 3;
        let gx0 = x0.div_euclid(SCALE) - MARGIN;
        let gy0 = y0.div_euclid(SCALE) - MARGIN;
        let gx1 = (x0 + rw).div_euclid(SCALE) + MARGIN;
        let gy1 = (y0 + rh).div_euclid(SCALE) + MARGIN;
        let lw = (gx1 - gx0 + 1) as usize;
        let lh = (gy1 - gy0 + 1) as usize;
        self.light.resize(lw, lh);
        self.light.clear();

        self.bloom.resize(rw as usize, rh as usize);

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

                    let e = world.emission[k as usize];
                    self.bloom.add(
                        (x - x0) as usize,
                        (y - y0) as usize,
                        e[0],
                        e[1],
                        e[2],
                    );
                }
            }
        }

        self.light.blur(2);
        self.light.finalize();

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

        if self.light_rows.len() != self.light.buf.len() {
            self.light_rows.clear();
            self.light_rows.resize(self.light.buf.len(), [0.0; 3]);
        }
        self.light_rows.copy_from_slice(&self.light.buf);

        if self.aurora_col.len() != rw as usize {
            self.aurora_col.clear();
            self.aurora_col.resize(rw as usize, 0.0);
        }
        for i in 0..rw as usize {
            let x = x0 + i as i32;
            self.aurora_col[i] = (x as f32 * 0.021 + time * 0.08).sin() * 34.0
                + (x as f32 * 0.007 - time * 0.03).sin() * 22.0
                + 90.0;
        }

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

            self.cloud_shadow[i] = 1.0 - smoothstep(0.58, 0.85, cv) * 0.16;
        }

        if self.vig_col.len() != rw as usize {
            self.vig_col.clear();
            self.vig_col.resize(rw as usize, 0.0);
        }
        for i in 0..rw as usize {
            let nx = (i as f32 / rw.max(1) as f32 - 0.5) * 2.0;
            self.vig_col[i] = nx * nx;
        }

        if self.biome_delta.len() != rw as usize {
            self.biome_delta.clear();
            self.biome_delta.resize(rw as usize, [0.0; 3]);
        }

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
            Weather::Fog => [0.86, 0.88, 0.90],
            Weather::Heatwave => [1.10, 1.02, 0.86],
            Weather::Sandstorm => [1.02, 0.92, 0.72],
        };
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1) as usize;
            let bt = world.biome_tint[x];
            self.biome_delta[i] = [bt[0] - 1.0, bt[1] - 1.0, bt[2] - 1.0];

            let k = [
                dim * sr * weather_sky[0] * (1.0 + (bt[0] - 1.0) * 0.55),
                dim * sg * weather_sky[1] * (1.0 + (bt[1] - 1.0) * 0.55),
                dim * sb * weather_sky[2] * (1.0 + (bt[2] - 1.0) * 0.55),
            ];
            self.sky_base[i] = [11.0 * k[0], 18.0 * k[1], 32.0 * k[2]];
            self.sky_slope[i] = [30.0 * k[0], 44.0 * k[1], 58.0 * k[2]];

            let (veil, vr, vg, vb) = match weather {
                Weather::Snow => (0.35, 235.0, 240.0, 250.0),
                Weather::Fog => (0.42, 198.0, 203.0, 208.0),
                Weather::Heatwave => (0.22, 255.0, 238.0, 196.0),
                Weather::Sandstorm => (0.40, 226.0, 198.0, 138.0),
                _ => (0.0, 0.0, 0.0, 0.0),
            };
            if veil > 0.0 {
                let keep = 1.0 - veil;
                let sb = self.sky_base[i];
                let ss = self.sky_slope[i];
                self.sky_base[i] = [
                    sb[0] * keep + vr * veil,
                    sb[1] * keep + vg * veil,
                    sb[2] * keep + vb * veil,
                ];
                self.sky_slope[i] = [ss[0] * keep, ss[1] * keep, ss[2] * keep];
            }
        }

        if self.sun_shaft.len() != rw as usize {
            self.sun_shaft.clear();
            self.sun_shaft.resize(rw as usize, 0.0);
        }

        let weather_dim: f32 = match weather {
            Weather::Clear => 0.0,
            Weather::Rain => 0.14,
            Weather::Storm => 0.30,
            Weather::Snow => 0.06,
            Weather::Meteor => 0.10,
            Weather::Fog => 0.18,
            Weather::Heatwave => 0.04,
            Weather::Sandstorm => 0.22,
        };
        let shaft_light = day * (1.0 - weather_dim * 1.4).max(0.0);
        const SSX: f32 = 30.0;
        const SSY: f32 = 13.0;
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1);

            let gx = x as f32 / SSX + 0.55;
            let gy = (world.ground[x as usize] as f32 * 0.35) / SSY;
            let gx0 = (gx.floor() as usize).min(cgw - 2);
            let gy0 = (gy.floor() as usize).min(cgh - 2);
            let tx = gx - gx0 as f32;
            let ty = gy - gy0 as f32;
            let c00 = self.cloud_row[gy0 * cgw + gx0];
            let c10 = self.cloud_row[gy0 * cgw + gx0 + 1];
            let c01 = self.cloud_row[(gy0 + 1) * cgw + gx0];
            let c11 = self.cloud_row[(gy0 + 1) * cgw + gx0 + 1];
            let cv = (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty) + (c01 * (1.0 - tx) + c11 * tx) * ty;

            self.sun_shaft[i] = smoothstep(0.30, 0.56, cv) * shaft_light;
        }

        self.bloom.blur();

        paint(
            world,
            buf,
            &self.cloud_row,
            &self.fx,
            &self.light_rows,
            &self.aurora_col,
            &self.biome_delta,
            &self.cloud_shadow,
            &self.sun_shaft,
            &self.vig_col,
            &self.bloom,
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
    }
}

#[allow(clippy::too_many_arguments)]
fn paint(
    world: &World,
    buf: &mut [u8],
    cloud_row: &[f32],
    fx: &[FxEntry; fx::FX_TABLE_LEN],
    light_rows: &[[f32; 3]],
    aurora_col: &[f32],
    biome_delta: &[[f32; 3]],
    cloud_shadow: &[f32],
    sun_shaft: &[f32],
    vig_col: &[f32],
    bloom: &Bloom,
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

    let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
    let mut light_level = 0.34 + 0.66 * day;

    let (weather_dim, weather_sky, weather_haze) = match weather {
        Weather::Clear => (0.0, [1.0, 1.0, 1.0], 0.0),
        Weather::Rain => (0.14, [0.78, 0.82, 0.90], 0.18),
        Weather::Storm => (0.30, [0.62, 0.66, 0.76], 0.26),
        Weather::Snow => (0.06, [0.92, 0.95, 1.02], 0.35),
        Weather::Meteor => (0.10, [0.88, 0.80, 0.86], 0.12),

        Weather::Fog => (0.18, [0.86, 0.88, 0.90], 0.55),

        Weather::Heatwave => (0.04, [1.10, 1.02, 0.86], 0.16),

        Weather::Sandstorm => (0.22, [1.02, 0.92, 0.72], 0.42),
    };
    light_level = (light_level * (1.0 - weather_dim)).clamp(0.08, 1.0);

    let flash = if weather == Weather::Storm {
        let p = (time * 0.83).fract();
        if p < 0.035 {
            1.0 - p / 0.035
        } else {
            0.0
        }
    } else {
        0.0
    };

    let shimmer = weather == Weather::Heatwave;

    let (sr, sg, sb) = match season {
        0 => (0.94f32, 1.0f32, 0.95f32),
        1 => (1.0, 0.99, 0.92),
        2 => (1.0, 0.93, 0.82),
        _ => (0.85, 0.92, 1.0),
    };

    let _sky_mul = [
        sr * weather_sky[0],
        sg * weather_sky[1],
        sb * weather_sky[2],
    ];

    let season_r = 0.96 + 0.04 * sr;
    let season_g = 0.96 + 0.04 * sg;
    let season_b = 0.96 + 0.04 * sb;
    let light_f = 0.62 + 0.38 * light_level;

    let haze = weather_haze;
    let haze_col = match weather {
        Weather::Fog => [196.0f32, 202.0, 205.0],
        Weather::Snow => [235.0, 240.0, 250.0],
        Weather::Storm => [150.0, 158.0, 172.0],
        Weather::Rain => [168.0, 176.0, 190.0],
        Weather::Sandstorm => [214.0, 186.0, 132.0],
        _ => [200.0, 200.0, 200.0],
    };

    let aurora = day < 0.32;

    let sun_x = (day_night * w as f32) as i32;
    let sun_height = (1.0 - (day_night - 0.5).abs() * 2.0).max(0.0);
    let sun_y = (h as f32 * (0.62 - 0.55 * sun_height)) as i32;
    let is_day = day > 0.25;

    const GSX: f32 = 30.0;
    const GSY: f32 = 13.0;
    let gw = (w as f32 / GSX) as usize + 2;
    let gh = (h as f32 / GSY) as usize + 2;

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

        let ny = (ry as f32 / rh.max(1) as f32 - 0.5) * 2.0;
        let ny2 = ny * ny;

        let glow_row = (ry as usize / bloom::STEP).min(bloom.h.saturating_sub(1));

        let kind_row = &kind[row_base..row_base + w as usize];
        let col_row = &col[row_base..row_base + w as usize];
        let heat_row = &heat[row_base..row_base + w as usize];

        let row_band = ((y as f32 * 0.28).sin() * 0.5 + 0.5)
            * ((y as f32 * 0.11).sin() * 0.5 + 0.5);

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
            let xr = x as usize;
            let o = (ry as usize * rw as usize + rx as usize) * 4;
            let e = kind_row[xr];

            let bt = biome_delta[rx as usize];

            if e == empty {
                let surf = ground[xr] as f32;
                if (y as f32) < surf {

                    let t = (y as f32 * ground_inv[xr]).clamp(0.0, 1.0);
                    let base = sky_base[rx as usize];
                    let slope = sky_slope[rx as usize];
                    let mut r = base[0] + t * slope[0];
                    let mut g = base[1] + t * slope[1];
                    let mut b = base[2] + t * slope[2];

                    if day < 0.38 && (y as f32) < surf * 0.7 {
                        let sn = noise(x, y);
                        if sn > 0.9968 {
                            let star = 90.0 + 110.0 * sn;
                            r = r.max(star * 0.85);
                            g = g.max(star * 0.9);
                            b = b.max(star);
                        }
                    }

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

                    {
                        let gcol = (rx as usize / bloom::STEP + 1).min(bloom.w - 1);
                        let gl = bloom.at(glow_row, gcol);
                        r += gl[0] * bloom::STRENGTH;
                        g += gl[1] * bloom::STRENGTH;
                        b += gl[2] * bloom::STRENGTH;
                    }

                    if flash > 0.0 {
                        r += flash * 80.0;
                        g += flash * 86.0;
                        b += flash * 104.0;
                    }

                    let vig = 1.0 - 0.30 * (vig_col[rx as usize] * 0.8 + ny2 * 0.5);
                    r *= vig;
                    g *= vig;
                    b *= vig;

                    buf[o] = clamp_u8(r);
                    buf[o + 1] = clamp_u8(g);
                    buf[o + 2] = clamp_u8(b);
                } else {

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

            let c = col_row[xr];
            let n = noise(x, y);
            let heat_v = heat_row[xr] as f32;

            let mut r = c[0] as f32;
            let mut g = c[1] as f32;
            let mut b = c[2] as f32;

            let depth = (y as f32 - ground[xr] as f32).max(0.0);
            let depth_fade = (1.0 - depth * 0.0032).clamp(0.62, 1.0);
            let shade = depth_fade * light_f * (0.94 + n * 0.12) * cloud_shadow[rx as usize];
            r *= shade;
            g *= shade;
            b *= shade;

            if depth > 26.0 {
                let zt = &DEPTH_TINT[(depth as usize / 8).min(31)];
                r *= zt[0];
                g *= zt[1];
                b *= zt[2];
            }

            r *= season_r;
            g *= season_g;
            b *= season_b;

            if season == 3 && depth < 3.0 {
                let s = (1.0 - depth * (1.0 / 3.0)) * 0.6;
                r += (232.0 - r) * s;
                g += (238.0 - g) * s;
                b += (248.0 - b) * s;
            }

            let fade = (1.0 - depth * (1.0 / 60.0)).clamp(0.0, 1.0);
            r += bt[0] * fade;
            g += bt[1] * fade;
            b += bt[2] * fade;

            if haze > 0.0 {
                let veiling = haze * (1.0 - depth * (1.0 / 90.0)).clamp(0.25, 1.0);
                r += (haze_col[0] - r) * veiling;
                g += (haze_col[1] - g) * veiling;
                b += (haze_col[2] - b) * veiling;
            }

            let fx = &fx[e as usize];
            match fx.kind {
                Fx::None => {}
                Fx::Wave => {
                    let wave = ((x as f32 * 0.17) + (y as f32 * 0.09) + time * 1.6).sin() * 0.5 + 0.5;
                    r += wave * fx.color[0];
                    g += wave * fx.color[1];
                    b += wave * fx.color[2];

                    let sky = sky_base[rx as usize];
                    r += sky[0] * 0.02;
                    g += sky[1] * 0.02;
                    b += sky[2] * 0.03;

                    let c1 = ((x as f32 * 0.21 + time * 1.7).sin()
                        + (y as f32 * 0.27 - time * 1.1).sin())
                        * 0.5;
                    let caustic = (c1 * c1 * 2.0).max(0.0) * 24.0;
                    r += caustic * 0.7;
                    g += caustic * 0.92;
                    b += caustic;

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

                    let v = (n - 0.5) * 26.0;
                    r = (r + fx.color[0] * v / 26.0).max(0.0);
                    g = (g + fx.color[1] * v / 26.0).max(0.0);
                    b = (b + fx.color[2] * v / 26.0).max(0.0);
                }
                Fx::Glint => {
                    b += 10.0;

                    let glint = ((time * 0.9 + n * 6.0).sin() * 0.5 + 0.5) * 8.0;
                    r += glint * fx.color[0];
                    g += glint * fx.color[1];
                    b += glint * fx.color[2];
                }
                Fx::Grain => {

                    r += fx.color[0] * n;
                    g += fx.color[1] * n;
                }
                Fx::Sheen => {

                    let sheen =
                        ((time * 0.5 + x as f32 * 0.05 + y as f32 * 0.03).sin() * 0.5 + 0.5) * 10.0;
                    r += sheen * fx.color[0];
                    g += sheen * fx.color[1];
                    b += sheen * fx.color[2];
                }
                Fx::Rot => {

                    let beat = (time * fx.speed + n * 7.0).sin() * 0.5 + 0.5;
                    let rot = 0.35 + beat * 0.65;
                    r += fx.color[0] * rot;
                    g += fx.color[1] * rot;
                    b += fx.color[2] * rot;

                    r *= 0.86;
                    g *= 0.9;
                    b *= 0.88;
                }
                Fx::Foul => {

                    let shimmer = ((time * fx.speed + x as f32 * 0.19 - y as f32 * 0.11).sin()
                        * 0.5
                        + 0.5)
                        * fx.gain;
                    r += fx.color[0] * shimmer;
                    g += fx.color[1] * shimmer;
                    b += fx.color[2] * shimmer;
                }
                Fx::Strata => {

                    let amt = row_band * 0.9 + n * 0.4;
                    r += fx.color[0] * amt;
                    g += fx.color[1] * amt;
                    b += fx.color[2] * amt;
                }
                Fx::Vein => {

                    let twinkle = ((time * 1.1 + n * 9.0).sin() * 0.5 + 0.5) * 0.7
                        + (n * 0.6);
                    let amt = twinkle * fx.gain;
                    r += fx.color[0] * amt;
                    g += fx.color[1] * amt;
                    b += fx.color[2] * amt;
                }
            }

            if !light_row.is_empty() {
                let lf = light_row[light_col];
                r += lf[0];
                g += lf[1];
                b += lf[2];
            }

            if depth < 26.0 {
                let shaft = sun_shaft[rx as usize] * (1.0 - depth * (1.0 / 26.0));
                r += shaft * 30.0;
                g += shaft * 27.0;
                b += shaft * 17.0;
            }

            if shimmer && depth < 9.0 {
                let sh = ((time * 3.2 + x as f32 * 0.21 + y as f32 * 0.13).sin()) * 6.0
                    * (1.0 - depth * (1.0 / 9.0));
                r += sh;
                g += sh * 0.8;
                b += sh * 0.35;
            }

            if flash > 0.0 {
                r += flash * 110.0;
                g += flash * 116.0;
                b += flash * 132.0;
            }

            {
                let gcol = (rx as usize / bloom::STEP + 1).min(bloom.w - 1);
                let gl = bloom.at(glow_row, gcol);
                r += gl[0] * bloom::STRENGTH;
                g += gl[1] * bloom::STRENGTH;
                b += gl[2] * bloom::STRENGTH;
            }

            let vig = 1.0 - 0.30 * (vig_col[rx as usize] * 0.8 + ny2 * 0.5);
            r *= vig;
            g *= vig;
            b *= vig;

            buf[o] = clamp_u8(r);
            buf[o + 1] = clamp_u8(g);
            buf[o + 2] = clamp_u8(b);
            buf[o + 3] = 255;
        }
    }
}

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

    (hash2(x, y) >> 8) as f32 / 16_777_216.0
}

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
