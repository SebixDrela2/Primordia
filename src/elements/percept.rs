//! What an element *looks like* to an animal.
//!
//! This is deliberately **not** the element's name or its properties. A
//! creature does not know that a cell of `Plant` is food, or that a cell of
//! `Poison` is poisonous. It knows that the one is green and soft and the
//! other is green and wet. Telling the difference is the brain's job, and
//! it is a job it has to learn.
//!
//! So this table is a *sensory* one: colour, light, warmth, how yielding
//! the material is, how wet it is and how glossy it is. Everything here
//! could in principle be measured by an animal standing in front of it.
//!
//! Two elements with the same look are indistinguishable to the brain, and
//! that is the point: `Plant` and `Crop` must look nearly the same, and so
//! must `Poison` and `Acid`, so that a brain which learns "green and wet is
//! poison" is right about both.

use super::Elem;

/// The appearance of a material.
#[derive(Clone, Copy, Default)]
pub struct Look {
    /// Red, green and blue, from 0 to 1.
    pub rgb: [f32; 3],
    /// How much it shines by itself, from 0 to 1.
    pub bright: f32,
    /// How warm it is: -1 is cold, 0 is ordinary, 1 is hot.
    pub warm: f32,
    /// How yielding it is: 0 is hard rock, 1 is soft enough to eat.
    pub soft: f32,
    /// How wet it is, from 0 to 1.
    pub wet: f32,
    /// How glossy it is, from 0 to 1.
    pub sheen: f32,
}

impl Look {
    /// The colour of this look, as a difference between green and red.
    ///
    /// This is what a creature actually reads off a plant: not "green",
    /// but "more green than red". It is what makes a plant stand out from
    /// the dirt it grows in.
    pub fn greenness(&self) -> f32 {
        self.rgb[1] - self.rgb[0]
    }

    /// How different this look is from plain grey.
    ///
    /// A dull brown thing and a grey thing are hard to tell apart; a
    /// green thing and a grey thing are not.
    pub fn colourfulness(&self) -> f32 {
        let r = self.rgb[0];
        let g = self.rgb[1];
        let b = self.rgb[2];
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        if mx <= 0.0 {
            0.0
        } else {
            (mx - mn) / mx
        }
    }
}

/// Build the table of looks, one for every element.
///
/// This is built once at startup and then only read, because the hot loops
/// cannot afford to match on the element enum.
pub fn look_table() -> [Look; Elem::TABLE_LEN] {
    let mut t = [Look::default(); Elem::TABLE_LEN];
    let mut i = 0;
    while i < Elem::TABLE_LEN {
        t[i] = look_of(Elem::from_u8(i as u8));
        i += 1;
    }
    t
}

/// The look of one element.
fn look_of(e: Elem) -> Look {
    let c = e.color();
    let rgb = [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    ];
    // How much it shines by itself. Only the brightest colour of the glow
    // counts, so a dim crystal is dim and a fire is not.
    let em = e.emission();
    let bright = em[0].max(em[1]).max(em[2]);
    Look {
        rgb,
        bright,
        warm: warmth_of(e),
        soft: softness_of(e),
        wet: wetness_of(e),
        sheen: sheen_of(e),
    }
}

/// How warm a material is.
fn warmth_of(e: Elem) -> f32 {
    match e {
        Elem::Fire | Elem::Lava | Elem::Magma | Elem::Plasma => 1.0,
        Elem::Ember => 0.7,
        Elem::Steam => 0.4,
        Elem::Water | Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Blood => -0.15,
        Elem::Ice | Elem::Snow => -0.7,
        _ => 0.0,
    }
}

/// How yielding a material is.
///
/// A powder gives way under a finger. A plant can be torn. A rock cannot.
fn softness_of(e: Elem) -> f32 {
    match e {
        // Living things and the dead give way easily.
        Elem::Plant
        | Elem::Spore
        | Elem::Fungus
        | Elem::Moss
        | Elem::Roots
        | Elem::Meat
        | Elem::Leather
        | Elem::Corpse
        | Elem::Egg
        | Elem::Food
        | Elem::Crop
        | Elem::Blood
        | Elem::Honey => 0.95,
        // Powders are soft but have no give at all.
        Elem::Sand
        | Elem::Ash
        | Elem::Dust
        | Elem::Salt
        | Elem::Sulfur
        | Elem::Gunpowder
        | Elem::Fertilizer
        | Elem::Mud
        | Elem::Snow
        | Elem::Rust => 0.7,
        // Liquids take the shape of whatever holds them.
        Elem::Water
        | Elem::Oil
        | Elem::Acid
        | Elem::Mercury
        | Elem::Poison
        | Elem::Alcohol
        | Elem::Tar => 0.4,
        Elem::Wood => 0.55,
        Elem::Charcoal => 0.5,
        _ => 0.0,
    }
}

/// How wet a material is.
fn wetness_of(e: Elem) -> f32 {
    match e {
        Elem::Water => 1.0,
        Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Poison | Elem::Alcohol => 0.85,
        Elem::Blood | Elem::Honey | Elem::Tar => 0.7,
        Elem::Snow => 0.6,
        Elem::Ice => 0.4,
        Elem::Mud => 0.5,
        Elem::Steam | Elem::Smoke => 0.3,
        _ => 0.0,
    }
}

/// How glossy a material is.
fn sheen_of(e: Elem) -> f32 {
    match e {
        Elem::Metal
        | Elem::Copper
        | Elem::Tin
        | Elem::Bronze
        | Elem::Iron
        | Elem::Steel
        | Elem::Gold
        | Elem::Silver
        | Elem::Glass
        | Elem::Crystal
        | Elem::Gem
        | Elem::Diamond
        | Elem::Obsidian
        | Elem::Wire
        | Elem::Circuit => 0.9,
        Elem::Water | Elem::Mercury | Elem::Ice => 0.75,
        Elem::Oil => 0.6,
        _ => 0.1,
    }
}
