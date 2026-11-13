//! Heritage: what a creature is, what stage of life it is in, what it is
//! adapted to and what its species has learned.
//!
//! Three things live here:
//!
//! - **Stage**: a creature is a juvenile, an adult or an elder. The stage
//!   decides how big it is, how fast it moves, whether it can fight and
//!   whether it can breed.
//! - **Affinity**: a species drifts towards being adapted to one thing —
//!   fire, water, crystal, metal, timber or stone. The affinity gives a
//!   permanent edge in that domain.
//! - **Culture**: what the species has *learned to do*. A colony that has
//!   farmed for a thousand generations farms better than one that has only
//!   just discovered grain.

use super::*;

/// The stage of a creature's life.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// Young: small, slow, and under the wing of the adults.
    Juvenile,
    /// Fully grown: the workhorse of the colony.
    Adult,
    /// Old: slower and frailer, but wise.
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

    /// What this stage means for the animal.
    pub fn blurb(self) -> &'static str {
        match self {
            Stage::Juvenile => "young: too small to fight or breed",
            Stage::Adult => "in the prime of its life",
            Stage::Elder => "old: slow, but knows things",
        }
    }

    /// The stage of a creature of this age.
    ///
    /// `maturity` is the fraction of a life that is spent as a juvenile:
    /// a prolific species grows up quickly, a prolonged one takes its time.
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

    /// How large the body is at this stage.
    pub fn size_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.55,
            Stage::Adult => 1.0,
            Stage::Elder => 1.08,
        }
    }

    /// How fast it moves at this stage.
    pub fn speed_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.72,
            Stage::Adult => 1.0,
            Stage::Elder => 0.78,
        }
    }

    /// How much health it has at this stage.
    pub fn health_factor(self) -> f32 {
        match self {
            Stage::Juvenile => 0.55,
            Stage::Adult => 1.0,
            Stage::Elder => 0.9,
        }
    }

    /// Whether it is strong enough to fight.
    pub fn can_fight(self) -> bool {
        self != Stage::Juvenile
    }

    /// Whether it is old enough to breed.
    pub fn can_breed(self) -> bool {
        self != Stage::Juvenile
    }

    /// Whether it is past the age of raising young.
    pub fn can_teach(self) -> bool {
        self == Stage::Elder
    }
}

/// What a species has become adapted to.
///
/// The affinity is a single choice that shapes a whole lineage: a fire
/// people are immune to lava but cannot bear the cold, a water people swim
/// but never mine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Affinity {
    None,
    /// Ember: fire does not hurt, and the blood runs hot.
    Ember,
    /// Tide: at home in water.
    Tide,
    /// Crystal: attuned to mana.
    Crystal,
    /// Iron: reads metal in the rock.
    Iron,
    /// Timber: builds and grows things.
    Timber,
    /// Stone: hard to break.
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

    /// Pick the affinity a species drifts into, given a roll.
    ///
    /// The roll comes from the genes, so the affinity is inherited and can
    /// change over generations.
    pub fn from_roll(roll: f32) -> Affinity {
        // Most species stay unadapted; the specialised ones are rare.
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

    /// How much a creature can carry, as a multiplier.
    pub fn carry_factor(self) -> f32 {
        match self {
            Affinity::Iron => 1.15,
            Affinity::Timber => 1.1,
            _ => 1.0,
        }
    }

    /// How fast mana comes back.
    pub fn mana_factor(self) -> f32 {
        match self {
            Affinity::Crystal => 2.0,
            _ => 1.0,
        }
    }

    /// How fast it builds.
    pub fn build_factor(self) -> f32 {
        match self {
            Affinity::Timber => 1.5,
            _ => 1.0,
        }
    }

    /// How fast it mines.
    pub fn mine_factor(self) -> f32 {
        match self {
            Affinity::Iron => 1.6,
            _ => 1.0,
        }
    }

    /// How well it takes damage.
    pub fn armor_factor(self) -> f32 {
        match self {
            Affinity::Stone => 0.7,
            _ => 1.0,
        }
    }

    /// Whether fire hurts it.
    pub fn fire_immune(self) -> bool {
        matches!(self, Affinity::Ember | Affinity::Stone)
    }

    /// How badly the cold bites.
    ///
    /// The ember people run hot: they shrug off the heat but suffer in a
    /// hard winter.
    pub fn cold_vulnerability(self) -> f32 {
        match self {
            Affinity::Ember => 2.0,
            Affinity::Stone => 0.0,
            _ => 1.0,
        }
    }
}

/// What a species has learned to do.
///
/// The culture is a bitmask of practices. It accumulates when the colony
/// does the thing, it is inherited by the young, and it makes the colony
/// better at whatever it has learned.
pub mod culture_bits {
    /// The colony has learned to farm.
    pub const CULTIVATION: u16 = 1 << 0;
    /// The colony has learned to dig.
    pub const MINING: u16 = 1 << 1;
    /// The colony has learned to fight.
    pub const WARFARE: u16 = 1 << 2;
    /// The colony has learned to trade.
    pub const COMMERCE: u16 = 1 << 3;
    /// The colony has learned to craft.
    pub const CRAFT: u16 = 1 << 4;
    /// The colony has learned to worship.
    pub const RITUAL: u16 = 1 << 5;
    /// The colony has learned to build.
    pub const ARCHITECTURE: u16 = 1 << 6;
    /// The colony has learned to hunt.
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

/// The practices a creature knows, as a bitmask.
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

/// How different two gene sets are, in the range 0..1.
///
/// The measure is used for hybrid vigour and for the inbreeding penalty:
/// two animals with almost the same genes produce a weak child.
pub fn gene_distance(a: &[f32; 32], b: &[f32; 32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..32 {
        sum += (a[i] - b[i]).abs();
    }
    (sum / 32.0).clamp(0.0, 1.0)
}

impl CreatureSystem {
    /// A creature picks up a practice from doing it.
    ///
    /// Learning is slow and uncertain, but what one animal works out the
    /// colony remembers: the practice joins the species' culture and is
    /// passed on to the young.
    pub(crate) fn learn_culture(&mut self, index: usize, culture: Culture) {
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
    /// How long this creature lives.
    ///
    /// A prolific species burns its life out quickly and a prolonged one
    /// hangs on for a long time. This is the r/K axis of evolution, and it
    /// changes everything about a species: how many young it raises, how big
    /// they are and how soon they grow up.
    pub fn lifespan(&self) -> u32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            4800
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            2100
        } else {
            MAX_AGE
        }
    }

    /// The fraction of a life spent as a juvenile.
    pub fn maturity_fraction(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            0.26
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            0.10
        } else {
            0.18
        }
    }

    /// How many young a creature raises at a time.
    ///
    /// A prolific species raises several small ones; a prolonged species
    /// raises one and invests everything in it.
    pub fn litter_size(&self) -> u32 {
        if self.traits & trait_bits::PROLIFIC != 0 {
            3
        } else if self.traits & trait_bits::PROLONGED != 0 {
            1
        } else {
            2
        }
    }

    /// How much a young one costs to raise.
    pub fn brood_cost(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            190.0
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            70.0
        } else {
            120.0
        }
    }

    /// How much a young one is worth.
    ///
    /// A prolonged species puts everything into its single young, so it
    /// starts out strong; a prolific one spreads itself thin.
    pub fn brood_value(&self) -> f32 {
        if self.traits & trait_bits::PROLONGED != 0 {
            1.5
        } else if self.traits & trait_bits::PROLIFIC != 0 {
            0.6
        } else {
            1.0
        }
    }

    /// The most health this creature can have at its current stage.
    ///
    /// A juvenile is a small animal with a lot of growing left to do, so it
    /// heals up over time as it becomes an adult.
    pub fn max_health(&self) -> f32 {
        let base = if self.role == Role::Soldier { 150.0 } else { 100.0 };
        let hardy = if self.traits & trait_bits::HARDY != 0 { 1.4 } else { 1.0 };
        let hollow = if self.traits & trait_bits::HOLLOW_BONED != 0 {
            0.75
        } else {
            1.0
        };
        // Steel armour, and the steel that goes into the body: a colony that
        // has worked out metallurgy is a hardier one.
        let steel = if self.knows_tech(Discovery::Metallurgy) { 1.2 } else { 1.0 };
        base * hardy * hollow * steel * self.stage.health_factor()
    }

    /// Does the species of this creature know this discovery?
    ///
    /// The creature carries the knowledge of its species with it, so that
    /// the effect of a discovery can be read where it is used.
    pub fn knows_tech(&self, d: Discovery) -> bool {
        self.tech & d.bit() != 0
    }

    /// The names of the discoveries this creature's species knows.
    pub fn tech_name(&self) -> String {
        crate::creature::discovery_list(self.tech)
            .iter()
            .map(|d| d.name())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// How much a salve heals.
    ///
    /// A colony that has worked out medicine heals more from the same
    /// salve, and a colony that has worked out alchemy heals more still.
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

    /// How much this creature can carry.
    ///
    /// The base capacity is the same for everybody, but a hoarder and a
    /// species adapted to iron both get more out of the same trip.
    pub fn carry_cap(&self) -> f32 {
        let mut cap = CARRY_CAP;
        if self.traits & trait_bits::HOARDER != 0 {
            cap *= 1.6;
        }
        cap * self.affinity.carry_factor()
    }

    /// How fast this creature can move, in cells per step.
    pub fn speed_cap(&self) -> f32 {
        let mut speed = MAX_SPEED;
        if self.traits & trait_bits::HOLLOW_BONED != 0 {
            speed *= 1.35;
        }
        speed * self.stage.speed_factor()
    }

    /// The size the creature is actually drawn and hit at.
    ///
    /// A juvenile is drawn at a fraction of its adult size, so a colony
    /// full of young looks like a nursery.
    pub fn body_size(&self) -> f32 {
        (self.size * self.stage.size_factor()).max(0.6)
    }

    /// How well this creature takes a hit.
    ///
    /// A colony that has worked out tactics fights as one: it covers each
    /// other, and less of the blow lands.
    pub fn damage_taken(&self, amount: f32) -> f32 {
        let mut dmg = amount * self.affinity.armor_factor();
        if self.knows_tech(Discovery::Tactics) {
            dmg *= 0.85;
        }
        dmg
    }

    /// Whether this creature knows a practice.
    pub fn knows(&self, c: Culture) -> bool {
        self.culture & c.bit() != 0
    }

    /// A multiplier for how well the creature does a particular job.
    ///
    /// The culture of the species is the big factor here: a colony that has
    /// farmed for a thousand generations farms far better than one that has
    /// only just discovered grain.
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

    /// How much a hunt yields.
    pub fn hunting_bonus(&self) -> f32 {
        self.culture_factor(Culture::Hunting)
    }

    /// How well the creature navigates the dark.
    ///
    /// Nocturnal animals see far better at night, and the torch-bearing
    /// ones far better by day.
    pub fn vision_factor(&self, day_night: f32) -> f32 {
        let nocturnal = self.traits & trait_bits::NOCTURNAL != 0;
        // 0.25 = deep night, 0.75 = high noon.
        let night = 1.0 - (day_night * 2.0 - 0.5).abs();
        if nocturnal {
            1.0 + night * 0.8
        } else {
            1.0 + (1.0 - night) * 0.35
        }
    }
}

