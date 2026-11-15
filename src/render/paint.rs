use super::bloom;
use super::fx::{Fx, FxEntry};
use super::{
    clamp_u8, is_shore, noise, sample_cloud, smoothstep, Clock, PaintGeometry, PaintTables, GSX,
    GSY,
};
use crate::elements::Elem;
use crate::world::{Biome, World, Weather, DEPTH_TINT};

/// The state of the sky while a picture is painted: how much light
/// there is, what the season and the weather do to it, where the sun
/// is and whether the storm is flashing.
struct SkyState {
    day: f32,
    light_f: f32,
    season: u8,
    season_r: f32,
    season_g: f32,
    season_b: f32,
    haze: f32,
    haze_col: [f32; 3],
    flash: f32,
    shimmer: bool,
    aurora: bool,
    sun_x: i32,
    sun_y: i32,
    is_day: bool,
    gw: usize,
    gh: usize,
    /// How much cloud there is, and the colour of it. These are the same
    /// for every pixel of the picture, so they are worked out once.
    cloud_cover: f32,
    cloud_cr: f32,
    cloud_cg: f32,
    cloud_cb: f32,
}

/// The cloud of one row. The cloud is only worked out every third column
/// and then carried along the row, so the amount of cloud is worked out
/// with it.
#[derive(Clone, Copy)]
struct Cloud {
    value: f32,
    amount: f32,
}

/// The colour of one pixel, before it is written into the picture.
#[derive(Clone, Copy)]
struct Rgb {
    r: f32,
    g: f32,
    b: f32,
}

/// One row of the picture: the row of the world it sits on, the light
/// row that goes with it and the band of colour that runs across it.
struct RowCtx<'a> {
    kind_row: &'a [u8],
    col_row: &'a [[u8; 3]],
    heat_row: &'a [u8],
    light_row: &'a [[f32; 3]],
    row_band: f32,
    ny2: f32,
    glow_row: usize,
    /// Where this row sits in the cloud grid, and how far into the cell
    /// it is. The same for every column of the row, so it is worked out
    /// once when the row is set up.
    cloud_gy0: usize,
    cloud_ty: f32,
}

/// One pixel of the picture: where it is, what is in it, and what the
/// sky is doing.
struct PixelCtx<'a> {
    world: &'a World,
    tables: &'a PaintTables<'a>,
    sky: &'a SkyState,
    row: &'a RowCtx<'a>,
    rx: usize,
    xr: usize,
    x: i32,
    y: i32,
    time: f32,
    light_col: usize,
}

/// Work out the state of the sky for this picture.
fn sky_state(world_w: usize, world_h: usize, clock: &Clock) -> SkyState {
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
        is_day: day > 0.25,
        gw: (world_w as f32 / GSX) as usize + 2,
        gh: (world_h as f32 / GSY) as usize + 2,
        cloud_cover: 0.20 + 0.32 * day + weather_dim * 0.9,
        cloud_cr: 206.0 - weather_dim * 90.0,
        cloud_cg: 214.0 - weather_dim * 90.0,
        cloud_cb: 232.0 - weather_dim * 60.0,
    }
}

/// The row of the world that the picture is painting, and the light row
/// that goes with it.
fn row_context<'a>(
    world: &'a World,
    tables: &'a PaintTables<'a>,
    geom: &PaintGeometry,
    row_y: i32,
    ry: i32,
) -> RowCtx<'a> {
    let world_w = world.w;
    let row_base = row_y as usize * world_w;
    let lw = geom.light_w;
    let lh = geom.light_h;
    let gy0 = geom.light_y;
    let ny = (ry as f32 / geom.region.h.max(1) as f32 - 0.5) * 2.0;

    RowCtx {
        kind_row: &world.kind[row_base..row_base + world_w],
        col_row: &world.col[row_base..row_base + world_w],
        heat_row: &world.heat[row_base..row_base + world_w],
        light_row: {
            let ly = (row_y.div_euclid(4) - gy0).max(0) as usize;
            if ly < lh {
                &tables.light_rows[ly * lw..ly * lw + lw]
            } else {
                &[]
            }
        },
        row_band: ((row_y as f32 * 0.28).sin() * 0.5 + 0.5)
            * ((row_y as f32 * 0.11).sin() * 0.5 + 0.5),
        ny2: ny * ny,
        glow_row: (ry as usize / bloom::STEP).min(tables.bloom.h.saturating_sub(1)),
        // The cloud grid position of this row. It is the same for every
        // column, so it is worked out once here.
        cloud_gy0: 0,
        cloud_ty: 0.0,
    }
}

/// Which column of the light grid a pixel of the picture reads.
fn light_column(row: &RowCtx<'_>, col_x: i32, gx0: i32, lw: usize) -> usize {
    if row.light_row.is_empty() {
        return 0;
    }
    let light_index = (col_x >> 2) - gx0;
    if light_index < 0 {
        0
    } else if light_index as usize >= lw {
        lw - 1
    } else {
        light_index as usize
    }
}

/// The gradient of the sky, with the stars in it and the sun.
fn sky_gradient(p: &PixelCtx<'_>) -> Rgb {
    let sky = p.sky;
    let surf = p.world.ground[p.xr] as f32;
    let sky_depth = (p.y as f32 * p.world.ground_inv[p.xr]).clamp(0.0, 1.0);
    let base = p.tables.sky_base[p.rx];
    let slope = p.tables.sky_slope[p.rx];
    let mut rgb = Rgb {
        r: base[0] + sky_depth * slope[0],
        g: base[1] + sky_depth * slope[1],
        b: base[2] + sky_depth * slope[2],
    };

    if sky.day < 0.38 && (p.y as f32) < surf * 0.7 {
        let sn = noise(p.x, p.y);
        if sn > 0.9968 {
            let star = 90.0 + 110.0 * sn;
            rgb.r = rgb.r.max(star * 0.85);
            rgb.g = rgb.g.max(star * 0.9);
            rgb.b = rgb.b.max(star);
        }
    }

    let sdx = p.x - sky.sun_x;
    let sdy = p.y - sky.sun_y;
    if sdx.abs() < 32 && sdy.abs() < 32 {
        let sd = (sdx * sdx + sdy * sdy) as f32;
        let halo = (1.0 - (sd / 900.0).sqrt()).max(0.0);
        let core = if sd < 36.0 { 1.0 } else { 0.0 };
        let (cr, cg, cb) = if sky.is_day {
            (255.0, 238.0, 190.0)
        } else {
            (200.0, 215.0, 245.0)
        };
        let amt = halo * halo * 0.55 + core;
        rgb.r += (cr - rgb.r) * amt;
        rgb.g += (cg - rgb.g) * amt;
        rgb.b += (cb - rgb.b) * amt;
    }

    rgb
}

/// The aurora, over the tundra.
fn sky_aurora(p: &PixelCtx<'_>, rgb: &mut Rgb) {
    let sky = p.sky;
    if !sky.aurora || p.world.biome[p.x as usize] != Biome::Tundra {
        return;
    }
    let band = p.tables.aurora_col[p.rx] - p.y as f32;
    if band.abs() >= 30.0 {
        return;
    }
    let amt = (1.0 - band.abs() / 30.0) * 0.5 * (1.0 - sky.day / 0.32);
    let (ar, ag, ab) = if band > 0.0 {
        (90.0, 255.0, 170.0)
    } else {
        (150.0, 110.0, 255.0)
    };
    rgb.r += (ar - rgb.r) * amt;
    rgb.g += (ag - rgb.g) * amt;
    rgb.b += (ab - rgb.b) * amt;
}

/// The cloud, which is only worked out every third column and then
/// carried along the row.
fn sky_cloud(p: &PixelCtx<'_>, rgb: &mut Rgb, cloud: &mut Cloud) {
    let sky = p.sky;
    let surf = p.world.ground[p.xr] as f32;
    if (p.y as f32) >= surf * 0.6 {
        return;
    }
    if p.rx.is_multiple_of(3) {
        let fx = p.x as f32 / GSX;
        let gx0 = (fx.floor() as usize).min(sky.gw - 2);
        let tx = fx - gx0 as f32;
        cloud.value = sample_cloud(p.tables.cloud_row, sky.gw, gx0, p.row.cloud_gy0, tx, p.row.cloud_ty);
        cloud.amount = smoothstep(0.56, 0.78, cloud.value) * sky.cloud_cover;
    }

    let amt = cloud.amount;
    if amt > 0.002 {
        rgb.r += (sky.cloud_cr - rgb.r) * amt;
        rgb.g += (sky.cloud_cg - rgb.g) * amt;
        rgb.b += (sky.cloud_cb - rgb.b) * amt;
    }
}

/// Paint the sky above the ground.
fn paint_sky(p: &PixelCtx<'_>, cloud: &mut Cloud) -> Rgb {
    let mut rgb = sky_gradient(p);
    sky_aurora(p, &mut rgb);
    sky_cloud(p, &mut rgb, cloud);
    rgb
}

/// Paint the dark ground under the surface.
fn paint_ground(p: &PixelCtx<'_>) -> Rgb {
    let surf = p.world.ground[p.xr] as f32;
    let depth =
        ((p.y as f32 - surf) / (p.world.h as f32 - surf).max(1.0)).clamp(0.0, 1.0);
    let base = 9.0 - depth * 4.0;
    let warmth = noise(p.x, p.y) * 1.5;
    Rgb {
        r: base * 0.7 + warmth,
        g: base * 0.9 + warmth,
        b: base * 1.2 + warmth,
    }
}

/// The colour of a cell that holds something, before the element's own
/// effect is put on it.
fn shade_cell(p: &PixelCtx<'_>, cell_color: [u8; 3], depth: f32, noise_value: f32) -> Rgb {
    let sky = p.sky;
    let mut rgb = Rgb {
        r: cell_color[0] as f32,
        g: cell_color[1] as f32,
        b: cell_color[2] as f32,
    };

    let depth_fade = (1.0 - depth * 0.0032).clamp(0.62, 1.0);
    let shade =
        depth_fade * sky.light_f * (0.94 + noise_value * 0.12) * p.tables.cloud_shadow[p.rx];
    rgb.r *= shade;
    rgb.g *= shade;
    rgb.b *= shade;

    if depth > 26.0 {
        let zt = &DEPTH_TINT[(depth as usize / 8).min(31)];
        rgb.r *= zt[0];
        rgb.g *= zt[1];
        rgb.b *= zt[2];
    }

    rgb.r *= sky.season_r;
    rgb.g *= sky.season_g;
    rgb.b *= sky.season_b;

    if sky.season == 3 && depth < 3.0 {
        let snow_fade = (1.0 - depth * (1.0 / 3.0)) * 0.6;
        rgb.r += (232.0 - rgb.r) * snow_fade;
        rgb.g += (238.0 - rgb.g) * snow_fade;
        rgb.b += (248.0 - rgb.b) * snow_fade;
    }

    let bt = p.tables.biome_delta[p.rx];
    let fade = (1.0 - depth * (1.0 / 60.0)).clamp(0.0, 1.0);
    rgb.r += bt[0] * fade;
    rgb.g += bt[1] * fade;
    rgb.b += bt[2] * fade;

    if sky.haze > 0.0 {
        let veiling = sky.haze * (1.0 - depth * (1.0 / 90.0)).clamp(0.25, 1.0);
        rgb.r += (sky.haze_col[0] - rgb.r) * veiling;
        rgb.g += (sky.haze_col[1] - rgb.g) * veiling;
        rgb.b += (sky.haze_col[2] - rgb.b) * veiling;
    }

    rgb
}

/// The wave of the water, with the light on it and the foam at the
/// shore.
fn fx_wave(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb) {
    let time = p.time;
    let wave =
        ((p.x as f32 * 0.17) + (p.y as f32 * 0.09) + time * 1.6).sin() * 0.5 + 0.5;
    rgb.r += wave * entry.color[0];
    rgb.g += wave * entry.color[1];
    rgb.b += wave * entry.color[2];

    let sky = p.tables.sky_base[p.rx];
    rgb.r += sky[0] * 0.02;
    rgb.g += sky[1] * 0.02;
    rgb.b += sky[2] * 0.03;

    let c1 = ((p.x as f32 * 0.21 + time * 1.7).sin()
        + (p.y as f32 * 0.27 - time * 1.1).sin())
        * 0.5;
    let caustic = (c1 * c1 * 2.0).max(0.0) * 24.0;
    rgb.r += caustic * 0.7;
    rgb.g += caustic * 0.92;
    rgb.b += caustic;

    if is_shore(
        &p.world.kind,
        p.world.w,
        p.world.h,
        p.x,
        p.y,
        Elem::Empty as u8,
        Elem::Water as u8,
    ) {
        let surf = ((p.x as f32 * 0.31) + (p.y as f32 * 0.19) - time * 2.4).sin() * 0.5 + 0.5;
        let foam = (0.35 + 0.65 * surf) * 46.0;
        rgb.r += foam * 0.9;
        rgb.g += foam * 0.95;
        rgb.b += foam;
    }
}

/// The fire that flickers.
fn fx_flicker(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32) {
    let time = p.time;
    let flicker =
        0.7 + 0.3 * ((time * 11.0 + noise_value * std::f32::consts::TAU).sin() * 0.5 + 0.5);
    rgb.r = rgb.r * 0.9 + entry.color[0] * flicker;
    rgb.g = rgb.g * 0.9 + entry.color[1] * flicker;
    rgb.b = rgb.b * 0.9 + entry.color[2] * flicker;
}

/// The heat that pulses.
fn fx_heat(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32, heat_v: f32) {
    let pulse = 0.5 + 0.5 * ((p.time * 2.1 + noise_value * std::f32::consts::TAU).sin());
    let hot = 40.0 + heat_v * 0.35 + pulse * 38.0;
    rgb.r += hot * entry.color[0];
    rgb.g += hot * entry.color[1];
    rgb.b += hot * entry.color[2];
}

/// The pulse of the magic and the ember.
fn fx_pulse(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32) {
    let pulse =
        0.5 + 0.5 * ((p.time * entry.speed + noise_value * std::f32::consts::TAU).sin());
    rgb.r += entry.color[0] * pulse;
    rgb.g += entry.color[1] * pulse;
    rgb.b += entry.color[2] * pulse;
}

/// The glitter of the crystal.
fn fx_sparkle(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32) {
    let sparkle = ((p.time * 3.0 + noise_value * std::f32::consts::TAU).sin() * 0.5 + 0.5)
        * 26.0
        * noise_value;
    rgb.r += sparkle * entry.color[0];
    rgb.g += sparkle * entry.color[1];
    rgb.b += sparkle * entry.color[2];
}

/// The leaves, the sheen and the grain of the ground.
fn fx_surface(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32) {
    let time = p.time;
    match entry.kind {
        Fx::Foliage => {
            let foliage_amount = (noise_value - 0.5) * 26.0;
            rgb.r = (rgb.r + entry.color[0] * foliage_amount / 26.0).max(0.0);
            rgb.g = (rgb.g + entry.color[1] * foliage_amount / 26.0).max(0.0);
            rgb.b = (rgb.b + entry.color[2] * foliage_amount / 26.0).max(0.0);
        }
        Fx::Glint => {
            rgb.b += 10.0;

            let glint = ((time * 0.9 + noise_value * 6.0).sin() * 0.5 + 0.5) * 8.0;
            rgb.r += glint * entry.color[0];
            rgb.g += glint * entry.color[1];
            rgb.b += glint * entry.color[2];
        }
        Fx::Grain => {
            rgb.r += entry.color[0] * noise_value;
            rgb.g += entry.color[1] * noise_value;
        }
        _ => {
            let sheen = ((time * 0.5 + p.x as f32 * 0.05 + p.y as f32 * 0.03).sin() * 0.5 + 0.5)
                * 10.0;
            rgb.r += sheen * entry.color[0];
            rgb.g += sheen * entry.color[1];
            rgb.b += sheen * entry.color[2];
        }
    }
}

/// The rot, the filth, the bands of rock and the vein of metal.
fn fx_motion(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32) {
    let time = p.time;
    match entry.kind {
        Fx::Rot => {
            let beat = (time * entry.speed + noise_value * 7.0).sin() * 0.5 + 0.5;
            let rot = 0.35 + beat * 0.65;
            rgb.r += entry.color[0] * rot;
            rgb.g += entry.color[1] * rot;
            rgb.b += entry.color[2] * rot;

            rgb.r *= 0.86;
            rgb.g *= 0.9;
            rgb.b *= 0.88;
        }
        Fx::Foul => {
            let shimmer = ((time * entry.speed + p.x as f32 * 0.19 - p.y as f32 * 0.11).sin()
                * 0.5
                + 0.5)
                * entry.gain;
            rgb.r += entry.color[0] * shimmer;
            rgb.g += entry.color[1] * shimmer;
            rgb.b += entry.color[2] * shimmer;
        }
        Fx::Strata => {
            let amt = p.row.row_band * 0.9 + noise_value * 0.4;
            rgb.r += entry.color[0] * amt;
            rgb.g += entry.color[1] * amt;
            rgb.b += entry.color[2] * amt;
        }
        _ => {
            let twinkle = ((time * 1.1 + noise_value * 9.0).sin() * 0.5 + 0.5) * 0.7
                + (noise_value * 0.6);
            let amt = twinkle * entry.gain;
            rgb.r += entry.color[0] * amt;
            rgb.g += entry.color[1] * amt;
            rgb.b += entry.color[2] * amt;
        }
    }
}

/// Put the element's own effect on the colour of the cell.
fn apply_fx(p: &PixelCtx<'_>, entry: &FxEntry, rgb: &mut Rgb, noise_value: f32, heat_v: f32) {
    match entry.kind {
        Fx::None => {}
        Fx::Wave => fx_wave(p, entry, rgb),
        Fx::Flicker => fx_flicker(p, entry, rgb, noise_value),
        Fx::Heat => fx_heat(p, entry, rgb, noise_value, heat_v),
        Fx::Pulse | Fx::Mystic | Fx::Ember => fx_pulse(p, entry, rgb, noise_value),
        Fx::Sparkle => fx_sparkle(p, entry, rgb, noise_value),
        Fx::Foliage | Fx::Glint | Fx::Grain | Fx::Sheen => {
            fx_surface(p, entry, rgb, noise_value);
        }
        Fx::Rot | Fx::Foul | Fx::Strata | Fx::Vein => fx_motion(p, entry, rgb, noise_value),
    }
}

/// Put the light, the shafts of the sun and the bloom on the colour of
/// the cell, and darken the edges of the picture.
fn finish_cell(p: &PixelCtx<'_>, rgb: &mut Rgb, depth: f32) {
    let sky = p.sky;
    let tables = p.tables;

    if !p.row.light_row.is_empty() {
        let lf = p.row.light_row[p.light_col];
        rgb.r += lf[0];
        rgb.g += lf[1];
        rgb.b += lf[2];
    }

    if depth < 26.0 {
        let shaft = tables.sun_shaft[p.rx] * (1.0 - depth * (1.0 / 26.0));
        rgb.r += shaft * 30.0;
        rgb.g += shaft * 27.0;
        rgb.b += shaft * 17.0;
    }

    if sky.shimmer && depth < 9.0 {
        let sh = ((p.time * 3.2 + p.x as f32 * 0.21 + p.y as f32 * 0.13).sin()) * 6.0
            * (1.0 - depth * (1.0 / 9.0));
        rgb.r += sh;
        rgb.g += sh * 0.8;
        rgb.b += sh * 0.35;
    }

    if sky.flash > 0.0 {
        rgb.r += sky.flash * 110.0;
        rgb.g += sky.flash * 116.0;
        rgb.b += sky.flash * 132.0;
    }

    {
        let gcol = (p.rx / bloom::STEP + 1).min(tables.bloom.w - 1);
        let gl = tables.bloom.at(p.row.glow_row, gcol);
        rgb.r += gl[0] * bloom::STRENGTH;
        rgb.g += gl[1] * bloom::STRENGTH;
        rgb.b += gl[2] * bloom::STRENGTH;
    }

    let vig = 1.0 - 0.30 * (tables.vig_col[p.rx] * 0.8 + p.row.ny2 * 0.5);
    rgb.r *= vig;
    rgb.g *= vig;
    rgb.b *= vig;
}

/// Paint a cell that holds something.
fn paint_cell(p: &PixelCtx<'_>) -> Rgb {
    let cell_color = p.row.col_row[p.xr];
    let noise_value = noise(p.x, p.y);
    let heat_v = p.row.heat_row[p.xr] as f32;
    let depth = (p.y as f32 - p.world.ground[p.xr] as f32).max(0.0);

    let mut rgb = shade_cell(p, cell_color, depth, noise_value);
    let entry = &p.tables.fx[p.row.kind_row[p.xr] as usize];
    apply_fx(p, entry, &mut rgb, noise_value, heat_v);
    finish_cell(p, &mut rgb, depth);
    rgb
}

/// Paint one pixel of the picture.
fn paint_pixel(p: &PixelCtx<'_>, elem_byte: u8, cloud: &mut Cloud) -> Rgb {
    if elem_byte != Elem::Empty as u8 {
        return paint_cell(p);
    }
    let surf = p.world.ground[p.xr] as f32;
    if (p.y as f32) >= surf {
        return paint_ground(p);
    }

    let mut rgb = paint_sky(p, cloud);
    let gcol = (p.rx / bloom::STEP + 1).min(p.tables.bloom.w - 1);
    let gl = p.tables.bloom.at(p.row.glow_row, gcol);
    rgb.r += gl[0] * bloom::STRENGTH;
    rgb.g += gl[1] * bloom::STRENGTH;
    rgb.b += gl[2] * bloom::STRENGTH;

    if p.sky.flash > 0.0 {
        rgb.r += p.sky.flash * 80.0;
        rgb.g += p.sky.flash * 86.0;
        rgb.b += p.sky.flash * 104.0;
    }

    let vig = 1.0 - 0.30 * (p.tables.vig_col[p.rx] * 0.8 + p.row.ny2 * 0.5);
    rgb.r *= vig;
    rgb.g *= vig;
    rgb.b *= vig;
    rgb
}

pub(super) fn paint(
    world: &World,
    buf: &mut [u8],
    tables: &PaintTables<'_>,
    geom: &PaintGeometry,
    clock: &Clock,
) {
    let PaintGeometry {
        region,
        light_w: lw,
        light_x: gx0,
        ..
    } = *geom;
    let x0 = region.x;
    let y0 = region.y;
    let rw = region.w;
    let rh = region.h;
    let world_w = world.w;
    let world_h = world.h;

    let sky = sky_state(world_w, world_h, clock);

    for ry in 0..rh {
        let row_y = y0 + ry;
        if row_y < 0 || row_y >= world_h as i32 {
            continue;
        }
        let mut cloud = Cloud { value: 0.0, amount: 0.0 };
        let mut row = row_context(world, tables, geom, row_y, ry);
        let fy = row_y as f32 / GSY;
        row.cloud_gy0 = (fy.floor() as usize).min(sky.gh - 2);
        row.cloud_ty = fy - row.cloud_gy0 as f32;

        for rx in 0..rw {
            let col_x = x0 + rx;
            if col_x < 0 || col_x >= world_w as i32 {
                continue;
            }

            let light_col = light_column(&row, col_x, gx0, lw);
            let xr = col_x as usize;
            let offset = (ry as usize * rw as usize + rx as usize) * 4;
            let elem_byte = row.kind_row[xr];

            let pixel = PixelCtx {
                world,
                tables,
                sky: &sky,
                row: &row,
                rx: rx as usize,
                xr,
                x: col_x,
                y: row_y,
                time: clock.time,
                light_col,
            };

            let rgb = paint_pixel(&pixel, elem_byte, &mut cloud);

            buf[offset] = clamp_u8(rgb.r);
            buf[offset + 1] = clamp_u8(rgb.g);
            buf[offset + 2] = clamp_u8(rgb.b);
            buf[offset + 3] = 255;
        }
    }
}
