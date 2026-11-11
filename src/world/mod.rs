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
//! - physics.rs            the per-step cell update
//! - behaviors.rs          how each element moves
//! - reactions.rs          chemical reactions and explosions
//! - maintain.rs           weather, regrowth and world upkeep
//! - census.rs             population statistics
//! - query.rs              queries used by the creature AI

mod behaviors;
mod cell;
mod census;
mod elements_extreme;
mod elements_placement;
mod generation;
mod maintain;
mod physics;
mod query;
mod reactions;
mod structures;
mod terrain;
mod vaults;

use crate::elements::{Behavior, Elem};
use crate::rng::Rng;

/// The climate bands that run across the world.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    Tundra,
    Forest,
    Grassland,
    Swamp,
    Desert,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Tundra => "Tundra",
            Biome::Forest => "Forest",
            Biome::Grassland => "Grassland",
            Biome::Swamp => "Swamp",
            Biome::Desert => "Desert",
        }
    }

    /// Temperature offset applied to the world temperature.
    pub fn temperature_offset(self) -> f32 {
        match self {
            Biome::Tundra => -14.0,
            Biome::Forest => -2.0,
            Biome::Grassland => 2.0,
            Biome::Swamp => 4.0,
            Biome::Desert => 12.0,
        }
    }

    /// Tint applied to the terrain of this biome.
    pub fn tint(self) -> [f32; 3] {
        match self {
            Biome::Tundra => [0.80, 0.93, 1.14],
            Biome::Forest => [0.80, 1.05, 0.84],
            Biome::Grassland => [1.02, 1.04, 0.90],
            Biome::Swamp => [0.84, 0.98, 0.80],
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
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Rain => "Rain",
            Weather::Storm => "Storm",
            Weather::Snow => "Snowfall",
            Weather::Meteor => "Meteor Shower",
        }
    }
}

pub const DIRS4: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
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
    /// Current weather and how long it lasts.
    pub weather: Weather,
    pub weather_timer: u32,
    /// Recent meteor impacts (x, y) so the renderer can flash explosions.
    pub meteor_events: Vec<(f32, f32)>,
    /// Frames left of a lightning flash (storms only).
    pub lightning_flash: u32,
    pub rng: Rng,
    pub frame: u64,
    /// Per-step budget for destructive resource gathering so creatures
    /// cannot strip-mine the entire world. Reset at the start of each step.
    pub mine_budget: i32,
    pub census: Census,
}


