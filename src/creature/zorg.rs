//! The ZORGS: the other thing that lives in this world.
//!
//! Every other species in the simulation is a *player*: it starts at the
//! surface, it grows plants and hunts animals, it drinks water and it
//! builds. The ZORGs are none of those things. They come from the bottom
//! of the caves and they are the deep's own ecology:
//!
//! - they eat **rock**. A ZORG chews stone and ore for food, and the
//!   tunnels it digs are how it comes up into the world,
//! - **acid and poison are their meat**. The things that burn a player
//!   alive sustain a ZORG, so a lake of acid is a feast to them,
//! - they are hostile to everything. There is no treaty with a ZORG and
//!   no ZORG will ever ally.
//!
//! They have a home of their own: a cave in the deepest rock of the world
//! (`world::zorg_cave`), lined with the poison they feed on. Every one of
//! them is born in that cave, and the cave is what they come back to when
//! they are hungry.
//!
//! The ZORGs are not a player team, so they can never *win*. If they are
//! the last thing standing the run is lost, and the genome carried into
//! the next revival belongs to the nearest player species instead.

use super::*;

/// Which side of the world a creature belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction {
    /// An ordinary lineage: it competes for the world with the others.
    Player,
    /// A ZORG: a thing of the deep caves that eats stone and poison.
    Zorg,
}

impl Faction {
    pub fn name(self) -> &'static str {
        match self {
            Faction::Player => "Player",
            Faction::Zorg => "ZORG",
        }
    }
}

/// The colour of the ZORGs.
///
/// Purple belongs to them and to nothing else: the player species are
/// coloured from a palette that leaves purple out, so a ZORG is never
/// mistaken for a player colony.
pub const ZORG_COLOR: [u8; 3] = [148, 62, 210];

/// The palette the player species are coloured from.
///
/// The hues are spread around the wheel but **purple is left out**: it
/// belongs to the ZORGs. Purple sits between about 270 and 330 degrees,
/// so that stretch of the wheel is skipped.
pub const PLAYER_HUES: [f32; 14] = [
    0.0,    // red
    18.0,   // vermilion
    35.0,   // orange
    50.0,   // amber
    65.0,   // yellow
    95.0,   // yellow-green
    125.0,  // green
    155.0,  // spring green
    180.0,  // cyan
    200.0,  // sky
    220.0,  // blue
    245.0,  // indigo
    335.0,  // rose
    352.0,  // pink
];

impl CreatureSystem {
    /// Is this species a ZORG?
    ///
    /// The ZORGs are their own species: one lineage of them, that lives in
    /// the deep and comes up through the caves.
    pub fn is_zorg(&self, species: u32) -> bool {
        species == ZORG_SPECIES
    }

    /// Is this creature a ZORG?
    pub fn is_zorg_creature(&self, c: &Creature) -> bool {
        c.species == ZORG_SPECIES
    }

    /// How many ZORGs are left.
    pub fn zorg_count(&self) -> u32 {
        self.list.iter().filter(|c| self.is_zorg(c.species)).count() as u32
    }
}

/// A ZORG eats.
///
/// It has its own ecology: it eats the rock and the poison of the deep,
/// and nothing it eats is ever bad for it. It chews its way to the food,
/// so it can reach a long way.
///
/// It only eats when it needs to, though. A ZORG that eats every step
/// would strip the cave bare in a few ticks and then starve in the hole
/// it made.
///
/// It feeds on hunger *and* on tiredness: the rock of the deep is its
/// bread as well as its feast, and a ZORG that has spent the night
/// chewing its way up through the rock needs the bread.
pub fn zorg_act_eat(world: &mut World, c: &mut Creature, hungry: bool) -> bool {
    if c.hunger < 15.0 && c.energy > c.max_energy * 0.55 {
        return false;
    }
    let gx = c.x.round() as i32;
    let gy = c.y.round() as i32;
    // It eats what it can reach. A ZORG that could eat the walls of its
    // own cave from across it would hollow the cave out and then starve in
    // the hole.
    let zradius = if hungry { 3 } else { 2 };
    let Some((fx, fy)) = world.find_zorg_food(gx, gy, zradius) else {
        return false;
    };
    let Some((nutrition, toxicity)) = world.eat_zorg(fx, fy) else {
        return false;
    };
    c.energy = (c.energy + nutrition * 4.0).min(c.max_energy);
    c.hunger = (c.hunger - nutrition * 4.0).max(0.0);
    // The acid of the deep is a feast to a ZORG: it heals it as well as
    // feeding it.
    let e = world.get(fx, fy);
    if zorg_nutrition(e) >= 40.0 {
        c.health = (c.health + 6.0).min(100.0);
    }
    if toxicity > 0.0 {
        c.health -= toxicity * 0.25;
    }
    true
}

/// A ZORG feeds on the rock it chews.
///
/// The tunnel it digs upwards is a meal as well as a way out. It does not
/// eat its way down through the floor of its own cave, though: that would
/// swallow the larder whole.
pub fn zorg_feed_on_rock(c: &mut Creature, elem: Elem, digging_up: bool) {
    if !digging_up {
        return;
    }
    let n = zorg_nutrition(elem);
    if n <= 0.0 {
        return;
    }
    if c.hunger <= 20.0 && c.energy >= c.max_energy * 0.5 {
        return;
    }
    c.energy = (c.energy + n * 4.0).min(c.max_energy);
    c.hunger = (c.hunger - n * 4.0).max(0.0);
}

/// Where a ZORG wants to go.
/// A ZORG belongs to the deep, but it is a predator of the surface, so it
/// pushes upward: out of the caves and into the world. It also does not
/// care about its own kind and it is never afraid of anything.
///
/// The cave in the deep is its home and its larder, though. A hungry ZORG
/// goes down to it and feeds on the poison pooled on its floor; a fed one
/// goes up through the rock after the colonies that live in the light.
pub fn zorg_steer(c: &Creature) -> (f32, f32, f32) {
    let hx = c.base_x - c.x;
    let hy = c.base_y - c.y;
    let hd = (hx * hx + hy * hy).sqrt().max(0.1);
    // It needs to feed when it is hungry, and it needs to feed when it is
    // tired: the rock of the deep is its bread as well as its feast.
    let needs_food = c.hunger >= 45.0 || c.energy < c.max_energy * 0.45;
    if needs_food {
        // It goes home, to the larder on the floor of the cave.
        (hx / hd * 1.8, hy / hd * 1.8, 1.1)
    } else {
        // It has eaten: it pushes up through the rock towards the world.
        (0.0, -0.55, 1.1)
    }
}

/// The species id of the ZORGs.
///
/// The ZORGs are reserved id zero: every player species is numbered from
/// one upwards, so id zero is free for the thing that is not a player.
pub const ZORG_SPECIES: u32 = 0;

/// How many ZORGs are waiting in the cave at the start of a run.
pub const ZORG_START: usize = 8;

/// How many ZORGs there can be at once.
///
/// The deep is finite: the ZORGs breed until they fill it, and then they
/// stop. They are a threat the players have to meet and beat, not a flood
/// that ends every run on its own.
pub const ZORG_POP_CAP: u32 = 90;

/// How many ZORGs the deep will not let die out.
///
/// The ZORGs are not a colony that can be wiped out: the deep keeps
/// breeding them in the cave, and as long as the cave is there there is
/// always something in it. A player can thin them out, but never empty
/// the deep for good.
pub const ZORG_MIN: u32 = 6;

/// The filth a ZORG leaves behind it.
///
/// A ZORG walks through the world and the world is dirtier for it: the
/// ground it crosses is poisoned and seeded with the little living things
/// that break down what is left, and the chlorine it leaves behind rises
/// up through the rock towards the light. This is how the deep spreads its
/// corruption into the surface colonies.
pub fn zorg_filth(roll: u32) -> Elem {
    match roll % 8 {
        0 => Elem::Virus,
        1 => Elem::Virus,
        2 => Elem::Virus,
        3 => Elem::Bacteria,
        4 => Elem::Bacteria,
        5 => Elem::Chlorine,
        6 => Elem::Poison,
        _ => Elem::Tar,
    }
}

/// The sickness a ZORG carries.
///
/// Every ZORG is filthy: it lives in the poison of the deep and it wears
/// the disease it was born with. It does not suffer from it - it is the
/// source of the sickness, not its victim - but anything that stands
/// close to it can catch what it is carrying.
pub fn zorg_carried_disease(roll: u32) -> Disease {
    match roll % 5 {
        0 => Disease::Plague,
        1 => Disease::Infection,
        2 => Disease::Parasite,
        3 => Disease::Madness,
        _ => Disease::Flu,
    }
}

/// Is this element something a ZORG can eat?
///
/// A ZORG eats rock and ore, and it feeds on the things that poison a
/// player: acid, tar, mercury, the raw stuff of the deep.
pub fn zorg_food(e: Elem) -> bool {
    use Elem::*;
    matches!(
        e,
        Stone
            | Granite
            | Obsidian
            | Sand
            | Salt
            | Sulfur
            | Ash
            | Dust
            | Mud
            | Metal
            | Copper
            | Tin
            | Bronze
            | Iron
            | Steel
            | Gold
            | Silver
            | Rust
            | Crystal
            | Gem
            | Diamond
            | Acid
            | Poison
            | Tar
            | Mercury
            | Chlorine
            | Oil
            | Corpse
            | Meat
            | Bone
    )
}

/// How much a ZORG gets out of a thing.
///
/// Rock is poor food and poison is rich: the things that would kill a
/// player fill a ZORG up. The rock is still bread: it has to be enough to
/// keep a ZORG alive on the long stretch between one feast and the next.
pub fn zorg_nutrition(e: Elem) -> f32 {
    use Elem::*;
    match e {
        Acid => 55.0,
        Poison => 45.0,
        Tar => 32.0,
        Mercury => 30.0,
        Chlorine => 28.0,
        Oil => 26.0,
        Sulfur => 22.0,
        Rust => 18.0,
        Gold => 16.0,
        Silver => 14.0,
        Crystal | Gem | Diamond => 14.0,
        Iron | Steel => 12.0,
        Bronze | Copper | Tin | Metal => 10.0,
        Bone => 9.0,
        Corpse => 26.0,
        Meat => 24.0,
        Granite => 16.0,
        Obsidian => 13.0,
        Stone => 14.0,
        Sand | Salt | Ash | Dust | Mud => 8.0,
        _ => 0.0,
    }
}

/// Is this element poison to a ZORG?
///
/// A ZORG is at home in the deep: the toxic things of the world do it no
/// harm at all, and some of them are its food.
pub fn zorg_toxicity(e: Elem) -> f32 {
    let _ = e;
    0.0
}
