
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
    /// A diplomat. Envoys walk to foreign colonies to negotiate treaties.
    Envoy = 8,
}

/// Evolutionary traits. A creature inherits its genes from its parents, and
/// the genes decide which traits a species evolves. Every trait has a real
/// effect on the simulation.
///
/// The bits are `u16`, so a species can carry up to sixteen of them.
pub mod trait_bits {
    pub const FEROCIOUS: u16 = 1 << 0;
    pub const INDUSTRIOUS: u16 = 1 << 1;
    pub const HARDY: u16 = 1 << 2;
    pub const FERTILE: u16 = 1 << 3;
    pub const KEEN_SENSES: u16 = 1 << 4;
    pub const SCAVENGER: u16 = 1 << 5;
    pub const PEACEFUL: u16 = 1 << 6;
    /// Melee attacks poison whatever they hit.
    pub const VENOMOUS: u16 = 1 << 7;
    /// Can swim through water instead of being blocked by it.
    pub const AMPHIBIOUS: u16 = 1 << 8;
    /// Hunts at night and rests badly in the day.
    pub const NOCTURNAL: u16 = 1 << 9;
    /// Works better in the company of its own kind.
    pub const GREGARIOUS: u16 = 1 << 10;
    /// Carries far more than its size suggests.
    pub const HOARDER: u16 = 1 << 11;
    /// Immune to the cold, and drawn to heat.
    pub const THERMOPHILIC: u16 = 1 << 12;
    /// Reads metal in the rock and digs towards it.
    pub const METALLOPHILIC: u16 = 1 << 13;
    /// Light bones: fast, but frail.
    pub const HOLLOW_BONED: u16 = 1 << 14;
    /// The colony shares one mind: the members know what the others know.
    pub const HIVEMIND: u16 = 1 << 15;
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
    Venomous,
    Amphibious,
    Nocturnal,
    Gregarious,
    Hoarder,
    Thermophilic,
    Metallophilic,
    HollowBoned,
    Hivemind,
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
            Trait::Venomous => "Venomous",
            Trait::Amphibious => "Amphibious",
            Trait::Nocturnal => "Nocturnal",
            Trait::Gregarious => "Gregarious",
            Trait::Hoarder => "Hoarder",
            Trait::Thermophilic => "Thermophilic",
            Trait::Metallophilic => "Metallophilic",
            Trait::HollowBoned => "Hollow-Boned",
            Trait::Hivemind => "Hivemind",
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
            Trait::Venomous => "melee attacks poison the wounded",
            Trait::Amphibious => "swims through water and damp",
            Trait::Nocturnal => "hunts at night, idles through the day",
            Trait::Gregarious => "works faster near its own kind",
            Trait::Hoarder => "carries 60% more",
            Trait::Thermophilic => "immune to cold, thrives near lava",
            Trait::Metallophilic => "senses metal and digs towards it",
            Trait::HollowBoned => "moves faster, but has less health",
            Trait::Hivemind => "the colony shares one mind",
        }
    }
}

/// Decode the trait flags of a gene set.
///
/// Every trait has its own gene. A high gene means the trait is expressed,
/// except for the `Peaceful` trait, which is the mirror image: a *low* gene
/// means the species has evolved a peaceful nature.
pub fn decode_traits(genes: &[f32; 32]) -> u16 {
    let mut bits = 0u16;
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
    if genes[7] > 0.76 {
        bits |= trait_bits::VENOMOUS;
    }
    if genes[8] > 0.76 {
        bits |= trait_bits::AMPHIBIOUS;
    }
    if genes[9] > 0.76 {
        bits |= trait_bits::NOCTURNAL;
    }
    if genes[10] > 0.76 {
        bits |= trait_bits::GREGARIOUS;
    }
    if genes[11] > 0.76 {
        bits |= trait_bits::HOARDER;
    }
    if genes[12] > 0.76 {
        bits |= trait_bits::THERMOPHILIC;
    }
    if genes[13] > 0.76 {
        bits |= trait_bits::METALLOPHILIC;
    }
    if genes[14] > 0.78 {
        bits |= trait_bits::HOLLOW_BONED;
    }
    if genes[15] > 0.80 {
        bits |= trait_bits::HIVEMIND;
    }
    bits
}

/// How many traits a genome expresses. Used to judge how specialised an
/// animal is.
pub fn trait_count(bits: u16) -> u32 {
    bits.count_ones()
}

pub fn trait_list(bits: u16) -> Vec<Trait> {
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
    if bits & trait_bits::VENOMOUS != 0 {
        v.push(Trait::Venomous);
    }
    if bits & trait_bits::AMPHIBIOUS != 0 {
        v.push(Trait::Amphibious);
    }
    if bits & trait_bits::NOCTURNAL != 0 {
        v.push(Trait::Nocturnal);
    }
    if bits & trait_bits::GREGARIOUS != 0 {
        v.push(Trait::Gregarious);
    }
    if bits & trait_bits::HOARDER != 0 {
        v.push(Trait::Hoarder);
    }
    if bits & trait_bits::THERMOPHILIC != 0 {
        v.push(Trait::Thermophilic);
    }
    if bits & trait_bits::METALLOPHILIC != 0 {
        v.push(Trait::Metallophilic);
    }
    if bits & trait_bits::HOLLOW_BONED != 0 {
        v.push(Trait::HollowBoned);
    }
    if bits & trait_bits::HIVEMIND != 0 {
        v.push(Trait::Hivemind);
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
    /// A trading post.
    Market = 8,
    /// A craftsman's hall.
    Workshop = 9,
    /// A food store.
    Granary = 10,
    /// A signal beacon.
    Beacon = 11,
}

/// The kinds of structure a colony can raise.
///
/// The number is stored in `Structure::kind`.
pub mod structure_kind {
    /// A plain building with no special effect.
    pub const PLAIN: u8 = 0;
    /// An armed tower.
    pub const TURRET: u8 = 1;
    /// A temple: it steadies the colony.
    pub const TEMPLE: u8 = 2;
    /// A farm plot.
    pub const FARM: u8 = 3;
    /// A market: trade beside it is more profitable.
    pub const MARKET: u8 = 4;
    /// A workshop: tools are made faster beside it.
    pub const WORKSHOP: u8 = 5;
    /// A granary: the colony does not go hungry beside it.
    pub const GRANARY: u8 = 6;
    /// A beacon: the colony sees further from it.
    pub const BEACON: u8 = 7;
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

