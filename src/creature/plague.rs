//! Disease: the plagues that run through a colony.
//!
//! A disease is not a label on a creature: it changes what the creature
//! can do, how fast it burns its energy and how long it has left. Some
//! diseases are catching, some kill, and some turn a colony mad.
//!
//! The point of the system is that a colony can be **destroyed from the
//! inside**. A species that has spread too wide, lived too long in one
//! place and let its neighbours trade with it can find its whole
//! population wiped out by something no sword can stop.

use super::*;

/// How a disease behaves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plague {
    /// Catches easily, weakens the body, and burns out on its own.
    Flu,
    /// Catches very easily and kills. It ends with the host.
    Plague,
    /// Spreads through wounds. The venom of a venomous animal.
    Poison,
    /// Does not spread, but wears the victim down.
    Curse,
    /// Spreads through the flesh and kills slowly.
    Infection,
    /// Lives on the host and drinks its energy without killing it.
    Parasite,
    /// Drives the host to attack its own kind.
    Madness,
}

impl Plague {
    /// Every disease.
    pub fn all() -> [Plague; 7] {
        [
            Plague::Flu,
            Plague::Plague,
            Plague::Poison,
            Plague::Curse,
            Plague::Infection,
            Plague::Parasite,
            Plague::Madness,
        ]
    }

    /// The name shown in the tooltip.
    pub fn name(self) -> &'static str {
        match self {
            Plague::Flu => "flu",
            Plague::Plague => "plague",
            Plague::Poison => "poison",
            Plague::Curse => "curse",
            Plague::Infection => "infection",
            Plague::Parasite => "parasite",
            Plague::Madness => "madness",
        }
    }

    /// What the disease does.
    pub fn blurb(self) -> &'static str {
        match self {
            Plague::Flu => "catches easily, weakens and burns out",
            Plague::Plague => "catches easily and kills",
            Plague::Poison => "enters through wounds",
            Plague::Curse => "wears the victim down",
            Plague::Infection => "spreads through the flesh and kills slowly",
            Plague::Parasite => "drinks the host's energy",
            Plague::Madness => "drives the host to attack its own kind",
        }
    }

    /// How likely the disease is to jump to a neighbour, per tick.
    ///
    /// A disease that spreads through the air catches far more easily than
    /// one that needs a wound.
    pub fn contagiousness(self) -> f32 {
        match self {
            Plague::Flu => 0.010,
            Plague::Plague => 0.014,
            Plague::Poison => 0.0,
            Plague::Curse => 0.0,
            Plague::Infection => 0.004,
            Plague::Parasite => 0.002,
            Plague::Madness => 0.003,
        }
    }

    /// How much health the disease takes per tick.
    pub fn lethality(self) -> f32 {
        match self {
            Plague::Flu => 0.0,
            Plague::Plague => 0.30,
            Plague::Poison => 0.12,
            Plague::Curse => 0.08,
            Plague::Infection => 0.10,
            Plague::Parasite => 0.0,
            Plague::Madness => 0.0,
        }
    }

    /// How much energy the disease burns per tick.
    pub fn drain(self) -> f32 {
        match self {
            Plague::Flu => 0.10,
            Plague::Plague => 0.22,
            Plague::Poison => 0.14,
            Plague::Curse => 0.12,
            Plague::Infection => 0.10,
            Plague::Parasite => 0.30,
            Plague::Madness => 0.08,
        }
    }

    /// How much the disease slows the host down.
    pub fn slowness(self) -> f32 {
        match self {
            Plague::Flu => 0.7,
            Plague::Plague => 0.8,
            Plague::Poison => 0.85,
            Plague::Curse => 0.9,
            Plague::Infection => 0.8,
            Plague::Parasite => 0.9,
            Plague::Madness => 0.85,
        }
    }

    /// Whether the disease drives the host to attack its own kind.
    pub fn maddens(self) -> bool {
        matches!(self, Plague::Madness)
    }

    /// How long the disease lasts, in ticks.
    ///
    /// A disease with no duration never ends on its own: it has to be
    /// survived or fatal.
    pub fn duration(self) -> u16 {
        match self {
            Plague::Flu => 420,
            Plague::Plague => 900,
            Plague::Poison => 300,
            Plague::Curse => 360,
            Plague::Infection => 700,
            Plague::Parasite => 1200,
            Plague::Madness => 480,
        }
    }

    /// Is this disease still around?
    pub fn is_active(self) -> bool {
        true
    }
}

impl Creature {
    /// What the disease does to this creature on a single tick.
    ///
    /// The return value is the health the disease took.
    pub fn apply_disease(&mut self) -> f32 {
        if self.disease == Disease::None {
            return 0.0;
        }
        let plague = Plague::from(self.disease);
        let harm = plague.lethality();
        self.health -= harm;
        self.energy = (self.energy - plague.drain()).max(0.0);
        harm
    }

    /// How fast this creature moves while it is sick.
    pub fn disease_speed_factor(&self) -> f32 {
        if self.disease == Disease::None {
            return 1.0;
        }
        Plague::from(self.disease).slowness()
    }

    /// Whether the disease makes this creature attack its own kind.
    pub fn is_mad(&self) -> bool {
        self.disease != Disease::None && Plague::from(self.disease).maddens()
    }

    /// The resistance this creature has to a disease.
    ///
    /// `0` is no resistance, `1` is resistance and `2` is immunity.
    pub fn resistance(&self, disease: Disease) -> u8 {
        self.immunity[disease as usize]
    }

    /// How likely the disease is to take hold in this creature.
    ///
    /// A resistant animal shrugs it off most of the time; an immune one
    /// never catches it at all.
    pub fn catch_chance(&self, disease: Disease) -> f32 {
        match self.resistance(disease) {
            0 => 1.0,
            1 => 0.35,
            _ => 0.0,
        }
    }
}

/// The genes that give resistance to a disease, and the disease they guard
/// against.
///
/// Resistance is genetic, so it is inherited and it evolves: a plague that
/// sweeps through a colony selects for the animals that can stand it, and
/// their descendants are born resistant.
pub const IMMUNITY_GENES: [(usize, Disease); 4] = [
    (23, Disease::Flu),
    (24, Disease::Plague),
    (25, Disease::Infection),
    (26, Disease::Madness),
];

/// The resistance a genome gives, worked out from the genes.
pub fn immunity_from_genes(genes: &[f32; 32]) -> [u8; 8] {
    let mut immunity = [0u8; 8];
    for (gene, disease) in IMMUNITY_GENES {
        let g = genes[gene];
        if g > 0.74 {
            immunity[disease as usize] = 2;
        } else if g > 0.58 {
            immunity[disease as usize] = 1;
        }
    }
    immunity
}

/// Turn the old disease enum into the richer plague table.
///
/// The enum stays as it is because it is stored in a byte and used by the
/// network, but the rules live in `Plague`.
impl From<Disease> for Plague {
    fn from(d: Disease) -> Plague {
        match d {
            Disease::Flu => Plague::Flu,
            Disease::Plague => Plague::Plague,
            Disease::Poison => Plague::Poison,
            Disease::Curse => Plague::Curse,
            Disease::Infection => Plague::Infection,
            Disease::Parasite => Plague::Parasite,
            Disease::Madness => Plague::Madness,
            Disease::None => Plague::Flu,
        }
    }
}

impl CreatureSystem {
    /// A disease breaks out in a crowded colony.
    ///
    /// Diseases do not only come from venom and curses: a colony that has
    /// grown too crowded breeds them. Every so often one animal falls ill,
    /// and from there the disease does its own work.
    pub(crate) fn maybe_outbreak(&mut self) {
        if self.tick % 400 != 0 || self.list.len() <= 20 {
            return;
        }
        let sick = self
            .list
            .iter()
            .filter(|c| c.disease != Disease::None)
            .count();
        // A colony that is already suffering is not ready for another
        // outbreak.
        if sick * 6 >= self.list.len() || !self.rng.chance(0.25) {
            return;
        }
        let victim = self.rng.range(self.list.len() as u32) as usize;
        if self.list[victim].disease != Disease::None {
            return;
        }
        const OPTIONS: [Disease; 5] = [
            Disease::Flu,
            Disease::Plague,
            Disease::Infection,
            Disease::Parasite,
            Disease::Madness,
        ];
        let pick = OPTIONS[self.rng.range(OPTIONS.len() as u32) as usize];
        self.list[victim].disease = pick;
        self.list[victim].disease_timer = Plague::from(pick).duration();
        self.plagues_started += 1;
        self.events.push(SimEvent::Plague {
            x: self.list[victim].x,
            y: self.list[victim].y,
            color: self.list[victim].team_color,
        });
    }

    /// Spread the diseases through the colonies.
    ///
    /// A sick animal infects its own kind: the disease moves through the
    /// colony, and a colony that has been trading with its neighbours gets
    /// it from them too.
    ///
    /// This runs every few ticks rather than every tick: a disease that
    /// jumps once per tick would sweep a colony in seconds.
    pub(crate) fn spread_diseases(&mut self, grid: &SpatialGrid) {
        if self.tick % 3 != 0 {
            return;
        }
        // Which creatures are sick, and with what.
        let sick: Vec<(usize, Disease, f32)> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, c)| c.disease != Disease::None)
            .map(|(i, c)| {
                (
                    i,
                    c.disease,
                    Plague::from(c.disease).contagiousness() * 3.0,
                )
            })
            .collect();
        if sick.is_empty() {
            return;
        }
        // The infections that happened this tick.
        let mut infected: Vec<(usize, Disease)> = Vec::new();
        for (i, disease, chance) in sick {
            if chance <= 0.0 {
                continue;
            }
            let (sx, sy, sspecies) = {
                let c = &self.list[i];
                (c.x, c.y, c.species)
            };
            grid.near(sx, sy, 14, |idx| {
                let j = idx as usize;
                if j >= self.list.len() || j == i {
                    return;
                }
                let other = &self.list[j];
                // A disease spreads through a colony, and also between
                // colonies that are close enough to meet.
                if other.species != sspecies && chance < 0.008 {
                    return;
                }
                if other.disease != Disease::None || other.health <= 0.0 {
                    return;
                }
                let d = ((other.x - sx).powi(2) + (other.y - sy).powi(2)).sqrt();
                if d > 14.0 {
                    return;
                }
                // A resistant animal mostly shrugs the disease off.
                let catch = other.catch_chance(disease);
                if catch <= 0.0 {
                    return;
                }
                if self.rng.chance(chance * catch) {
                    infected.push((j, disease));
                }
            });
        }
        for (j, disease) in infected {
            self.list[j].disease = disease;
            self.list[j].disease_timer = Plague::from(disease).duration();
        }
    }
}
