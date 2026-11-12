//! Creature module - the agents that live in the world.
//!
//! Split by responsibility:
//! - mod.rs        shared types (Creature, CreatureSystem, events, genome)
//! - brain.rs      the tiny feed-forward network
//! - enums.rs      roles, tools, diseases, spells, moods
//! - names.rs      species names and colours
//! - senses.rs     what a creature perceives
//! - colony.rs     spawning, reproduction and lineages
//! - step.rs       the simulation step (the behaviour loop)
//! - actions.rs    the individual behaviours a creature can pick
//! - combat.rs     projectiles, bombs, turrets and deaths
//! - diplomacy.rs  relations, treaties and negotiations between species
//! - stats.rs      population statistics

mod actions;
mod brain;
mod building;
mod colony;
mod combat;
mod diplomacy;
mod enums;
mod heritage;
mod mating;
mod names;
mod personality;
mod plague;
mod senses;
mod speciation;
mod stats;
mod step;

use crate::elements::Elem;
use crate::rng::Rng;
use crate::world::{World, DIRS4, DIRS8};

pub const INPUTS: usize = 252;
pub const OUTPUTS: usize = 25;

/// The network outputs, in order.
///
/// The first twenty are the original behaviour outputs:
///
/// ```text
///  0 move_x      1 move_y      2 eat         3 reproduce
///  4 attack      5 build       6 craft       7 cast_spell
///  8 mount       9 trade      10 rest       11 switch_spell
/// 12 dig/mine   13 gather_water 14 shoot    15 flee
/// 16 seek       17 forage     18 rally      19 defend
/// ```
///
/// The last five drive the diplomacy and courtship sectors.
pub mod outputs {
    /// Open talks with a foreign creature.
    pub const NEGOTIATE: usize = 20;
    /// Make a hostile move.
    pub const HOSTILE: usize = 21;
    /// Offer an alliance or a pact.
    pub const ALLY: usize = 22;
    /// Demand tribute.
    pub const TRIBUTE: usize = 23;
    /// Court a mate of its own kind.
    pub const SEEK_MATE: usize = 24;
}
const MAX_POP: usize = 500;
/// Above this population creatures stop reproducing so the world stays lively
/// without grinding to a halt (the hard cap is still MAX_POP).
const REPRO_SOFT_CAP: usize = 320;
/// Creatures that reach this age die of old age, driving generational turnover.
const MAX_AGE: u32 = 3200;
const BASE_RADIUS: i32 = 40;
const BUILD_COST: f32 = 80.0;
const CARRY_CAP: f32 = 400.0;
const MANA_REGEN: f32 = 0.15;
const STAMINA_REGEN: f32 = 0.2;

/// The energy cost of every action a creature can take.
///
/// Actions used to be free, so a creature could mine, build, craft and fight
/// without ever needing to eat. Now everything costs energy, which is what
/// makes foraging and the food web matter.
pub mod energy_cost {
    /// Moving at all.
    pub const MOVE: f32 = 0.020;
    /// Shoving through loose debris while moving.
    pub const SHOVE: f32 = 0.35;
    /// Attacking in melee.
    pub const ATTACK: f32 = 6.0;
    /// Loosing a shot.
    pub const SHOOT: f32 = 4.0;
    /// Digging a tunnel through rock.
    pub const DIG: f32 = 5.0;
    /// Extracting ore or rock from the ground.
    pub const MINE: f32 = 3.0;
    /// Raising a structure.
    pub const BUILD: f32 = 8.0;
    /// Crafting a tool, bomb or salve.
    pub const CRAFT: f32 = 5.0;
    /// Throwing a bomb.
    pub const BOMB: f32 = 6.0;
    /// Using a salve on a wounded colonymate.
    pub const SALVE: f32 = 4.0;
    /// Making a trade.
    pub const TRADE: f32 = 2.0;
    /// Casting a spell.
    pub const SPELL: f32 = 10.0;
}

/// How many farms a single colony may keep.
///
/// Each plot takes 4 wood and a builder's time, and a plot that nobody
/// tends is just a field of weeds. A handful is enough to feed a colony
/// and keeps the wood for other things.
pub const MAX_FARMS: usize = 4;

/// How fast a creature may move, in cells per step.
///
/// The old limit was 2.6, which made a creature cross the whole screen in
/// well under a second: it looked like zooming rather than walking.
pub const MAX_SPEED: f32 = 0.9;
/// The speed multiplier a mounted creature gets.
pub const MOUNT_SPEED_BONUS: f32 = 1.5;

/// Distance from a point to a line segment. Used for swept projectile hits so

pub struct Projectile {
    pub x: f32,
    pub y: f32,
    pub px: f32,
    pub py: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u32,
    pub damage: f32,
    pub species: u32,
    pub color: [u8; 3],
    pub elem: Elem,
}

/// A player-built structure that remembers who owns it, so turrets know
/// which creatures to shoot at.
pub struct Structure {
    pub x: f32,
    pub y: f32,
    pub species: u32,
    pub color: [u8; 3],
    pub hp: f32,
    pub cooldown: u32,
    pub armed: bool,
    /// 0 = generic, 1 = turret, 2 = temple (grants morale to its colony),
    /// 3 = farm (a plot that grows crops).
    pub kind: u8,
}

/// A thrown bomb: arcs towards its target and detonates.
pub struct Bomb {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u32,
    pub owner: u32,
    pub color: [u8; 3],
}

/// Things that happened, so the renderer can animate them.
pub enum SimEvent {
    Hit { x: f32, y: f32, color: [u8; 3], damage: f32 },
    Shoot { x: f32, y: f32, color: [u8; 3] },
    Craft { x: f32, y: f32, color: [u8; 3] },
    Build { x: f32, y: f32, color: [u8; 3] },
    Trade { x: f32, y: f32, color: [u8; 3] },
    Birth { x: f32, y: f32, color: [u8; 3] },
    Death { x: f32, y: f32, color: [u8; 3], size: f32 },
    Heal { x: f32, y: f32, color: [u8; 3] },
    Explosion { x: f32, y: f32, color: [u8; 3] },
    /// A diplomatic moment: a treaty signed, a tribute demanded, talks held.
    Treaty { x: f32, y: f32, color: [u8; 3], label: String },
    /// War has been declared between two species.
    War { x: f32, y: f32, color: [u8; 3] },
    /// A group of animals has left its colony and become a species of
    /// its own.
    Speciation { x: f32, y: f32, color: [u8; 3] },
    /// A disease has broken out in a colony.
    Plague { x: f32, y: f32, color: [u8; 3] },
}

/// Uniform spatial hash so neighbour queries do not scan the whole colony.

pub struct Creature {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub health: f32,
    pub age: u32,
    pub net: Net,
    pub color: [u8; 3],
    pub species: u32,
    pub generation: u32,
    pub fitness: f32,
    pub offspring: u32,
    pub kills: u32,
    pub cooldown: u32,
    /// Cooldown between ranged shots.
    pub fire_cooldown: u32,
    pub size: f32,
    pub last_x: f32,
    pub last_y: f32,
    // Identity
    pub name: [u8; 10], // null-terminated name
    pub team_color: [u8; 3], // species color for rendering
    // Colony fields
    pub role: Role,
    pub base_x: f32,
    pub base_y: f32,
    pub carried: f32,
    pub build_cooldown: u32,
    pub target_x: f32,
    pub target_y: f32,
    // Tech/Tools
    pub tool: Tool,
    /// Bombs and healing salves the creature has crafted.
    pub bombs: u8,
    pub salves: u8,
    pub tool_durability: u16,
    pub tech_level: u8,
    pub inventory: [u16; 16],
    pub craft_cooldown: u32,
    // Social
    pub disease: Disease,
    pub disease_timer: u16,
    pub immunity: [u8; 8],
    pub belief: Belief,
    pub devotion: u8,
    pub mood: Mood,
    pub mood_timer: u16,
    pub loyalty: i8,
    pub reputation: i16,
    pub trade_partner: Option<u32>,
    pub squad_id: u16,
    pub squad_role: u8,
    // Magic
    pub mana: f32,
    pub max_mana: f32,
    pub spell_cooldown: u16,
    pub known_spells: u16,
    pub active_spell: Spell,
    pub spell_cd: [u16; 10],
    // Vehicle/Mount
    pub mount: Option<u32>,
    pub vehicle: Option<u32>,
    pub mount_kind: MountKind,
    pub mount_stamina: f32,
    /// Genetics
    pub genes: [f32; 32],
    pub mutations: u16,
    /// Evolutionary traits decoded from the genes.
    pub traits: u16,
    /// The individual's temperament: the part of it that is not a species
    /// trait but a personal one.
    pub personality: Personality,
    /// Which stage of life it is in.
    pub stage: Stage,
    /// What its species has become adapted to.
    pub affinity: Affinity,
    /// What its species has learned to do.
    pub culture: u16,
    /// The mate it is courting, if any.
    pub mate: Option<u32>,
    /// How many children it has sired with a mate.
    pub sired: u32,
    /// This animal has left its colony and is on its own.
    pub migrating: bool,
    // Needs
    pub hunger: f32,
    pub thirst: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub resting: bool,
    // Memory (tiny blackboard for learned behavior)
    pub memory: [f32; 8],
    // Diplomacy
    /// Ticks until this creature is willing to make another diplomatic move.
    pub diplo_cooldown: u16,
    /// How far the current negotiation has progressed (0..1).
    pub diplo_progress: f32,
    /// The species this creature is currently negotiating with.
    pub diplo_partner: Option<u32>,
    /// The creature this one is negotiating with.
    pub diplo_with: Option<u32>,
    /// How many treaties this creature has personally concluded.
    pub pacts_signed: u32,
}

#[derive(Default)]
pub struct CreatureStats {
    pub population: u32,
    pub species: u32,
    pub avg_fitness: f32,
    pub avg_energy: f32,
    pub max_generation: u32,
    pub births: u32,
    pub deaths: u32,
    pub workers: u32,
    pub soldiers: u32,
    pub queens: u32,
    pub priests: u32,
    pub merchants: u32,
    pub builders: u32,
    pub explorers: u32,
    pub shamans: u32,
    pub envoys: u32,
    pub tools_pickaxe: u32,
    pub tools_sword: u32,
    pub tools_hammer: u32,
    pub tools_drill: u32,
    pub tools_scanner: u32,
    pub tools_bow: u32,
    pub tools_spear: u32,
    pub tools_shield: u32,
    pub tools_staff: u32,
    pub tools_amulet: u32,
    pub tools_backpack: u32,
    pub tools_cart: u32,
    pub tools_boat: u32,
    pub tools_glider: u32,
    pub tech_stone: u32,
    pub tech_copper: u32,
    pub tech_iron: u32,
    pub tech_gold: u32,
    pub tech_crystal: u32,
    pub avg_mana: f32,
    pub avg_loyalty: f32,
    pub diseases: u32,
    pub spells_cast: u32,
    pub trades: u32,
    pub mounts: u32,
    pub structures: u32,
    pub avg_hunger: f32,
    pub avg_thirst: f32,
    pub avg_stamina: f32,
    pub shots: u32,
    pub towers: u32,
    /// How many creatures are still young.
    pub juveniles: u32,
    /// How many creatures are old.
    pub elders: u32,
    /// How many pairs were made in the last run of ticks.
    pub matings: u32,
    /// How many species have split off from another.
    pub splits: u32,
    /// How many practices the species have learned.
    pub practices: u32,
}

pub struct CreatureSystem {
    pub list: Vec<Creature>,
    pub rng: Rng,
    next_id: u32,
    next_species: u32,
    pub tick: u64,
    pub stats: CreatureStats,
    births_total: u32,
    deaths_total: u32,
    pub eats_total: u64,
    pub builds_total: u64,
    pub trades_total: u64,
    pub spells_cast: u64,
    /// How many pairs have mated in this run.
    pub matings_total: u32,
    /// How many young have been born to two parents.
    pub two_parent_births: u32,
    next_squad: u16,
    vehicles: Vec<Vehicle>,
    next_vehicle: u32,
    pub day_night: f32, // 0.0 = midnight, 0.5 = noon, 1.0 = midnight
    pub temperature: f32, // world temperature affecting creatures
    pub season: u8, // 0=spring,1=summer,2=autumn,3=winter
    pub season_timer: u64,
    /// Shots currently in flight.
    pub projectiles: Vec<Projectile>,
    /// Player-built structures (turrets) with an owner.
    pub structures: Vec<Structure>,
    /// Events for the renderer, drained every frame by the UI.
    pub events: Vec<SimEvent>,
    /// Bombs currently in flight.
    pub bombs: Vec<Bomb>,
    /// The relations between the species of this world.
    pub diplomacy: Diplomacy,
    /// What each species has collectively learned, by species id.
    ///
    /// A practice that one animal discovers is remembered by the colony
    /// and passed on to the young, so a species that learns to farm stays a
    /// farming species even after the individual that worked it out dies.
    pub culture_pool: Vec<(u32, u16)>,
    /// The groups that have left their colonies to found a new species.
    pub migrations: Vec<Migration>,
    /// How many species have split off from another in this run.
    pub splits_total: u32,
    /// How many groups have left their colonies in this run.
    pub migrations_started: u32,
    /// How many disease outbreaks have begun in this run.
    pub plagues_started: u32,
}

struct Vehicle {
    id: u32,
    x: f32,
    y: f32,
    vtype: VehicleType,
    owner: u32,
    capacity: u8,
    passengers: Vec<u32>,
    health: f32,
    fuel: f32,
}


pub enum CreatureGenesis<'a> {
    Random,
    Child(&'a Creature),
    /// A descendant of a previous run's dominant species. The genome is cloned
    /// with extra mutation so the winning lineage diversifies again.
    Revival(&'a RevivalGenome),
}

/// Snapshot of the winning species' best genome, carried into the next run.
pub struct RevivalGenome {
    pub net: Net,
    pub color: [u8; 3],
    pub genes: [f32; 32],
    pub size: f32,
    pub generation: u32,
}

pub use actions::*;
pub use brain::*;
pub use building::*;
pub use diplomacy::*;
pub use enums::*;
pub use heritage::*;
pub use mating::*;
pub use names::*;
pub use personality::*;
pub use plague::*;
pub use senses::*;
pub use speciation::*;
