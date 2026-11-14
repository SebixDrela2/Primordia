use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction {

    Player,

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

pub const ZORG_COLOR: [u8; 3] = [148, 62, 210];

pub const PLAYER_HUES: [f32; 14] = [
    0.0,
    18.0,
    35.0,
    50.0,
    65.0,
    95.0,
    125.0,
    155.0,
    180.0,
    200.0,
    220.0,
    245.0,
    335.0,
    352.0,
];

impl CreatureSystem {

    pub fn is_zorg(&self, species: u32) -> bool {
        species == ZORG_SPECIES
    }

    pub fn is_zorg_creature(&self, c: &Creature) -> bool {
        c.species == ZORG_SPECIES
    }

    pub fn zorg_count(&self) -> u32 {
        self.list.iter().filter(|c| self.is_zorg(c.species)).count() as u32
    }
}

pub fn zorg_act_eat(world: &mut World, c: &mut Creature, hungry: bool) -> bool {
    if c.hunger < 15.0 && c.energy > c.max_energy * 0.55 {
        return false;
    }
    let gx = c.x.round() as i32;
    let gy = c.y.round() as i32;

    let zradius = if hungry { 3 } else { 2 };
    let Some((fx, fy)) = world.find_zorg_food(gx, gy, zradius) else {
        return false;
    };
    let eaten_elem = world.get(fx, fy);
    let Some((nutrition, toxicity)) = world.eat_zorg(fx, fy) else {
        return false;
    };

    c.last_meal = eaten_elem;
    c.energy = (c.energy + nutrition * 4.0).min(c.max_energy);
    c.hunger = (c.hunger - nutrition * 4.0).max(0.0);

    let e = world.get(fx, fy);
    if zorg_nutrition(e) >= 40.0 {
        c.health = (c.health + 6.0).min(100.0);
    }
    if toxicity > 0.0 {
        c.health -= toxicity * 0.25;
    }
    true
}

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

pub fn zorg_steer(c: &Creature) -> (f32, f32, f32) {
    let hx = c.base_x - c.x;
    let hy = c.base_y - c.y;
    let hd = (hx * hx + hy * hy).sqrt().max(0.1);

    let needs_food = c.hunger >= 45.0 || c.energy < c.max_energy * 0.45;
    if needs_food {

        (hx / hd * 1.8, hy / hd * 1.8, 1.1)
    } else {

        (0.0, -0.55, 1.1)
    }
}

pub const ZORG_SPECIES: u32 = 0;

pub const ZORG_START: usize = 8;

pub const ZORG_POP_CAP: u32 = 90;

pub const ZORG_MIN: u32 = 6;

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

pub fn zorg_carried_disease(roll: u32) -> Disease {
    match roll % 5 {
        0 => Disease::Plague,
        1 => Disease::Infection,
        2 => Disease::Parasite,
        3 => Disease::Madness,
        _ => Disease::Flu,
    }
}

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

pub fn zorg_toxicity(e: Elem) -> f32 {
    let _ = e;
    0.0
}
