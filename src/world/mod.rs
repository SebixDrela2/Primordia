//! World module - the cellular automaton the creatures live in.
//!
//! Split by responsibility:
//! - mod.rs                shared types (World, Biome, Weather, Census)
//! - cell.rs               cell access, movement and swapping
//! - terrain.rs            surface shape, biomes, cached ground level
//! - generation.rs         world generation (terrain, water, ores, caves)
//! - elements_placement.rs element placement (museum, diversity)
//! - elements_extreme.rs   rare / extreme element placement
//! - vaults.rs             rare vaults
//! - structures.rs         generated ruins and surface structures
//! - landmarks.rs          the places worth living in
//! - physics.rs            the per-step cell update
//! - behaviors.rs          how each element moves
//! - reactions.rs          chemical reactions and explosions
//! - maintain.rs           weather, regrowth and world upkeep
//! - census.rs             population statistics
//! - query.rs              queries used by the creature AI

mod behaviors;
mod cell;
mod census;
mod depth;
mod elements_extreme;
mod elements_placement;
mod generation;
mod landmarks;
mod maintain;
mod physics;
mod query;
mod reactions;
mod structures;
mod quakes;
mod terrain;
mod volcano;
mod vaults;
mod zorg_cave;

pub use depth::*;
pub use landmarks::*;
pub use zorg_cave::*;

use crate::elements::{Behavior, Elem};
use crate::rng::Rng;
pub use volcano::Volcano;

/// The climate bands that run across the world.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    Tundra,
    /// Scorched ground: the ground the volcano has already burned over.
    Ashland,
    Forest,
    /// A flowering wood: the same as a forest, but in bloom.
    Bloomwood,
    Grassland,
    Swamp,
    /// A fungal mire: wet ground, spore air and a great deal of fungus.
    Mire,
    Desert,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Tundra => "Tundra",
            Biome::Ashland => "Ashland",
            Biome::Forest => "Forest",
            Biome::Bloomwood => "Bloomwood",
            Biome::Grassland => "Grassland",
            Biome::Swamp => "Swamp",
            Biome::Mire => "Mire",
            Biome::Desert => "Desert",
        }
    }

    /// Temperature offset applied to the world temperature.
    pub fn temperature_offset(self) -> f32 {
        match self {
            Biome::Tundra => -14.0,
            Biome::Ashland => 16.0,
            Biome::Forest => -2.0,
            Biome::Bloomwood => -1.0,
            Biome::Grassland => 2.0,
            Biome::Swamp => 4.0,
            Biome::Mire => 6.0,
            Biome::Desert => 12.0,
        }
    }

    /// Tint applied to the terrain of this biome.
    pub fn tint(self) -> [f32; 3] {
        match self {
            Biome::Tundra => [0.80, 0.93, 1.14],
            Biome::Ashland => [1.10, 0.86, 0.78],
            Biome::Forest => [0.80, 1.05, 0.84],
            Biome::Bloomwood => [1.02, 0.98, 1.06],
            Biome::Grassland => [1.02, 1.04, 0.90],
            Biome::Swamp => [0.84, 0.98, 0.80],
            Biome::Mire => [0.80, 0.92, 0.72],
            Biome::Desert => [1.16, 1.0, 0.76],
        }
    }
}

/// The weather rolling over the world.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Storm,
    Snow,
    Meteor,
    /// A fog: the world closes in and nothing can see far.
    Fog,
    /// A heatwave: the sun beats down and everything burns a little.
    Heatwave,
    /// A sandstorm: the wind carries the desert itself across the world.
    Sandstorm,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Rain => "Rain",
            Weather::Storm => "Storm",
            Weather::Snow => "Snowfall",
            Weather::Meteor => "Meteor Shower",
            Weather::Fog => "Fog",
            Weather::Heatwave => "Heatwave",
            Weather::Sandstorm => "Sandstorm",
        }
    }

    /// How fast a creature moves in this weather.
    ///
    /// The wind and the cold slow everything down, and a storm slows it
    /// most of all.
    pub fn speed_factor(self) -> f32 {
        match self {
            Weather::Clear => 1.0,
            Weather::Rain => 0.9,
            Weather::Storm => 0.72,
            Weather::Snow => 0.84,
            Weather::Meteor => 0.9,
            Weather::Fog => 0.88,
            Weather::Heatwave => 0.95,
            Weather::Sandstorm => 0.78,
        }
    }

    /// What the weather costs a creature in energy.
    ///
    /// The figures are multipliers on the ordinary energy drain, so a
    /// stormy day is a hungry day.
    pub fn energy_factor(self) -> f32 {
        match self {
            Weather::Clear => 1.0,
            Weather::Rain => 1.3,
            Weather::Storm => 2.1,
            Weather::Snow => 1.7,
            Weather::Meteor => 1.2,
            Weather::Fog => 1.1,
            Weather::Heatwave => 1.6,
            Weather::Sandstorm => 1.5,
        }
    }

    /// How much the weather frightens a creature.
    ///
    /// A meteor shower is terrifying: the animals run further and sooner.
    pub fn fear_factor(self) -> f32 {
        match self {
            Weather::Clear => 0.0,
            Weather::Rain => 0.0,
            Weather::Storm => 8.0,
            Weather::Snow => 2.0,
            Weather::Meteor => 22.0,
            Weather::Fog => 3.0,
            Weather::Heatwave => 5.0,
            Weather::Sandstorm => 6.0,
        }
    }

    /// Whether the creatures are looking for shelter.
    ///
    /// In a storm they huddle near their colony instead of wandering.
    pub fn shelters(self) -> bool {
        matches!(
            self,
            Weather::Storm | Weather::Meteor | Weather::Heatwave
        )
    }

    /// How much the rain quenches a creature's thirst.
    pub fn thirst_factor(self) -> f32 {
        match self {
            Weather::Rain => 1.8,
            Weather::Storm => 2.2,
            Weather::Heatwave => 2.6,
            Weather::Sandstorm => 0.7,
            _ => 1.0,
        }
    }

    /// How far a creature can see in this weather.
    ///
    /// A fog closes the world in: nothing sees far, and the colonies stop
    /// noticing what is coming until it is on top of them.
    pub fn vision_factor(self) -> f32 {
        match self {
            Weather::Fog => 0.45,
            Weather::Snow => 0.85,
            Weather::Storm => 0.8,
            Weather::Meteor => 0.9,
            // A sandstorm closes the world in just as a fog does, and it
            // is harder on the eyes because it moves.
            Weather::Sandstorm => 0.5,
            _ => 1.0,
        }
    }

    /// How much the weather changes the temperature.
    pub fn temperature_offset(self) -> f32 {
        match self {
            Weather::Snow => -6.0,
            Weather::Storm => -4.0,
            Weather::Fog => -2.0,
            Weather::Heatwave => 11.0,
            Weather::Sandstorm => 4.0,
            _ => 0.0,
        }
    }
}

pub const DIRS4: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

/// How far a farm has to reach for water to be able to work.
///
/// The same radius is used when a plot is chosen and when it is worked, so
/// a farm that can be built is always a farm that can grow.
pub const FARM_WATER_RADIUS: i32 = 6;
pub const DIRS8: [(i32, i32); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

/// The environmental pressure on the world.
///
/// Difficulty is deliberately **not** one number that gets multiplied into
/// everything. Each kind of pressure rises for its own reason, is driven by
/// its own part of the world, and hurts in its own way. The player can watch
/// the HUD and see exactly which pressure is rising and why.
#[derive(Clone, Copy, Default)]
pub struct Pressure {
    /// Food and water are running out: the surface is being picked clean.
    /// Rises while the vegetation is below what the world used to support,
    /// and recovers slowly when the vegetation comes back.
    pub scarcity: f32,
    /// The ore is being mined faster than the geology can replace it.
    /// Rises every time ore forms, because every vein that appears is one
    /// step closer to the end of the world's metal.
    pub depletion: f32,
    /// The weather is turning violent: storms and meteors grow more frequent.
    /// Rises with the time the world spends in violent weather.
    pub harshness: f32,
    /// The temperature swings wider: harder winters, fiercer summers.
    /// Rises with every season that turns over.
    pub extremes: f32,
    /// The deep rock is heating up: more magma, more lava, less safe mining.
    /// Rises steadily as the run goes on.
    pub volcanic: f32,
}

impl Pressure {
    /// How hard the world is overall, in the range 0..=1. This is a summary
    /// for the HUD only; the simulation uses the individual pressures.
    pub fn overall(&self) -> f32 {
        let sum = self.scarcity + self.depletion + self.harshness + self.extremes + self.volcanic;
        (sum / 5.0).clamp(0.0, 1.0)
    }
}

#[derive(Default)]
pub struct Census {
    pub particles: u32,
    pub plants: u32,
    pub water: u32,
    pub energy: u32,
    pub stone: u32,
    pub metal: u32,
    pub copper: u32,
    pub gold: u32,
    pub crystal: u32,
    pub gem: u32,
    pub wood: u32,
    pub oil: u32,
    pub sulfur: u32,
}

pub struct World {
    pub w: usize,
    pub h: usize,
    pub kind: Vec<u8>,
    pub col: Vec<[u8; 3]>,
    pub heat: Vec<u8>,
    pub age: Vec<u32>,
    /// Ground level per column (topmost supported solid cell). Cached so the
    /// renderer does not have to scan every column on every frame.
    pub ground: Vec<i32>,
    /// Reciprocal of the ground level per column. The renderer uses this to
    /// turn the per-cell sky gradient into a multiply.
    pub ground_inv: Vec<f32>,
    /// Cached per-column biome tint. The biome layout never changes, so the
    /// renderer reads this instead of recomputing the blend every frame.
    pub biome_tint: Vec<[f32; 3]>,
    /// Cached per-column biome, used for auroras and the HUD.
    pub biome: Vec<Biome>,
    /// Indices of the cells that can still change. Rebuilt each step from a
    /// cheap scan so the simulation never touches the (vast) inert bulk of
    /// the world.
    pub active: Vec<u32>,
    /// Scratch list of cells that became active while the current step was
    /// being processed.
    next_active: Vec<u32>,
    /// Per-cell flag: is this cell already queued in `next_active`?
    listed: Vec<bool>,
    /// Lookup: is this element byte completely inert?
    pub inert: [bool; Elem::TABLE_LEN],
    /// Lookup: can this element be eaten (and does it feed)?
    food_table: [bool; Elem::TABLE_LEN],
    /// Lookup: can this element be gathered as a resource?
    gatherable: [bool; Elem::TABLE_LEN],
    /// Lookup: does this element radiate light?
    pub emissive: [bool; Elem::TABLE_LEN],
    /// Lookup: the light colour an element radiates.
    pub emission: [[f32; 3]; Elem::TABLE_LEN],
    /// Lookup: can a ZORG eat this element? A ZORG eats rock and poison,
    /// which is the exact opposite of what a player eats.
    pub zorg_food_table: [bool; Elem::TABLE_LEN],
    /// Lookup: how much a ZORG gets out of this element.
    pub zorg_nutrition_table: [f32; Elem::TABLE_LEN],
    /// The row of the floor of the ZORG cave. The ZORGs keep to it: it is
    /// the one level place in the deep where the poison pools are.
    pub zorg_cave_floor: i32,
    /// The volcano: where it is and what it is doing.
    pub volcano: Option<(i32, i32)>,
    pub volcano_state: Volcano,
    pub volcano_timer: u32,
    /// The season the world is in, kept by the creature clock.
    /// Current weather and how long it lasts.
    pub season: u8,
    /// How much daylight there is, from 0.0 at midnight to 1.0 at noon.
    /// The plants use it to breathe, and the fire uses what they leave.
    pub daylight: f32,
    pub weather: Weather,
    pub weather_timer: u32,
    /// Recent meteor impacts (x, y) so the renderer can flash explosions.
    pub meteor_events: Vec<(f32, f32, u32)>,
    pub rng: Rng,
    pub frame: u64,
    /// Per-step budget for destructive resource gathering so creatures
    /// cannot strip-mine the entire world. Reset at the start of each step.
    pub mine_budget: i32,
    /// The environmental pressure on the world. See `Pressure`.
    pub pressure: Pressure,
    /// How many plants this world has recently been able to support.
    ///
    /// The surface does not have a fixed carrying capacity: a wet world grows
    /// more vegetation than a dry one, and a world that has been grazed hard
    /// takes time to recover. Scarcity is therefore measured against this
    /// learned high-water mark rather than against a hard-coded number, so
    /// the pressure reflects the world's *own* productivity being eaten into.
    pub plant_capacity: f32,
    pub census: Census,
    /// The places worth living in. A colony that settles beside one of
    /// them does better than a colony that does not.
    pub landmarks: Vec<LandmarkSite>,
    /// The heart of the ZORG cave: the chamber in the deep that belongs to
    /// the ZORGs. Every founding ZORG is born in it.
    pub zorg_cave: Option<(i32, i32)>,
    /// Has the ZORG cave been sealed?
    ///
    /// A ward of sanctified fire raised inside the cave closes it, and the
    /// deep can breed no more of them. This is the only way a run can
    /// actually end against the ZORGs.
    pub zorg_cave_sealed: bool,
    /// How much deeper the colonies are digging.
    ///
    /// The best mining any species knows about, as a number of extra cells
    /// the world lets itself be dug into in a tick.
    pub deep_mining: u32,
}


