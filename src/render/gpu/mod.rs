//! The renderer that paints the picture on the graphics card.
//!
//! It does the same work as the renderer on the processor, but the
//! per-pixel loop is a shader: the world, the light, the bloom and the
//! sky are handed to the card as textures, and one fragment does the
//! colour of a pixel at a time.
//!
//! The floats that go into the textures are put in as their raw bits,
//! so that the card has the very same numbers the processor had. A
//! float that has been through eight bits would not.

use macroquad::prelude::*;
use macroquad::window::miniquad::{ShaderSource, UniformDesc, UniformType};

use super::{noise, sample_cloud, sky_state, smoothstep, Clock, Region, SkyState, Tables, GSX, GSY};
use crate::creature::CreatureSystem;
use crate::render::fx::{self, Fx};
use crate::world::{World, DEPTH_TINT};

const VERTEX: &str = r"#version 400 core
layout(location = 0) in vec3 position;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
}
";

const FRAGMENT: &str = include_str!("shader.glsl");

/// Where the picture goes on the screen.
#[derive(Clone, Copy)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The renderer that paints the picture on the graphics card.
pub struct GpuRenderer {
    material: Material,
    tables: Tables,
    cell: Tex,
    cols: Tex,
    light: Tex,
    bloom: Tex,
    cloudamt: Tex,
    rows: Tex,
    fx: Tex,
    depth: Tex,
    /// The sines that the element's effects need, cell by cell.
    sins: Tex,
    /// The sine of the heatwave's shimmer, cell by cell.
    shim: Tex,
    /// The amount of cloud at every anchor of every row.
    cloud_amt: Vec<f32>,
    /// The cells whose sine was written last time, so that it can be
    /// cleared again without wiping the whole texture.
    dirty: Vec<usize>,
}

/// A texture the shader reads, with the bytes that are going into it.
struct Tex {
    image: Image,
    texture: Option<Texture2D>,
}

impl Tex {
    fn new() -> Self {
        Tex {
            image: Image {
                bytes: Vec::new(),
                width: 0,
                height: 0,
            },
            texture: None,
        }
    }

    /// Make the texture the right size, and empty it. It says whether
    /// the texture was made afresh.
    fn fit(&mut self, width: usize, height: usize) -> bool {
        if self.image.width as usize != width || self.image.height as usize != height {
            self.image = Image {
                bytes: vec![0; width * height * 4],
                width: width as u16,
                height: height as u16,
            };
            self.texture = None;
            return true;
        }
        false
    }

    /// Put the bytes on the card.
    fn upload(&mut self) {
        match self.texture.as_ref() {
            None => {
                let texture =
                    Texture2D::from_rgba8(self.image.width, self.image.height, &self.image.bytes);
                texture.set_filter(FilterMode::Nearest);
                self.texture = Some(texture);
            }
            Some(texture) => texture.update(&self.image),
        }
    }
}

/// Put a float into four bytes, as its raw bits.
fn put_float(bytes: &mut [u8], at: usize, value: f32) {
    bytes[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

impl GpuRenderer {
    /// Make the renderer and load its shader.
    ///
    /// # Panics
    ///
    /// It panics when the shader of the card does not compile.
    #[must_use]
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                // The Model, Projection and _Time uniforms are put in
                // by macroquad itself, so they are not listed here.
                uniforms: vec![
                    UniformDesc::new("u_region", UniformType::Float4),
                    UniformDesc::new("u_world", UniformType::Float4),
                    UniformDesc::new("u_sky", UniformType::Float4),
                    UniformDesc::new("u_sky2", UniformType::Float4),
                    UniformDesc::new("u_sky3", UniformType::Float4),
                    UniformDesc::new("u_sky4", UniformType::Float4),
                    UniformDesc::new("u_grid", UniformType::Float4),
                    UniformDesc::new("u_place", UniformType::Float4),
                    UniformDesc::new("u_counts", UniformType::Float4),
                    UniformDesc::new("u_sun", UniformType::Float4),
                    UniformDesc::new("u_bloom", UniformType::Float4),
                ],
                textures: vec![
                    "t_cell".to_owned(),
                    "t_cols".to_owned(),
                    "t_light".to_owned(),
                    "t_bloom".to_owned(),
                    "t_cloudamt".to_owned(),
                    "t_rows".to_owned(),
                    "t_fx".to_owned(),
                    "t_depth".to_owned(),
                    "t_sin".to_owned(),
                    "t_shim".to_owned(),
                ],
                ..Default::default()
            },
        )
        .expect("the shader of the graphics card renderer should compile");

        GpuRenderer {
            material,
            tables: Tables::new(),
            cell: Tex::new(),
            cols: Tex::new(),
            light: Tex::new(),
            bloom: Tex::new(),
            cloudamt: Tex::new(),
            rows: Tex::new(),
            fx: Tex::new(),
            depth: Tex::new(),
            sins: Tex::new(),
            shim: Tex::new(),
            cloud_amt: Vec::new(),
            dirty: Vec::new(),
        }
    }

    /// Paint the window of the world and draw it on the screen where it
    /// belongs.
    pub fn render_region(
        &mut self,
        world: &World,
        _creatures: &CreatureSystem,
        region: Region,
        clock: Clock,
        place: Place,
    ) {
        let (lw, lh, gx0, gy0) = self.tables.build(world, region, &clock);
        let sky = sky_state(world.w, world.h, &clock);

        self.put_cells(world, region);
        self.put_columns(world, region, &clock);
        self.put_light(lw, lh);
        self.put_bloom();
        self.put_cloud_amounts(world, region, &sky);
        self.put_rows(region, &clock);
        self.put_fx();
        self.put_depth();
        self.put_sins(world, region, &clock, &sky);

        self.cell.upload();
        self.cols.upload();
        self.light.upload();
        self.bloom.upload();
        self.cloudamt.upload();
        self.rows.upload();
        self.fx.upload();
        self.depth.upload();
        self.sins.upload();
        self.shim.upload();

        self.draw(world, region, lw, lh, gx0, gy0, &sky, &clock, place);
    }

    /// The cells of the region, with a border of one cell around them
    /// so that the shader can see the neighbours.
    fn put_cells(&mut self, world: &World, region: Region) {
        let rw = region.w as usize;
        let rh = region.h as usize;
        self.cell.fit((rw + 2) * 2, rh + 2);

        let w = world.w;
        let h = world.h;
        let bytes = &mut self.cell.image.bytes;
        for cy in 0..rh + 2 {
            let wy = region.y - 1 + cy as i32;
            let row = if wy < 0 || wy >= h as i32 {
                None
            } else {
                Some(wy as usize * w)
            };
            for cx in 0..rw + 2 {
                let wx = region.x - 1 + cx as i32;
                let at = (cy * (rw + 2) * 2 + cx * 2) * 4;
                if row.is_none() || wx < 0 || wx >= w as i32 {
                    bytes[at] = 0;
                    bytes[at + 1] = 0;
                    bytes[at + 2] = 0;
                    bytes[at + 3] = 0;
                    bytes[at + 4] = 0;
                    bytes[at + 5] = 0;
                    bytes[at + 6] = 0;
                    bytes[at + 7] = 0;
                    continue;
                }
                let index = row.unwrap() + wx as usize;
                bytes[at] = world.kind[index];
                bytes[at + 1] = world.heat[index];
                bytes[at + 2] = 0;
                bytes[at + 3] = 0;
                let colour = world.col[index];
                bytes[at + 4] = colour[0];
                bytes[at + 5] = colour[1];
                bytes[at + 6] = colour[2];
                bytes[at + 7] = 0;
            }
        }
    }

    /// The tables of every column of the region.
    fn put_columns(&mut self, world: &World, region: Region, clock: &Clock) {
        let rw = region.w as usize;
        self.cols.fit(rw, 17);
        let bytes = &mut self.cols.image.bytes;

        let w = world.w;
        for rx in 0..rw {
            let wx = region.x + rx as i32;

            let base = self.tables.sky_base[rx];
            let slope = self.tables.sky_slope[rx];
            let delta = self.tables.biome_delta[rx];
            for c in 0..3 {
                put_float(bytes, (c * rw + rx) * 4, base[c]);
                put_float(bytes, ((3 + c) * rw + rx) * 4, slope[c]);
                put_float(bytes, ((6 + c) * rw + rx) * 4, delta[c]);
            }
            put_float(bytes, (9 * rw + rx) * 4, self.tables.aurora_col[rx]);
            put_float(bytes, (10 * rw + rx) * 4, self.tables.cloud_shadow[rx]);
            put_float(bytes, (11 * rw + rx) * 4, self.tables.sun_shaft[rx]);
            put_float(bytes, (12 * rw + rx) * 4, self.tables.vig_col[rx]);

            if wx < 0 || wx >= w as i32 {
                for row in 13..17 {
                    put_float(bytes, (row * rw + rx) * 4, 0.0);
                }
                continue;
            }
            put_float(bytes, (13 * rw + rx) * 4, world.ground[wx as usize] as f32);
            put_float(bytes, (14 * rw + rx) * 4, world.ground_inv[wx as usize]);
            put_float(bytes, (15 * rw + rx) * 4, world.biome[wx as usize] as u8 as f32);
            // The light on the water, down this column.
            put_float(
                bytes,
                (16 * rw + rx) * 4,
                fx::caustic_col_sin(wx, clock.time),
            );
        }
    }

    /// The light field.
    fn put_light(&mut self, lw: usize, lh: usize) {
        self.light.fit(lw, lh * 3);
        let bytes = &mut self.light.image.bytes;
        for ly in 0..lh {
            for lx in 0..lw {
                let cell = self.tables.light_rows[ly * lw + lx];
                for (c, value) in cell.iter().enumerate() {
                    put_float(bytes, ((ly * 3 + c) * lw + lx) * 4, *value);
                }
            }
        }
    }

    /// The bloom.
    fn put_bloom(&mut self) {
        let bloom = &self.tables.bloom;
        self.bloom.fit(bloom.w, bloom.h * 3);
        let bytes = &mut self.bloom.image.bytes;
        for row in 0..bloom.h {
            for col in 0..bloom.w {
                let cell = bloom.at(row, col);
                for (c, value) in cell.iter().enumerate() {
                    put_float(bytes, ((row * 3 + c) * bloom.w + col) * 4, *value);
                }
            }
        }
    }

    /// The amount of cloud at every anchor of every row.
    ///
    /// The cloud is only worked out every third column and then carried
    /// along the row, so the card reads the amount at the anchor.
    fn put_cloud_amounts(&mut self, world: &World, region: Region, sky: &SkyState) {
        let rw = region.w as usize;
        let rh = region.h as usize;
        let anchors = rw / 3 + 1;
        if self.cloud_amt.len() != anchors * rh {
            self.cloud_amt.clear();
            self.cloud_amt.resize(anchors * rh, 0.0);
        }
        self.cloudamt.fit(anchors, rh);

        let gw = sky.gw;
        let gh = sky.gh;
        let w = world.w;
        let h = world.h;
        for ry in 0..rh {
            let row_y = region.y + ry as i32;
            let mut amount = 0.0f32;
            let mut gy0 = 0usize;
            let mut ty = 0.0f32;
            if row_y >= 0 && row_y < h as i32 {
                let fy = row_y as f32 / GSY;
                gy0 = (fy.floor() as usize).min(gh - 2);
                ty = fy - gy0 as f32;
            }
            for k in 0..anchors {
                let rx = k * 3;
                if rx < rw {
                    let col_x = region.x + rx as i32;
                    if row_y >= 0 && row_y < h as i32 && col_x >= 0 && col_x < w as i32 {
                        // The cloud is only worked out again where the
                        // sky is showing. Anywhere else the amount is
                        // carried along the row as it was.
                        let index = row_y as usize * w + col_x as usize;
                        let surf = world.ground[col_x as usize] as f32;
                        if world.kind[index] == crate::elements::Elem::Empty as u8
                            && (row_y as f32) < surf * 0.6
                        {
                            let fx = col_x as f32 / GSX;
                            let gx0 = (fx.floor() as usize).min(gw - 2);
                            let tx = fx - gx0 as f32;
                            let value = sample_cloud(&self.tables.cloud_row, gw, gx0, gy0, tx, ty);
                            amount = smoothstep(0.56, 0.78, value) * sky.cloud_cover;
                        }
                    }
                }
                self.cloud_amt[ry * anchors + k] = amount;
            }
        }

        let bytes = &mut self.cloudamt.image.bytes;
        for i in 0..anchors * rh {
            put_float(bytes, i * 4, self.cloud_amt[i]);
        }
    }

    /// The band of colour that runs across each row, and the light on
    /// the water that runs across it.
    fn put_rows(&mut self, region: Region, clock: &Clock) {
        let rh = region.h as usize;
        self.rows.fit(rh, 2);
        let bytes = &mut self.rows.image.bytes;
        for ry in 0..rh {
            let row_y = region.y + ry as i32;
            let band = ((row_y as f32 * 0.28).sin() * 0.5 + 0.5)
                * ((row_y as f32 * 0.11).sin() * 0.5 + 0.5);
            put_float(bytes, ry * 4, band);
            put_float(bytes, (rh + ry) * 4, fx::caustic_row_sin(row_y, clock.time));
        }
    }

    /// The effect of every element.
    fn put_fx(&mut self) {
        self.fx.fit(256, 6);
        let bytes = &mut self.fx.image.bytes;
        for kind in 0..256 {
            let entry = self.tables.fx[kind];
            put_float(bytes, kind * 4, entry.kind as u8 as f32);
            for c in 0..3 {
                put_float(bytes, ((1 + c) * 256 + kind) * 4, entry.color[c]);
            }
            put_float(bytes, (4 * 256 + kind) * 4, entry.speed);
            put_float(bytes, (5 * 256 + kind) * 4, entry.gain);
        }
    }

    /// The tint of every depth.
    fn put_depth(&mut self) {
        self.depth.fit(32, 3);
        let bytes = &mut self.depth.image.bytes;
        for (z, tint) in DEPTH_TINT.iter().enumerate() {
            for (c, value) in tint.iter().enumerate() {
                put_float(bytes, (c * 32 + z) * 4, *value);
            }
        }
    }

    /// The sines that the element's effects need, cell by cell.
    ///
    /// The sine of the card is not the sine of the processor, so the
    /// processor works these out and hands them over. Only the cells
    /// whose effect needs a sine get one, and only those are cleared
    /// again the next time round.
    fn put_sins(&mut self, world: &World, region: Region, clock: &Clock, sky: &SkyState) {
        let rw = region.w as usize;
        let rh = region.h as usize;
        if self.sins.fit(rw * 2, rh) {
            self.dirty.clear();
        }

        // Clear the cells that were written last time.
        let dirty = std::mem::take(&mut self.dirty);
        {
            let bytes = &mut self.sins.image.bytes;
            for at in &dirty {
                bytes[*at..*at + 8].fill(0);
            }
        }

        let w = world.w;
        let h = world.h;
        let time = clock.time;
        let table = self.tables.fx;
        let bytes = &mut self.sins.image.bytes;
        let dirty = &mut self.dirty;
        for ry in 0..rh {
            let wy = region.y + ry as i32;
            if wy < 0 || wy >= h as i32 {
                continue;
            }
            let row = wy as usize * w;
            for rx in 0..rw {
                let wx = region.x + rx as i32;
                if wx < 0 || wx >= w as i32 {
                    continue;
                }
                let index = row + wx as usize;
                let entry = table[world.kind[index] as usize];                let noise_value = noise(wx, wy);
                let sin = match entry.kind {
                    Fx::Wave => Some(fx::wave_sin(wx, wy, time)),
                    Fx::Flicker => Some(fx::flicker_sin(time, noise_value)),
                    Fx::Heat => Some(fx::heat_sin(time, noise_value)),
                    Fx::Pulse | Fx::Mystic | Fx::Ember => {
                        Some(fx::pulse_sin(time, entry.speed, noise_value))
                    }
                    Fx::Sparkle => Some(fx::sparkle_sin(time, noise_value)),
                    Fx::Glint => Some(fx::glint_sin(time, noise_value)),
                    Fx::Sheen => Some(fx::sheen_sin(time, wx, wy)),
                    Fx::Rot => Some(fx::rot_sin(time, entry.speed, noise_value)),
                    Fx::Foul => Some(fx::foul_sin(time, entry.speed, wx, wy)),
                    Fx::Vein => Some(fx::vein_sin(time, noise_value)),
                    _ => None,
                };
                let foam = if entry.kind == Fx::Wave {
                    Some(fx::foam_sin(wx, wy, time))
                } else {
                    None
                };
                if sin.is_none() && foam.is_none() {
                    continue;
                }
                let at = (ry * rw * 2 + rx * 2) * 4;
                put_float(bytes, at, sin.unwrap_or(0.0));
                put_float(bytes, at + 4, foam.unwrap_or(0.0));
                dirty.push(at);
            }
        }

        // The heatwave shimmers on the ground near the surface. The
        // card does not need the texture at all when there is no
        // heatwave, so a tiny one stands in for it.
        if sky.shimmer {
            self.shim.fit(rw, rh);
            self.shim.image.bytes.fill(0);
            let bytes = &mut self.shim.image.bytes;
            for ry in 0..rh {
                let wy = region.y + ry as i32;
                if wy < 0 || wy >= h as i32 {
                    continue;
                }
                for rx in 0..rw {
                    let wx = region.x + rx as i32;
                    if wx < 0 || wx >= w as i32 {
                        continue;
                    }
                    let depth = (wy as f32 - world.ground[wx as usize] as f32).max(0.0);
                    if depth >= 9.0 {
                        continue;
                    }
                    let at = (ry * rw + rx) * 4;
                    put_float(bytes, at, fx::shimmer_sin(time, wx, wy));
                }
            }
        } else {
            self.shim.fit(1, 1);
        }
    }

    /// Draw the picture.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        world: &World,
        region: Region,
        lw: usize,
        lh: usize,
        gx0: i32,
        gy0: i32,
        sky: &SkyState,
        clock: &Clock,
        place: Place,
    ) {
        self.material.set_texture("t_cell", self.cell.texture.clone().unwrap());
        self.material.set_texture("t_cols", self.cols.texture.clone().unwrap());
        self.material.set_texture("t_light", self.light.texture.clone().unwrap());
        self.material.set_texture("t_bloom", self.bloom.texture.clone().unwrap());
        self.material.set_texture("t_cloudamt", self.cloudamt.texture.clone().unwrap());
        self.material.set_texture("t_rows", self.rows.texture.clone().unwrap());
        self.material.set_texture("t_fx", self.fx.texture.clone().unwrap());
        self.material.set_texture("t_depth", self.depth.texture.clone().unwrap());
        self.material.set_texture("t_sin", self.sins.texture.clone().unwrap());
        self.material.set_texture("t_shim", self.shim.texture.clone().unwrap());

        self.material.set_uniform(
            "u_region",
            [
                region.x as f32,
                region.y as f32,
                region.w as f32,
                region.h as f32,
            ],
        );
        self.material.set_uniform("u_world", [world.w as f32, world.h as f32, 0.0, 0.0]);
        self.material.set_uniform(
            "u_sky",
            [
                sky.day,
                sky.light_f,
                sky.season as f32,
                sky.season_r,
            ],
        );
        self.material.set_uniform(
            "u_sky2",
            [
                sky.season_g,
                sky.season_b,
                sky.haze,
                sky.flash,
            ],
        );
        self.material.set_uniform(
            "u_sky3",
            [
                sky.haze_col[0],
                sky.haze_col[1],
                sky.haze_col[2],
                sky.cloud_cover,
            ],
        );
        self.material.set_uniform(
            "u_sky4",
            [
                sky.cloud_cr,
                sky.cloud_cg,
                sky.cloud_cb,
                if sky.aurora { 1.0 } else { 0.0 },
            ],
        );
        self.material.set_uniform(
            "u_grid",
            [
                gx0 as f32,
                gy0 as f32,
                lw as f32,
                lh as f32,
            ],
        );
        self.material.set_uniform("u_place", [place.x, place.y, place.w, place.h]);
        self.material.set_uniform(
            "u_counts",
            [
                sky.gw as f32,
                sky.gh as f32,
                (region.w as usize / 3 + 1) as f32,
                screen_height(),
            ],
        );
        self.material.set_uniform(
            "u_sun",
            [
                sky.sun_x as f32,
                sky.sun_y as f32,
                clock.time,
                if sky.shimmer { 1.0 } else { 0.0 },
            ],
        );
        self.material.set_uniform(
            "u_bloom",
            [
                self.tables.bloom.w as f32,
                self.tables.bloom.h as f32,
                super::bloom::STRENGTH,
                0.0,
            ],
        );

        gl_use_material(&self.material);
        draw_rectangle(place.x, place.y, place.w, place.h, WHITE);
        gl_use_default_material();
    }
}

impl Default for GpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}
