
// ---------------------------------------------------------------------------
// Creature
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Worker = 0,
    Soldier = 1,
    Queen = 2,
    Priest = 3,
    Merchant = 4,
    Builder = 5,
    Explorer = 6,
    Shaman = 7,
}

/// Evolutionary traits. A creature inherits its genes from its parents, and
/// the genes decide which traits a species evolves. Every trait has a real
/// effect on the simulation.
pub mod trait_bits {
    pub const FEROCIOUS: u8 = 1 << 0;
    pub const INDUSTRIOUS: u8 = 1 << 1;
    pub const HARDY: u8 = 1 << 2;
    pub const FERTILE: u8 = 1 << 3;
    pub const KEEN_SENSES: u8 = 1 << 4;
    pub const SCAVENGER: u8 = 1 << 5;
    pub const PEACEFUL: u8 = 1 << 6;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trait {
    Ferocious,
    Industrious,
    Hardy,
    Fertile,
    KeenSenses,
    Scavenger,
    Peaceful,
}

impl Trait {
    pub fn name(self) -> &'static str {
        match self {
            Trait::Ferocious => "Ferocious",
            Trait::Industrious => "Industrious",
            Trait::Hardy => "Hardy",
            Trait::Fertile => "Fertile",
            Trait::KeenSenses => "Keen Senses",
            Trait::Scavenger => "Scavenger",
            Trait::Peaceful => "Peaceful",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Trait::Ferocious => "+35% melee damage, attacks more readily",
            Trait::Industrious => "builds and gathers faster",
            Trait::Hardy => "+40% health, hungers more slowly",
            Trait::Fertile => "cheaper, more frequent offspring",
            Trait::KeenSenses => "sees and senses further",
            Trait::Scavenger => "finds food and corpses further away",
            Trait::Peaceful => "prefers trade over violence",
        }
    }
}

/// Decode the trait flags of a gene set.
pub fn decode_traits(genes: &[f32; 32]) -> u8 {
    let mut bits = 0u8;
    if genes[0] > 0.74 {
        bits |= trait_bits::FEROCIOUS;
    }
    if genes[1] > 0.74 {
        bits |= trait_bits::INDUSTRIOUS;
    }
    if genes[2] > 0.74 {
        bits |= trait_bits::HARDY;
    }
    if genes[3] > 0.74 {
        bits |= trait_bits::FERTILE;
    }
    if genes[4] > 0.74 {
        bits |= trait_bits::KEEN_SENSES;
    }
    if genes[5] > 0.74 {
        bits |= trait_bits::SCAVENGER;
    }
    if genes[6] < 0.18 {
        bits |= trait_bits::PEACEFUL;
    }
    bits
}

pub fn trait_list(bits: u8) -> Vec<Trait> {
    let mut v = Vec::new();
    if bits & trait_bits::FEROCIOUS != 0 {
        v.push(Trait::Ferocious);
    }
    if bits & trait_bits::INDUSTRIOUS != 0 {
        v.push(Trait::Industrious);
    }
    if bits & trait_bits::HARDY != 0 {
        v.push(Trait::Hardy);
    }
    if bits & trait_bits::FERTILE != 0 {
        v.push(Trait::Fertile);
    }
    if bits & trait_bits::KEEN_SENSES != 0 {
        v.push(Trait::KeenSenses);
    }
    if bits & trait_bits::SCAVENGER != 0 {
        v.push(Trait::Scavenger);
    }
    if bits & trait_bits::PEACEFUL != 0 {
        v.push(Trait::Peaceful);
    }
    v
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    None = 0,
    Pickaxe = 1,
    Sword = 2,
    Hammer = 3,
    Drill = 4,
    Scanner = 5,
    Bow = 6,
    Spear = 7,
    Shield = 8,
    Staff = 9,
    Amulet = 10,
    Backpack = 11,
    Cart = 12,
    Boat = 13,
    Glider = 14,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Disease {
    None = 0,
    Flu = 1,
    Plague = 2,
    Poison = 3,
    Curse = 4,
    Infection = 5,
    Parasite = 6,
    Madness = 7,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Belief {
    None = 0,
    Nature = 1,
    Fire = 2,
    Sun = 3,
    Moon = 4,
    Earth = 5,
    Water = 6,
    Metal = 7,
    Stars = 8,
    Void = 9,
    Ancestors = 10,
    Machines = 11,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Neutral = 0,
    Happy = 1,
    Angry = 2,
    Fearful = 3,
    Curious = 4,
    Loyal = 5,
    Rebellious = 6,
    Depressed = 7,
    Zealous = 8,
}

// ---------------------------------------------------------------------------
// Spells & magic
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Spell {
    None = 0,
    Fireball = 1,
    Heal = 2,
    Teleport = 3,
    Wall = 4,
    Light = 5,
    Summon = 6,
    Curse = 7,
    Bless = 8,
    Lightning = 9,
}

// ---------------------------------------------------------------------------
// Buildable structures (functional)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buildable {
    Wall = 0,
    Door = 1,
    Chest = 2,
    Furnace = 3,
    Farm = 4,
    Temple = 5,
    Tower = 6,
    Bridge = 7,
}

/// A shot in flight (arrows, mana bolts, tower fire).

// ---------------------------------------------------------------------------
// Vehicle / mount
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MountKind {
    None = 0,
    Beast = 1,
    Cart = 2,
    Boat = 3,
    Glider = 4,
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VehicleType {
    Cart = 0,
    Boat = 1,
    Glider = 2,
    Sled = 3,
}

