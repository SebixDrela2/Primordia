use crate::elements::Elem;
use crate::rng::Rng;
use crate::world::{World, DIRS4, DIRS8};

pub const INPUTS: usize = 160;
pub const OUTPUTS: usize = 14;
const MAX_POP: usize = 500;
/// Above this population creatures stop reproducing so the world stays lively
/// without grinding to a halt (the hard cap is still MAX_POP).
const REPRO_SOFT_CAP: usize = 240;
/// Creatures that reach this age die of old age, driving generational turnover.
const MAX_AGE: u32 = 3200;
const BASE_RADIUS: i32 = 40;
const BUILD_COST: f32 = 80.0;
const CARRY_CAP: f32 = 400.0;
const MANA_REGEN: f32 = 0.15;
const STAMINA_REGEN: f32 = 0.2;

// ---------------------------------------------------------------------------
// Tiny feed-forward network
// ---------------------------------------------------------------------------

pub struct Net {
    sizes: Vec<usize>,
    w: Vec<Vec<f32>>,
    b: Vec<Vec<f32>>,
}

impl Net {
    pub fn random(sizes: &[usize], rng: &mut Rng) -> Self {
        let mut w = Vec::new();
        let mut b = Vec::new();
        for layer in 0..sizes.len() - 1 {
            let fan_in = sizes[layer];
            let fan_out = sizes[layer + 1];
            let scale = (2.0 / (fan_in + fan_out) as f32).sqrt();
            let mut ws = Vec::with_capacity(fan_in * fan_out);
            for _ in 0..fan_in * fan_out {
                ws.push((rng.f32() * 2.0 - 1.0) * scale);
            }
            let mut bs = Vec::with_capacity(fan_out);
            for _ in 0..fan_out {
                bs.push((rng.f32() * 2.0 - 1.0) * 0.1);
            }
            w.push(ws);
            b.push(bs);
        }
        Net {
            sizes: sizes.to_vec(),
            w,
            b,
        }
    }

    pub fn clone_net(&self) -> Net {
        Net {
            sizes: self.sizes.clone(),
            w: self.w.iter().map(|r| r.clone()).collect(),
            b: self.b.iter().map(|r| r.clone()).collect(),
        }
    }

    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut act = input.to_vec();
        let last = self.w.len() - 1;
        for layer in 0..self.w.len() {
            let out_n = self.sizes[layer + 1];
            let in_n = self.sizes[layer];
            let ws = &self.w[layer];
            let bs = &self.b[layer];
            let mut next = vec![0.0f32; out_n];
            for o in 0..out_n {
                let mut sum = bs[o];
                let row = o * in_n;
                for i in 0..in_n {
                    sum += act[i] * ws[row + i];
                }
                next[o] = if layer == last { sum.tanh() } else { sum.max(0.0) };
            }
            act = next;
        }
        act
    }

    pub fn mutate(&mut self, rng: &mut Rng, rate: f32, strength: f32) {
        for ws in &mut self.w {
            for v in ws.iter_mut() {
                if rng.chance(rate) {
                    *v += (rng.f32() * 2.0 - 1.0) * strength;
                }
            }
        }
        for bs in &mut self.b {
            for v in bs.iter_mut() {
                if rng.chance(rate) {
                    *v += (rng.f32() * 2.0 - 1.0) * strength;
                }
            }
        }
    }
}

// Name generator
fn generate_name(rng: &mut Rng, role: Role, species: u32) -> [u8; 10] {
    const PREFIXES: &[&str] = &[
        "A", "Ba", "Ca", "Da", "Ea", "Fa", "Ga", "Ha", "Ia", "Ja",
        "Ka", "La", "Ma", "Na", "Oa", "Pa", "Qa", "Ra", "Sa", "Ta",
        "Ua", "Va", "Wa", "Xa", "Ya", "Za", "Kra", "Dra", "Gra", "Tra",
    ];
    const MIDDLES: &[&str] = &[
        "r", "t", "v", "d", "m", "k", "z", "x", "q", "v",
        "m", "d", "g", "th", "v", "z", "p", "n", "l", "v",
        "g", "dr", "k", "v", "z", "my", "ty", "k", "v", "z",
    ];
    const SUFFIXES: &[&str] = &[
        "", "s", "r", "x", "x", "x", "n", "n", "n", "n",
        "k", "k", "k", "k", "k", "r", "r", "r", "l", "l",
    ];
    
    let prefix = PREFIXES[rng.range(PREFIXES.len() as u32) as usize];
    let middle = MIDDLES[rng.range(MIDDLES.len() as u32) as usize];
    let suffix = SUFFIXES[rng.range(SUFFIXES.len() as u32) as usize];
    
    let role_suffix = match role {
        Role::Queen => "a",
        Role::Soldier => "x",
        Role::Worker => "",
        Role::Priest => "o",
        Role::Merchant => "i",
        Role::Builder => "u",
        Role::Explorer => "e",
        Role::Shaman => "y",
    };
    
    let species_mod = (species * 7 + 11) as usize % MIDDLES.len();
    let middle2 = MIDDLES[species_mod];
    
    let full_name = format!("{}{}{}{}{}", prefix, middle, middle2, suffix, role_suffix);
    let mut name_bytes = [0u8; 10];
    let bytes = full_name.as_bytes();
    let len = bytes.len().min(9);
    name_bytes[..len].copy_from_slice(&bytes[..len]);
    name_bytes
}

fn species_team_color(rng: &mut Rng, species: u32) -> [u8; 3] {
    let mut seed = species.wrapping_mul(0x9E3779B9);
    seed ^= seed >> 13;
    seed ^= seed << 17;
    seed ^= seed >> 5;
    let r = ((seed >> 16) & 0xFF) as u8;
    let g = ((seed >> 8) & 0xFF) as u8;
    let b = (seed & 0xFF) as u8;
    [r.max(80), g.max(80), b.max(80)]
}

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
    // Genetics
    pub genes: [f32; 32],
    pub mutations: u16,
    // Needs
    pub hunger: f32,
    pub thirst: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub resting: bool,
    // Memory (tiny blackboard for learned behavior)
    pub memory: [f32; 8],
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
    next_squad: u16,
    vehicles: Vec<Vehicle>,
    next_vehicle: u32,
    pub day_night: f32, // 0.0 = midnight, 0.5 = noon, 1.0 = midnight
    pub temperature: f32, // world temperature affecting creatures
    pub season: u8, // 0=spring,1=summer,2=autumn,3=winter
    pub season_timer: u64,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum VehicleType {
    Cart = 0,
    Boat = 1,
    Glider = 2,
    Sled = 3,
}

impl CreatureSystem {
    pub fn new(seed: u64) -> Self {
        CreatureSystem {
            list: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5_1234_9E37_79B9),
            next_id: 1,
            next_species: 1,
            tick: 0,
            stats: CreatureStats::default(),
            births_total: 0,
            deaths_total: 0,
            eats_total: 0,
            builds_total: 0,
            trades_total: 0,
            spells_cast: 0,
            next_squad: 1,
            vehicles: Vec::new(),
            next_vehicle: 1,
            day_night: 0.25,
            temperature: 20.0,
            season: 0,
            season_timer: 0,
        }
    }

    pub fn populate(&mut self, world: &World, count: usize) {
        for _ in 0..count {
            self.spawn_near_surface(world);
        }
        self.update_stats();
    }

    /// Snapshot the fittest living member of `species` so the next run can be
    /// seeded from the winning lineage.
    pub fn champion_from(&self, species: u32) -> Option<RevivalGenome> {
        self.list
            .iter()
            .filter(|c| c.species == species)
            .max_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| RevivalGenome {
                net: c.net.clone_net(),
                color: c.color,
                genes: c.genes,
                size: c.size,
                generation: c.generation,
            })
    }

    /// Seed a fresh world with mutated descendants of a previous champion.
    /// Every spawn becomes a new species (and therefore starts as a queen), so
    /// the winning lineage immediately diversifies into competing clades.
    pub fn populate_revival(&mut self, world: &World, count: usize, genome: &RevivalGenome) {
        for _ in 0..count {
            if self.list.len() >= MAX_POP {
                break;
            }
            let x = self.rng.range_i(3, world.w as i32 - 3);
            let surface = world.surface_y(x);
            let y = surface - 2;
            let (sx, sy) = world
                .find_empty_passable_near(x, y, 8)
                .unwrap_or((x, y));
            self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Revival(genome));
        }
        self.update_stats();
    }

    pub fn spawn_near_surface(&mut self, world: &World) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let x = self.rng.range_i(3, world.w as i32 - 3);
        let surface = world.surface_y(x);
        let y = surface - 2;
        // Find a nearby empty/passable spot so creatures never spawn stuck in solids or deep water.
        let (sx, sy) = world
            .find_empty_passable_near(x, y, 8)
            .unwrap_or((x, y));
        self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Random)
    }

    fn spawn(&mut self, world: &World, x: f32, y: f32, genesis: CreatureGenesis<'_>) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(1));

        // Extract parent data if child
        let (parent_role, parent_base_x, parent_base_y) = match genesis {
            CreatureGenesis::Child(parent) => (parent.role, parent.base_x, parent.base_y),
            _ => (Role::Worker, 0.0, 0.0),
        };
        let is_child = matches!(genesis, CreatureGenesis::Child(_));
        let start_energy = if is_child { 300.0 } else { 520.0 };

        let (net, color, species, generation, size, genes, mutations) = match genesis {
            CreatureGenesis::Random => {
                let mut g = [0.0f32; 32];
                for gi in g.iter_mut() {
                    *gi = rng.f32();
                }
                (
                    Net::random(&[INPUTS, 24, 12, OUTPUTS], &mut rng),
                    [
                        rng.range_i(90, 256) as u8,
                        rng.range_i(100, 256) as u8,
                        rng.range_i(120, 256) as u8,
                    ],
                    {
                        self.next_species += 1;
                        self.next_species
                    },
                    0,
                    1.0 + rng.f32() * 1.3,
                    g,
                    0,
                )
            }
            CreatureGenesis::Child(parent) => {
                let mut net = parent.net.clone_net();
                net.mutate(&mut rng, 0.12, 0.22);
                let color = [
                    (parent.color[0] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                    (parent.color[1] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                    (parent.color[2] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                ];
                // No ongoing speciation: species only disappear through
                // extinction, so a run can actually converge on one survivor.
                // New diversity is injected between runs by `Revival`.
                let species = parent.species;
                let size = (parent.size + rng.f32() * 0.3 - 0.15).clamp(1.0, 2.4);
                let mut g = parent.genes;
                for gi in g.iter_mut() {
                    *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.08).clamp(0.0, 1.0);
                }
                (
                    net,
                    color,
                    species,
                    parent.generation + 1,
                    size,
                    g,
                    parent.mutations.saturating_add(1),
                )
            }
            CreatureGenesis::Revival(base) => {
                let mut net = base.net.clone_net();
                // Stronger shake-up: the winning lineage diverges into several
                // new species that then compete again.
                net.mutate(&mut rng, 0.20, 0.30);
                let color = [
                    (base.color[0] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                    (base.color[1] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                    (base.color[2] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                ];
                self.next_species += 1;
                let species = self.next_species;
                let size = (base.size + rng.f32() * 0.4 - 0.2).clamp(1.0, 2.4);
                let mut g = base.genes;
                for gi in g.iter_mut() {
                    *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.12).clamp(0.0, 1.0);
                }
                (
                    net,
                    color,
                    species,
                    base.generation + 1,
                    size,
                    g,
                    1,
                )
            }
        };

        self.rng = rng;

        // Determine role: first of species = Queen, then ~10% Soldiers, rest Workers
        // Children of queens become workers/soldiers; children of workers/soldiers inherit with small mutation
        let role = if is_child {
            match parent_role {
                Role::Queen => {
                    // Queen's offspring: 85% worker, 10% soldier, 5% queen (rare princess)
                    let r = self.rng.f32();
                    if r < 0.85 { Role::Worker }
                    else if r < 0.95 { Role::Soldier }
                    else { Role::Queen }
                }
                Role::Worker => {
                    // Worker offspring: mostly workers, rare soldier or a new
                    // princess that can found a fresh colony/lineage.
                    let r = self.rng.f32();
                    if r < 0.05 { Role::Queen }
                    else if r < 0.06 { Role::Soldier }
                    else { Role::Worker }
                }
                Role::Soldier => {
                    // Soldiers mostly breed soldiers, occasionally a queen.
                    let r = self.rng.f32();
                    if r < 0.03 { Role::Queen }
                    else if r < 0.04 { Role::Worker }
                    else { Role::Soldier }
                }
                Role::Priest => {
                    if self.rng.chance(0.05) { Role::Shaman } else { Role::Priest }
                }
                Role::Merchant => { Role::Merchant }
                Role::Builder => { Role::Builder }
                Role::Explorer => { Role::Explorer }
                Role::Shaman => { Role::Shaman }
            }
        } else {
            let species_count = self.list.iter().filter(|c| c.species == species).count();
            if species_count == 0 {
                Role::Queen
            } else if self.rng.chance(0.12) {
                Role::Soldier
            } else {
                Role::Worker
            }
        };

        // Base position: Queen sets it; children inherit; random spawns pick a spot
        let (base_x, base_y) = if is_child {
            (parent_base_x, parent_base_y)
        } else if role == Role::Queen {
            (x, y)
        } else {
            // Find species base or use spawn pos
            self.list.iter()
                .find(|c| c.species == species && c.role == Role::Queen)
                .map(|q| (q.x, q.y))
                .unwrap_or((x, y))
        };

        // Spell learning at birth: shamans/priests start with spells, others rarely
        let initial_spells = match role {
            Role::Shaman => self.rng.range(2) + 1,
            Role::Priest => if self.rng.chance(0.5) { 1 } else { 0 },
            _ => if self.rng.chance(0.08) { 1 } else { 0 },
        };

        // Starting resources so creatures can immediately craft and build
        let mut starting_inv = [0u16; 16];
        starting_inv[0] = (self.rng.range(10) + 5) as u16;  // stone
        starting_inv[2] = (self.rng.range(4) + 1) as u16;   // copper - enables tech progression
        starting_inv[6] = (self.rng.range(6) + 2) as u16;   // wood/plant
        if role == Role::Builder {
            starting_inv[0] = starting_inv[0].saturating_add(20u16);
            starting_inv[1] = self.rng.range(5) as u16;   // metal
        }
        if role == Role::Shaman || role == Role::Priest {
            starting_inv[4] = self.rng.range(3) as u16;   // crystal
        }

        let creature = Creature {
            id: self.next_id,
            x: x.clamp(2.0, world.w as f32 - 3.0),
            y: y.clamp(2.0, world.h as f32 - 3.0),
            vx: 0.0,
            vy: 0.0,
            energy: start_energy,
            max_energy: if role == Role::Soldier { 1500.0 } else { 1200.0 },
            health: if role == Role::Soldier { 150.0 } else { 100.0 },
            age: 0,
            net,
            color,
            species,
            generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 150,
            size,
            last_x: x,
            last_y: y,
            name: generate_name(&mut self.rng, role, species),
            team_color: species_team_color(&mut self.rng, species),
            role,
            base_x,
            base_y,
            carried: 0.0,
            build_cooldown: 0,
            target_x: x,
            target_y: y,
            tool: Tool::None,
            tool_durability: 0,
            tech_level: 0,
            inventory: starting_inv,
            craft_cooldown: 0,
            disease: Disease::None,
            disease_timer: 0,
            immunity: [0; 8],
            belief: Belief::None,
            devotion: 0,
            mood: Mood::Neutral,
            mood_timer: 0,
            loyalty: 0,
            reputation: 0,
            trade_partner: None,
            squad_id: 0,
            squad_role: 0,
            mana: 0.0,
            max_mana: 50.0,
            spell_cooldown: 0,
            known_spells: initial_spells as u16,
            active_spell: if initial_spells > 0 { Spell::Fireball } else { Spell::None },
            spell_cd: [0; 10],
            mount: None,
            vehicle: None,
            mount_kind: MountKind::None,
            mount_stamina: 100.0,
            genes,
            mutations,
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
        };
        self.next_id += 1;
        self.list.push(creature);
        self.births_total += 1;
        true
    }

    pub fn step(&mut self, world: &mut World) {
        self.tick += 1;

        // Advance day/night cycle (full cycle = 7200 ticks ~ 2 min at 60fps)
        self.day_night += 0.000139;
        if self.day_night > 1.0 { self.day_night -= 1.0; }
        // Temperature follows season + day/night
        let base_temp = match self.season { 0 => 18.0, 1 => 26.0, 2 => 16.0, _ => 6.0 };
        let day_variance = (self.day_night * std::f32::consts::PI * 2.0).sin() * 6.0;
        self.temperature = base_temp + day_variance;
        // Season progression
        self.season_timer += 1;
        if self.season_timer > 7200 { self.season_timer = 0; self.season = (self.season + 1) % 4; }

        let mut dead: Vec<usize> = Vec::new();
        let mut births: Vec<(f32, f32, Creature)> = Vec::new();
        let mut attackers: Vec<usize> = Vec::new();
        let mut traded_total: u64 = 0;
        let mut structures_total: u32 = 0;
        let mut mounts_total: u32 = 0;
        let mut spells_cast_total: u32 = 0;
        let mut spell_targets: Vec<(usize, usize, f32)> = Vec::new();
        let mut disease_targets: Vec<usize> = Vec::new();
        let positions: Vec<(usize, u32, f32, f32, u32)> =
            self.list.iter().enumerate().map(|(i, c)| (i, c.id, c.x, c.y, c.species)).collect();
        let pop_now = self.list.len();

        for index in 0..self.list.len() {
            let mut died = false;
            let mut wants_attack = false;
            let mut wants_child = false;
            let mut ate_something = false;
            let mut built_something = false;
            let mut cast_spell = false;
            let mut traded = false;

            {
                let c = &mut self.list[index];
                c.age += 1;

                // Needs decay
                c.hunger = (c.hunger + 0.06).clamp(0.0, 100.0);
                c.thirst = (c.thirst + 0.045).clamp(0.0, 100.0);
                c.stamina = (c.stamina - 0.05).clamp(0.0, c.max_stamina);
                if c.resting {
                    c.stamina = (c.stamina + STAMINA_REGEN).clamp(0.0, c.max_stamina);
                    c.energy = (c.energy + 0.02).min(c.max_energy);
                } else {
                    c.energy -= 0.08;
                }
                c.mana = (c.mana + MANA_REGEN).clamp(0.0, c.max_mana);

                // Cooldowns
                if c.cooldown > 0 { c.cooldown -= 1; }
                if c.build_cooldown > 0 { c.build_cooldown -= 1; }
                if c.craft_cooldown > 0 { c.craft_cooldown -= 1; }
                if c.spell_cooldown > 0 { c.spell_cooldown -= 1; }
                for cd in &mut c.spell_cd { if *cd > 0 { *cd -= 1; } }
                if c.disease_timer > 0 { c.disease_timer -= 1; }
                else { c.disease = Disease::None; }

                // Mood drift
                if c.mood_timer == 0 {
                    c.mood = match self.rng.range(10) {
                        0 => Mood::Happy, 1 => Mood::Angry, 2 => Mood::Fearful,
                        3 => Mood::Curious, 4 => Mood::Loyal, 5 => Mood::Rebellious,
                        6 => Mood::Depressed, 7 => Mood::Zealous, _ => Mood::Neutral,
                    };
                    c.mood_timer = (self.rng.range(120) + 60) as u16;
                } else { c.mood_timer -= 1; }

                let input = Self::sense(&positions, c, world, self.day_night, self.temperature, self.season);
                let out = c.net.forward(&input);

                // Outputs: [0]=move_x, [1]=move_y, [2]=eat, [3]=reproduce, [4]=attack,
                // [5]=build, [6]=craft, [7]=cast_spell, [8]=mount/dismount, [9]=trade,
                // [10]=rest, [11]=switch_spell, [12]=dig/mine, [13]=gather_water
                c.vx = out[0] * 2.2;
                c.vy = (out[1] * 2.2 + 0.5).clamp(-2.6, 2.6);

                // Movement with collision (mounted moves faster)
                let mut speed_mult = 1.0;
                if c.mount_kind != MountKind::None && c.mount_stamina > 0.0 {
                    speed_mult = 1.6;
                    c.mount_stamina = (c.mount_stamina - 0.3).max(0.0);
                }
                let nx = c.x + c.vx * speed_mult;
                let ny = c.y + c.vy * speed_mult;

                // A creature can shove through loose debris (sand, ice, ash...)
                // and can tunnel through solid rock only with a pickaxe/drill.
                let can_dig = |e: Elem, _tool: Tool| -> bool {
                    // Walking only shoves loose debris aside. Solid rock is never
                    // excavated just by pathing - that happens through the
                    // deliberate, budgeted mining action instead, otherwise a
                    // colony of pickaxe carriers hollows out the entire world.
                    World::is_loose(e)
                };

                let tx = nx.round() as i32;
                let ty = ny.round() as i32;
                let mut ok = !world.is_blocked(tx, ty);
                if !ok && can_dig(world.get(tx, ty), c.tool) {
                    world.set_xy(tx, ty, Elem::Empty);
                    c.stamina = (c.stamina - 0.35).max(0.0);
                    ok = true;
                }
                if ok {
                    c.x = nx.clamp(1.0, world.w as f32 - 2.0);
                    c.y = ny.clamp(1.0, world.h as f32 - 2.0);
                } else {
                    let ax = (c.x + c.vx * speed_mult).round() as i32;
                    let ay = c.y.round() as i32;
                    let mut okx = !world.is_blocked(ax, ay);
                    if !okx && can_dig(world.get(ax, ay), c.tool) {
                        world.set_xy(ax, ay, Elem::Empty);
                        c.stamina = (c.stamina - 0.35).max(0.0);
                        okx = true;
                    }
                    if okx {
                        c.x = (c.x + c.vx * speed_mult).clamp(1.0, world.w as f32 - 2.0);
                    }
                    let bx = c.x.round() as i32;
                    let by = (c.y + c.vy * speed_mult).round() as i32;
                    let mut oky = !world.is_blocked(bx, by);
                    if !oky && can_dig(world.get(bx, by), c.tool) {
                        world.set_xy(bx, by, Elem::Empty);
                        c.stamina = (c.stamina - 0.35).max(0.0);
                        oky = true;
                    }
                    if oky {
                        c.y = (c.y + c.vy * speed_mult).clamp(1.0, world.h as f32 - 2.0);
                    }
                }

                // If debris settled directly on top of us, clear our own cell so
                // we are never permanently entombed. Hard rock without a tool is
                // escaped by nudging to the nearest open cell instead.
                let cx = c.x.round() as i32;
                let cy = c.y.round() as i32;
                if world.is_blocked(cx, cy) {
                    let here = world.get(cx, cy);
                    if World::is_loose(here) || matches!(c.tool, Tool::Pickaxe | Tool::Drill) {
                        world.clear_burrow(cx, cy);
                        c.stamina = (c.stamina - 0.6).max(0.0);
                        c.energy -= 0.5;
                    } else if let Some((ex, ey)) = world.find_empty_passable_near(cx, cy, 4) {
                        c.x = ex as f32;
                        c.y = ey as f32;
                        c.stamina = (c.stamina - 2.0).max(0.0);
                    }
                }

                // Eat / gather resources (output[2]=eat, [13]=gather water)
                {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    // Hungry creatures actively search a little further for food.
                    let hungry = c.hunger > 55.0 || c.energy < c.max_energy * 0.45;
                    let radius = if out[2] > 0.3 {
                        6
                    } else if hungry {
                        5
                    } else {
                        2
                    };
                    if let Some((fx, fy)) = world.find_food(gx, gy, radius) {
                        let eaten_elem = world.get(fx, fy);
                        if let Some((nutrition, toxicity)) = world.eat(fx, fy) {
                            // Modest returns: food is a real resource, so a
                            // colony has to keep foraging instead of trivially
                            // maxing out after a couple of bites.
                            c.energy = (c.energy + nutrition * 4.0).min(c.max_energy);
                            c.hunger = (c.hunger - nutrition * 4.0).max(0.0);
                            if eaten_elem == Elem::Water {
                                c.thirst = (c.thirst - 40.0).max(0.0);
                            } else if eaten_elem.nutrition() > 0.0 {
                                c.thirst = (c.thirst - nutrition * 4.0).max(0.0);
                            }
                            // Gathering resources gives inventory
                            match eaten_elem {
                                Elem::Stone | Elem::Brick | Elem::Concrete | Elem::Wall => {
                                    c.inventory[0] = (c.inventory[0].saturating_add(5)).min(255);
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Metal | Elem::Wire => {
                                    c.inventory[1] = (c.inventory[1].saturating_add(3)).min(255);
                                    c.carried = (c.carried + 40.0).min(CARRY_CAP);
                                }
                                Elem::Copper => {
                                    c.inventory[2] = (c.inventory[2].saturating_add(3)).min(255);
                                    c.carried = (c.carried + 40.0).min(CARRY_CAP);
                                }
                                Elem::Gold => {
                                    c.inventory[3] = (c.inventory[3].saturating_add(2)).min(255);
                                    c.carried = (c.carried + 50.0).min(CARRY_CAP);
                                }
                                Elem::Crystal => {
                                    c.inventory[4] = (c.inventory[4].saturating_add(2)).min(255);
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Gem => {
                                    c.inventory[5] = (c.inventory[5].saturating_add(1)).min(255);
                                    c.carried = (c.carried + 60.0).min(CARRY_CAP);
                                }
                                Elem::Wood | Elem::Plant | Elem::Fungus => {
                                    c.inventory[6] = (c.inventory[6].saturating_add(4)).min(255);
                                    c.carried = (c.carried + 20.0).min(CARRY_CAP);
                                }
                                Elem::Energy => {
                                    c.inventory[7] = (c.inventory[7].saturating_add(10)).min(255);
                                }
                                Elem::Glass | Elem::Circuit | Elem::Tool | Elem::Machine => {
                                    c.carried = (c.carried + 30.0).min(CARRY_CAP);
                                }
                                Elem::Water => {
                                    c.inventory[8] = (c.inventory[8].saturating_add(5)).min(255);
                                }
                                _ => {}
                            }
                            if toxicity > 0.0 {
                                c.health -= toxicity * 0.25;
                            }
                            if c.energy > c.max_energy * 0.75 {
                                c.health = (c.health + 0.5).min(100.0);
                            }
                            ate_something = true;
                        }
                    }
                    // Drink: look for nearby water when thirsty, rather than
                    // requiring the creature to already stand on a water cell.
                    if c.thirst > 40.0 {
                        let drink_radius = if out[13] > 0.5 { 6 } else { 4 };
                        if let Some((wx, wy)) = world.find_water(gx, gy, drink_radius) {
                            world.set_xy(wx, wy, Elem::Empty);
                            c.thirst = (c.thirst - 60.0).max(0.0);
                            c.energy = (c.energy + 5.0).min(c.max_energy);
                            c.inventory[8] = (c.inventory[8].saturating_add(2)).min(255);
                            ate_something = true;
                        }
                    }
                }

                // Mining / gathering (output[12]). Takes solid resources out of
                // the world for building but is explicitly *not* food.
                let wants_mine = out[12] > 0.35 && world.mine_budget > 0;
                let exploratory_mine = c.carried < 60.0
                    && c.stamina > 8.0
                    && matches!(c.role, Role::Worker | Role::Builder | Role::Explorer)
                    && (c.age.wrapping_add(c.id) % 400) < 1;
                if (wants_mine || (exploratory_mine && world.mine_budget > 0)) && c.stamina > 5.0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    if let Some((mx, my)) = world.find_resource(gx, gy, 4) {
                        world.mine_budget -= 1;
                        let me = world.get(mx, my);
                        world.set_xy(mx, my, Elem::Empty);
                        c.stamina = (c.stamina - 0.8).max(0.0);
                        c.energy -= 0.8;
                        c.carried = (c.carried + 15.0).min(CARRY_CAP);
                        // Every excavation also yields a little stone.
                        c.inventory[0] = c.inventory[0].saturating_add(1).min(255);
                        match me {
                            Elem::Stone | Elem::Sand => {
                                c.inventory[0] = c.inventory[0].saturating_add(3).min(255);
                            }
                            Elem::Metal | Elem::Iron | Elem::Steel | Elem::Wire => {
                                c.inventory[1] = c.inventory[1].saturating_add(2).min(255);
                            }
                            Elem::Copper => {
                                c.inventory[2] = c.inventory[2].saturating_add(2).min(255);
                            }
                            Elem::Gold => {
                                c.inventory[3] = c.inventory[3].saturating_add(2).min(255);
                            }
                            Elem::Crystal => {
                                c.inventory[4] = c.inventory[4].saturating_add(2).min(255);
                            }
                            Elem::Gem | Elem::Diamond => {
                                c.inventory[5] = c.inventory[5].saturating_add(1).min(255);
                            }
                            Elem::Wood => {
                                c.inventory[6] = c.inventory[6].saturating_add(3).min(255);
                            }
                            _ => {}
                        }
                    }
                }

                // Build / place structures (output[5])
                let nn_wants_build = out[5] > 0.2;
                let exploratory_build = c.carried > 80.0 && c.build_cooldown == 0
                    && (c.age * 17 + c.id * 23) % 100 < 1;
                let built = (nn_wants_build || exploratory_build) && c.carried >= 20.0 && c.build_cooldown == 0;
                if built {
                    c.energy -= 8.0;
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    let bx = c.base_x.round() as i32;
                    let by = c.base_y.round() as i32;
                    let dist_to_base = ((gx - bx).pow(2) + (gy - by).pow(2)) as f32;
                    let mut placed = false;

                    // Choose structure type based on tech/carried
                    let want_wall = c.inventory[0] >= 3 && c.tech_level < 2;
                    let want_door = c.inventory[0] >= 5 && c.role == Role::Builder;
                    let want_chest = c.inventory[1] >= 3 && c.carried > 150.0;
                    let want_farm = c.inventory[6] >= 4 && c.tech_level >= 1;
                    let want_temple = c.inventory[4] >= 2 && c.tech_level >= 3 && c.role == Role::Priest;
                    let want_tower = c.inventory[0] >= 10 && c.tech_level >= 2 && c.role == Role::Explorer;

                    let chosen = if want_wall { Buildable::Wall }
                        else if want_door { Buildable::Door }
                        else if want_chest { Buildable::Chest }
                        else if want_farm { Buildable::Farm }
                        else if want_temple { Buildable::Temple }
                        else if want_tower { Buildable::Tower }
                        else { Buildable::Bridge };

                    // Try base perimeter first if near base
                    if dist_to_base < (BASE_RADIUS * BASE_RADIUS) as f32 {
                        for r in 3..=BASE_RADIUS {
                            for angle in 0..24 {
                                let tx = bx + (r as f32 * (angle as f32 * 0.26).cos()) as i32;
                                let ty = by + (r as f32 * (angle as f32 * 0.26).sin()) as i32;
                                if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                    world.set_xy(tx, ty, Elem::Wall);
                                    c.carried = (c.carried - 15.0).max(0.0);
                                    c.build_cooldown = 10;
                                    placed = true;
                                    built_something = true;
                                    structures_total += 1;
                                    break;
                                }
                            }
                            if placed { break; }
                        }
                    }

                    // Fallback: build adjacent to self
                    if !placed {
                        for (dx, dy) in DIRS8 {
                            let tx = gx + dx;
                            let ty = gy + dy;
                            if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                world.set_xy(tx, ty, Elem::Wall);
                                c.carried = (c.carried - 15.0).max(0.0);
                                c.build_cooldown = 10;
                                built_something = true;
                                structures_total += 1;
                                placed = true;
                                break;
                            }
                        }
                    }

                    // Last resort: build anywhere in 5x5 area
                    if !placed {
                        for dy in -2..=2 {
                            for dx in -2..=2 {
                                let tx = gx + dx;
                                let ty = gy + dy;
                                if world.get(tx, ty) == Elem::Empty && !world.is_blocked(tx, ty) {
                                    world.set_xy(tx, ty, Elem::Wall);
                                    c.carried = (c.carried - 15.0).max(0.0);
                                    c.build_cooldown = 10;
                                    built_something = true;
                                    structures_total += 1;
                                    placed = true;
                                    break;
                                }
                            }
                            if placed { break; }
                        }
                    }
                }

                // Tool crafting (output[6] = craft)
                let nn_wants_craft = out[6] > 0.25;
                let exploratory_craft = c.tool == Tool::None && c.craft_cooldown == 0
                    && (c.age * 13 + c.id * 17) % 25 < 1;
                let wants_craft = nn_wants_craft || exploratory_craft;
                if wants_craft {
                    // Pickaxe: 5 stone + 2 wood (tech 0)
                    if c.tool == Tool::None && c.inventory[0] >= 5 && c.inventory[6] >= 2 {
                        c.inventory[0] -= 5;
                        c.inventory[6] -= 2;
                        c.tool = Tool::Pickaxe;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 30;
                    }
                    // Sword: 3 metal + 2 wood (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[1] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[1] -= 3;
                        c.inventory[6] -= 2;
                        c.tool = Tool::Sword;
                        c.tool_durability = 800;
                        c.craft_cooldown = 30;
                    }
                    // Hammer: 5 stone + 3 metal (tech 1) - better building
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[0] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[0] -= 5;
                        c.inventory[1] -= 3;
                        c.tool = Tool::Hammer;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 30;
                    }
                    // Drill: 3 copper + 2 metal (tech 2) - fast mining
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[2] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[2] -= 3;
                        c.inventory[1] -= 2;
                        c.tool = Tool::Drill;
                        c.tool_durability = 600;
                        c.craft_cooldown = 40;
                    }
                    // Scanner: 2 gold + 1 crystal (tech 3) - long range sensing
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[3] >= 2 && c.inventory[4] >= 1 {
                        c.inventory[3] -= 2;
                        c.inventory[4] -= 1;
                        c.tool = Tool::Scanner;
                        c.tool_durability = 2000;
                        c.craft_cooldown = 50;
                    }
                    // Bow: 3 wood + 2 string(plant) (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[6] >= 2 {
                        c.inventory[6] -= 5;
                        c.tool = Tool::Bow;
                        c.tool_durability = 500;
                        c.craft_cooldown = 30;
                    }
                    // Spear: 2 wood + 1 stone (tech 0)
                    else if c.tool == Tool::None && c.inventory[6] >= 2 && c.inventory[0] >= 1 {
                        c.inventory[6] -= 2;
                        c.inventory[0] -= 1;
                        c.tool = Tool::Spear;
                        c.tool_durability = 400;
                        c.craft_cooldown = 20;
                    }
                    // Shield: 3 wood + 2 metal (tech 1)
                    else if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[1] >= 2 {
                        c.inventory[6] -= 3;
                        c.inventory[1] -= 2;
                        c.tool = Tool::Shield;
                        c.tool_durability = 700;
                        c.craft_cooldown = 30;
                    }
                    // Staff: 1 wood + 1 crystal (tech 3) - for shaman spells
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 1 && c.inventory[4] >= 1 {
                        c.inventory[6] -= 1;
                        c.inventory[4] -= 1;
                        c.tool = Tool::Staff;
                        c.tool_durability = 1200;
                        c.craft_cooldown = 40;
                    }
                    // Amulet: 1 gold + 1 gem (tech 4)
                    else if c.tool == Tool::None && c.tech_level >= 4 && c.inventory[3] >= 1 && c.inventory[5] >= 1 {
                        c.inventory[3] -= 1;
                        c.inventory[5] -= 1;
                        c.tool = Tool::Amulet;
                        c.tool_durability = 1500;
                        c.craft_cooldown = 50;
                    }
                    // Backpack: 4 leather + 2 wood (tech 2) - more carry
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 2 {
                        c.inventory[6] -= 4;
                        c.tool = Tool::Backpack;
                        c.tool_durability = 800;
                        c.craft_cooldown = 40;
                    }
                    // Cart: 5 wood + 3 metal (tech 2)
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 5 && c.inventory[1] >= 3 {
                        c.inventory[6] -= 5;
                        c.inventory[1] -= 3;
                        c.tool = Tool::Cart;
                        c.tool_durability = 1000;
                        c.craft_cooldown = 60;
                    }
                    // Boat: 6 wood + 1 rope (tech 2)
                    else if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 6 {
                        c.inventory[6] -= 6;
                        c.tool = Tool::Boat;
                        c.tool_durability = 900;
                        c.craft_cooldown = 60;
                    }
                    // Glider: 4 wood + 2 cloth (tech 3)
                    else if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 4 {
                        c.inventory[6] -= 4;
                        c.tool = Tool::Glider;
                        c.tool_durability = 700;
                        c.craft_cooldown = 70;
                    }
                }

                // Tool durability decay
                if c.tool != Tool::None {
                    if c.tool_durability > 0 {
                        c.tool_durability = c.tool_durability.saturating_sub(1);
                    } else {
                        c.tool = Tool::None;
                    }
                }

                // Tech progression based on resources gathered
                if c.tech_level == 0 && c.inventory[2] >= 1 { c.tech_level = 1; }
                else if c.tech_level == 1 && c.inventory[1] >= 3 { c.tech_level = 2; }
                else if c.tech_level == 2 && c.inventory[3] >= 1 { c.tech_level = 3; }
                else if c.tech_level == 3 && c.inventory[4] >= 1 { c.tech_level = 4; }

                // Attack intent (output[4] = attack)
                if out[4] > 0.45 {
                    wants_attack = true;
                }

                // Cast spell (output[7] = cast, [11] = switch spell)
                if out[7] > 0.5 && c.spell_cooldown == 0 && c.known_spells > 0 && c.mana >= 10.0 {
                    // Switch active spell if requested
                    if out[11] > 0.5 {
                        let next = (c.active_spell as u8 + 1) % 10;
                        c.active_spell = match next {
                            1 => Spell::Fireball, 2 => Spell::Heal, 3 => Spell::Teleport,
                            4 => Spell::Wall, 5 => Spell::Light, 6 => Spell::Summon,
                            7 => Spell::Curse, 8 => Spell::Bless, 9 => Spell::Lightning,
                            _ => Spell::None,
                        };
                    }
                    cast_spell = true;
                }

                // Mount / dismount (output[8] = mount)
                let exploratory_mount = c.mount_kind == MountKind::None && c.energy > 100.0
                    && (c.age * 11 + c.id * 7) % 60 < 1; // ~1.6% chance per tick
                if out[8] > 0.45 || exploratory_mount {
                    if c.mount_kind == MountKind::None {
                        if c.energy > 100.0 {
                            c.mount_kind = MountKind::Beast;
                            c.mount_stamina = 100.0;
                            c.energy -= 30.0;
                            mounts_total += 1;
                        }
                    } else {
                        c.mount_kind = MountKind::None;
                        c.mount_stamina = 100.0;
                    }
                }

                // Trade (output[9] = trade) - with nearby creature
                if out[9] > 0.5 && c.craft_cooldown == 0 {
                    for &(list_idx, oid, ox, oy, ospecies) in &positions {
                        if oid == c.id { continue; }
                        let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                        if d < 6.0 && ospecies != c.species {
                            traded = true;
                            c.reputation = (c.reputation as i32 + 1).clamp(-100, 100) as i16;
                            c.energy = (c.energy + 10.0).min(c.max_energy);
                            break;
                        }
                    }
                }

                // Rest (output[10] = rest) - recover stamina/energy but vulnerable
                if out[10] > 0.5 && c.stamina < c.max_stamina * 0.5 {
                    c.resting = true;
                } else {
                    c.resting = false;
                }

                // Dig / mine (output[12] = dig) - break through weak walls
                if out[12] > 0.5 && c.tool_durability > 0 && world.mine_budget > 0 {
                    let gx = c.x.round() as i32;
                    let gy = c.y.round() as i32;
                    for (dx, dy) in DIRS4 {
                        let tx = gx + dx;
                        let ty = gy + dy;
                        let e = world.get(tx, ty);
                        if e == Elem::Wall || e == Elem::Stone {
                            if c.tool == Tool::Pickaxe || c.tool == Tool::Drill {
                                world.mine_budget -= 1;
                                world.set_xy(tx, ty, Elem::Empty);
                                c.tool_durability = c.tool_durability.saturating_sub(5);
                                c.inventory[0] = (c.inventory[0].saturating_add(2)).min(255);
                                break;
                            }
                        }
                    }
                }

                // Reproduction. Queens are the main lineage; every other role can
                // also occasionally bud off a child so a colony keeps evolving.
                // A soft population cap keeps the sim responsive while still
                // allowing real generational turnover.
                let nn_wants_reproduce = out[3] > 0.05;
                let exploratory_reproduce = c.energy > 500.0 && c.cooldown == 0
                    && c.hunger < 60.0 && c.thirst < 70.0
                    && (c.age * 13 + c.id * 29) % 120 < 2;
                let can_reproduce = match c.role {
                    Role::Queen => c.energy > 360.0,
                    Role::Worker => c.energy > 700.0 && c.hunger < 60.0 && (c.age * 17 + c.id * 23) % 300 < 2,
                    Role::Soldier => c.energy > 800.0 && c.hunger < 60.0 && (c.age * 19 + c.id * 29) % 300 < 2,
                    Role::Priest => c.energy > 750.0 && c.hunger < 60.0 && (c.age * 13 + c.id * 17) % 300 < 2,
                    Role::Merchant => c.energy > 800.0 && c.hunger < 60.0 && (c.age * 19 + c.id * 23) % 300 < 2,
                    Role::Builder => c.energy > 700.0 && c.hunger < 60.0 && (c.age * 11 + c.id * 13) % 300 < 2,
                    Role::Explorer => c.energy > 650.0 && c.hunger < 60.0 && (c.age * 7 + c.id * 11) % 300 < 2,
                    Role::Shaman => c.energy > 750.0 && c.hunger < 60.0 && (c.age * 17 + c.id * 19) % 300 < 2,
                };
                if can_reproduce && (nn_wants_reproduce || exploratory_reproduce)
                    && c.energy > 340.0 && c.cooldown == 0 && pop_now < REPRO_SOFT_CAP {
                    c.energy -= 110.0;
                    c.hunger = (c.hunger + 15.0).clamp(0.0, 100.0);
                    c.cooldown = 70;
                    c.offspring += 1;
                    wants_child = true;
                }

                // Spell casting effects
                if cast_spell {
                    c.spell_cooldown = 30;
                    c.mana = (c.mana - 10.0).max(0.0);
                    c.spell_cd[c.active_spell as usize] = 60;
                    spells_cast_total += 1;
                    match c.active_spell {
                        Spell::Fireball => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 10.0 {
                                    spell_targets.push((index, list_idx, 35.0));
                                }
                            }
                        }
                        Spell::Heal => {
                            c.health = (c.health + 40.0).min(100.0);
                            c.energy = (c.energy + 30.0).min(c.max_energy);
                        }
                        Spell::Teleport => {
                            c.x = self.rng.range_i(10, world.w as i32 - 10) as f32;
                            c.y = self.rng.range_i(10, world.h as i32 - 10) as f32;
                        }
                        Spell::Wall => {
                            let gx = c.x.round() as i32;
                            let gy = c.y.round() as i32;
                            for (dx, dy) in DIRS4 {
                                if world.get(gx + dx, gy + dy) == Elem::Empty {
                                    world.set_xy(gx + dx, gy + dy, Elem::Wall);
                                    break;
                                }
                            }
                        }
                        Spell::Light => {
                            c.fitness += 1.0;
                        }
                        Spell::Summon => {
                            // Will spawn a minion (handled in births)
                        }
                        Spell::Curse => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 8.0 {
                                    disease_targets.push(list_idx);
                                }
                            }
                        }
                        Spell::Bless => {
                            c.health = (c.health + 20.0).min(100.0);
                            c.energy = (c.energy + 20.0).min(c.max_energy);
                        }
                        Spell::Lightning => {
                            for &(list_idx, oid, ox, oy, ospecies) in &positions {
                                if oid == c.id { continue; }
                                if ospecies == c.species { continue; }
                                let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                                if d < 6.0 {
                                    spell_targets.push((index, list_idx, 50.0));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Disease spread
                if c.disease != Disease::None && self.rng.chance(0.02) {
                    for &(list_idx, oid, ox, oy, ospecies) in &positions {
                        if oid == c.id { continue; }
                        let d = ((ox - c.x).powi(2) + (oy - c.y).powi(2)).sqrt();
                        if d < 4.0 && ospecies != c.species && self.rng.chance(0.15) {
                            disease_targets.push(list_idx);
                        }
                    }
                }
                if c.disease != Disease::None {
                    c.health -= 0.5;
                    c.energy -= 0.3;
                }

                // Fitness: colony success metrics
                c.fitness = (c.age as f32 / MAX_AGE as f32) * 0.15
                    + (c.energy / c.max_energy) * 0.12
                    + c.offspring as f32 * 0.18
                    + c.kills as f32 * 0.12
                    + (c.carried / CARRY_CAP) * 0.08
                    + (c.tech_level as f32) * 0.1
                    + (c.tool as u8 as f32) * 0.04
                    + (c.mana / c.max_mana) * 0.06
                    + (c.stamina / c.max_stamina) * 0.05
                    - (c.hunger / 100.0) * 0.05
                    - (c.thirst / 100.0) * 0.05;

                if c.energy <= 0.0 || c.health <= 0.0 || c.age > MAX_AGE + c.id.wrapping_mul(2654435761) % 1400 {
                    died = true;
                }

                c.last_x = c.x;
                c.last_y = c.y;

                if wants_child {
                    let child = c.child_blueprint();
                    births.push((c.base_x, c.base_y, child));
                }
            }

            if wants_attack {
                attackers.push(index);
            }
            if ate_something {
                self.eats_total += 1;
            }
            if built_something {
                self.builds_total += 1;
            }
            if traded {
                traded_total += 1;
            }
            if died {
                dead.push(index);
            }
        }

        // Resolve attacks - soldiers target enemy queens/bases, workers defend
        for &a in &attackers {
            let (ax, ay, a_species, a_role) = {
                let c = &self.list[a];
                (c.x, c.y, c.species, c.role)
            };
            let mut best: Option<(usize, f32)> = None;
            for (j, other) in self.list.iter().enumerate() {
                if j == a { continue; }
                if other.species == a_species { continue; }
                let priority = match (a_role, other.role) {
                    (Role::Soldier, Role::Queen) => 0.0,
                    (Role::Soldier, Role::Soldier) => 10.0,
                    (Role::Soldier, Role::Worker) => 20.0,
                    (Role::Worker, _) => 50.0,
                    _ => 30.0,
                };
                let d = ((other.x - ax).powi(2) + (other.y - ay).powi(2)).sqrt() + priority;
                let max_range = if a_role == Role::Soldier { 14.0 } else { 6.0 };
                if d < max_range && best.map_or(true, |b| d < b.1) {
                    best = Some((j, d));
                }
            }
            if let Some((j, _)) = best {
                if self.list[j].health > 0.0 {
                    let attacker = &self.list[a];
                    let mut dmg = if a_role == Role::Soldier { 45.0 } else { 25.0 };
                    dmg += match attacker.tool {
                        Tool::Sword => 35.0,
                        Tool::Pickaxe => 18.0,
                        Tool::Hammer => 22.0,
                        Tool::Drill => 28.0,
                        Tool::Spear => 20.0,
                        Tool::Staff => 15.0,
                        _ => 0.0,
                    };
                    if attacker.active_spell == Spell::Fireball && attacker.spell_cd[1] == 0 { dmg += 20.0; }
                    self.list[j].health -= dmg;
                    self.list[a].energy = (self.list[a].energy + 20.0).min(self.list[a].max_energy);
                    if self.list[j].health <= 0.0 {
                        self.list[a].kills += 1;
                        self.list[a].energy = (self.list[a].energy + 140.0).min(self.list[a].max_energy);
                    }
                }
            }
        }

        // Remove the dead
        for &i in dead.iter().rev() {
            let c = &self.list[i];
            world.scavenge(c.x.round() as i32, c.y.round() as i32);
            self.list.remove(i);
            self.deaths_total += 1;
        }

        // Births
        for (x, y, mut child) in births {
            if self.list.len() >= MAX_POP { break; }
            // Summon spell creates a minion at caster pos
            let cx = x + self.rng.range_i(-4, 5) as f32;
            let cy = y + self.rng.range_i(-4, 5) as f32;
            if world.is_blocked(cx.round() as i32, cy.round() as i32) { continue; }
            if self.spawn(world, cx, cy, CreatureGenesis::Child(&child)) { }
        }

        // No automatic re-seeding: species must be able to go extinct (and one
        // last lineage to win). A fully dead world is handled by the
        // extinction/revival screen in the main loop instead.

        // Apply accumulated stats
        self.trades_total = self.trades_total.saturating_add(traded_total);
        self.stats.structures = self.stats.structures.saturating_add(structures_total);
        self.stats.mounts = self.stats.mounts.saturating_add(mounts_total);
        self.stats.spells_cast = self.stats.spells_cast.saturating_add(spells_cast_total);

        // Apply spell damage and disease to targets
        for (caster_idx, target_idx, damage) in spell_targets {
            if let Some(target) = self.list.get_mut(target_idx) {
                target.health -= damage;
                if target.health <= 0.0 {
                    if let Some(caster) = self.list.get_mut(caster_idx) {
                        caster.kills += 1;
                        caster.energy = (caster.energy + 140.0).min(caster.max_energy);
                    }
                }
            }
        }
        for &target_idx in &disease_targets {
            if let Some(target) = self.list.get_mut(target_idx) {
                target.disease = Disease::Curse;
                target.disease_timer = 300;
            }
        }

        if self.tick % 15 == 0 {
            self.update_stats();
        }
    }

    fn sense(positions: &[(usize, u32, f32, f32, u32)], c: &Creature, world: &World, day_night: f32, temperature: f32, season: u8) -> Vec<f32> {
        let mut input = vec![0.0f32; INPUTS];
        let cx = c.x.round() as i32;
        let cy = c.y.round() as i32;

        // 8 directions × 6 channels = 48 (food, hazard, water, solid, metal, crystal)
        for (d, (dx, dy)) in DIRS8.iter().enumerate() {
            let mut food = 0.0;
            let mut hazard = 0.0;
            let mut water = 0.0;
            let mut solid = 0.0;
            let mut metal = 0.0;
            let mut crystal = 0.0;
            for step in 1..=8i32 {
                let e = world.get(cx + dx * step, cy + dy * step);
                if e == Elem::Empty { continue; }
                let w = 1.0 / step as f32;
                if e.nutrition() > 0.0 { food += w; }
                if e.toxicity() > 0.0 { hazard += w; }
                if e == Elem::Water { water += w; }
                if e.is_solid() { solid += w; }
                if matches!(e, Elem::Metal | Elem::Copper | Elem::Gold | Elem::Iron | Elem::Silver | Elem::Wire | Elem::Circuit) { metal += w; }
                if matches!(e, Elem::Crystal | Elem::Gem | Elem::Diamond | Elem::Mana) { crystal += w; }
            }
            let base = d * 6;
            input[base] = food.min(1.0);
            input[base + 1] = hazard.min(1.0);
            input[base + 2] = water.min(1.0);
            input[base + 3] = solid.min(1.0);
            input[base + 4] = metal.min(1.0);
            input[base + 5] = crystal.min(1.0);
        }

        // Creature proximity (nearest other, count nearby)
        let mut nearest_dist = f32::MAX;
        let mut nearest_dx = 0.0;
        let mut nearest_dy = 0.0;
        let mut others = 0.0;
        for &(list_idx, id, ox, oy, _ospecies) in positions {
            if id == c.id {
                continue;
            }
            let dx = ox - c.x;
            let dy = oy - c.y;
            let d2 = dx * dx + dy * dy;
            if d2 < 400.0 {
                others += 1.0;
            }
            if d2 < nearest_dist {
                nearest_dist = d2;
                nearest_dx = dx;
                nearest_dy = dy;
            }
        }

        // Internal state (16) - slots 32-47
        input[32] = (c.energy / c.max_energy).clamp(0.0, 1.0);
        input[33] = (c.health / (if c.role == Role::Soldier { 150.0 } else { 100.0 })).clamp(0.0, 1.0);
        input[34] = (c.age as f32 / MAX_AGE as f32).clamp(0.0, 1.0);
        input[35] = (c.vx / 2.2).clamp(-1.0, 1.0);
        input[36] = (c.vy / 2.2).clamp(-1.0, 1.0);
        input[37] = (nearest_dx / 20.0f32).clamp(-1.0f32, 1.0f32);
        input[38] = (nearest_dy / 20.0f32).clamp(-1.0f32, 1.0f32);
        input[39] = (others / 10.0f32).clamp(0.0, 1.0);
        input[40] = c.hunger / 100.0;
        input[41] = c.thirst / 100.0;
        input[42] = c.stamina / c.max_stamina;
        input[43] = (c.mana / c.max_mana).clamp(0.0, 1.0);
        input[44] = (c.craft_cooldown as f32 / 50.0).clamp(0.0, 1.0);
        input[45] = (c.spell_cooldown as f32 / 60.0).clamp(0.0, 1.0);
        input[46] = if c.resting { 1.0 } else { 0.0 };
        input[47] = (c.carried / CARRY_CAP).clamp(0.0, 1.0);

        // Colony awareness (8) - slots 48-55
        let base_dx = c.base_x - c.x;
        let base_dy = c.base_y - c.y;
        let base_dist = (base_dx * base_dx + base_dy * base_dy).sqrt().max(1.0);
        input[48] = (base_dx / base_dist).clamp(-1.0, 1.0);
        input[49] = (base_dy / base_dist).clamp(-1.0, 1.0);
        input[50] = 1.0 - (base_dist / 100.0).min(1.0);
        // Same-species count nearby
        let mut same = 0.0;
        let mut enemy = 0.0;
        for &(list_idx, id, ox, oy, ospecies) in positions {
            if id == c.id { continue; }
            let d2 = (ox - c.x).powi(2) + (oy - c.y).powi(2);
            if d2 < 400.0 {
                if ospecies == c.species { same += 1.0; } else { enemy += 1.0; }
            }
        }
        input[51] = (same / 10.0f32).clamp(0.0f32, 1.0f32);
        input[52] = (enemy / 10.0f32).clamp(0.0f32, 1.0f32);
        input[53] = match c.role { Role::Queen => 1.0, _ => 0.0 };
        input[54] = match c.role { Role::Soldier => 1.0, _ => 0.0 };
        input[55] = day_night;

        // Role one-hot - slots 56-62
        input[56] = if c.role == Role::Worker { 1.0 } else { 0.0 };
        input[57] = if c.role == Role::Priest { 1.0 } else { 0.0 };
        input[58] = if c.role == Role::Merchant { 1.0 } else { 0.0 };
        input[59] = if c.role == Role::Builder { 1.0 } else { 0.0 };
        input[60] = if c.role == Role::Explorer { 1.0 } else { 0.0 };
        input[61] = if c.role == Role::Shaman { 1.0 } else { 0.0 };
        input[62] = temperature / 40.0;

        // Tool & tech state (10) - slots 63-72
        input[63] = (c.tool as u8 as f32 / 14.0).clamp(0.0, 1.0);
        input[64] = (c.tool_durability as f32 / 2000.0).clamp(0.0, 1.0);
        input[65] = (c.tech_level as f32 / 4.0).clamp(0.0, 1.0);
        input[66] = (c.inventory.iter().sum::<u16>() as f32 / 2000.0).clamp(0.0, 1.0);
        input[67] = (c.craft_cooldown as f32 / 70.0).clamp(0.0, 1.0);
        input[68] = (c.inventory[0] as f32 / 50.0).clamp(0.0, 1.0); // stone
        input[69] = (c.inventory[1] as f32 / 50.0).clamp(0.0, 1.0); // metal
        input[70] = (c.inventory[2] as f32 / 30.0).clamp(0.0, 1.0); // copper
        input[71] = (c.inventory[3] as f32 / 20.0).clamp(0.0, 1.0); // gold
        input[72] = (c.inventory[4] as f32 / 20.0).clamp(0.0, 1.0); // crystal

        // Magic state (8) - slots 73-80
        input[73] = (c.mana / c.max_mana).clamp(0.0, 1.0);
        input[74] = (c.active_spell as u8 as f32 / 10.0).clamp(0.0, 1.0);
        input[75] = if c.known_spells > 0 { 1.0 } else { 0.0 };
        input[76] = (c.spell_cd[0] as f32 / 60.0).clamp(0.0, 1.0);
        input[77] = (c.spell_cd[1] as f32 / 60.0).clamp(0.0, 1.0);
        input[78] = (c.spell_cd[2] as f32 / 60.0).clamp(0.0, 1.0);
        input[79] = (c.spell_cd[3] as f32 / 60.0).clamp(0.0, 1.0);
        input[80] = (c.spell_cd[4] as f32 / 60.0).clamp(0.0, 1.0);

        // Mount/vehicle state (4) - slots 81-84
        input[81] = if c.mount_kind != MountKind::None { 1.0 } else { 0.0 };
        input[82] = c.mount_stamina / 100.0;
        input[83] = if c.vehicle.is_some() { 1.0 } else { 0.0 };
        input[84] = (c.reputation as f32 / 100.0).clamp(-1.0, 1.0);

        // Memory blackboard (8) - slots 85-92
        for i in 0..8 { input[85 + i] = c.memory[i]; }

        // Resource inventory detailed (16) - slots 93-108
        for i in 0..16 {
            input[93 + i] = (c.inventory[i] as f32 / 100.0).clamp(0.0, 1.0);
        }

        // Disease & social (8) - slots 109-116
        input[109] = match c.disease { Disease::None => 0.0, _ => 1.0 };
        input[110] = (c.disease_timer as f32 / 300.0).clamp(0.0, 1.0);
        input[111] = (c.loyalty as f32 / 100.0).clamp(-1.0, 1.0);
        input[112] = (c.devotion as f32 / 100.0).clamp(0.0, 1.0);
        input[113] = (c.mood as u8 as f32 / 9.0).clamp(0.0, 1.0);
        input[114] = (c.mood_timer as f32 / 180.0).clamp(0.0, 1.0);
        input[115] = (c.trade_partner.map_or(0, |p| p) as f32 / 1000.0).clamp(0.0, 1.0);
        input[116] = (c.squad_id as f32 / 10.0).clamp(0.0, 1.0);

        // Genetics (16) - slots 117-132
        for i in 0..16 { input[117 + i] = c.genes[i]; }

        // Offspring & kills (4) - slots 133-136
        input[133] = (c.offspring as f32 / 20.0).clamp(0.0, 1.0);
        input[134] = (c.kills as f32 / 20.0).clamp(0.0, 1.0);
        input[135] = (c.generation as f32 / 50.0).clamp(0.0, 1.0);
        input[136] = (c.mutations as f32 / 20.0).clamp(0.0, 1.0);

        // Season (4) - slots 137-140
        input[137] = if season == 0 { 1.0 } else { 0.0 };
        input[138] = if season == 1 { 1.0 } else { 0.0 };
        input[139] = if season == 2 { 1.0 } else { 0.0 };
        input[140] = if season == 3 { 1.0 } else { 0.0 };

        input
    }

    pub fn update_stats(&mut self) {
        let pop = self.list.len();
        if pop == 0 {
            self.stats = CreatureStats {
                population: 0,
                species: 0,
                ..Default::default()
            };
            self.stats.births = self.births_total;
            self.stats.deaths = self.deaths_total;
            return;
        }

        let mut species_ids: Vec<u32> = self.list.iter().map(|c| c.species).collect();
        species_ids.sort_unstable();
        species_ids.dedup();

        let mut fit = 0.0;
        let mut energy = 0.0;
        let mut mana = 0.0;
        let mut loyalty_sum = 0;
        let mut max_gen = 0;
        let mut workers = 0;
        let mut soldiers = 0;
        let mut queens = 0;
        let mut priests = 0;
        let mut merchants = 0;
        let mut builders = 0;
        let mut explorers = 0;
        let mut shamans = 0;
        let mut tools_pickaxe = 0;
        let mut tools_sword = 0;
        let mut tools_hammer = 0;
        let mut tools_drill = 0;
        let mut tools_scanner = 0;
        let mut tools_bow = 0;
        let mut tools_spear = 0;
        let mut tools_shield = 0;
        let mut tools_staff = 0;
        let mut tools_amulet = 0;
        let mut tools_backpack = 0;
        let mut tools_cart = 0;
        let mut tools_boat = 0;
        let mut tools_glider = 0;
        let mut tech_stone = 0;
        let mut tech_copper = 0;
        let mut tech_iron = 0;
        let mut tech_gold = 0;
        let mut tech_crystal = 0;
        let mut disease_count = 0;
        let mut spell_count = 0u32;
        let mut trade_count = 0u32;
        let mut mount_count = 0u32;
        let mut hunger_sum = 0.0;
        let mut thirst_sum = 0.0;
        let mut stamina_sum = 0.0;

        for c in &self.list {
            fit += c.fitness;
            energy += c.energy;
            mana += c.mana;
            loyalty_sum += c.loyalty as i32;
            max_gen = max_gen.max(c.generation);
            hunger_sum += c.hunger;
            thirst_sum += c.thirst;
            stamina_sum += c.stamina;
            if c.active_spell != Spell::None { spell_count += 1; }
            if c.mount_kind != MountKind::None { mount_count += 1; }
            if c.disease != Disease::None {
                disease_count += 1;
            }
            match c.role {
                Role::Worker => workers += 1,
                Role::Soldier => soldiers += 1,
                Role::Queen => queens += 1,
                Role::Priest => priests += 1,
                Role::Merchant => merchants += 1,
                Role::Builder => builders += 1,
                Role::Explorer => explorers += 1,
                Role::Shaman => shamans += 1,
            }
            match c.tool {
                Tool::Pickaxe => tools_pickaxe += 1,
                Tool::Sword => tools_sword += 1,
                Tool::Hammer => tools_hammer += 1,
                Tool::Drill => tools_drill += 1,
                Tool::Scanner => tools_scanner += 1,
                Tool::Bow => tools_bow += 1,
                Tool::Spear => tools_spear += 1,
                Tool::Shield => tools_shield += 1,
                Tool::Staff => tools_staff += 1,
                Tool::Amulet => tools_amulet += 1,
                Tool::Backpack => tools_backpack += 1,
                Tool::Cart => tools_cart += 1,
                Tool::Boat => tools_boat += 1,
                Tool::Glider => tools_glider += 1,
                _ => {}
            }
            match c.tech_level {
                0 => tech_stone += 1,
                1 => tech_copper += 1,
                2 => tech_iron += 1,
                3 => tech_gold += 1,
                4 => tech_crystal += 1,
                _ => {}
            }
        }

        self.stats = CreatureStats {
            population: pop as u32,
            species: species_ids.len() as u32,
            avg_fitness: fit / pop as f32,
            avg_energy: energy / pop as f32,
            max_generation: max_gen,
            births: self.births_total,
            deaths: self.deaths_total,
            workers,
            soldiers,
            queens,
            priests,
            merchants,
            builders,
            explorers,
            shamans,
            tools_pickaxe,
            tools_sword,
            tools_hammer,
            tools_drill,
            tools_scanner,
            tools_bow,
            tools_spear,
            tools_shield,
            tools_staff,
            tools_amulet,
            tools_backpack,
            tools_cart,
            tools_boat,
            tools_glider,
            tech_stone,
            tech_copper,
            tech_iron,
            tech_gold,
            tech_crystal,
            avg_mana: mana / pop as f32,
            avg_loyalty: loyalty_sum as f32 / pop as f32,
            diseases: disease_count,
            spells_cast: spell_count,
            trades: self.trades_total as u32,
            mounts: mount_count,
            structures: self.stats.structures,
            avg_hunger: hunger_sum / pop as f32,
            avg_thirst: thirst_sum / pop as f32,
            avg_stamina: stamina_sum / pop as f32,
        };
    }
}

impl Creature {
    fn child_blueprint(&self) -> Creature {
        Creature {
            id: 0,
            x: self.x,
            y: self.y,
            vx: 0.0,
            vy: 0.0,
            energy: 380.0,
            max_energy: if self.role == Role::Soldier { 1500.0 } else { 1200.0 },
            health: if self.role == Role::Soldier { 150.0 } else { 100.0 },
            age: 0,
            net: self.net.clone_net(),
            color: self.color,
            species: self.species,
            generation: self.generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 0,
            size: self.size,
            last_x: self.x,
            last_y: self.y,
            name: generate_name(&mut Rng::new(self.id.wrapping_mul(0x9E3779B9) as u64), self.role, self.species),
            team_color: self.team_color,
            role: self.role,
            base_x: self.base_x,
            base_y: self.base_y,
            carried: 0.0,
            build_cooldown: 0,
            target_x: self.base_x,
            target_y: self.base_y,
            tool: Tool::None,
            tool_durability: 0,
            tech_level: self.tech_level,
            inventory: [0; 16],
            craft_cooldown: 0,
            disease: Disease::None,
            disease_timer: 0,
            immunity: [0; 8],
            belief: self.belief,
            devotion: 0,
            mood: Mood::Neutral,
            mood_timer: 0,
            loyalty: 0,
            reputation: 0,
            trade_partner: None,
            squad_id: 0,
            squad_role: 0,
            mana: 0.0,
            max_mana: 50.0,
            spell_cooldown: 0,
            known_spells: self.known_spells,
            active_spell: self.active_spell,
            spell_cd: [0; 10],
            mount: None,
            vehicle: None,
            mount_kind: MountKind::None,
            mount_stamina: 100.0,
            genes: {
                let mut g = self.genes;
                for i in 0..32 {
                    // Deterministic mutation based on id, age, and gene index
                    let mut seed = self.id.wrapping_mul(i as u32 + 1).wrapping_add(self.age);
                    seed ^= seed >> 13;
                    seed ^= seed << 17;
                    seed ^= seed >> 5;
                    let rand_val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                    if (seed % 20) == 0 { // 5% chance
                        g[i] = (g[i] + rand_val * 0.1).clamp(0.0, 1.0);
                    }
                }
                g
            },
            mutations: self.mutations + 1,
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
        }
    }
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
