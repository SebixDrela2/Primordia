//! Elements: the material palette of the world.
//!
//! Split by responsibility:
//! - mod.rs        the element enum plus the lookup tables
//! - properties.rs the per-element behaviour, colours and descriptions
//! - percept.rs    what each element looks like to an animal
//!
//! `Elem` is `#[repr(u8)]`, so the discriminant *is* the byte stored in a
//! world cell. The lookup tables in this module turn those bytes into
//! properties without ever matching on the enum inside a per-cell loop.

mod percept;
mod properties;

pub use percept::{Look, look_table};

/// Number of element bytes the lookup tables cover. It is larger than the
/// number of elements so the tables stay a fixed size even when new
/// materials are added.
pub const TABLE_LEN: usize = 96;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Elem {
    Empty = 0,
    // Powders
    Sand,
    Sulfur,
    Salt,
    Ash,
    Dust,
    Gunpowder,
    Fertilizer,
    Mud,
    // Liquids
    Water,
    Oil,
    Acid,
    Mercury,
    Blood,
    Poison,
    Alcohol,
    Honey,
    Tar,
    // Gases
    Steam,
    Smoke,
    Gas,
    Plasma,
    Chlorine,
    Methane,
    Oxygen,
    // Solids
    Stone,
    Metal,
    Copper,
    Tin,
    Bronze,
    Iron,
    Steel,
    Gold,
    Silver,
    Glass,
    Crystal,
    Gem,
    Diamond,
    Obsidian,
    Granite,
    Rust,
    // Organic
    Wood,
    Plant,
    Spore,
    Fungus,
    Moss,
    Roots,
    Meat,
    Bone,
    Leather,
    Charcoal,
    // Energy/Exotic
    Fire,
    Energy,
    Ice,
    Lava,
    Magma,
    Void,
    Mana,
    Soul,
    Ember,
    // Constructed
    Brick,
    Concrete,
    Wire,
    Circuit,
    Tool,
    Machine,
    Wall,
    Door,
    Chest,
    Furnace,
    Anvil,
    Forge,
    Farm,
    Temple,
    Tower,
    Bridge,
    Road,
    // Magical
    Rune,
    Ward,
    Portal,
    // Biological
    Egg,
    Corpse,
    Virus,
    Bacteria,
    Food,
    Snow,
    Crop,
    // Settlement (built by creatures, or found in ruins)
    /// A trading post: the colony does better business beside it.
    Market,
    /// A craftsman's hall: tools are made faster beside it.
    Workshop,
    /// A food store: the colony does not go hungry beside it.
    Granary,
    /// A signal beacon: the colony sees further from it.
    Beacon,
    /// A hearth: the colony does not suffer from the cold beside it.
    Hearth,
    /// An animal totem: a symbol of the tribe.
    Totem,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    Static,
    Fall,
    Flow,
    Rise,
    Diffuse,
    Burn,
    Grow,
    Energy,
    Conduct, // for electricity/wires
    Spread,  // for viruses/bacteria
    Hatch,   // for eggs
    Decay,   // for corpses
    Cultivate, // for farms: they sow and ripen crops
}

impl Elem {
    /// Number of element bytes the lookup tables cover. It is larger than the
    /// number of elements so the tables stay a fixed size even when new
    /// materials are added.
    pub const TABLE_LEN: usize = 96;

    /// Convert a raw byte back into an element. `Elem` is `#[repr(u8)]`, so the
    /// discriminant is authoritative. This must stay in lock-step with the enum
    /// declaration (it previously hand-duplicated a shifted table and silently
    /// mapped every stored cell to the wrong element).
    pub fn from_u8(v: u8) -> Elem {
        if v <= Elem::MAX {
            // SAFETY: `Elem` is `#[repr(u8)]` and `v` is a declared discriminant.
            unsafe { std::mem::transmute::<u8, Elem>(v) }
        } else {
            Elem::Empty
        }
    }

    /// The largest valid element discriminant (inclusive).
    pub const MAX: u8 = Elem::Totem as u8;

    // ------------------------------------------------------------------
    // Lookup tables
    // ------------------------------------------------------------------

    /// Lookup table: is this element byte inert? Built once at startup.
    pub fn inert_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_inert();
            i += 1;
        }
        t
    }

    /// Lookup table: is this element byte solid? Built once at startup.
    pub fn solid_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_solid();
            i += 1;
        }
        t
    }

    /// Lookup table: is this element edible?
    pub fn edible_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_edible();
            i += 1;
        }
        t
    }

    /// Lookup table: how much nutrition does this element carry?
    pub fn nutrition_table() -> [f32; TABLE_LEN] {
        let mut t = [0.0f32; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).nutrition();
            i += 1;
        }
        t
    }

    /// Lookup table: how toxic is this element?
    pub fn toxicity_table() -> [f32; TABLE_LEN] {
        let mut t = [0.0f32; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).toxicity();
            i += 1;
        }
        t
    }

    /// Lookup table: does this element emit light? Used by the renderer to
    /// build the light field, so it must stay cheap to query per cell.
    pub fn emissive_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).emission()[0] > 0.0;
            i += 1;
        }
        t
    }

    /// Lookup table: the light colour (and intensity) an element radiates.
    pub fn emission_table() -> [[f32; 3]; TABLE_LEN] {
        let mut t = [[0.0f32; 3]; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).emission();
            i += 1;
        }
        t
    }

    /// Lookup table: the behaviour of an element byte.
    pub fn behavior_table() -> [Behavior; TABLE_LEN] {
        let mut t = [Behavior::Static; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).behavior();
            i += 1;
        }
        t
    }

    /// Lookup table: how long an element byte lives before it decays.
    pub fn lifetime_table() -> [u32; TABLE_LEN] {
        let mut t = [u32::MAX; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).lifetime();
            i += 1;
        }
        t
    }

    /// Lookup table: does this element take part in a chemical reaction?
    pub fn reactive_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_reactive();
            i += 1;
        }
        t
    }
}
