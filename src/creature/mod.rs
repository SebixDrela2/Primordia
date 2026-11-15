mod actions;
mod bonds;
mod brain;
mod building;
mod colony;
mod combat;
mod diplomacy;
mod enums;
mod growth;
mod heritage;
mod mating;
mod names;
mod personality;
mod plague;
mod predation;
mod raids;
mod sanctify;
pub mod senses;
mod speciation;
mod spells;
mod succession;
mod stats;
mod step;
mod tech;
mod technology;
mod zorg;

use crate::elements::Elem;
use crate::rng::Rng;
use crate::world::{World, DIRS4, DIRS8};

pub const INPUTS: usize = crate::creature::senses::TOTAL_INPUTS;
pub const OUTPUTS: usize = 33;

pub mod outputs {

    pub const NEGOTIATE: usize = 20;

    pub const HOSTILE: usize = 21;

    pub const ALLY: usize = 22;

    pub const TRIBUTE: usize = 23;

    pub const SEEK_MATE: usize = 24;

    pub const DESIRE: usize = 25;

    pub const DESIRE_COUNT: usize = 8;
}
const MAX_POP: usize = 500;

const REPRO_SOFT_CAP: usize = 320;

const MAX_AGE: u32 = 3200;
const BASE_RADIUS: i32 = 40;
const CARRY_CAP: f32 = 400.0;
const MANA_REGEN: f32 = 0.15;
const STAMINA_REGEN: f32 = 0.2;

pub mod energy_cost {

    pub const MOVE: f32 = 0.020;

    pub const SHOVE: f32 = 0.35;

    pub const CHEW: f32 = 0.06;

    pub const ATTACK: f32 = 6.0;

    pub const SHOOT: f32 = 4.0;

    pub const DIG: f32 = 5.0;

    pub const MINE: f32 = 3.0;

    pub const BUILD: f32 = 8.0;

    pub const CRAFT: f32 = 5.0;

    pub const BOMB: f32 = 6.0;

    pub const SALVE: f32 = 4.0;

    pub const TRADE: f32 = 2.0;

    pub const SPELL: f32 = 10.0;
}

pub const MAX_FARMS: usize = 4;

pub const MAX_TOWERS: usize = 4;

pub const MAX_STRUCTURES: usize = 420;

pub const MAX_SPEED: f32 = 0.9;

pub const MOUNT_SPEED_BONUS: f32 = 1.5;

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

pub struct Structure {
    pub x: f32,
    pub y: f32,
    pub species: u32,
    pub color: [u8; 3],
    pub hp: f32,
    pub cooldown: u32,
    pub armed: bool,

    pub kind: u8,
}

pub struct Bomb {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u32,
    pub owner: u32,
    pub color: [u8; 3],
}

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

    Treaty { x: f32, y: f32, color: [u8; 3], label: String },

    War { x: f32, y: f32, color: [u8; 3] },

    Speciation { x: f32, y: f32, color: [u8; 3] },

    Plague { x: f32, y: f32, color: [u8; 3] },

    Discovery { x: f32, y: f32, color: [u8; 3] },
}

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

    pub faction: Faction,
    pub generation: u32,
    pub fitness: f32,
    pub offspring: u32,
    pub kills: u32,
    pub cooldown: u32,

    pub fire_cooldown: u32,

    pub out_cache: [f32; OUTPUTS],

    pub brain_ready: bool,

    pub percept_cache: Percept,

    pub desire_dir: usize,

    pub hurt_dir: i8,

    pub kill_dir: i8,

    pub health_seen: f32,

    pub kills_seen: u32,

    pub last_meal: Elem,
    pub size: f32,
    pub last_x: f32,
    pub last_y: f32,

    pub name: [u8; 10],
    pub team_color: [u8; 3],

    pub role: Role,
    pub base_x: f32,
    pub base_y: f32,
    pub carried: f32,
    pub build_cooldown: u32,
    pub target_x: f32,
    pub target_y: f32,

    pub tool: Tool,

    pub bombs: u8,
    pub salves: u8,
    pub tool_durability: u16,
    pub tech_level: u8,
    pub inventory: [u16; 16],
    pub craft_cooldown: u32,

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

    pub mana: f32,
    pub max_mana: f32,
    pub spell_cooldown: u16,
    pub known_spells: u16,
    pub active_spell: Spell,
    pub spell_cd: [u16; 10],

    pub mount: Option<u32>,
    pub vehicle: Option<u32>,
    pub mount_kind: MountKind,

    pub air_time: u32,
    pub mount_stamina: f32,

    pub genes: [f32; 32],
    pub mutations: u16,

    pub traits: u32,

    pub personality: Personality,

    pub stage: Stage,

    pub affinity: Affinity,

    pub culture: u16,

    pub mate: Option<u32>,

    pub sired: u32,

    pub migrating: bool,

    pub bond: Option<u32>,

    pub hunger: f32,
    pub thirst: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub resting: bool,

    pub memory: [f32; 8],

    pub tech: u32,

    pub diplo_cooldown: u16,

    pub diplo_progress: f32,

    pub diplo_partner: Option<u32>,

    pub diplo_with: Option<u32>,

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

    pub juveniles: u32,

    pub elders: u32,

    pub matings: u32,

    pub splits: u32,

    pub practices: u32,

    pub discoveries: u32,
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

    pub eats_by_elem: [u32; 128],
    pub builds_total: u64,
    pub trades_total: u64,
    pub spells_cast: u64,

    pub matings_total: u32,

    pub two_parent_births: u32,
    next_squad: u16,
    pub day_night: f32,
    pub temperature: f32,
    pub season: u8,
    pub season_timer: u64,

    pub projectiles: Vec<Projectile>,

    pub structures: Vec<Structure>,

    pub claims: Vec<(u32, f32, f32, f32)>,

    pub predation: PredationMemory,

    pub succession: Vec<(u32, u64)>,

    pub fire_lit: Vec<bool>,

    pub events: Vec<SimEvent>,

    pub bombs: Vec<Bomb>,

    pub diplomacy: Diplomacy,

    pub culture_pool: Vec<(u32, u16)>,

    pub migrations: Vec<Migration>,

    pub splits_total: u32,

    pub migrations_started: u32,

    pub plagues_started: u32,

    pub discoveries: Vec<(u32, u32)>,

    pub activity: Vec<(u32, u32)>,

    pub deepest: Vec<(u32, u32)>,

    pub last_discovery: Option<(Discovery, u32)>,
}

pub enum CreatureGenesis<'a> {
    Random,
    Child(&'a Creature),

    Revival(&'a RevivalGenome),

    Zorg,
}

pub struct RevivalGenome {
    pub net: Net,
    pub color: [u8; 3],
    pub genes: [f32; 32],
    pub size: f32,
    pub generation: u32,

    pub culture: u16,

    pub personality: Personality,
}

use actions::{act_steer, act_move, act_eat, act_mine, act_craft, act_combat};
pub use brain::*;
use building::act_build;
pub use diplomacy::*;
pub use enums::*;
use growth::grow_and_learn;
pub use heritage::*;
pub use names::*;
pub use personality::*;
pub use plague::*;
pub use predation::*;
pub use spells::*;
pub use tech::*;
pub use senses::*;
pub use speciation::*;
pub use succession::*;
pub use technology::*;
pub use zorg::*;
