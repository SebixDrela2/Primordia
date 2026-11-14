//! The senses: what a creature can actually perceive.
//!
//! This is the heart of the change that made the brain work. Before, the
//! senses told the network what things *were*: "there is food that way",
//! "that is metal", "that is poison". A creature did not have to learn
//! anything, because the answer was handed to it, and the hard-coded
//! action layer did the rest: `out[2] > 0.3` meant "eat", and the code
//! went and found food within six cells and ate it. The network never had
//! to perceive anything at all.
//!
//! Now the senses tell the network what things *look like*. The network
//! gets colour, light, warmth, texture, wetness, gloss, smell and sound —
//! and nothing else. It has to work out for itself that green and soft
//! and sweet-smelling is worth eating, and that green and wet and
//! acid-smelling is worth avoiding.
//!
//! What a creature knows about *itself* — its hunger, its health, its
//! direction of travel — is still there. That is proprioception, not
//! knowledge of the world, and every animal has it.
//!
//! The other half of the change is in `src/creature/actions.rs`: the
//! actions are now *local*. A creature bites the cell it is next to, not
//! the best thing within a radius. To eat at all it has to perceive the
//! food, desire to go towards it, and arrive.

use super::*;
use crate::elements::Look;

/// The channels of the sight sense.
///
/// Sight is the wide sense: it reports what is over there, in every
/// direction at once, but only in terms of how things look.
pub mod sight {
    /// How far away the nearest thing in that direction is, from 0 (far) to
    /// 1 (touching).
    pub const NEAR: usize = 0;
    /// How much light comes back from that direction.
    pub const LIGHT: usize = 1;
    /// How much of the red, green and blue there is.
    pub const RED: usize = 2;
    pub const GREEN: usize = 3;
    pub const BLUE: usize = 4;
    /// How warm it is over there.
    pub const WARM: usize = 5;
    /// How yielding it is over there.
    pub const SOFT: usize = 6;
    /// How wet it is over there.
    pub const WET: usize = 7;
    /// How glossy it is over there.
    pub const SHEEN: usize = 8;
    /// Something alive is over there.
    ///
    /// This is the sight of another creature: a shape that moves. It says
    /// nothing about what the creature is, whether it is friendly, or
    /// whether it is good to eat. That is the network's work.
    pub const LIFE: usize = 9;
    /// How many channels the sense has.
    pub const CHANNELS: usize = 10;
}

/// The channels of the smell sense.
///
/// Smell is the sense that carries most of what sight cannot: an animal
/// can find food it cannot see. Each channel is a different kind of
/// scent, and the network has to work out which one means "worth eating".
pub mod smell {
    /// The sweet smell of something ripe.
    pub const SWEET: usize = 0;
    /// The sharp smell of rot.
    pub const ROTTEN: usize = 1;
    /// The acrid smell of chemicals and acid.
    pub const ACID: usize = 2;
    /// The smell of stone and metal.
    pub const MINERAL: usize = 3;
    /// The smell of smoke and burning.
    pub const SMOKE: usize = 4;
    /// The smell of my own kind.
    ///
    /// Every animal smells its own kind: it is how a creature that has
    /// wandered off finds the herd again, and it is how it knows that the
    /// shapes it can see are of its own species and not strangers. It
    /// carries much further than sight, which is the point of it.
    pub const KIN: usize = 5;
    /// How many channels the sense has.
    pub const CHANNELS: usize = 6;
}

/// The channels of the hearing sense.
///
/// Hearing tells a creature that something is happening, and roughly
/// where. It does not say what.
pub mod hearing {
    /// How loud it is.
    pub const LOUD: usize = 0;
    /// How many channels the sense has.
    pub const CHANNELS: usize = 1;
}

/// How many directions a creature can look in.
pub const DIRS: usize = 8;

/// How many channels one direction of perception has.
pub const DIR_CHANNELS: usize = sight::CHANNELS + smell::CHANNELS + hearing::CHANNELS;

/// Where the perception block starts in the input vector.
pub const PERCEPT_BASE: usize = 0;

/// Where the body block starts in the input vector.
pub const BODY_BASE: usize = PERCEPT_BASE + DIRS * DIR_CHANNELS;

/// How many channels the body has.
pub const BODY_CHANNELS: usize = 30;

/// The body channel that says whether there is open sky above.
pub const BODY_OPEN_SKY: usize = 28;

/// Which of the eight directions is up.
///
/// The directions run clockwise from south, so up is the fifth.
pub const DIR_UP: usize = 4;

/// Where the kin block starts in the input vector.
pub const KIN_BASE: usize = BODY_BASE + BODY_CHANNELS;

/// How many channels the kin block has.
pub const KIN_CHANNELS: usize = 15;

/// How many inputs the network has in total.
pub const TOTAL_INPUTS: usize = KIN_BASE + KIN_CHANNELS;

/// A uniform spatial hash so neighbour queries do not scan the whole colony.
///
/// This is not a sense: it is the bookkeeping that lets a creature find out
/// what is near it without walking the whole population.
pub(crate) struct SpatialGrid {
    cell: usize,
    gw: usize,
    gh: usize,
    buckets: Vec<Vec<u32>>,
}

impl SpatialGrid {
    pub(crate) fn new(w: usize, h: usize, cell: usize) -> Self {
        let gw = w / cell + 2;
        let gh = h / cell + 2;
        SpatialGrid {
            cell,
            gw,
            gh,
            buckets: (0..gw * gh).map(|_| Vec::new()).collect(),
        }
    }

    pub(crate) fn clear(&mut self) {
        for b in self.buckets.iter_mut() {
            b.clear();
        }
    }

    pub(crate) fn insert(&mut self, x: f32, y: f32, idx: u32) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        if gx < self.gw && gy < self.gh {
            self.buckets[gy * self.gw + gx].push(idx);
        }
    }

    /// Visit every creature whose bucket is within `radius` cells.
    pub(crate) fn near(&self, x: f32, y: f32, radius: usize, mut f: impl FnMut(u32)) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        let r = radius / self.cell + 1;
        let y0 = gy.saturating_sub(r);
        let y1 = (gy + r + 1).min(self.gh);
        let x0 = gx.saturating_sub(r);
        let x1 = (gx + r + 1).min(self.gw);
        for by in y0..y1 {
            for bx in x0..x1 {
                for &i in self.buckets[by * self.gw + bx].iter() {
                    f(i);
                }
            }
        }
    }
}

/// The primitive description of the world as one animal sees it.
///
/// This replaces the old `LocalScan`, which used to carry the answer key:
/// the direction of the nearest food, the direction of the nearest ore,
/// the direction of the nearest poison. All of that is gone. What is left
/// is only what an animal could work out by looking, listening and
/// smelling.
#[derive(Clone, Copy)]
pub(crate) struct Percept {
    /// What each of the eight directions looks, smells and sounds like.
    ///
    /// The layout inside each direction is: the sight channels, then the
    /// smell channels, then the sound.
    pub dir: [[f32; DIR_CHANNELS]; DIRS],
}

impl Percept {
    pub(crate) fn empty() -> Self {
        Percept {
            dir: [[0.0; DIR_CHANNELS]; DIRS],
        }
    }

    /// The sight channels of one direction.
    pub(crate) fn sight_of(&self, d: usize) -> &[f32] {
        &self.dir[d][..sight::CHANNELS]
    }

    /// The smell channels of one direction.
    pub(crate) fn smell_of(&self, d: usize) -> &[f32] {
        &self.dir[d][sight::CHANNELS..sight::CHANNELS + smell::CHANNELS]
    }
}

/// Element lookup tables, so the hot scan does not match on the enum.
pub(crate) struct ElemTables {
    /// What each element looks like.
    pub look: [Look; Elem::TABLE_LEN],
    /// How each element smells, per channel.
    pub scent: [[f32; smell::CHANNELS]; Elem::TABLE_LEN],
    /// How loud each element is.
    pub noise: [f32; Elem::TABLE_LEN],
}

impl ElemTables {
    pub(crate) fn new() -> Self {
        ElemTables {
            look: crate::elements::look_table(),
            scent: scent_table(),
            noise: noise_table(),
        }
    }
}

/// How each element smells.
///
/// The scents are what let a creature find food it cannot see, and what
/// let it learn that one smell is worth having and another is not. A
/// rotten thing and a sweet thing must smell different, or nothing can
/// be learnt from them.
///
/// Note that the sweet smell belongs to the things that are *ripe*, and
/// the acid smell to the things that will hurt. A plant and a pool of
/// poison both look green; only the nose tells them apart, and even then
/// the animal has to learn which is which.
fn scent_table() -> [[f32; smell::CHANNELS]; Elem::TABLE_LEN] {
    let mut t = [[0.0f32; smell::CHANNELS]; Elem::TABLE_LEN];
    let mut i = 0;
    while i < Elem::TABLE_LEN {
        let e = Elem::from_u8(i as u8);
        let s = match e {
            // Ripe and sweet: the things worth eating.
            Elem::Plant | Elem::Crop | Elem::Food | Elem::Honey => [1.0, 0.0, 0.0, 0.0, 0.0],
            Elem::Meat | Elem::Egg | Elem::Blood => [0.4, 0.3, 0.0, 0.0, 0.0],
            // Rot.
            Elem::Corpse => [0.0, 1.0, 0.2, 0.0, 0.0],
            Elem::Bacteria | Elem::Virus => [0.0, 0.9, 0.1, 0.0, 0.0],
            // The deep's reek.
            Elem::Fungus | Elem::Spore | Elem::Moss => [0.2, 0.5, 0.0, 0.0, 0.0],
            // Acrid.
            Elem::Acid | Elem::Poison | Elem::Chlorine => [0.0, 0.2, 1.0, 0.0, 0.0],
            Elem::Alcohol => [0.2, 0.0, 0.7, 0.0, 0.0],
            // Stone and metal.
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
            // Burning.
            Elem::Fire | Elem::Ember | Elem::Lava | Elem::Magma | Elem::Plasma => {
                [0.0, 0.0, 0.0, 0.0, 1.0]
            }
            Elem::Smoke | Elem::Ash | Elem::Charcoal | Elem::Gunpowder | Elem::Tar => {
                [0.0, 0.3, 0.1, 0.0, 0.8]
            }
            Elem::Oil => [0.1, 0.0, 0.0, 0.4, 0.2],
            // Damp.
            Elem::Water | Elem::Mud | Elem::Snow | Elem::Ice => [0.0, 0.0, 0.0, 0.0, 0.0],
            Elem::Roots | Elem::Wood | Elem::Leather => [0.2, 0.2, 0.0, 0.1, 0.0],
            _ => [0.0; 5],
        };
        // The sixth channel is the smell of the creature's own kind, and
        // that is not something a *material* has: it is filled in from the
        // creatures themselves, further down.
        let mut row = [0.0f32; smell::CHANNELS];
        row[..5].copy_from_slice(&s);
        t[i] = row;
        i += 1;
    }
    t
}

/// How loud each element is.
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

/// Which of the eight directions a thing lies in.
///
/// The directions run clockwise from south, so the fifth of them is up.
/// A thing that is exactly between two directions belongs to the closer
/// of the two, and a thing that is exactly on the observer does not
/// belong to any of them.
pub(crate) fn direction_of(dx: f32, dy: f32) -> usize {
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

/// Look at the world around a creature and report what it can perceive.
///
/// This is the whole sensory apparatus of an animal now. There is no
/// `find_food` in here, no "this is metal", no "this is poison". There is
/// only what is over there and how it looks, smells and sounds.
///
/// The other creatures are part of that: they are seen like anything else
/// (a shape that moves) and smelled like anything else (their own kind
/// smells like home). What a creature *makes* of them is not decided here.
pub(crate) fn scan_percept(
    world: &World,
    cx: i32,
    cy: i32,
    species: u32,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    grid: &SpatialGrid,
) -> Percept {
    TABLES.with(|tab| scan_percept_with(world, cx, cy, species, agents, grid, tab))
}

/// How far an animal can see.
const SIGHT_RANGE: i32 = 8;

/// How far an animal can smell.
///
/// Smell carries much further than sight. A creature can tell that
/// something sweet is over there long before it can see what it is, and
/// that is the whole reason it can find food at all: the surface is
/// thinly covered in plants, so a creature that can only smell eight
/// cells ahead would spend its life starving a few steps from dinner.
const SMELL_RANGE: i32 = 30;

/// How quickly a smell fades with distance.
///
/// The nose does not work like an eye: a smell does not vanish the moment
/// it is far away, it only gets fainter. So the falloff is gentle - a
/// plant twenty cells off still reads as clearly sweet - and it is
/// deliberately far flatter than the `1/step` weighting the sight uses.
const SMELL_FALLOFF: f32 = 8.0;

/// How far the smell of your own kind carries.
///
/// Much further than any other smell. A colony is held together by it: an
/// animal that has wandered off can still smell where its own kind is,
/// and that is what brings it home. This is the one thing in the world
/// that a creature knows by its nose alone, and it is what replaced the
/// hard-coded pull towards the base.
///
/// Forty cells: far enough to find the herd, near enough that a creature
/// on the far side of the map is not dragged across it by the smell of
/// its own kind.
const KIN_RANGE: f32 = 40.0;

/// How quickly the smell of your own kind fades.
///
/// Very gently: a colony two dozen cells away still smells like home.
const KIN_FALLOFF: f32 = 20.0;

/// How finely the far end of the smell is sampled.
///
/// Beyond the sight range every second cell is smelled, so the nose does
/// not have to look at every cell of a twenty-cell ray.
const SMELL_STEP: i32 = 2;

pub(crate) fn scan_percept_with(
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
        let mut found = false;
        let mut near = 0.0f32;
        let mut light = 0.0f32;
        let mut red = 0.0f32;
        let mut green = 0.0f32;
        let mut blue = 0.0f32;
        let mut warm = 0.0f32;
        let mut soft = 0.0f32;
        let mut wet = 0.0f32;
        let mut sheen = 0.0f32;
        // Smell does not come from one cell: it drifts. So the scent of a
        // direction is everything that can be smelled along it, and the
        // strongest smell wins, the way a real nose works.
        let mut scent = [0.0f32; smell::CHANNELS];
        // Sound carries further than sight.
        let mut loud = 0.0f32;

        for step in 1..=SMELL_RANGE {
            // The far end of the ray is sampled more coarsely: the nose is
            // good at telling that something is there, not at reading every
            // cell of it.
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
            // The smell falls off gently, so that a distant thing is still
            // worth walking towards.
            let sw = 1.0 / (1.0 + (step - 1) as f32 / SMELL_FALLOFF);
            if !found && step <= SIGHT_RANGE {
                // The first thing in this direction is the thing it sees.
                found = true;
                near = 1.0 - (step - 1) as f32 / SIGHT_RANGE as f32;
                light = lk.bright;
                red = lk.rgb[0];
                green = lk.rgb[1];
                blue = lk.rgb[2];
                warm = lk.warm;
                soft = lk.soft;
                wet = lk.wet;
                sheen = lk.sheen;
            } else {
                // Further things add their glow behind the nearer one.
                light = light.max(lk.bright * w);
                warm = warm.max(lk.warm * w);
            }
            // The smell along this direction.
            //
            // Smell carries much further than sight, so every cell along
            // the ray is smelled, not just the first one. Each kind of
            // scent is tracked on its own: the strongest *sweet* thing in
            // that direction is what the nose picks out of it, even if
            // there is plain stone in between.
            let sc = &tab.scent[k];
            for c in 0..smell::CHANNELS {
                if sc[c] * sw > scent[c] {
                    scent[c] = sc[c] * sw;
                }
            }
            // Loud things carry.
            let n = tab.noise[k];
            if n * sw > loud {
                loud = n * sw;
            }
        }

        let s = &mut p.dir[d];
        s[sight::NEAR] = near;
        s[sight::LIGHT] = light;
        s[sight::RED] = red;
        s[sight::GREEN] = green;
        s[sight::BLUE] = blue;
        s[sight::WARM] = warm;
        s[sight::SOFT] = soft;
        s[sight::WET] = wet;
        s[sight::SHEEN] = sheen;
        let sb = sight::CHANNELS;
        for c in 0..smell::CHANNELS {
            s[sb + c] = scent[c];
        }
        s[sb + smell::CHANNELS] = loud;
    }

    // ---- The other creatures ----
    //
    // A creature is seen like anything else: a shape that moves, over
    // there. And it is smelled: an animal can smell its own kind from
    // further away than it can see it, which is how a creature that has
    // wandered off finds the herd again.
    //
    // What the creature *makes* of what it sees is not decided here. This
    // is only the shape of the thing and the smell of it.
    {
        let mut life = [0.0f32; DIRS];
        let mut kin = [0.0f32; DIRS];
        grid.near(cx as f32, cy as f32, KIN_RANGE as usize, |idx| {
            let a = &agents[idx as usize];
            let dx = a.1 - cx as f32;
            let dy = a.2 - cy as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 0.5 || d > KIN_RANGE {
                return;
            }
            let dir = direction_of(dx, dy);
            // Seen: the nearer it is, the more of it there is.
            if d <= SIGHT_RANGE as f32 {
                let v = (1.0 - (d - 1.0) / SIGHT_RANGE as f32).max(0.0);
                if v > life[dir] {
                    life[dir] = v;
                }
            }
            // Smelt: only its own kind smells like home, and it carries a
            // long way.
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

    p
}

/// Which direction a thing is strongest in.
///
/// The animal does not know what it is smelling, only that the smell is
/// stronger one way than another. This picks that way out, and it refuses
/// to answer when the answer is not clear: a smell that is the same in
/// every direction is no direction at all, and teaching an animal to walk
/// towards a smell that is everywhere teaches it to walk in circles.
///
/// The strength that comes back is zero in that case, so the caller does
/// not have to think about it.
pub(crate) fn strongest_direction<F>(p: &Percept, f: F) -> (usize, f32)
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
    // Clear enough if it stands out, or if nothing else smells at all.
    if best_v > second_v * 1.25 || second_v < 0.05 {
        (best, best_v)
    } else {
        (best, 0.0)
    }
}

/// Write the body block: everything the creature knows about itself.
///
/// This is proprioception. An animal knows how hungry it is and which way
/// it is going, and that is not knowledge of the world: it is knowledge
/// of its own body. The distinction matters, because this is the *only*
/// place the network is allowed to be told a fact outright.
pub(crate) fn write_body(
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
    // How much light reaches the animal's own eyes.
    input[b + 23] = (0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos()).clamp(0.0, 1.0);
    // How warm it is.
    input[b + 24] = (temperature / 40.0).clamp(-1.0, 1.0);
    // How fresh the air is.
    input[b + 25] = world.fresh_air(
        c.x.round() as i32,
        c.y.round() as i32,
        2,
    );
    // Rain on the skin, and the wind. These are what a creature feels of
    // the weather: it does not know it is a "sandstorm", it knows the air
    // is full of grit and it is being blown about.
    input[b + 26] = match world.weather {
        crate::world::Weather::Rain | crate::world::Weather::Storm => 1.0,
        _ => 0.0,
    };
    input[b + 27] = match world.weather {
        crate::world::Weather::Storm | crate::world::Weather::Sandstorm => 1.0,
        crate::world::Weather::Rain => 0.4,
        _ => 0.0,
    };
    // How enclosed the animal is: it can feel whether it is under open
    // sky or shut in the rock.
    let cx = c.x.round() as i32;
    let cy = c.y.round() as i32;
    let open = world.get(cx, cy - 4) == Elem::Empty;
    input[b + 28] = if open { 1.0 } else { 0.0 };
    // How much light reaches the place it is standing in.
    //
    // This is not a reading of the depth: an animal does not know how far
    // it is under the ground, it only knows that it is dark. The light
    // says the same thing and says it honestly - the deep is dark, and
    // the surface is not.
    let depth = (cy - world.surface_y(cx)).max(0) as f32 / 240.0;
    let sky_light = (0.5 - 0.5 * (day_night * std::f32::consts::TAU).cos()).clamp(0.0, 1.0);
    input[b + 29] = (sky_light * (1.0 - depth.min(1.0))).clamp(0.0, 1.0);
}

/// Write the kin block: what the creature can see of its own kind, and of
/// others.
///
/// A creature recognises its own kind by how they look, and it can see
/// the colour of a stranger. It cannot see a treaty, and it is not told
/// one: whether the red ones are friends or enemies is something it has
/// to work out for itself.
pub(crate) fn write_kin(
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
    // Which way they are. The network has to be able to tie the colour
    // it sees to the direction it is in, or it can never learn that the
    // red ones are the ones to run from.
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
                // The colour of the nearest stranger, as the creature sees
                // it: the species colour is exactly what is on show.
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

/// Write the perception block into the input vector.
pub(crate) fn write_percept(p: &Percept, input: &mut [f32]) {
    for (d, dir) in p.dir.iter().enumerate() {
        let b = PERCEPT_BASE + d * DIR_CHANNELS;
        for c in 0..DIR_CHANNELS {
            input[b + c] = dir[c];
        }
    }
}

