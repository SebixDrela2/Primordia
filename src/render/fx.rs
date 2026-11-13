//! Element-driven visual effects, as lookup tables.
//!
//! The pixel loop must not match on the element enum, so every element's
//! animated effect is described by a small table entry that the renderer
//! reads directly.

use crate::elements::Elem;

/// What kind of animated effect an element has.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fx {
    /// No effect.
    None,
    /// Water waves: a moving ripple.
    Wave,
    /// Fire flicker: a fast, bright pulse.
    Flicker,
    /// Lava / magma: a slow heat pulse scaled by the cell's temperature.
    Heat,
    /// Plasma / energy: a fast cool pulse.
    Pulse,
    /// Mana and friends: a slow mystical pulse.
    Mystic,
    /// Crystals: a sharp sparkle.
    Sparkle,
    /// Embers: a dying coal glow.
    Ember,
    /// Plants: a per-cell hue variation.
    Foliage,
    /// Ice and snow: a cold specular glint.
    Glint,
    /// Charcoal: a faint warm grain.
    Grain,
    /// Tar: a slow glossy sheen.
    Sheen,
    /// The corruption of the deep: a sick, pulsing rot.
    Rot,
    /// The poison of the deep: a slow, oily shimmer.
    Foul,
    /// Rock: horizontal bands of colour, the way a cliff is layered.
    Strata,
    /// Ore: a metallic glint, brighter the deeper it lies.
    Vein,
}

/// One entry per element byte.
#[derive(Clone, Copy)]
pub struct FxEntry {
    pub kind: Fx,
    /// The colour the effect adds.
    pub color: [f32; 3],
    /// How fast the effect animates.
    pub speed: f32,
    /// How strong the effect is.
    pub gain: f32,
}

/// The table has an entry for every possible element byte, so the pixel
/// loop can index it with the byte itself and never has to check the
/// bounds.
pub const FX_TABLE_LEN: usize = 256;

const NONE: FxEntry = FxEntry {
    kind: Fx::None,
    color: [0.0; 3],
    speed: 0.0,
    gain: 0.0,
};

/// Build the lookup table. It is created once and then only read.
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

    // The corruption of the deep. It is not pretty: it pulses, it rots,
    // and it shimmers the way a pool of oil does.
    set(Elem::Virus, Fx::Rot, [26.0, 4.0, 10.0], 2.4, 1.0);
    set(Elem::Bacteria, Fx::Rot, [22.0, 16.0, 2.0], 1.6, 1.0);
    set(Elem::Poison, Fx::Foul, [10.0, 30.0, 10.0], 0.8, 1.0);
    set(Elem::Acid, Fx::Foul, [24.0, 34.0, 4.0], 1.2, 1.0);
    set(Elem::Chlorine, Fx::Foul, [8.0, 24.0, 14.0], 0.6, 1.0);

    // The rock of the deep world is layered, the way a cliff is layered.
    // The bands are what makes the underground read as stone rather than
    // as a flat grey field.
    set(Elem::Stone, Fx::Strata, [16.0, 15.0, 13.0], 0.0, 1.0);
    set(Elem::Granite, Fx::Strata, [18.0, 15.0, 16.0], 0.0, 1.0);
    set(Elem::Obsidian, Fx::Strata, [14.0, 11.0, 22.0], 0.0, 1.0);
    set(Elem::Sand, Fx::Strata, [20.0, 17.0, 9.0], 0.0, 1.0);
    set(Elem::Mud, Fx::Strata, [12.0, 9.0, 6.0], 0.0, 1.0);
    set(Elem::Dust, Fx::Strata, [12.0, 11.0, 10.0], 0.0, 1.0);

    // The ore catches the light. The deeper it lies, the more it glints.
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
