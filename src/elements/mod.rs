mod percept;
mod properties;

pub use percept::{Look, look_table};

pub const TABLE_LEN: usize = 96;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Elem {
    Empty = 0,

    Sand,
    Sulfur,
    Salt,
    Ash,
    Dust,
    Gunpowder,
    Fertilizer,
    Mud,

    Water,
    Oil,
    Acid,
    Mercury,
    Blood,
    Poison,
    Alcohol,
    Honey,
    Tar,

    Steam,
    Smoke,
    Gas,
    Plasma,
    Chlorine,
    Methane,
    Oxygen,

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

    Fire,
    Energy,
    Ice,
    Lava,
    Magma,
    Void,
    Mana,
    Soul,
    Ember,

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

    Rune,
    Ward,
    Portal,

    Egg,
    Corpse,
    Virus,
    Bacteria,
    Food,
    Snow,
    Crop,

    Market,

    Workshop,

    Granary,

    Beacon,

    Hearth,

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
    Conduct,
    Spread,
    Hatch,
    Decay,
    Cultivate,
}

impl Elem {

    pub const TABLE_LEN: usize = 96;

    #[must_use]
    pub fn from_u8(v: u8) -> Elem {
        if v <= Elem::MAX {

            unsafe { std::mem::transmute::<u8, Elem>(v) }
        } else {
            Elem::Empty
        }
    }

    pub const MAX: u8 = Elem::Totem as u8;

    #[must_use]
    pub fn inert_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_inert();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn solid_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_solid();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn edible_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).is_edible();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn nutrition_table() -> [f32; TABLE_LEN] {
        let mut t = [0.0f32; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).nutrition();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn toxicity_table() -> [f32; TABLE_LEN] {
        let mut t = [0.0f32; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).toxicity();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn emissive_table() -> [bool; TABLE_LEN] {
        let mut t = [false; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).emission()[0] > 0.0;
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn emission_table() -> [[f32; 3]; TABLE_LEN] {
        let mut t = [[0.0f32; 3]; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).emission();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn behavior_table() -> [Behavior; TABLE_LEN] {
        let mut t = [Behavior::Static; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).behavior();
            i += 1;
        }
        t
    }

    #[must_use]
    pub fn lifetime_table() -> [u32; TABLE_LEN] {
        let mut t = [u32::MAX; TABLE_LEN];
        let mut i = 0;
        while i < TABLE_LEN {
            t[i] = Elem::from_u8(i as u8).lifetime();
            i += 1;
        }
        t
    }

    #[must_use]
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
