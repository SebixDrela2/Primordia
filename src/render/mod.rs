pub mod bloom;
pub mod fx;
pub mod light;
mod paint;

use crate::creature::CreatureSystem;
use crate::world::{World, Weather};
use bloom::Bloom;
use fx::{fx_table, FxEntry};
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

/// The tables the pixel loop reads while it paints: the per-element
/// effects, the light field, the sky and the weather's veils. They are
/// the renderer's own tables, so they travel together.
struct PaintTables<'a> {
    cloud_row: &'a [f32],
    fx: &'a [FxEntry; fx::FX_TABLE_LEN],
    light_rows: &'a [[f32; 3]],
    aurora_col: &'a [f32],
    biome_delta: &'a [[f32; 3]],
    cloud_shadow: &'a [f32],
    sun_shaft: &'a [f32],
    vig_col: &'a [f32],
    bloom: &'a Bloom,
    sky_base: &'a [[f32; 3]],
    sky_slope: &'a [[f32; 3]],
}

/// The geometry of the picture: the window of the world, the size of the
/// light grid and where that grid starts in the world.
struct PaintGeometry {
    region: Region,
    light_w: usize,
    light_h: usize,
    light_x: i32,
    light_y: i32,
}

/// The window of the light grid: where it starts in the world and how
/// far it reaches.
struct LightGrid {
    gx0: i32,
    gy0: i32,
    gx1: i32,
    gy1: i32,
}

/// The size of the grid the cloud is sampled on.
fn cloud_grid(w: usize, h: usize) -> (usize, usize) {
    ((w as f32 / CSX) as usize + 2, (h as f32 / CSY) as usize + 2)
}

/// Read the cloud cover at a point, with the corners of the cell it
/// falls in.
fn sample_cloud(cloud_row: &[f32], gw: usize, gx0: usize, gy0: usize, tx: f32, ty: f32) -> f32 {
    let base = gy0 * gw + gx0;
    let c00 = cloud_row[base];
    let c10 = cloud_row[base + 1];
    let c01 = cloud_row[base + gw];
    let c11 = cloud_row[base + gw + 1];
    (c00 * (1.0 - tx) + c10 * tx) * (1.0 - ty) + (c01 * (1.0 - tx) + c11 * tx) * ty
}

impl Renderer {
    #[must_use]
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
        clock: Clock,
    ) {
        self.render_region(
            world,
            creatures,
            buf,
            Region {
                x: 0,
                y: 0,
                w: world.w as i32,
                h: world.h as i32,
            },
            clock,
        );
    }

    pub fn render_region(
        &mut self,
        world: &World,
        _creatures: &CreatureSystem,
        buf: &mut [u8],
        region: Region,
        clock: Clock,
    ) {
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
        self.build_sky(world, x0, rw, w, &clock);
        self.build_sun_shafts(world, x0, rw, w, h, &clock);

        self.bloom.blur();

        let tables = PaintTables {
            cloud_row: &self.cloud_row,
            fx: &self.fx,
            light_rows: &self.light_rows,
            aurora_col: &self.aurora_col,
            biome_delta: &self.biome_delta,
            cloud_shadow: &self.cloud_shadow,
            sun_shaft: &self.sun_shaft,
            vig_col: &self.vig_col,
            bloom: &self.bloom,
            sky_base: &self.sky_base,
            sky_slope: &self.sky_slope,
        };
        let geometry = PaintGeometry {
            region,
            light_w: lw,
            light_h: lh,
            light_x: gx0,
            light_y: gy0,
        };

        paint::paint(world, buf, &tables, &geometry, &clock);
    }

    /// Walk the window of the world and put the light and the bloom of
    /// every cell that gives off light into the two fields.
    fn scan_lights(&mut self, world: &World, region: Region, grid: &LightGrid) {
        let w = world.w;
        let h = world.h;
        let sx0 = (grid.gx0 * SCALE).max(0);
        let sy0 = (grid.gy0 * SCALE).max(0);
        let sx1 = ((grid.gx1 + 1) * SCALE).min(w as i32);
        let sy1 = ((grid.gy1 + 1) * SCALE).min(h as i32);
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

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
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
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn clamp_u8(v: f32) -> u8 {
    // A float to u8 cast in Rust saturates, so it already clamps to
    // 0..=255. The explicit clamp was a second pair of comparisons on
    // every channel of every pixel.
    v as u8
}

const SCALE: i32 = light::LIGHT_SCALE as i32;
const MARGIN: i32 = 3;
const GSX: f32 = 30.0;
const GSY: f32 = 13.0;
const CSX: f32 = 30.0;
const CSY: f32 = 13.0;
const SSX: f32 = 30.0;
const SSY: f32 = 13.0;