use super::*;

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

