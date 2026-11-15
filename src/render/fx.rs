use crate::elements::Elem;

const TAU: f32 = std::f32::consts::TAU;

/// The wave that runs over the water.
#[must_use]
pub fn wave_sin(x: i32, y: i32, time: f32) -> f32 {
    ((x as f32 * 0.17) + (y as f32 * 0.09) + time * 1.6).sin()
}

/// The foam at the shore of the water.
#[must_use]
pub fn foam_sin(x: i32, y: i32, time: f32) -> f32 {
    ((x as f32 * 0.31) + (y as f32 * 0.19) - time * 2.4).sin()
}

/// The light on the water, down a column.
#[must_use]
pub fn caustic_col_sin(x: i32, time: f32) -> f32 {
    (x as f32 * 0.21 + time * 1.7).sin()
}

/// The light on the water, across a row.
#[must_use]
pub fn caustic_row_sin(y: i32, time: f32) -> f32 {
    (y as f32 * 0.27 - time * 1.1).sin()
}

/// The fire that flickers.
#[must_use]
pub fn flicker_sin(time: f32, noise: f32) -> f32 {
    (time * 11.0 + noise * TAU).sin()
}

/// The heat that pulses.
#[must_use]
pub fn heat_sin(time: f32, noise: f32) -> f32 {
    (time * 2.1 + noise * TAU).sin()
}

/// The pulse of the magic and of the ember.
#[must_use]
pub fn pulse_sin(time: f32, speed: f32, noise: f32) -> f32 {
    (time * speed + noise * TAU).sin()
}

/// The glitter of the crystal.
#[must_use]
pub fn sparkle_sin(time: f32, noise: f32) -> f32 {
    (time * 3.0 + noise * TAU).sin()
}

/// The glitter of the ice.
#[must_use]
pub fn glint_sin(time: f32, noise: f32) -> f32 {
    (time * 0.9 + noise * 6.0).sin()
}

/// The sheen of the tar.
#[must_use]
pub fn sheen_sin(time: f32, x: i32, y: i32) -> f32 {
    (time * 0.5 + x as f32 * 0.05 + y as f32 * 0.03).sin()
}

/// The beat of the rot.
#[must_use]
pub fn rot_sin(time: f32, speed: f32, noise: f32) -> f32 {
    (time * speed + noise * 7.0).sin()
}

/// The shimmer of the filth.
#[must_use]
pub fn foul_sin(time: f32, speed: f32, x: i32, y: i32) -> f32 {
    (time * speed + x as f32 * 0.19 - y as f32 * 0.11).sin()
}

/// The vein of the metal.
#[must_use]
pub fn vein_sin(time: f32, noise: f32) -> f32 {
    (time * 1.1 + noise * 9.0).sin()
}

/// The shimmer of the heatwave on the ground.
#[must_use]
pub fn shimmer_sin(time: f32, x: i32, y: i32) -> f32 {
    (time * 3.2 + x as f32 * 0.21 + y as f32 * 0.13).sin()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fx {

    None,

    Wave,

    Flicker,

    Heat,

    Pulse,

    Mystic,

    Sparkle,

    Ember,

    Foliage,

    Glint,

    Grain,

    Sheen,

    Rot,

    Foul,

    Strata,

    Vein,
}

#[derive(Clone, Copy)]
pub struct FxEntry {
    pub kind: Fx,

    pub color: [f32; 3],

    pub speed: f32,

    pub gain: f32,
}

pub const FX_TABLE_LEN: usize = 256;

const NONE: FxEntry = FxEntry {
    kind: Fx::None,
    color: [0.0; 3],
    speed: 0.0,
    gain: 0.0,
};

#[must_use]
pub fn fx_table() -> [FxEntry; FX_TABLE_LEN] {
    let mut t = [NONE; FX_TABLE_LEN];
    let mut set = |e: Elem, kind: Fx, color: [f32; 3], speed: f32, gain: f32| {
        t[e as usize] = FxEntry {
            kind,
            color,
            speed,
            gain,
        };
    };

    set(Elem::Water, Fx::Wave, [7.0, 13.0, 22.0], 1.6, 1.0);
    set(Elem::Fire, Fx::Flicker, [90.0, 46.0, 12.0], 11.0, 1.0);
    set(Elem::Lava, Fx::Heat, [1.0, 0.36, 0.12], 2.1, 1.0);
    set(Elem::Magma, Fx::Heat, [1.0, 0.36, 0.12], 2.1, 1.0);
    set(Elem::Plasma, Fx::Pulse, [16.0, 32.0, 64.0], 6.0, 1.0);
    set(Elem::Energy, Fx::Pulse, [16.0, 32.0, 64.0], 6.0, 1.0);
    set(Elem::Mana, Fx::Mystic, [24.0, 18.0, 46.0], 2.6, 1.0);
    set(Elem::Soul, Fx::Mystic, [24.0, 18.0, 46.0], 2.6, 1.0);
    set(Elem::Portal, Fx::Mystic, [24.0, 18.0, 46.0], 2.6, 1.0);
    set(Elem::Rune, Fx::Mystic, [24.0, 18.0, 46.0], 2.6, 1.0);
    set(Elem::Ward, Fx::Mystic, [24.0, 18.0, 46.0], 2.6, 1.0);
    set(Elem::Crystal, Fx::Sparkle, [18.2, 23.4, 26.0], 3.0, 1.0);
    set(Elem::Gem, Fx::Sparkle, [18.2, 23.4, 26.0], 3.0, 1.0);
    set(Elem::Diamond, Fx::Sparkle, [18.2, 23.4, 26.0], 3.0, 1.0);
    set(Elem::Ember, Fx::Ember, [70.0, 26.0, 6.0], 1.7, 1.0);
    set(Elem::Plant, Fx::Foliage, [-13.0, 26.0, -5.2], 0.0, 1.0);
    set(Elem::Moss, Fx::Foliage, [-13.0, 26.0, -5.2], 0.0, 1.0);
    set(Elem::Fungus, Fx::Foliage, [-13.0, 26.0, -5.2], 0.0, 1.0);
    set(Elem::Spore, Fx::Foliage, [-13.0, 26.0, -5.2], 0.0, 1.0);
    set(Elem::Roots, Fx::Foliage, [-13.0, 26.0, -5.2], 0.0, 1.0);
    set(Elem::Ice, Fx::Glint, [4.8, 6.4, 8.0], 0.9, 1.0);
    set(Elem::Snow, Fx::Glint, [4.8, 6.4, 8.0], 0.9, 1.0);
    set(Elem::Charcoal, Fx::Grain, [6.0, 3.0, 0.0], 0.0, 1.0);
    set(Elem::Tar, Fx::Sheen, [5.0, 4.0, 3.5], 0.5, 1.0);

    set(Elem::Virus, Fx::Rot, [26.0, 4.0, 10.0], 2.4, 1.0);
    set(Elem::Bacteria, Fx::Rot, [22.0, 16.0, 2.0], 1.6, 1.0);
    set(Elem::Poison, Fx::Foul, [10.0, 30.0, 10.0], 0.8, 1.0);
    set(Elem::Acid, Fx::Foul, [24.0, 34.0, 4.0], 1.2, 1.0);
    set(Elem::Chlorine, Fx::Foul, [8.0, 24.0, 14.0], 0.6, 1.0);

    set(Elem::Stone, Fx::Strata, [16.0, 15.0, 13.0], 0.0, 1.0);
    set(Elem::Granite, Fx::Strata, [18.0, 15.0, 16.0], 0.0, 1.0);
    set(Elem::Obsidian, Fx::Strata, [14.0, 11.0, 22.0], 0.0, 1.0);
    set(Elem::Sand, Fx::Strata, [20.0, 17.0, 9.0], 0.0, 1.0);
    set(Elem::Mud, Fx::Strata, [12.0, 9.0, 6.0], 0.0, 1.0);
    set(Elem::Dust, Fx::Strata, [12.0, 11.0, 10.0], 0.0, 1.0);

    set(Elem::Metal, Fx::Vein, [26.0, 24.0, 20.0], 0.0, 1.0);
    set(Elem::Copper, Fx::Vein, [30.0, 16.0, 6.0], 0.0, 1.0);
    set(Elem::Tin, Fx::Vein, [22.0, 22.0, 24.0], 0.0, 1.0);
    set(Elem::Bronze, Fx::Vein, [28.0, 18.0, 8.0], 0.0, 1.0);
    set(Elem::Iron, Fx::Vein, [20.0, 20.0, 24.0], 0.0, 1.0);
    set(Elem::Steel, Fx::Vein, [24.0, 26.0, 30.0], 0.0, 1.0);
    set(Elem::Gold, Fx::Vein, [34.0, 26.0, 6.0], 0.0, 1.0);
    set(Elem::Silver, Fx::Vein, [28.0, 30.0, 32.0], 0.0, 1.0);
    set(Elem::Rust, Fx::Vein, [26.0, 12.0, 5.0], 0.0, 1.0);

    t
}
