//! Technology: what a species has worked out for itself.
//!
//! A culture is a *practice* — the colony farms, or mines, or fights. A
//! discovery is a *piece of knowledge* — the colony has worked out how to
//! forge a blade properly, or how to make a bow without iron, or how to
//! read the stars.
//!
//! Discoveries are earned. A species has to have the practice for a long
//! time, and to have lived through enough generations, before it works
//! out the next thing. Once it has, the knowledge belongs to the species
//! and every member of it uses it.

use super::*;

/// The things a species can discover.
pub mod discovery_bits {
    /// The tools are made properly.
    pub const SMITHING: u16 = 1 << 0;
    /// The crops are grown properly.
    pub const HUSBANDRY: u16 = 1 << 1;
    /// The bow is no longer a curiosity.
    pub const RANGED: u16 = 1 << 2;
    /// The salve heals.
    pub const MEDICINE: u16 = 1 << 3;
    /// The stars can be read.
    pub const ASTRONOMY: u16 = 1 << 4;
    /// The old rites are understood.
    pub const SANCTUARY: u16 = 1 << 5;
    /// The metal can be worked into something better.
    pub const METALLURGY: u16 = 1 << 6;
    /// The buildings can be bigger.
    pub const ARCHITECTURE: u16 = 1 << 7;
}

/// A single piece of knowledge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Discovery {
    Smithing,
    Husbandry,
    Ranged,
    Medicine,
    Astronomy,
    Sanctuary,
    Metallurgy,
    Architecture,
}

impl Discovery {
    pub fn name(self) -> &'static str {
        match self {
            Discovery::Smithing => "Smithing",
            Discovery::Husbandry => "Husbandry",
            Discovery::Ranged => "Ranged Craft",
            Discovery::Medicine => "Medicine",
            Discovery::Astronomy => "Astronomy",
            Discovery::Sanctuary => "Sanctuary",
            Discovery::Metallurgy => "Metallurgy",
            Discovery::Architecture => "Architecture",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Discovery::Smithing => "the tools are made properly: +30% melee damage",
            Discovery::Husbandry => "the crops are grown properly: the colony eats better",
            Discovery::Ranged => "a bow can be made from wood alone, and it flies further",
            Discovery::Medicine => "a salve can be made from plants alone, and it heals more",
            Discovery::Astronomy => "the stars can be read: the colony sees 40% further",
            Discovery::Sanctuary => "the old rites are understood: mana comes faster",
            Discovery::Metallurgy => "the metal is worked into steel: +20% health",
            Discovery::Architecture => "the buildings can be bigger and cost less",
        }
    }

    pub fn bit(self) -> u16 {
        match self {
            Discovery::Smithing => discovery_bits::SMITHING,
            Discovery::Husbandry => discovery_bits::HUSBANDRY,
            Discovery::Ranged => discovery_bits::RANGED,
            Discovery::Medicine => discovery_bits::MEDICINE,
            Discovery::Astronomy => discovery_bits::ASTRONOMY,
            Discovery::Sanctuary => discovery_bits::SANCTUARY,
            Discovery::Metallurgy => discovery_bits::METALLURGY,
            Discovery::Architecture => discovery_bits::ARCHITECTURE,
        }
    }

    pub fn from_bit(bit: u16) -> Option<Discovery> {
        Some(match bit {
            discovery_bits::SMITHING => Discovery::Smithing,
            discovery_bits::HUSBANDRY => Discovery::Husbandry,
            discovery_bits::RANGED => Discovery::Ranged,
            discovery_bits::MEDICINE => Discovery::Medicine,
            discovery_bits::ASTRONOMY => Discovery::Astronomy,
            discovery_bits::SANCTUARY => Discovery::Sanctuary,
            discovery_bits::METALLURGY => Discovery::Metallurgy,
            discovery_bits::ARCHITECTURE => Discovery::Architecture,
            _ => return None,
        })
    }

    /// Every discovery.
    pub fn all() -> [Discovery; 8] {
        [
            Discovery::Smithing,
            Discovery::Husbandry,
            Discovery::Ranged,
            Discovery::Medicine,
            Discovery::Astronomy,
            Discovery::Sanctuary,
            Discovery::Metallurgy,
            Discovery::Architecture,
        ]
    }

    /// What a species has to have been doing before it can work this out.
    ///
    /// A discovery is the child of a practice: you cannot forge a blade
    /// properly in a society that has never made anything.
    pub fn requires(self) -> &'static [Culture] {
        match self {
            Discovery::Smithing => &[Culture::Craft, Culture::Mining],
            Discovery::Husbandry => &[Culture::Cultivation],
            Discovery::Ranged => &[Culture::Craft, Culture::Warfare],
            Discovery::Medicine => &[Culture::Craft, Culture::Ritual],
            Discovery::Astronomy => &[Culture::Ritual],
            Discovery::Sanctuary => &[Culture::Ritual],
            Discovery::Metallurgy => &[Culture::Craft, Culture::Mining],
            Discovery::Architecture => &[Culture::Architecture],
        }
    }

    /// How many generations the species must have lived through.
    pub fn generations(self) -> u32 {
        match self {
            Discovery::Husbandry | Discovery::Architecture => 2,
            Discovery::Smithing | Discovery::Ranged => 3,
            Discovery::Medicine | Discovery::Astronomy => 4,
            Discovery::Metallurgy => 5,
            Discovery::Sanctuary => 6,
        }
    }
}

/// The discoveries a species holds.
pub fn discovery_list(bits: u16) -> Vec<Discovery> {
    let mut v = Vec::new();
    for d in Discovery::all() {
        if bits & d.bit() != 0 {
            v.push(d);
        }
    }
    v
}

impl CreatureSystem {
    /// What a species has worked out for itself.
    pub fn species_discoveries(&self, species: u32) -> u16 {
        self.discoveries
            .iter()
            .find(|(id, _)| *id == species)
            .map(|(_, d)| *d)
            .unwrap_or(0)
    }

    /// Has this species worked this out yet?
    pub fn has_discovery(&self, species: u32, d: Discovery) -> bool {
        self.species_discoveries(species) & d.bit() != 0
    }

    /// Record a discovery.
    pub fn note_discovery(&mut self, species: u32, bit: u16) {
        if let Some(entry) = self.discoveries.iter_mut().find(|(id, _)| *id == species) {
            entry.1 |= bit;
        } else {
            self.discoveries.push((species, bit));
        }
    }

    /// Look for the next thing a species could work out.
    ///
    /// A discovery needs the practice behind it, enough generations of
    /// living to build on, and a little luck. It is checked now and then
    /// rather than every tick, because knowledge comes slowly.
    pub(crate) fn check_discoveries(&mut self) {
        if self.tick % 300 != 0 {
            return;
        }
        // The species that are still alive, and how old they are.
        let mut species: Vec<(u32, u32, u16, u32)> = Vec::new();
        for c in self.list.iter() {
            let entry = match species.iter_mut().find(|(id, _, _, _)| *id == c.species) {
                Some(e) => e,
                None => {
                    species.push((c.species, 0, 0, 0));
                    species.last_mut().unwrap()
                }
            };
            entry.1 = entry.1.max(c.generation);
            entry.2 |= c.culture;
            entry.3 += 1;
        }
        for (sp, generation, culture, members) in species {
            if members < 3 {
                continue;
            }
            let known = self.species_discoveries(sp);
            for d in Discovery::all() {
                if known & d.bit() != 0 {
                    continue;
                }
                // The practice has to be there.
                let mut has_practice = true;
                for needed in d.requires() {
                    if culture & needed.bit() == 0 {
                        has_practice = false;
                        break;
                    }
                }
                if !has_practice {
                    continue;
                }
                // And the species has to have lived long enough to build on
                // what it knows.
                if generation < d.generations() {
                    continue;
                }
                // And it has to be lucky.
                if self.rng.chance(0.22) {
                    self.note_discovery(sp, d.bit());
                    let name = c_name(sp);
                    self.diplomacy.log_event(
                        format!("DISCOVERY: the {} work out {}", name, d.name()),
                        [255, 220, 130],
                        self.tick,
                    );
                    self.events.push(SimEvent::Discovery {
                        x: 0.0,
                        y: 0.0,
                        color: [255, 220, 130],
                    });
                    break;
                }
            }
        }
    }
}

/// The name of a species, for the record.
fn c_name(species: u32) -> String {
    species_name_for(species)
}
