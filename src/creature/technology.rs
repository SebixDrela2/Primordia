use super::{Culture, CreatureSystem, Treaty, World, ZORG_SPECIES, SimEvent, MAX_ARCH, species_name_for};

pub mod discovery_bits {

    pub const SMITHING: u32 = 1 << 0;

    pub const HUSBANDRY: u32 = 1 << 1;

    pub const POTTERY: u32 = 1 << 2;

    pub const RANGED: u32 = 1 << 3;

    pub const ARCHITECTURE: u32 = 1 << 4;

    pub const MASONRY: u32 = 1 << 5;

    pub const METALLURGY: u32 = 1 << 6;

    pub const COMMERCE: u32 = 1 << 7;

    pub const CARTOGRAPHY: u32 = 1 << 8;

    pub const MEDICINE: u32 = 1 << 9;

    pub const HERBALISM: u32 = 1 << 10;

    pub const SURGERY: u32 = 1 << 11;

    pub const ASTRONOMY: u32 = 1 << 12;

    pub const SANCTUARY: u32 = 1 << 13;

    pub const NAVIGATION: u32 = 1 << 14;

    pub const DEEPMINING: u32 = 1 << 15;

    pub const GEODESY: u32 = 1 << 16;

    pub const SMELTING: u32 = 1 << 17;

    pub const ANTITOXINS: u32 = 1 << 18;

    pub const PURGE: u32 = 1 << 19;

    pub const WARDCRAFT: u32 = 1 << 20;

    pub const SANCTIFY: u32 = 1 << 21;

    pub const SUNFIRECRAFT: u32 = 1 << 22;

    pub const RADIANCE: u32 = 1 << 23;

    pub const SIEGECRAFT: u32 = 1 << 24;

    pub const TACTICS: u32 = 1 << 25;

    pub const IRONWILL: u32 = 1 << 26;

    pub const ALCHEMY: u32 = 1 << 27;

    pub const DEEPDIVING: u32 = 1 << 28;

    pub const DOMINION: u32 = 1 << 29;

    pub const COGNITION: u32 = 1 << 30;

    pub const GRANDCOGNITION: u32 = 1 << 31;
}

pub const DISCOVERY_COUNT: usize = 32;

pub const TECH_TIERS: u8 = 10;

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

    Cognition,

    GrandCognition,
}

impl Discovery {

    #[must_use]
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

    #[must_use]
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

    #[must_use]
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

    #[must_use]
    pub fn tier(self) -> u8 {
        match self {
            Discovery::Husbandry | Discovery::Pottery | Discovery::Cognition => 1,
            Discovery::Smithing | Discovery::Ranged => 2,
            Discovery::Architecture | Discovery::Masonry | Discovery::GrandCognition => 3,
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
        }
    }

    #[must_use]
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

    #[must_use]
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

    #[must_use]
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

    #[must_use]
    pub fn requires(self) -> &'static [Culture] {
        match self {
            Discovery::Smithing | Discovery::Metallurgy | Discovery::Smelting | Discovery::GrandCognition => &[Culture::Craft, Culture::Mining],
            Discovery::Husbandry => &[Culture::Cultivation],
            Discovery::Pottery | Discovery::Cognition => &[Culture::Craft],
            Discovery::Ranged | Discovery::Cartography | Discovery::Siegecraft => &[Culture::Craft, Culture::Warfare],
            Discovery::Architecture => &[Culture::Architecture],
            Discovery::Masonry => &[Culture::Craft, Culture::Architecture],
            Discovery::Commerce => &[Culture::Commerce, Culture::Ritual],
            Discovery::Medicine | Discovery::Antitoxins | Discovery::Sanctification | Discovery::Sunfirecraft | Discovery::Alchemy | Discovery::Deepdiving => &[Culture::Craft, Culture::Ritual],
            Discovery::Herbalism => &[Culture::Cultivation, Culture::Ritual],
            Discovery::Surgery | Discovery::Geodesy | Discovery::Purge | Discovery::Radiance => &[Culture::Ritual, Culture::Craft],
            Discovery::Astronomy | Discovery::Sanctuary => &[Culture::Ritual],
            Discovery::Navigation => &[Culture::Warfare, Culture::Craft],
            Discovery::DeepMining => &[Culture::Mining, Culture::Craft],
            Discovery::Wardcraft => &[Culture::Architecture, Culture::Ritual],
            Discovery::Tactics => &[Culture::Warfare, Culture::Ritual],
            Discovery::Ironwill => &[Culture::Warfare, Culture::Cultivation],
            Discovery::Dominion => &[Culture::Architecture, Culture::Craft],
        }
    }

    #[must_use]
    pub fn depth(self) -> u32 {
        match self.tier() {
            0 | 1 => 0,
            2 => 20,
            3 => 40,
            4 => 55,
            5 => 70,
            6 => 105,
            7 => 150,
            8 => 175,
            _ => 210,
        }
    }

    #[must_use]
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

#[must_use]
pub fn discovery_list(bits: u32) -> Vec<Discovery> {
    let mut v = Vec::new();
    for d in Discovery::all() {
        if bits & d.bit() != 0 {
            v.push(d);
        }
    }
    v
}

#[must_use]
pub fn discovery_tier(tier: u8) -> Vec<Discovery> {
    Discovery::all()
        .into_iter()
        .filter(|d| d.tier() == tier)
        .collect()
}

pub mod activity {

    pub const EAT: u32 = 1;

    pub const CRAFT: u32 = 1 << 1;

    pub const BUILD: u32 = 1 << 2;

    pub const TRADE: u32 = 1 << 3;

    pub const MINE: u32 = 1 << 4;

    pub const CAST: u32 = 1 << 5;

    pub const ATTACK: u32 = 1 << 6;

    #[must_use]
    pub fn for_tier(tier: u8) -> u32 {
        match tier {
            0..=2 => CRAFT,
            4 => TRADE,
            5 => EAT,
            6 => MINE,
            7 | 8 => CAST,
            9 => ATTACK,
            _ => BUILD,
        }
    }
}

impl CreatureSystem {

    #[must_use]
    pub fn species_discoveries(&self, species: u32) -> u32 {
        self.discoveries
            .iter()
            .find(|(id, _)| *id == species)
            .map_or(0, |(_, d)| *d)
    }

    #[must_use]
    pub fn species_activity(&self, species: u32) -> u32 {
        self.activity
            .iter()
            .find(|(sp, _)| *sp == species)
            .map_or(0, |(_, a)| *a)
    }

    #[must_use]
    pub fn has_discovery(&self, species: u32, d: Discovery) -> bool {
        self.species_discoveries(species) & d.bit() != 0
    }

    #[must_use]
    pub fn species_deepest(&self, species: u32) -> u32 {
        self.deepest
            .iter()
            .find(|(sp, _)| *sp == species)
            .map_or(0, |(_, d)| *d)
    }

    pub fn note_discovery(&mut self, species: u32, bit: u32) {
        if let Some(entry) = self.discoveries.iter_mut().find(|(id, _)| *id == species) {
            entry.1 |= bit;
        } else {
            self.discoveries.push((species, bit));
        }
    }

    #[must_use]
    pub fn tech_tier(&self) -> u8 {
        let mut all = 0u32;
        for (_, d) in &self.discoveries {
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

    #[must_use]
    pub fn discoveries_found(&self) -> Vec<(Discovery, u32)> {
        let mut out = Vec::new();
        for d in Discovery::all() {

            let mut holders = Vec::new();
            for (sp, bits) in &self.discoveries {
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

    #[must_use]
    pub fn greatest_discovery(&self) -> Option<Discovery> {
        let mut best: Option<Discovery> = None;
        for (_, bits) in &self.discoveries {
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

    #[must_use]
    pub fn latest_discovery(&self) -> Option<(Discovery, u32)> {
        self.last_discovery
    }

    pub fn share_discoveries(&mut self) {
        if !self.tick.is_multiple_of(240) {
            return;
        }
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for rel in &self.diplomacy.relations {
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

                for d in Discovery::all() {
                    if shared & d.bit() != 0 {
                        self.grow_brains(b, d);
                    }
                }

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

    pub fn check_discoveries(&mut self, world: &World) {
        if !self.tick.is_multiple_of(300) {
            return;
        }

        let mut species: Vec<(u32, u32, u16, u32, bool)> = Vec::new();
        for c in &self.list {
            let entry = if let Some(e) = species.iter_mut().find(|(id, _, _, _, _)| *id == c.species) { e } else {
                species.push((c.species, 0, 0, 0, false));
                species.last_mut().unwrap()
            };
            entry.1 = entry.1.max(c.generation);
            entry.2 |= c.culture;
            entry.3 += 1;

            if world.landmark_bonus(c.x, c.y).lore {
                entry.4 = true;
            }
        }
        for (sp, generation, culture, members, lore) in species {
            if members < 3 {
                continue;
            }

            if sp == ZORG_SPECIES {
                continue;
            }
            let known = self.species_discoveries(sp);
            for d in Discovery::all() {
                if known & d.bit() != 0 {
                    continue;
                }

                if d.tier() > 1 && !self.tier_complete(known, d.tier() - 1) {
                    continue;
                }

                let doing = self.species_activity(sp);
                if doing & activity::for_tier(d.tier()) == 0 {
                    continue;
                }

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

                if generation < d.generations() {
                    continue;
                }

                if self.species_deepest(sp) < d.depth() {
                    continue;
                }

                let chance = if lore { 0.34 } else { 0.22 };
                if self.rng.chance(chance) {
                    self.note_discovery(sp, d.bit());
                    self.last_discovery = Some((d, sp));

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

    pub fn grow_brains(&mut self, species: u32, d: Discovery) {
        let target = match d {
            Discovery::Cognition => 1,
            Discovery::GrandCognition => MAX_ARCH,
            _ => return,
        };
        let rng = &mut self.rng;
        for c in &mut self.list {
            if c.species == species && c.net.arch < target {
                let _ = c.net.grow_to(target, rng);
            }
        }
    }

    #[must_use]
    pub fn tier_complete(&self, known: u32, tier: u8) -> bool {
        discovery_tier(tier)
            .iter()
            .all(|d| known & d.bit() != 0)
    }

    pub fn refresh_tech(&mut self) {

        let mut known: Vec<u32> = vec![0; self.next_species as usize + 8];
        for (sp, d) in &self.discoveries {
            if (*sp as usize) < known.len() {
                known[*sp as usize] = *d;
            }
        }
        for c in &mut self.list {
            let k = known.get(c.species as usize).copied().unwrap_or(0);
            if c.tech != k {
                c.tech = k;
            }
        }
    }

    #[must_use]
    pub fn best_deep_mining(&self) -> u32 {
        let mut best = 0;
        for (_, d) in &self.discoveries {
            if d & discovery_bits::DEEPMINING != 0 {
                best = best.max(1);
            }
        }
        best
    }
}

fn c_name(species: u32) -> String {
    species_name_for(species)
}
