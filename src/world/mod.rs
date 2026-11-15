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
pub use zorg_cave::*;
pub use landmarks::*;

pub mod counters {
    use std::sync::atomic::AtomicU32;

    pub static MEALS: [AtomicU32; 256] = [const { AtomicU32::new(0) }; 256];

    pub static CORPSE_REFUSED: AtomicU32 = AtomicU32::new(0);

    pub static CORPSE_EATEN: AtomicU32 = AtomicU32::new(0);
}

use crate::elements::{Behavior, Elem};
use crate::rng::Rng;
pub use volcano::Volcano;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    Tundra,

    Ashland,
    Forest,

    Bloomwood,
    Grassland,
    Swamp,

    Mire,
    Desert,
}

impl Biome {
    #[must_use]
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

    #[must_use]
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

    #[must_use]
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Storm,
    Snow,
    Meteor,

    Fog,

    Heatwave,

    Sandstorm,
}

impl Weather {
    #[must_use]
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

    #[must_use]
    pub fn speed_factor(self) -> f32 {
        match self {
            Weather::Clear => 1.0,
            Weather::Rain | Weather::Meteor => 0.9,
            Weather::Storm => 0.72,
            Weather::Snow => 0.84,
            Weather::Fog => 0.88,
            Weather::Heatwave => 0.95,
            Weather::Sandstorm => 0.78,
        }
    }

    #[must_use]
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

    #[must_use]
    pub fn fear_factor(self) -> f32 {
        match self {
            Weather::Clear | Weather::Rain => 0.0,
            Weather::Storm => 8.0,
            Weather::Snow => 2.0,
            Weather::Meteor => 22.0,
            Weather::Fog => 3.0,
            Weather::Heatwave => 5.0,
            Weather::Sandstorm => 6.0,
        }
    }

    #[must_use]
    pub fn shelters(self) -> bool {
        matches!(
            self,
            Weather::Storm | Weather::Meteor | Weather::Heatwave
        )
    }

    #[must_use]
    pub fn thirst_factor(self) -> f32 {
        match self {
            Weather::Rain => 1.8,
            Weather::Storm => 2.2,
            Weather::Heatwave => 2.6,
            Weather::Sandstorm => 0.7,
            _ => 1.0,
        }
    }

    #[must_use]
    pub fn vision_factor(self) -> f32 {
        match self {
            Weather::Fog => 0.45,
            Weather::Snow => 0.85,
            Weather::Storm => 0.8,
            Weather::Meteor => 0.9,

            Weather::Sandstorm => 0.5,
            _ => 1.0,
        }
    }

    #[must_use]
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

#[derive(Clone, Copy, Default)]
pub struct Pressure {

    pub scarcity: f32,

    pub depletion: f32,

    pub harshness: f32,

    pub extremes: f32,

    pub volcanic: f32,
}

impl Pressure {

    #[must_use]
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

    pub owner: Vec<u8>,
    pub col: Vec<[u8; 3]>,
    pub heat: Vec<u8>,
    pub age: Vec<u32>,

    pub ground: Vec<i32>,

    pub ground_inv: Vec<f32>,

    pub biome_tint: Vec<[f32; 3]>,

    pub biome: Vec<Biome>,

    pub active: Vec<u32>,

    next_active: Vec<u32>,

    listed: Vec<bool>,

    pub inert: [bool; Elem::TABLE_LEN],

    food_table: [bool; Elem::TABLE_LEN],

    gatherable: [bool; Elem::TABLE_LEN],

    pub emissive: [bool; Elem::TABLE_LEN],

    pub emission: [[f32; 3]; Elem::TABLE_LEN],

    pub zorg_food_table: [bool; Elem::TABLE_LEN],

    pub zorg_nutrition_table: [f32; Elem::TABLE_LEN],

    pub zorg_cave_floor: i32,

    pub volcano: Option<(i32, i32)>,
    pub volcano_state: Volcano,
    pub volcano_timer: u32,

    pub season: u8,

    pub daylight: f32,
    pub weather: Weather,
    pub weather_timer: u32,

    pub meteor_events: Vec<(f32, f32, u32)>,
    pub rng: Rng,
    pub frame: u64,

    pub mine_budget: i32,

    pub pressure: Pressure,

    pub difficulty: f32,

    pub plant_capacity: f32,
    pub census: Census,

    pub landmarks: Vec<LandmarkSite>,

    pub zorg_cave: Option<(i32, i32)>,

    pub zorg_cave_sealed: bool,

    pub deep_mining: u32,
}
