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
//! and every member of it uses that.
//!
//! The discoveries are arranged in **ten tiers**. A tier is the current
//! edge of what the world knows, and the tiers build on each other: the
//! fifth tier cannot be worked out before the fourth is complete. The
//! tree is the shape of a civilisation's progress, from the first fire to
//! the light of the surface.

use super::*;

/// The things a species can discover.
pub mod discovery_bits {
    /// The tools are made properly.
    pub const SMITHING: u32 = 1 << 0;
    /// The crops are grown properly.
    pub const HUSBANDRY: u32 = 1 << 1;
    /// The pots hold, and the stores keep.
    pub const POTTERY: u32 = 1 << 2;
    /// The bow is no longer a curiosity.
    pub const RANGED: u32 = 1 << 3;
    /// The buildings can be bigger and cost less.
    pub const ARCHITECTURE: u32 = 1 << 4;
    /// The walls stand against what comes for them.
    pub const MASONRY: u32 = 1 << 5;
    /// The metal is worked into steel.
    pub const METALLURGY: u32 = 1 << 6;
    /// The colony has something worth trading.
    pub const COMMERCE: u32 = 1 << 7;
    /// The colony knows the way about its own country.
    pub const CARTOGRAPHY: u32 = 1 << 8;
    /// The salve heals.
    pub const MEDICINE: u32 = 1 << 9;
    /// The herbs are worth more than they look.
    pub const HERBALISM: u32 = 1 << 10;
    /// The wounded mend.
    pub const SURGERY: u32 = 1 << 11;
    /// The stars can be read.
    pub const ASTRONOMY: u32 = 1 << 12;
    /// The old rites are understood.
    pub const SANCTUARY: u32 = 1 << 13;
    /// The colony moves together.
    pub const NAVIGATION: u32 = 1 << 14;
    /// The colony digs deeper.
    pub const DEEPMINING: u32 = 1 << 15;
    /// The colony knows the shape of the deep.
    pub const GEODESY: u32 = 1 << 16;
    /// The ore is worked where it is found.
    pub const SMELTING: u32 = 1 << 17;
    /// The colony can hold its own against poison.
    pub const ANTITOXINS: u32 = 1 << 18;
    /// The filth of the deep can be scoured out of a colony.
    pub const PURGE: u32 = 1 << 19;
    /// The colony can raise a ward of sanctified fire.
    pub const WARDCRAFT: u32 = 1 << 20;
    /// Sanctified fire: the light of the surface, made into a weapon.
    pub const SANCTIFY: u32 = 1 << 21;
    /// The fire is made to burn the deep.
    pub const SUNFIRECRAFT: u32 = 1 << 22;
    /// The colony's magic is brighter.
    pub const RADIANCE: u32 = 1 << 23;
    /// The colony's shots fly further and hit harder.
    pub const SIEGECRAFT: u32 = 1 << 24;
    /// The colony fights as one.
    pub const TACTICS: u32 = 1 << 25;
    /// The colony does not break.
    pub const IRONWILL: u32 = 1 << 26;
    /// The colony turns the deep into medicine.
    pub const ALCHEMY: u32 = 1 << 27;
    /// The colony can live in the deep.
    pub const DEEPDIVING: u32 = 1 << 28;
    /// The colony closes the deep against itself.
    pub const DOMINION: u32 = 1 << 29;
    /// The colony works out that a mind can be shaped: every brain grows.
    pub const COGNITION: u32 = 1 << 30;
    /// The colony works out how to learn inside its own life: the brains
    /// grow again, and they remember what worked.
    pub const GRANDCOGNITION: u32 = 1 << 31;
}

/// How many discoveries there are in all.
pub const DISCOVERY_COUNT: usize = 32;

/// How many tiers the tree has.
pub const TECH_TIERS: u8 = 10;

/// A single piece of knowledge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Discovery {
    Smithing,
    Husbandry,
    Pottery,
    Ranged,
    Architecture,
    Masonry,
    Metallurgy,
    Commerce,
    Cartography,
    Medicine,
    Herbalism,
    Surgery,
    Astronomy,
    Sanctuary,
    Navigation,
    DeepMining,
    Geodesy,
    Smelting,
    Antitoxins,
    Purge,
    Wardcraft,
    Sanctification,
    Sunfirecraft,
    Radiance,
    Siegecraft,
    Tactics,
    Ironwill,
    Alchemy,
    Deepdiving,
    Dominion,
    /// The mind can be shaped: the colony's brains grow.
    Cognition,
    /// The mind can learn: the brains grow again and learn inside a life.
    GrandCognition,
}

impl Discovery {
    /// Every discovery, in the order of the tree.
    pub fn all() -> [Discovery; DISCOVERY_COUNT] {
        [
            Discovery::Smithing,
            Discovery::Husbandry,
            Discovery::Pottery,
            Discovery::Ranged,
            Discovery::Architecture,
            Discovery::Masonry,
            Discovery::Metallurgy,
            Discovery::Commerce,
            Discovery::Cartography,
            Discovery::Medicine,
            Discovery::Herbalism,
            Discovery::Surgery,
            Discovery::Astronomy,
            Discovery::Sanctuary,
            Discovery::Navigation,
            Discovery::DeepMining,
            Discovery::Geodesy,
            Discovery::Smelting,
            Discovery::Antitoxins,
            Discovery::Purge,
            Discovery::Wardcraft,
            Discovery::Sanctification,
            Discovery::Sunfirecraft,
            Discovery::Radiance,
            Discovery::Siegecraft,
            Discovery::Tactics,
            Discovery::Ironwill,
            Discovery::Alchemy,
            Discovery::Deepdiving,
            Discovery::Dominion,
            Discovery::Cognition,
            Discovery::GrandCognition,
        ]
    }

    /// The name shown in the tech tree.
    pub fn name(self) -> &'static str {
        match self {
            Discovery::Smithing => "Smithing",
            Discovery::Husbandry => "Husbandry",
            Discovery::Pottery => "Pottery",
            Discovery::Ranged => "Ranged Craft",
            Discovery::Architecture => "Architecture",
            Discovery::Masonry => "Masonry",
            Discovery::Metallurgy => "Metallurgy",
            Discovery::Commerce => "Commerce",
            Discovery::Cartography => "Cartography",
            Discovery::Medicine => "Medicine",
            Discovery::Herbalism => "Herbalism",
            Discovery::Surgery => "Surgery",
            Discovery::Astronomy => "Astronomy",
            Discovery::Sanctuary => "Sanctuary",
            Discovery::Navigation => "Navigation",
            Discovery::DeepMining => "Deep Mining",
            Discovery::Geodesy => "Geodesy",
            Discovery::Smelting => "Smelting",
            Discovery::Antitoxins => "Antitoxins",
            Discovery::Purge => "Purge",
            Discovery::Wardcraft => "Wardcraft",
            Discovery::Sanctification => "Sanctification",
            Discovery::Sunfirecraft => "Sunfirecraft",
            Discovery::Radiance => "Radiance",
            Discovery::Siegecraft => "Siegecraft",
            Discovery::Tactics => "Tactics",
            Discovery::Ironwill => "Ironwill",
            Discovery::Alchemy => "Alchemy",
            Discovery::Deepdiving => "Deepdiving",
            Discovery::Dominion => "Dominion",
            Discovery::Cognition => "Cognition",
            Discovery::GrandCognition => "Grand Cognition",
        }
    }

    /// What the discovery does.
    ///
    /// Every one of these is a real effect in the simulation, not a line of
    /// flavour: this is what the player sees the world do differently.
    pub fn effect(self) -> &'static str {
        match self {
            Discovery::Smithing => "+30% melee damage",
            Discovery::Husbandry => "the colony eats better",
            Discovery::Pottery => "hunger grows 15% slower",
            Discovery::Ranged => "a bow from wood alone, and it flies 25% further",
            Discovery::Architecture => "buildings cost 30% less",
            Discovery::Masonry => "buildings stand 40% longer",
            Discovery::Metallurgy => "+20% health",
            Discovery::Commerce => "trade brings in 50% more",
            Discovery::Cartography => "+20% sight",
            Discovery::Medicine => "salves heal 40% more",
            Discovery::Herbalism => "food is worth 15% more",
            Discovery::Surgery => "wounds mend 40% faster",
            Discovery::Astronomy => "+40% sight",
            Discovery::Sanctuary => "mana comes 50% faster",
            Discovery::Navigation => "the colony moves 15% faster",
            Discovery::DeepMining => "the colony digs 50% deeper",
            Discovery::Geodesy => "the colony finds food 30% further off",
            Discovery::Smelting => "+25% melee damage",
            Discovery::Antitoxins => "poison does half the harm",
            Discovery::Purge => "the sickness of the deep is resisted",
            Discovery::Wardcraft => "wards burn 50% wider",
            Discovery::Sanctification => "the sunfire scours the ZORGs",
            Discovery::Sunfirecraft => "the sunfire burns 30% harder",
            Discovery::Radiance => "+40% mana",
            Discovery::Siegecraft => "shots fly 25% further and hit 25% harder",
            Discovery::Tactics => "15% less damage taken",
            Discovery::Ironwill => "sickness does half the harm",
            Discovery::Alchemy => "wounds mend 30% faster, salves 20% more",
            Discovery::Deepdiving => "the deep does no harm",
            Discovery::Dominion => "the deep is closed: it breeds far more slowly",
            Discovery::Cognition => "every mind grows: the colony thinks bigger",
            Discovery::GrandCognition => "the minds grow again, and learn from what works",
        }
    }

    /// The tier this discovery belongs to.
    ///
    /// The tiers build on each other: nothing in a tier can be worked out
    /// before the tier below it is complete.
    pub fn tier(self) -> u8 {
        match self {
            Discovery::Husbandry | Discovery::Pottery => 1,
            Discovery::Smithing | Discovery::Ranged => 2,
            Discovery::Architecture | Discovery::Masonry => 3,
            Discovery::Commerce | Discovery::Cartography => 4,
            Discovery::Medicine | Discovery::Herbalism | Discovery::Surgery => 5,
            Discovery::DeepMining | Discovery::Geodesy | Discovery::Smelting => 6,
            Discovery::Astronomy | Discovery::Sanctuary | Discovery::Navigation => 7,
            Discovery::Sanctification
            | Discovery::Wardcraft
            | Discovery::Antitoxins
            | Discovery::Purge => 8,
            Discovery::Siegecraft
            | Discovery::Tactics
            | Discovery::Ironwill
            | Discovery::Sunfirecraft
            | Discovery::Radiance => 9,
            Discovery::Metallurgy
            | Discovery::Alchemy
            | Discovery::Deepdiving
            | Discovery::Dominion => 10,
            Discovery::Cognition => 1,
            Discovery::GrandCognition => 3,
        }
    }

    /// The name of the tier this discovery belongs to.
    pub fn tier_name(self) -> &'static str {
        match self.tier() {
            1 => "The First Fire",
            2 => "The Craft",
            3 => "The Trade",
            4 => "The Medicine",
            5 => "The Stars",
            6 => "The Deep",
            7 => "The Corruption",
            8 => "The Light",
            9 => "The War",
            _ => "The Crown",
        }
    }

    pub fn bit(self) -> u32 {
        match self {
            Discovery::Smithing => discovery_bits::SMITHING,
            Discovery::Husbandry => discovery_bits::HUSBANDRY,
            Discovery::Pottery => discovery_bits::POTTERY,
            Discovery::Ranged => discovery_bits::RANGED,
            Discovery::Architecture => discovery_bits::ARCHITECTURE,
            Discovery::Masonry => discovery_bits::MASONRY,
            Discovery::Metallurgy => discovery_bits::METALLURGY,
            Discovery::Commerce => discovery_bits::COMMERCE,
            Discovery::Cartography => discovery_bits::CARTOGRAPHY,
            Discovery::Medicine => discovery_bits::MEDICINE,
            Discovery::Herbalism => discovery_bits::HERBALISM,
            Discovery::Surgery => discovery_bits::SURGERY,
            Discovery::Astronomy => discovery_bits::ASTRONOMY,
            Discovery::Sanctuary => discovery_bits::SANCTUARY,
            Discovery::Navigation => discovery_bits::NAVIGATION,
            Discovery::DeepMining => discovery_bits::DEEPMINING,
            Discovery::Geodesy => discovery_bits::GEODESY,
            Discovery::Smelting => discovery_bits::SMELTING,
            Discovery::Antitoxins => discovery_bits::ANTITOXINS,
            Discovery::Purge => discovery_bits::PURGE,
            Discovery::Wardcraft => discovery_bits::WARDCRAFT,
            Discovery::Sanctification => discovery_bits::SANCTIFY,
            Discovery::Sunfirecraft => discovery_bits::SUNFIRECRAFT,
            Discovery::Radiance => discovery_bits::RADIANCE,
            Discovery::Siegecraft => discovery_bits::SIEGECRAFT,
            Discovery::Tactics => discovery_bits::TACTICS,
            Discovery::Ironwill => discovery_bits::IRONWILL,
            Discovery::Alchemy => discovery_bits::ALCHEMY,
            Discovery::Deepdiving => discovery_bits::DEEPDIVING,
            Discovery::Dominion => discovery_bits::DOMINION,
            Discovery::Cognition => discovery_bits::COGNITION,
            Discovery::GrandCognition => discovery_bits::GRANDCOGNITION,
        }
    }

    pub fn from_bit(bit: u32) -> Option<Discovery> {
        Some(match bit {
            discovery_bits::SMITHING => Discovery::Smithing,
            discovery_bits::HUSBANDRY => Discovery::Husbandry,
            discovery_bits::POTTERY => Discovery::Pottery,
            discovery_bits::RANGED => Discovery::Ranged,
            discovery_bits::ARCHITECTURE => Discovery::Architecture,
            discovery_bits::MASONRY => Discovery::Masonry,
            discovery_bits::METALLURGY => Discovery::Metallurgy,
            discovery_bits::COMMERCE => Discovery::Commerce,
            discovery_bits::CARTOGRAPHY => Discovery::Cartography,
            discovery_bits::MEDICINE => Discovery::Medicine,
            discovery_bits::HERBALISM => Discovery::Herbalism,
            discovery_bits::SURGERY => Discovery::Surgery,
            discovery_bits::ASTRONOMY => Discovery::Astronomy,
            discovery_bits::SANCTUARY => Discovery::Sanctuary,
            discovery_bits::NAVIGATION => Discovery::Navigation,
            discovery_bits::DEEPMINING => Discovery::DeepMining,
            discovery_bits::GEODESY => Discovery::Geodesy,
            discovery_bits::SMELTING => Discovery::Smelting,
            discovery_bits::ANTITOXINS => Discovery::Antitoxins,
            discovery_bits::PURGE => Discovery::Purge,
            discovery_bits::WARDCRAFT => Discovery::Wardcraft,
            discovery_bits::SANCTIFY => Discovery::Sanctification,
            discovery_bits::SUNFIRECRAFT => Discovery::Sunfirecraft,
            discovery_bits::RADIANCE => Discovery::Radiance,
            discovery_bits::SIEGECRAFT => Discovery::Siegecraft,
            discovery_bits::TACTICS => Discovery::Tactics,
            discovery_bits::IRONWILL => Discovery::Ironwill,
            discovery_bits::ALCHEMY => Discovery::Alchemy,
            discovery_bits::DEEPDIVING => Discovery::Deepdiving,
            discovery_bits::DOMINION => Discovery::Dominion,
            discovery_bits::COGNITION => Discovery::Cognition,
            discovery_bits::GRANDCOGNITION => Discovery::GrandCognition,
            _ => return None,
        })
    }

    /// What a species has to have been doing before it can work this out.
    ///
    /// A discovery is the child of a practice: you cannot forge a blade
    /// properly in a society that has never made anything.
    pub fn requires(self) -> &'static [Culture] {
        match self {
            Discovery::Smithing => &[Culture::Craft, Culture::Mining],
            Discovery::Husbandry => &[Culture::Cultivation],
            Discovery::Pottery => &[Culture::Craft],
            Discovery::Ranged => &[Culture::Craft, Culture::Warfare],
            Discovery::Architecture => &[Culture::Architecture],
            Discovery::Masonry => &[Culture::Craft, Culture::Architecture],
            Discovery::Metallurgy => &[Culture::Craft, Culture::Mining],
            Discovery::Commerce => &[Culture::Commerce, Culture::Ritual],
            Discovery::Cartography => &[Culture::Craft, Culture::Warfare],
            Discovery::Medicine => &[Culture::Craft, Culture::Ritual],
            Discovery::Herbalism => &[Culture::Cultivation, Culture::Ritual],
            Discovery::Surgery => &[Culture::Ritual, Culture::Craft],
            Discovery::Astronomy => &[Culture::Ritual],
            Discovery::Sanctuary => &[Culture::Ritual],
            Discovery::Navigation => &[Culture::Warfare, Culture::Craft],
            Discovery::DeepMining => &[Culture::Mining, Culture::Craft],
            Discovery::Geodesy => &[Culture::Ritual, Culture::Craft],
            Discovery::Smelting => &[Culture::Craft, Culture::Mining],
            Discovery::Antitoxins => &[Culture::Craft, Culture::Ritual],
            Discovery::Purge => &[Culture::Ritual, Culture::Craft],
            Discovery::Wardcraft => &[Culture::Architecture, Culture::Ritual],
            Discovery::Sanctification => &[Culture::Craft, Culture::Ritual],
            Discovery::Sunfirecraft => &[Culture::Craft, Culture::Ritual],
            Discovery::Radiance => &[Culture::Ritual, Culture::Craft],
            Discovery::Siegecraft => &[Culture::Craft, Culture::Warfare],
            Discovery::Tactics => &[Culture::Warfare, Culture::Ritual],
            Discovery::Ironwill => &[Culture::Warfare, Culture::Cultivation],
            Discovery::Alchemy => &[Culture::Craft, Culture::Ritual],
            Discovery::Deepdiving => &[Culture::Craft, Culture::Ritual],
            Discovery::Dominion => &[Culture::Architecture, Culture::Craft],
            Discovery::Cognition => &[Culture::Craft],
            Discovery::GrandCognition => &[Culture::Craft, Culture::Mining],
        }
    }

    /// How many generations the species must have lived through.
    pub fn generations(self) -> u32 {
        match self.tier() {
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 6,
            6 => 7,
            7 => 8,
            8 => 9,
            9 => 10,
            _ => 12,
        }
    }
}

/// The discoveries a species holds.
pub fn discovery_list(bits: u32) -> Vec<Discovery> {
    let mut v = Vec::new();
    for d in Discovery::all() {
        if bits & d.bit() != 0 {
            v.push(d);
        }
    }
    v
}

/// The discoveries of one tier.
pub fn discovery_tier(tier: u8) -> Vec<Discovery> {
    Discovery::all()
        .into_iter()
        .filter(|d| d.tier() == tier)
        .collect()
}

impl CreatureSystem {
    /// What a species has worked out for itself.
    pub fn species_discoveries(&self, species: u32) -> u32 {
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
    pub fn note_discovery(&mut self, species: u32, bit: u32) {
        if let Some(entry) = self.discoveries.iter_mut().find(|(id, _)| *id == species) {
            entry.1 |= bit;
        } else {
            self.discoveries.push((species, bit));
        }
    }

    /// The highest tier the world has reached.
    ///
    /// This is the measure of how far the civilisation has come: a tier is
    /// only reached when every discovery in it has been worked out by
    /// somebody.
    pub fn tech_tier(&self) -> u8 {
        let mut all = 0u32;
        for (_, d) in self.discoveries.iter() {
            all |= *d;
        }
        let mut tier = 0;
        for t in 1..=TECH_TIERS {
            let complete = discovery_tier(t)
                .iter()
                .all(|d| all & d.bit() != 0);
            if complete {
                tier = t;
            } else {
                break;
            }
        }
        tier
    }

    /// The discoveries the world has worked out, in the order they were
    /// found.
    pub fn discoveries_found(&self) -> Vec<(Discovery, u32)> {
        let mut out = Vec::new();
        for d in Discovery::all() {
            // Who knows it?
            let mut holders = Vec::new();
            for (sp, bits) in self.discoveries.iter() {
                if bits & d.bit() != 0 {
                    holders.push(*sp);
                }
            }
            if !holders.is_empty() {
                out.push((d, holders.len() as u32));
            }
        }
        out
    }

    /// The most important discovery the world has made.
    ///
    /// That is the one furthest up the tree: the light of the surface, if
    /// anybody has worked it out, matters more than another row of grain.
    pub fn greatest_discovery(&self) -> Option<Discovery> {
        let mut best: Option<Discovery> = None;
        for (_, bits) in self.discoveries.iter() {
            for d in Discovery::all() {
                if bits & d.bit() != 0 {
                    best = match best {
                        Some(b) if b.tier() >= d.tier() => Some(b),
                        _ => Some(d),
                    };
                }
            }
        }
        best
    }

    /// The discovery that was made last, and the species that made it.
    pub fn latest_discovery(&self) -> Option<(Discovery, u32)> {
        self.last_discovery
    }

    /// Allies share what they know.
    ///
    /// Knowledge travels along a treaty: an allied species can pick up a
    /// discovery its partner worked out, which is what makes an alliance
    /// worth having.
    pub(crate) fn share_discoveries(&mut self) {
        if self.tick % 240 != 0 {
            return;
        }
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for rel in self.diplomacy.relations.iter() {
            if rel.has(Treaty::Alliance) {
                pairs.push((rel.a, rel.b));
            }
        }
        for (a, b) in pairs {
            let da = self.species_discoveries(a);
            let db = self.species_discoveries(b);
            if da & !db != 0 {
                let shared = da & !db;
                self.note_discovery(b, shared);
                // A mind that grows is grown for everybody, including the
                // species that were handed the knowledge over a treaty.
                for d in Discovery::all() {
                    if shared & d.bit() != 0 {
                        self.grow_brains(b, d);
                    }
                }
                // The last thing that was shared is the last thing found.
                if let Some(d) = Discovery::all().into_iter().find(|d| da & d.bit() != 0) {
                    self.last_discovery = Some((d, b));
                }
                self.diplomacy.log_event(
                    format!("species {} shares its knowledge with species {}", a, b),
                    self.species_color(a),
                    self.tick,
                );
            } else if db & !da != 0 {
                let shared = db & !da;
                self.note_discovery(a, shared);
                for d in Discovery::all() {
                    if shared & d.bit() != 0 {
                        self.grow_brains(a, d);
                    }
                }
                self.diplomacy.log_event(
                    format!("species {} shares its knowledge with species {}", b, a),
                    self.species_color(b),
                    self.tick,
                );
            }
        }
    }

    /// Look for the next thing a species could work out.
    ///
    /// A discovery needs the practice behind it, enough generations of
    /// living to build on, and a little luck. It is checked now and then
    /// rather than every tick, because knowledge comes slowly.
    ///
    /// The tiers hold each other up: a species cannot work out anything in
    /// the fifth tier before it has finished the fourth.
    pub(crate) fn check_discoveries(&mut self, world: &World) {
        if self.tick % 300 != 0 {
            return;
        }
        // The species that are still alive, and how old they are.
        let mut species: Vec<(u32, u32, u16, u32, bool)> = Vec::new();
        for c in self.list.iter() {
            let entry = match species.iter_mut().find(|(id, _, _, _, _)| *id == c.species) {
                Some(e) => e,
                None => {
                    species.push((c.species, 0, 0, 0, false));
                    species.last_mut().unwrap()
                }
            };
            entry.1 = entry.1.max(c.generation);
            entry.2 |= c.culture;
            entry.3 += 1;
            // A colony beside the drowned temple learns faster.
            if world.landmark_bonus(c.x, c.y).lore {
                entry.4 = true;
            }
        }
        for (sp, generation, culture, members, lore) in species {
            if members < 3 {
                continue;
            }
            // The ZORGs never work anything out. They are the deep's own
            // ecology, not a people: they cannot learn the light of the
            // surface, and they cannot learn the thing that undoes them.
            if sp == ZORG_SPECIES {
                continue;
            }
            let known = self.species_discoveries(sp);
            for d in Discovery::all() {
                if known & d.bit() != 0 {
                    continue;
                }
                // The tier below has to be finished first.
                if d.tier() > 1 && !self.tier_complete(known, d.tier() - 1) {
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
                // And it has to be lucky. A colony beside the drowned
                // temple is luckier than most.
                let chance = if lore { 0.34 } else { 0.22 };
                if self.rng.chance(chance) {
                    self.note_discovery(sp, d.bit());
                    self.last_discovery = Some((d, sp));
                    // A discovery about thinking changes the thinking: every
                    // mind of that species grows a tier, and keeps what it
                    // already knew.
                    self.grow_brains(sp, d);
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

    /// Grow the minds of a species.
    ///
    /// A colony that works out how to think better does not think better
    /// with the same brain: every animal of the species is given a bigger
    /// one, and the weights it already had are carried over, so nothing
    /// that was learnt is lost.
    pub fn grow_brains(&mut self, species: u32, d: Discovery) {
        let target = match d {
            Discovery::Cognition => 1,
            Discovery::GrandCognition => MAX_ARCH,
            _ => return,
        };
        let rng = &mut self.rng;
        for c in self.list.iter_mut() {
            if c.species == species && c.net.arch < target {
                c.net.grow_to(target, rng);
            }
        }
    }

    /// Has a species finished a whole tier?
    pub fn tier_complete(&self, known: u32, tier: u8) -> bool {
        discovery_tier(tier)
            .iter()
            .all(|d| known & d.bit() != 0)
    }

    /// Give every creature the knowledge of its species.
    ///
    /// The creature carries its species' discoveries with it, so that the
    /// effects of a discovery can be read where they are used.
    pub(crate) fn refresh_tech(&mut self) {
        // The knowledge of every species, worked out once.
        let mut known: Vec<u32> = vec![0; self.next_species as usize + 8];
        for (sp, d) in self.discoveries.iter() {
            if (*sp as usize) < known.len() {
                known[*sp as usize] = *d;
            }
        }
        for c in self.list.iter_mut() {
            let k = known.get(c.species as usize).copied().unwrap_or(0);
            if c.tech != k {
                c.tech = k;
            }
        }
    }

    /// The best mining any species knows about.
    ///
    /// The world lets itself be dug into a little deeper for every species
    /// that has worked out how to go down.
    pub fn best_deep_mining(&self) -> u32 {
        let mut best = 0;
        for (_, d) in self.discoveries.iter() {
            if d & discovery_bits::DEEPMINING != 0 {
                best = best.max(1);
            }
        }
        best
    }
}

/// The name of a species, for the record.
fn c_name(species: u32) -> String {
    species_name_for(species)
}
