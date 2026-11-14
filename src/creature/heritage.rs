use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {

    Juvenile,

    Adult,

    Elder,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Juvenile => "juvenile",
            Stage::Adult => "adult",
            Stage::Elder => "elder",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Stage::Juvenile => "young: too small to fight or breed",
            Stage::Adult => "in the prime of its life",
            Stage::Elder => "old: slow, but knows things",
        }
    }

    pub fn of(age: u32, max_age: u32, maturity: f32) -> Stage {
        let span = max_age.max(1) as f32;
        let t = age as f32 / span;
        if t < maturity {
            Stage::Juvenile
        } else if t < 0.78 {
            Stage::Adult
        } else {
            Stage::Elder
        }
    }

    pub fn size_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.55,
            Stage::Adult => 1.0,
            Stage::Elder => 1.08,
        }
    }

    pub fn speed_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.72,
            Stage::Adult => 1.0,
            Stage::Elder => 0.78,
        }
    }

    pub fn health_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.55,
            Stage::Adult => 1.0,
            Stage::Elder => 0.9,
        }
    }

    pub fn can_fight(self) -> bool {
        self != Stage::Juvenile
    }

    pub fn can_breed(self) -> bool {
        self != Stage::Juvenile
    }

    pub fn can_teach(self) -> bool {
        self == Stage::Elder
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Affinity {
    None,

    Ember,

    Tide,

    Crystal,

    Iron,

    Timber,

    Stone,
}

impl Affinity {
    pub fn name(self) -> &'static str {
        match self {
            Affinity::None => "unadapted",
            Affinity::Ember => "ember",
            Affinity::Tide => "tide",
            Affinity::Crystal => "crystal",
            Affinity::Iron => "iron",
            Affinity::Timber => "timber",
            Affinity::Stone => "stone",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Affinity::None => "no special adaptation",
            Affinity::Ember => "immune to fire and heat",
            Affinity::Tide => "swims freely, thirsts slowly",
            Affinity::Crystal => "mana flows through it",
            Affinity::Iron => "senses metal and digs towards it",
            Affinity::Timber => "builds and grows faster",
            Affinity::Stone => "hard to injure, hard to kill",
        }
    }

    pub fn from_roll(roll: f32) -> Affinity {

        if roll > 0.93 {
            Affinity::Ember
        } else if roll > 0.88 {
            Affinity::Tide
        } else if roll > 0.83 {
            Affinity::Crystal
        } else if roll > 0.78 {
            Affinity::Iron
        } else if roll > 0.73 {
            Affinity::Timber
        } else if roll > 0.70 {
            Affinity::Stone
        } else {
            Affinity::None
        }
    }

    pub fn carry_factor(self) -> f32 {
        match self {
            Affinity::Iron => 1.15,
            Affinity::Timber => 1.1,
            _ => 1.0,
        }
    }

    pub fn mana_factor(self) -> f32 {
        match self {
            Affinity::Crystal => 2.0,
            _ => 1.0,
        }
    }

    pub fn build_factor(self) -> f32 {
        match self {
            Affinity::Timber => 1.5,
            _ => 1.0,
        }
    }

    pub fn mine_factor(self) -> f32 {
        match self {
            Affinity::Iron => 1.6,
            _ => 1.0,
        }
    }

    pub fn armor_factor(self) -> f32 {
        match self {
            Affinity::Stone => 0.7,
            _ => 1.0,
        }
    }

    pub fn fire_immune(self) -> bool {
        matches!(self, Affinity::Ember | Affinity::Stone)
    }

    pub fn cold_vulnerability(self) -> f32 {
        match self {
            Affinity::Ember => 2.0,
            Affinity::Stone => 0.0,
            _ => 1.0,
        }
    }
}

pub mod culture_bits {

    pub const CULTIVATION: u16 = 1 << 0;

    pub const MINING: u16 = 1 << 1;

    pub const WARFARE: u16 = 1 << 2;

    pub const COMMERCE: u16 = 1 << 3;

    pub const CRAFT: u16 = 1 << 4;

    pub const RITUAL: u16 = 1 << 5;

    pub const ARCHITECTURE: u16 = 1 << 6;

    pub const HUNTING: u16 = 1 << 7;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Culture {
    Cultivation,
    Mining,
    Warfare,
    Commerce,
    Craft,
    Ritual,
    Architecture,
    Hunting,
}

impl Culture {
    pub fn name(self) -> &'static str {
        match self {
            Culture::Cultivation => "Cultivation",
            Culture::Mining => "Mining",
            Culture::Warfare => "Warfare",
            Culture::Commerce => "Commerce",
            Culture::Craft => "Craft",
            Culture::Ritual => "Ritual",
            Culture::Architecture => "Architecture",
            Culture::Hunting => "Hunting",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Culture::Cultivation => "the colony knows how to grow grain",
            Culture::Mining => "the colony knows how to dig",
            Culture::Warfare => "the colony knows how to fight",
            Culture::Commerce => "the colony knows how to trade",
            Culture::Craft => "the colony knows how to make things",
            Culture::Ritual => "the colony keeps the old rites",
            Culture::Architecture => "the colony knows how to build",
            Culture::Hunting => "the colony knows how to hunt",
        }
    }

    pub fn bit(self) -> u16 {
        match self {
            Culture::Cultivation => culture_bits::CULTIVATION,
            Culture::Mining => culture_bits::MINING,
            Culture::Warfare => culture_bits::WARFARE,
            Culture::Commerce => culture_bits::COMMERCE,
            Culture::Craft => culture_bits::CRAFT,
            Culture::Ritual => culture_bits::RITUAL,
            Culture::Architecture => culture_bits::ARCHITECTURE,
            Culture::Hunting => culture_bits::HUNTING,
        }
    }

    pub fn from_bit(bit: u16) -> Option<Culture> {
        Some(match bit {
            culture_bits::CULTIVATION => Culture::Cultivation,
            culture_bits::MINING => Culture::Mining,
            culture_bits::WARFARE => Culture::Warfare,
            culture_bits::COMMERCE => Culture::Commerce,
            culture_bits::CRAFT => Culture::Craft,
            culture_bits::RITUAL => Culture::Ritual,
            culture_bits::ARCHITECTURE => Culture::Architecture,
            culture_bits::HUNTING => Culture::Hunting,
            _ => return None,
        })
    }
}

pub fn culture_list(bits: u16) -> Vec<Culture> {
    let mut v = Vec::new();
    for bit in [
        culture_bits::CULTIVATION,
        culture_bits::MINING,
        culture_bits::WARFARE,
        culture_bits::COMMERCE,
        culture_bits::CRAFT,
        culture_bits::RITUAL,
        culture_bits::ARCHITECTURE,
        culture_bits::HUNTING,
    ] {
        if bits & bit != 0 {
            if let Some(c) = Culture::from_bit(bit) {
                v.push(c);
            }
        }
    }
    v
}

pub fn gene_distance(a: &[f32; 32], b: &[f32; 32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..32 {
        sum += (a[i] - b[i]).abs();
    }
    (sum / 32.0).clamp(0.0, 1.0)
}

impl CreatureSystem {

    pub fn learn_culture(&mut self, index: usize, culture: Culture) {
        if index >= self.list.len() {
            return;
        }
        let bit = culture.bit();
        if self.list[index].culture & bit != 0 {
            return;
        }
        if self.rng.chance(0.05) {
            let species = self.list[index].species;
            self.list[index].culture |= bit;
            self.note_culture(species, bit);
        }
    }
}

impl Creature {

    pub fn lifespan(&self) -> u32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            4800
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            2100
        } else {
            MAX_AGE
        }
    }

    pub fn maturity_fraction(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            0.26
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            0.10
        } else {
            0.18
        }
    }

    pub fn litter_size(&self) -> u32 {
        if self.traits & trait_bits::PROLIFIC != 0 {
            3
        } else if self.traits & trait_bits::PROLONGED != 0 {
            1
        } else {
            2
        }
    }

    pub fn brood_cost(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            190.0
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            70.0
        } else {
            120.0
        }
    }

    pub fn brood_value(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            1.5
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            0.6
        } else {
            1.0
        }
    }

    pub fn max_health(&self) -> f32 {
        let base = if self.role == Role::Soldier { 150.0 } else { 100.0 };
        let hardy = if self.traits & trait_bits::HARDY != 0 { 1.4 } else { 1.0 };
        let hollow = if self.traits & trait_bits::HOLLOW_BONED != 0 {
            0.75
        } else {
            1.0
        };

        let steel = if self.knows_tech(Discovery::Metallurgy) { 1.2 } else { 1.0 };
        base * hardy * hollow * steel * self.stage.health_factor()
    }

    pub fn knows_tech(&self, d: Discovery) -> bool {
        self.tech & d.bit() != 0
    }

    pub fn tech_name(&self) -> String {
        crate::creature::discovery_list(self.tech)
            .iter()
            .map(|d| d.name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn salve_power(&self, discoveries: u32) -> f32 {
        let mut power = 1.0;
        if discoveries & discovery_bits::MEDICINE != 0 {
            power = 1.5;
        }
        if discoveries & discovery_bits::ALCHEMY != 0 {
            power *= 1.2;
        }
        power
    }

    pub fn carry_cap(&self) -> f32 {
        let mut cap = CARRY_CAP;
        if self.traits & trait_bits::HOARDER != 0 {
            cap *= 1.6;
        }
        cap * self.affinity.carry_factor()
    }

    pub fn speed_cap(&self) -> f32 {
        let mut speed = MAX_SPEED;
        if self.traits & trait_bits::HOLLOW_BONED != 0 {
            speed *= 1.35;
        }
        speed * self.stage.speed_factor()
    }

    pub fn body_size(&self) -> f32 {
        (self.size * self.stage.size_factor()).max(0.6)
    }

    pub fn damage_taken(&self, amount: f32) -> f32 {
        let mut dmg = amount * self.affinity.armor_factor();
        if self.knows_tech(Discovery::Tactics) {
            dmg *= 0.85;
        }
        dmg
    }

    pub fn knows(&self, c: Culture) -> bool {
        self.culture & c.bit() != 0
    }

    pub fn culture_factor(&self, c: Culture) -> f32 {
        if !self.knows(c) {
            return 1.0;
        }
        match c {
            Culture::Cultivation => 1.5,
            Culture::Mining => 1.4,
            Culture::Warfare => 1.25,
            Culture::Commerce => 1.35,
            Culture::Craft => 1.3,
            Culture::Ritual => 1.25,
            Culture::Architecture => 1.35,
            Culture::Hunting => 1.3,
        }
    }

    pub fn hunting_bonus(&self) -> f32 {
        self.culture_factor(Culture::Hunting)
    }

    pub fn vision_factor(&self, day_night: f32) -> f32 {
        let nocturnal = self.traits & trait_bits::NOCTURNAL != 0;

        let night = 1.0 - (day_night * 2.0 - 0.5).abs();
        if nocturnal {
            1.0 + night * 0.8
        } else {
            1.0 + (1.0 - night) * 0.35
        }
    }
}
