use super::{Elem, DIRS8, World, Creature, MAX_AGE, Disease, species_color};
use crate::elements::Look;

pub mod sight {

    pub const NEAR: usize = 0;

    pub const LIGHT: usize = 1;

    pub const RED: usize = 2;
    pub const GREEN: usize = 3;
    pub const BLUE: usize = 4;

    pub const WARM: usize = 5;

    pub const SOFT: usize = 6;

    pub const WET: usize = 7;

    pub const SHEEN: usize = 8;

    pub const LIFE: usize = 9;

    pub const CHANNELS: usize = 10;
}

pub mod smell {

    pub const SWEET: usize = 0;

    pub const ROTTEN: usize = 1;

    pub const ACID: usize = 2;

    pub const MINERAL: usize = 3;

    pub const SMOKE: usize = 4;

    pub const KIN: usize = 5;

    pub const CHANNELS: usize = 6;
}

pub mod hearing {

    pub const LOUD: usize = 0;

    pub const CHANNELS: usize = 1;
}

pub const DIRS: usize = 8;

pub const DIR_CHANNELS: usize = sight::CHANNELS + smell::CHANNELS + hearing::CHANNELS;

pub const PERCEPT_BASE: usize = 0;

pub const BODY_BASE: usize = PERCEPT_BASE + DIRS * DIR_CHANNELS;

pub const BODY_CHANNELS: usize = 31;

pub const BODY_CROWDED: usize = 30;

pub const BODY_OPEN_SKY: usize = 28;

pub const BODY_LIGHT: usize = 29;

pub const BODY_AIR: usize = 25;

pub const DIR_UP: usize = 4;

pub const KIN_BASE: usize = BODY_BASE + BODY_CHANNELS;

pub const KIN_CHANNELS: usize = 15;

pub const TOTAL_INPUTS: usize = KIN_BASE + KIN_CHANNELS;

pub struct SpatialGrid {
    cell: usize,
    gw: usize,
    gh: usize,
    buckets: Vec<Vec<u32>>,
}

impl SpatialGrid {
    #[must_use]
    pub fn new(w: usize, h: usize, cell: usize) -> Self {
        let gw = w / cell + 2;
        let gh = h / cell + 2;
        SpatialGrid {
            cell,
            gw,
            gh,
            buckets: (0..gw * gh).map(|_| Vec::new()).collect(),
        }
    }

    pub fn clear(&mut self) {
        for b in &mut self.buckets {
            b.clear();
        }
    }

    pub fn insert(&mut self, x: f32, y: f32, idx: u32) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        if gx < self.gw && gy < self.gh {
            self.buckets[gy * self.gw + gx].push(idx);
        }
    }

    pub fn near(&self, x: f32, y: f32, radius: usize, mut f: impl FnMut(u32)) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        let r = radius / self.cell + 1;
        let y0 = gy.saturating_sub(r);
        let y1 = (gy + r + 1).min(self.gh);
        let x0 = gx.saturating_sub(r);
        let x1 = (gx + r + 1).min(self.gw);
        for by in y0..y1 {
            for bx in x0..x1 {
                for &i in &self.buckets[by * self.gw + bx] {
                    f(i);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Percept {

    pub dir: [[f32; DIR_CHANNELS]; DIRS],
}

impl Percept {
    #[must_use]
    pub fn empty() -> Self {
        Percept {
            dir: [[0.0; DIR_CHANNELS]; DIRS],
        }
    }

    #[must_use]
    pub fn sight_of(&self, d: usize) -> &[f32] {
        &self.dir[d][..sight::CHANNELS]
    }

    #[must_use]
    pub fn smell_of(&self, d: usize) -> &[f32] {
        &self.dir[d][sight::CHANNELS..sight::CHANNELS + smell::CHANNELS]
    }
}

pub struct ElemTables {

    pub look: [Look; Elem::TABLE_LEN],

    pub scent: [[f32; smell::CHANNELS]; Elem::TABLE_LEN],

    pub noise: [f32; Elem::TABLE_LEN],
}

impl ElemTables {
    #[must_use]
    pub fn new() -> Self {
        ElemTables {
            look: crate::elements::look_table(),
            scent: scent_table(),
            noise: noise_table(),
        }
    }
}

fn scent_table() -> [[f32; smell::CHANNELS]; Elem::TABLE_LEN] {
    let mut t = [[0.0f32; smell::CHANNELS]; Elem::TABLE_LEN];
    let mut i = 0;
    while i < Elem::TABLE_LEN {
        let e = Elem::from_u8(i as u8);
        let s = match e {

            Elem::Plant | Elem::Crop | Elem::Food | Elem::Honey => [1.0, 0.0, 0.0, 0.0, 0.0],
            Elem::Meat | Elem::Egg | Elem::Blood => [0.4, 0.3, 0.0, 0.0, 0.0],

            Elem::Corpse => [0.0, 1.0, 0.2, 0.0, 0.0],
            Elem::Bacteria | Elem::Virus => [0.0, 0.9, 0.1, 0.0, 0.0],

            Elem::Fungus | Elem::Spore | Elem::Moss => [0.2, 0.5, 0.0, 0.0, 0.0],

            Elem::Acid | Elem::Poison | Elem::Chlorine => [0.0, 0.2, 1.0, 0.0, 0.0],
            Elem::Alcohol => [0.2, 0.0, 0.7, 0.0, 0.0],

            Elem::Stone
            | Elem::Granite
            | Elem::Metal
            | Elem::Copper
            | Elem::Tin
            | Elem::Bronze
            | Elem::Iron
            | Elem::Steel
            | Elem::Gold
            | Elem::Silver
            | Elem::Rust
            | Elem::Glass
            | Elem::Crystal
            | Elem::Gem
            | Elem::Diamond
            | Elem::Obsidian
            | Elem::Brick
            | Elem::Concrete => [0.0, 0.0, 0.0, 0.7, 0.0],
            Elem::Salt | Elem::Sulfur => [0.0, 0.1, 0.3, 0.5, 0.0],

            Elem::Fire | Elem::Ember | Elem::Lava | Elem::Magma | Elem::Plasma => {
                [0.0, 0.0, 0.0, 0.0, 1.0]
            }
            Elem::Smoke | Elem::Ash | Elem::Charcoal | Elem::Gunpowder | Elem::Tar => {
                [0.0, 0.3, 0.1, 0.0, 0.8]
            }
            Elem::Oil => [0.1, 0.0, 0.0, 0.4, 0.2],

            Elem::Water | Elem::Mud | Elem::Snow | Elem::Ice => [0.0, 0.0, 0.0, 0.0, 0.0],
            Elem::Roots | Elem::Wood | Elem::Leather => [0.2, 0.2, 0.0, 0.1, 0.0],
            _ => [0.0; 5],
        };

        let mut row = [0.0f32; smell::CHANNELS];
        row[..5].copy_from_slice(&s);
        t[i] = row;
        i += 1;
    }
    t
}

fn noise_table() -> [f32; Elem::TABLE_LEN] {
    let mut t = [0.0f32; Elem::TABLE_LEN];
    let mut i = 0;
    while i < Elem::TABLE_LEN {
        t[i] = match Elem::from_u8(i as u8) {
            Elem::Fire | Elem::Lava | Elem::Magma | Elem::Plasma => 0.9,
            Elem::Water | Elem::Steam | Elem::Acid | Elem::Poison => 0.5,
            Elem::Gunpowder => 1.0,
            Elem::Smoke => 0.2,
            _ => 0.0,
        };
        i += 1;
    }
    t
}

thread_local! {
    static TABLES: ElemTables = ElemTables::new();
}

#[must_use]
pub fn direction_of(dx: f32, dy: f32) -> usize {
    let mut best = 0usize;
    let mut best_dot = f32::MIN;
    for (i, (bx, by)) in DIRS8.iter().enumerate() {
        let len = ((bx * bx + by * by) as f32).sqrt();
        let dot = (dx * *bx as f32 + dy * *by as f32) / len;
        if dot > best_dot {
            best_dot = dot;
            best = i;
        }
    }
    best
}

#[must_use]
pub fn scan_percept(
    world: &World,
    cx: i32,
    cy: i32,
    species: u32,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
) -> Percept {
    TABLES.with(|tab| scan_percept_with(world, cx, cy, species, agents, grid, tab))
}

const SIGHT_RANGE: i32 = 8;

const SMELL_RANGE: i32 = 42;

const SMELL_FALLOFF: f32 = 8.0;

const KIN_RANGE: f32 = 120.0;

const KIN_FALLOFF: f32 = 40.0;

const SMELL_STEP: i32 = 2;

/// What one ray of the senses came up with: the sight of the nearest
/// thing in that direction, its colours and its textures, the scents
/// that drift down the ray and the noise that carries along it.
struct RaySense {
    found: bool,
    near: f32,
    light: f32,
    red: f32,
    green: f32,
    blue: f32,
    warm: f32,
    soft: f32,
    wet: f32,
    sheen: f32,
    scent: [f32; smell::CHANNELS],
    loud: f32,
}

impl RaySense {
    fn empty() -> Self {
        RaySense {
            found: false,
            near: 0.0,
            light: 0.0,
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            warm: 0.0,
            soft: 0.0,
            wet: 0.0,
            sheen: 0.0,
            scent: [0.0; smell::CHANNELS],
            loud: 0.0,
        }
    }
}

fn scan_ray(world: &World, tab: &ElemTables, cx: i32, cy: i32, dx: i32, dy: i32) -> RaySense {
    let mut ray = RaySense::empty();

    for step in 1..=SMELL_RANGE {
        let step_size = if step <= SIGHT_RANGE { 1 } else { SMELL_STEP };
        if step > SIGHT_RANGE && step % step_size != 0 {
            continue;
        }
        let sx = cx + dx * step;
        let sy = cy + dy * step;
        if sx < 0 || sy < 0 || sx >= world.w as i32 || sy >= world.h as i32 {
            continue;
        }
        let k = world.kind[sy as usize * world.w + sx as usize] as usize;
        if k == Elem::Empty as usize {
            continue;
        }
        let lk = &tab.look[k];
        let w = 1.0 / step as f32;

        let sw = 1.0 / (1.0 + (step - 1) as f32 / SMELL_FALLOFF);
        if !ray.found && step <= SIGHT_RANGE {
            ray.found = true;
            ray.near = 1.0 - (step - 1) as f32 / SIGHT_RANGE as f32;
            ray.light = lk.bright;
            ray.red = lk.rgb[0];
            ray.green = lk.rgb[1];
            ray.blue = lk.rgb[2];
            ray.warm = lk.warm;
            ray.soft = lk.soft;
            ray.wet = lk.wet;
            ray.sheen = lk.sheen;
        } else {
            ray.light = ray.light.max(lk.bright * w);
            ray.warm = ray.warm.max(lk.warm * w);
        }

        let sc = &tab.scent[k];
        for (c, &scent_value) in sc.iter().enumerate() {
            if scent_value * sw > ray.scent[c] {
                ray.scent[c] = scent_value * sw;
            }
        }

        let n = tab.noise[k];
        if n * sw > ray.loud {
            ray.loud = n * sw;
        }
    }

    ray
}

fn scan_kin(
    p: &mut Percept,
    cx: i32,
    cy: i32,
    species: u32,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
) {
    let mut life = [0.0f32; DIRS];
    let mut kin = [0.0f32; DIRS];
    grid.near(cx as f32, cy as f32, KIN_RANGE as usize, |idx| {
        let a = &agents[idx as usize];
        let dx = a.1 - cx as f32;
        let dy = a.2 - cy as f32;
        let d = (dx * dx + dy * dy).sqrt();
        if !(0.5..=KIN_RANGE).contains(&d) {
            return;
        }
        let dir = direction_of(dx, dy);

        if d <= SIGHT_RANGE as f32 {
            let v = (1.0 - (d - 1.0) / SIGHT_RANGE as f32).max(0.0);
            if v > life[dir] {
                life[dir] = v;
            }
        }

        if a.3 == species {
            let w = 1.0 / (1.0 + (d - 1.0) / KIN_FALLOFF);
            if w > kin[dir] {
                kin[dir] = w;
            }
        }
    });
    for d in 0..DIRS {
        p.dir[d][sight::LIFE] = life[d];
        p.dir[d][sight::CHANNELS + smell::KIN] = kin[d];
    }
}

#[must_use]
pub fn scan_percept_with(
    world: &World,
    cx: i32,
    cy: i32,
    species: u32,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
    tab: &ElemTables,
) -> Percept {
    let mut p = Percept::empty();

    for (d, (dx, dy)) in DIRS8.iter().enumerate() {
        let ray = scan_ray(world, tab, cx, cy, *dx, *dy);

        let s = &mut p.dir[d];
        s[sight::NEAR] = ray.near;
        s[sight::LIGHT] = ray.light;
        s[sight::RED] = ray.red;
        s[sight::GREEN] = ray.green;
        s[sight::BLUE] = ray.blue;
        s[sight::WARM] = ray.warm;
        s[sight::SOFT] = ray.soft;
        s[sight::WET] = ray.wet;
        s[sight::SHEEN] = ray.sheen;
        let sb = sight::CHANNELS;
        s[sb..sb + smell::CHANNELS].copy_from_slice(&ray.scent[..smell::CHANNELS]);
        s[sb + smell::CHANNELS] = ray.loud;
    }

    scan_kin(&mut p, cx, cy, species, agents, grid);

    p
}

#[must_use]
pub fn strongest_direction<F>(p: &Percept, f: F) -> (usize, f32)
where
    F: Fn(&[f32], &[f32]) -> f32,
{
    let mut best = 0usize;
    let mut best_v = f32::MIN;
    let mut second_v = 0.0f32;
    for d in 0..DIRS {
        let v = f(p.sight_of(d), p.smell_of(d));
        if v > best_v {
            second_v = best_v.max(0.0);
            best_v = v;
            best = d;
        } else if v > second_v {
            second_v = v;
        }
    }
    if best_v <= 0.0 {
        return (0, 0.0);
    }

    if best_v > second_v * 1.25 || second_v < 0.05 {
        (best, best_v)
    } else {
        (best, 0.0)
    }
}

pub fn write_body(
    c: &Creature,
    world: &World,
    day_night: f32,
    temperature: f32,
    input: &mut [f32],
) {
    let b = BODY_BASE;
    input[b] = (c.energy / c.max_energy).clamp(0.0, 1.0);
    input[b + 1] = (c.health / c.max_health()).clamp(0.0, 1.0);
    input[b + 2] = (c.hunger / 100.0).clamp(0.0, 1.0);
    input[b + 3] = (c.thirst / 100.0).clamp(0.0, 1.0);
    input[b + 4] = (c.stamina / c.max_stamina).clamp(0.0, 1.0);
    input[b + 5] = (c.mana / c.max_mana).clamp(0.0, 1.0);
    input[b + 6] = if c.resting { 1.0 } else { 0.0 };
    input[b + 7] = (c.carried / c.carry_cap()).clamp(0.0, 1.0);
    input[b + 8] = (c.age as f32 / MAX_AGE as f32).clamp(0.0, 1.0);
    input[b + 9] = (c.vx / 2.2).clamp(-1.0, 1.0);
    input[b + 10] = (c.vy / 2.2).clamp(-1.0, 1.0);
    input[b + 11] = (c.vx.abs() + c.vy.abs()).clamp(0.0, 1.0);
    input[b + 12] = (c.tool as u8 as f32 / 15.0).clamp(0.0, 1.0);
    input[b + 13] = (c.tool_durability as f32 / 2000.0).clamp(0.0, 1.0);
    input[b + 14] = (c.tech_level as f32 / 4.0).clamp(0.0, 1.0);
    input[b + 15] = (c.inventory.iter().sum::<u16>() as f32 / 2000.0).clamp(0.0, 1.0);
    input[b + 16] = (c.craft_cooldown as f32 / 70.0).clamp(0.0, 1.0);
    input[b + 17] = (c.spell_cooldown as f32 / 60.0).clamp(0.0, 1.0);
    input[b + 18] = (c.fire_cooldown as f32 / 60.0).clamp(0.0, 1.0);
    input[b + 19] = (c.build_cooldown as f32 / 10.0).clamp(0.0, 1.0);
    input[b + 20] = if c.disease == Disease::None { 0.0 } else { 1.0 };
    input[b + 21] = (c.disease_timer as f32 / 300.0).clamp(0.0, 1.0);
    input[b + 22] = (c.mood as u8 as f32 / 9.0).clamp(0.0, 1.0);

    input[b + 23] = (0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos()).clamp(0.0, 1.0);

    input[b + 24] = (temperature / 40.0).clamp(-1.0, 1.0);

    input[b + 25] = world.fresh_air(
        c.x.round() as i32,
        c.y.round() as i32,
        2,
    );

    input[b + 26] = match world.weather {
        crate::world::Weather::Rain | crate::world::Weather::Storm => 1.0,
        _ => 0.0,
    };
    input[b + 27] = match world.weather {
        crate::world::Weather::Storm | crate::world::Weather::Sandstorm => 1.0,
        crate::world::Weather::Rain => 0.4,
        _ => 0.0,
    };

    let cx = c.x.round() as i32;
    let cy = c.y.round() as i32;
    let open = world.get(cx, cy - 4) == Elem::Empty;
    input[b + 28] = if open { 1.0 } else { 0.0 };

    let depth = (cy - world.surface_y(cx)).max(0) as f32 / 240.0;
    let sky_light = (0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos()).clamp(0.0, 1.0);
    input[b + 29] = (sky_light * (1.0 - depth.min(1.0))).clamp(0.0, 1.0);
}

pub fn write_kin(
    c: &Creature,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
    input: &mut [f32],
) {
    let k = KIN_BASE;
    let mut same_dist = 0.0f32;
    let mut same_count = 0.0f32;
    let mut other_dist = 0.0f32;
    let mut other_count = 0.0f32;
    let mut other_col = [0.0f32; 3];
    let mut other_size = 0.0f32;

    let mut same_dir = [0.0f32; 2];
    let mut other_dir = [0.0f32; 2];

    grid.near(c.x, c.y, 30, |idx| {
        let a = &agents[idx as usize];
        if a.0 == c.id {
            return;
        }
        let dx = a.1 - c.x;
        let dy = a.2 - c.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d > 30.0 {
            return;
        }
        if a.3 == c.species {
            same_count += 1.0;
            if same_dist == 0.0 || d < same_dist {
                same_dist = d;
                same_dir = [dx / d, dy / d];
            }
        } else {
            other_count += 1.0;
            if other_dist == 0.0 || d < other_dist {
                other_dist = d;
                other_dir = [dx / d, dy / d];

                let col = species_color(a.3);
                other_col = [
                    col[0] as f32 / 255.0,
                    col[1] as f32 / 255.0,
                    col[2] as f32 / 255.0,
                ];
                other_size = a.5;
            }
        }
    });

    let own = species_color(c.species);
    let own_col = [
        own[0] as f32 / 255.0,
        own[1] as f32 / 255.0,
        own[2] as f32 / 255.0,
    ];

    input[k] = if same_count > 0.0 {
        (1.0 - same_dist / 30.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    input[k + 1] = (same_count / 10.0).clamp(0.0, 1.0);
    input[k + 2] = if other_count > 0.0 {
        (1.0 - other_dist / 30.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    input[k + 3] = (other_count / 10.0).clamp(0.0, 1.0);
    input[k + 4] = other_col[0];
    input[k + 5] = other_col[1];
    input[k + 6] = other_col[2];
    input[k + 7] = other_size;
    input[k + 8] = own_col[0];
    input[k + 9] = own_col[1];
    input[k + 10] = own_col[2];
    input[k + 11] = same_dir[0];
    input[k + 12] = same_dir[1];
    input[k + 13] = other_dir[0];
    input[k + 14] = other_dir[1];
}

pub fn write_percept(p: &Percept, input: &mut [f32]) {
    for (d, dir) in p.dir.iter().enumerate() {
        let b = PERCEPT_BASE + d * DIR_CHANNELS;
        input[b..b + DIR_CHANNELS].copy_from_slice(&dir[..DIR_CHANNELS]);
    }
}

impl Default for ElemTables {
    fn default() -> Self {
        Self::new()
    }
}
