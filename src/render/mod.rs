pub mod bloom;
pub mod fx;
pub mod light;
pub mod sprites;
pub mod textures;
mod gpu;

pub use gpu::{GpuRenderer, Place};

use crate::world::{World, Weather};
use bloom::Bloom;
use fx::{fx_table, FxEntry};
use light::LightField;

/// What the world looks like at this moment: where the clock has got
/// to, how far round the day it is, what the season is and what the
/// weather is doing. These belong together, so they travel in one struct
/// rather than as a list of arguments.
#[derive(Clone, Copy)]
pub struct Clock {
    pub time: f32,
    pub day_night: f32,
    pub season: u8,
    pub weather: Weather,
}

/// The window of the world that is being drawn.
#[derive(Clone, Copy)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Everything the renderer works out before it paints a single pixel:
/// the light, the bloom, the sky and the weather's veils.
///
/// The renderer on the processor and the renderer on the graphics card
/// both read these, and they are worked out in one place, so the two
/// renderers cannot drift apart.
pub struct Tables {
    pub light: LightField,

    pub cloud_row: Vec<f32>,

    pub light_rows: Vec<[f32; 3]>,

    pub fx: [FxEntry; fx::FX_TABLE_LEN],

    pub aurora_col: Vec<f32>,

    pub biome_delta: Vec<[f32; 3]>,

    pub sky_base: Vec<[f32; 3]>,

    pub sky_slope: Vec<[f32; 3]>,

    pub cloud_shadow: Vec<f32>,

    pub sun_shaft: Vec<f32>,

    pub vig_col: Vec<f32>,

    pub bloom: Bloom,

    /// The lamps of the region: where they are and how bright they
    /// are. The card uses them to light the face of a cell from the
    /// side the nearest lamp is on.
    pub emitters: Vec<Emitter>,
}

/// A lamp in the world.
#[derive(Clone, Copy)]
pub struct Emitter {
    pub x: i32,
    pub y: i32,
    pub power: f32,
}

/// The state of the sky while a picture is painted: how much light
/// there is, what the season and the weather do to it, where the sun
/// is and whether the storm is flashing.
pub(crate) struct SkyState {
    pub day: f32,
    pub light_f: f32,
    pub season: u8,
    pub season_r: f32,
    pub season_g: f32,
    pub season_b: f32,
    pub haze: f32,
    pub haze_col: [f32; 3],
    pub flash: f32,
    pub shimmer: bool,
    pub aurora: bool,
    pub sun_x: i32,
    pub sun_y: i32,
    /// Where the moon is, how much of it is lit and how much of it is
    /// in the sky.
    pub moon_x: i32,
    pub moon_y: i32,
    pub moon_phase: f32,
    /// How much of the sun is in the sky, and how much of the moon.
    pub sun_seen: f32,
    pub moon_seen: f32,
    pub gw: usize,
    pub gh: usize,
    /// How much cloud there is, and the colour of it. These are the same
    /// for every pixel of the picture, so they are worked out once.
    pub cloud_cover: f32,
    pub cloud_cr: f32,
    pub cloud_cg: f32,
    pub cloud_cb: f32,
}

/// Work out the state of the sky for this picture.
pub(crate) fn sky_state(world_w: usize, world_h: usize, clock: &Clock) -> SkyState {
    let time = clock.time;
    let day_night = clock.day_night;
    let weather = clock.weather;

    let day = 0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos();
    let mut light_level = 0.34 + 0.66 * day;

    let (weather_dim, weather_haze) = match weather {
        Weather::Clear => (0.0f32, 0.0f32),
        Weather::Rain => (0.14, 0.18),
        Weather::Storm => (0.30, 0.26),
        Weather::Snow => (0.06, 0.35),
        Weather::Meteor => (0.10, 0.12),
        Weather::Fog => (0.18, 0.55),
        Weather::Heatwave => (0.04, 0.16),
        Weather::Sandstorm => (0.22, 0.42),
    };
    light_level = (light_level * (1.0 - weather_dim)).clamp(0.08, 1.0);

    let flash = if weather == Weather::Storm {
        let flash_phase = (time * 0.83).fract();
        if flash_phase < 0.035 {
            1.0 - flash_phase / 0.035
        } else {
            0.0
        }
    } else {
        0.0
    };

    let shimmer = weather == Weather::Heatwave;

    let (sr, sg, sb) = match clock.season {
        0 => (0.94f32, 1.0f32, 0.95f32),
        1 => (1.0, 0.99, 0.92),
        2 => (1.0, 0.93, 0.82),
        _ => (0.85, 0.92, 1.0),
    };

    let haze_col = match weather {
        Weather::Fog => [196.0f32, 202.0, 205.0],
        Weather::Snow => [235.0, 240.0, 250.0],
        Weather::Storm => [150.0, 158.0, 172.0],
        Weather::Rain => [168.0, 176.0, 190.0],
        Weather::Sandstorm => [214.0, 186.0, 132.0],
        _ => [200.0, 200.0, 200.0],
    };

    let sun_height = (1.0 - (day_night - 0.5).abs() * 2.0).max(0.0);

    // The moon travels on the same road as the sun, half a day behind
    // it, so the two of them cross the sky in the same direction. One
    // of them is up and the other is down, so they are never seen
    // together.
    let moon_t = (day_night + 0.5).rem_euclid(1.0);
    let moon_height = (1.0 - (moon_t - 0.5).abs() * 2.0).max(0.0);
    let moon_x = (moon_t * world_w as f32) as i32;
    let moon_y = (world_h as f32 * (0.62 - 0.55 * moon_height)) as i32;
    let sun_seen = smoothstep(0.52, 0.70, sun_height);
    let moon_seen = smoothstep(0.52, 0.70, moon_height);

    // The moon turns over the month. It starts as a full moon, so
    // that there is something to look at in the first night.
    let moon_phase = (clock.time / (120.0 * 29.0) + 0.36).fract();

    SkyState {
        day,
        light_f: 0.62 + 0.38 * light_level,
        season: clock.season,
        season_r: 0.96 + 0.04 * sr,
        season_g: 0.96 + 0.04 * sg,
        season_b: 0.96 + 0.04 * sb,
        haze: weather_haze,
        haze_col,
        flash,
        shimmer,
        aurora: day < 0.32,
        sun_x: (day_night * world_w as f32) as i32,
        sun_y: (world_h as f32 * (0.62 - 0.55 * sun_height)) as i32,
        moon_x,
        moon_y,
        moon_phase,
        sun_seen,
        moon_seen,
        gw: (world_w as f32 / GSX) as usize + 2,
        gh: (world_h as f32 / GSY) as usize + 2,
        cloud_cover: 0.20 + 0.32 * day + weather_dim * 0.9,
        cloud_cr: 206.0 - weather_dim * 90.0,
        cloud_cg: 214.0 - weather_dim * 90.0,
        cloud_cb: 232.0 - weather_dim * 60.0,
    }
}

/// The window of the light grid: where it starts in the world and how
/// far it reaches.
pub(crate) struct LightGrid {
    pub gx0: i32,
    pub gy0: i32,
    pub gx1: i32,
    pub gy1: i32,
}

/// The size of the grid the cloud is sampled on.
fn cloud_grid(w: usize, h: usize) -> (usize, usize) {
    ((w as f32 / CSX) as usize + 2, (h as f32 / CSY) as usize + 2)
}

/// Read the cloud cover at a point, with the corners of the cell it
/// falls in.
pub(crate) fn sample_cloud(
    cloud_row: &[f32],
    gw: usize,
    gx0: usize,
    gy0: usize,
    tx: f32,
    ty: f32,
) -> f32 {
    let base = gy0 * gw + gx0;
    let c00 = cloud_row[base];
    let c10 = cloud_row[base + 1];
    let c01 = cloud_row[base + gw];
    let c11 = cloud_row[base + gw + 1];
    (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty) + (c01 * (1.0 - tx) + c11 * tx) * ty
}

impl Tables {
    #[must_use]
    pub fn new() -> Self {
        Tables {
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
            emitters: Vec::new(),
        }
    }

    /// Work out everything the picture needs, and return the size of the
    /// light grid and where it starts.
    pub fn build(&mut self, world: &World, region: Region, clock: &Clock) -> (usize, usize, i32, i32) {
        let x0 = region.x;
        let y0 = region.y;
        let rw = region.w;
        let rh = region.h;
        let w = world.w;
        let h = world.h;

        let gx0 = x0.div_euclid(SCALE) - MARGIN;
        let gy0 = y0.div_euclid(SCALE) - MARGIN;
        let gx1 = (x0 + rw).div_euclid(SCALE) + MARGIN;
        let gy1 = (y0 + rh).div_euclid(SCALE) + MARGIN;
        let lw = (gx1 - gx0 + 1) as usize;
        let lh = (gy1 - gy0 + 1) as usize;

        self.light.resize(lw, lh);
        self.light.clear();
        self.bloom.resize(rw as usize, rh as usize);

        self.scan_lights(world, region, &LightGrid { gx0, gy0, gx1, gy1 });
        self.light.blur(2);
        self.light.finalize();
        self.copy_light_rows();

        self.build_cloud_row(w, h, clock.time);
        self.build_aurora(x0, rw, clock.time);
        self.build_cloud_shadows(world, x0, rw, w, h);
        self.build_vignette(rw);
        self.build_sky(world, x0, rw, w, clock);
        self.build_sun_shafts(world, x0, rw, w, h, clock);

        self.bloom.blur();

        (lw, lh, gx0, gy0)
    }

    /// Walk the window of the world and put the light and the bloom of
    /// every cell that gives off light into the two fields.
    ///
    /// The lamps are also put in a list, so that every cell can be
    /// told which lamp is the nearest one to it. That is the light
    /// that falls on the face of the cell.
    fn scan_lights(&mut self, world: &World, region: Region, grid: &LightGrid) {
        let w = world.w;
        let h = world.h;
        let sx0 = (grid.gx0 * SCALE).max(0);
        let sy0 = (grid.gy0 * SCALE).max(0);
        let sx1 = ((grid.gx1 + 1) * SCALE).min(w as i32);
        let sy1 = ((grid.gy1 + 1) * SCALE).min(h as i32);
        self.emitters.clear();
        for y in sy0..sy1 {
            let row = y as usize * w;
            let ly = (y.div_euclid(SCALE) - grid.gy0) as usize;
            for x in sx0..sx1 {
                let k = world.kind[row + x as usize];
                if world.emissive[k as usize] {
                    let lx = (x.div_euclid(SCALE) - grid.gx0) as usize;
                    self.light.add(lx, ly, world.emission[k as usize]);

                    let e = world.emission[k as usize];
                    self.bloom.add(
                        (x - region.x) as usize,
                        (y - region.y) as usize,
                        e[0],
                        e[1],
                        e[2],
                    );

                    let power = e[0].max(e[1]).max(e[2]);
                    if power > 0.0 {
                        self.emitters.push(Emitter {
                            x,
                            y,
                            power,
                        });
                    }
                }
            }
        }
    }

    /// Copy the light field into the rows the pixel loop reads.
    fn copy_light_rows(&mut self) {
        if self.light_rows.len() != self.light.buf.len() {
            self.light_rows.clear();
            self.light_rows.resize(self.light.buf.len(), [0.0; 3]);
        }
        self.light_rows.copy_from_slice(&self.light.buf);
    }

    /// Work out the cloud cover of the whole world.
    fn build_cloud_row(&mut self, w: usize, h: usize, time: f32) {
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
    }

    /// Work out where the aurora hangs for each column of the window.
    fn build_aurora(&mut self, x0: i32, rw: i32, time: f32) {
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
    }

    /// Work out how much of the sky each column loses to the cloud.
    fn build_cloud_shadows(&mut self, world: &World, x0: i32, rw: i32, w: usize, h: usize) {
        if self.cloud_shadow.len() != rw as usize {
            self.cloud_shadow.clear();
            self.cloud_shadow.resize(rw as usize, 1.0);
        }
        let (cgw, cgh) = cloud_grid(w, h);
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1);
            let gy = (world.ground[x as usize] as f32 * 0.35) / CSY;
            let gx = x as f32 / CSX;
            let gx0 = (gx.floor() as usize).min(cgw - 2);
            let gy0 = (gy.floor() as usize).min(cgh - 2);
            let tx = gx - gx0 as f32;
            let ty = gy - gy0 as f32;
            let cv = sample_cloud(&self.cloud_row, cgw, gx0, gy0, tx, ty);

            self.cloud_shadow[i] = 1.0 - smoothstep(0.58, 0.85, cv) * 0.16;
        }
    }

    /// Work out how dark the edges of the picture are.
    fn build_vignette(&mut self, rw: i32) {
        if self.vig_col.len() != rw as usize {
            self.vig_col.clear();
            self.vig_col.resize(rw as usize, 0.0);
        }
        for i in 0..rw as usize {
            let nx = (i as f32 / rw.max(1) as f32 - 0.5) * 2.0;
            self.vig_col[i] = nx * nx;
        }
    }

    /// Work out the colour of the sky in each column of the window.
    fn build_sky(&mut self, world: &World, x0: i32, rw: i32, w: usize, clock: &Clock) {
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
        let day = 0.5 - 0.5 * (clock.day_night * std::f32::consts::TAU).cos();
        let dim = 0.45 + 0.55 * day;
        let (sr, sg, sb) = match clock.season {
            0 => (0.94f32, 1.0f32, 0.95f32),
            1 => (1.0, 0.99, 0.92),
            2 => (1.0, 0.93, 0.82),
            _ => (0.85, 0.92, 1.0),
        };
        let weather_sky = match clock.weather {
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

            let (veil, vr, vg, vb) = match clock.weather {
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
    }

    /// Work out where the sun comes through the cloud in each column.
    fn build_sun_shafts(&mut self, world: &World, x0: i32, rw: i32, w: usize, h: usize, clock: &Clock) {
        if self.sun_shaft.len() != rw as usize {
            self.sun_shaft.clear();
            self.sun_shaft.resize(rw as usize, 0.0);
        }

        let day = 0.5 - 0.5 * (clock.day_night * std::f32::consts::TAU).cos();
        let weather_dim: f32 = match clock.weather {
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
        let (cgw, cgh) = cloud_grid(w, h);
        for i in 0..rw as usize {
            let x = (x0 + i as i32).clamp(0, w as i32 - 1);

            let gx = x as f32 / SSX + 0.55;
            let gy = (world.ground[x as usize] as f32 * 0.35) / SSY;
            let gx0 = (gx.floor() as usize).min(cgw - 2);
            let gy0 = (gy.floor() as usize).min(cgh - 2);
            let tx = gx - gx0 as f32;
            let ty = gy - gy0 as f32;
            let cv = sample_cloud(&self.cloud_row, cgw, gx0, gy0, tx, ty);

            self.sun_shaft[i] = smoothstep(0.30, 0.56, cv) * shaft_light;
        }
    }
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub(crate) fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

#[inline]
pub(crate) fn noise(x: i32, y: i32) -> f32 {
    (hash2(x, y) >> 8) as f32 / 16_777_216.0
}

#[inline]
pub(crate) fn vnoise(x: f32, y: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = x - xi;
    let yf = y - yi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let ix = xi as i32;
    let iy = yi as i32;
    let c00 = noise(ix, iy);
    let c10 = noise(ix + 1, iy);
    let c01 = noise(ix, iy + 1);
    let c11 = noise(ix + 1, iy + 1);
    (c00 * (1.0 - u) + c10 * u) * (1.0 - v) + (c01 * (1.0 - u) + c11 * u) * v
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
pub(crate) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(crate) const SCALE: i32 = light::LIGHT_SCALE as i32;
pub(crate) const MARGIN: i32 = 3;
pub(crate) const GSX: f32 = 30.0;
pub(crate) const GSY: f32 = 13.0;
pub(crate) const CSX: f32 = 30.0;
pub(crate) const CSY: f32 = 13.0;
pub(crate) const SSX: f32 = 30.0;
pub(crate) const SSY: f32 = 13.0;
