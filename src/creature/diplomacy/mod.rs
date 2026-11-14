mod negotiation;

pub use negotiation::*;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationState {

    War,

    Hostile,

    Neutral,

    Cordial,

    Allied,

    Vassal,

    Overlord,
}

impl RelationState {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            RelationState::War => "WAR",
            RelationState::Hostile => "HOSTILE",
            RelationState::Neutral => "NEUTRAL",
            RelationState::Cordial => "CORDIAL",
            RelationState::Allied => "ALLIED",
            RelationState::Vassal => "VASSAL",
            RelationState::Overlord => "OVERLORD",
        }
    }

    #[must_use]
    pub fn blurb(self) -> &'static str {
        match self {
            RelationState::War => "banners raised, no talks",
            RelationState::Hostile => "cold, treaties broken",
            RelationState::Neutral => "no strong opinion",
            RelationState::Cordial => "friendly, talks welcome",
            RelationState::Allied => "bound by treaty",
            RelationState::Vassal => "pays tribute",
            RelationState::Overlord => "takes tribute",
        }
    }

    #[must_use]
    pub fn color(self) -> [u8; 3] {
        match self {
            RelationState::War => [255, 74, 74],
            RelationState::Hostile => [235, 130, 80],
            RelationState::Neutral => [150, 160, 172],
            RelationState::Cordial => [110, 215, 165],
            RelationState::Allied => [90, 200, 255],
            RelationState::Vassal => [225, 190, 90],
            RelationState::Overlord => [255, 215, 110],
        }
    }

    #[must_use]
    pub fn bias(self) -> f32 {
        match self {
            RelationState::War => -1.0,
            RelationState::Hostile => -0.35,
            RelationState::Neutral => 0.0,
            RelationState::Cordial => 0.2,
            RelationState::Allied => 0.45,
            RelationState::Vassal => -0.1,
            RelationState::Overlord => 0.1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Treaty {

    Alliance,

    TradePact,

    Ceasefire,

    NonAggression,

    Tributary,
}

impl Treaty {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Treaty::Alliance => "Alliance",
            Treaty::TradePact => "Trade Pact",
            Treaty::Ceasefire => "Ceasefire",
            Treaty::NonAggression => "Non-Aggression",
            Treaty::Tributary => "Tributary",
        }
    }

    #[must_use]
    pub fn blurb(self) -> &'static str {
        match self {
            Treaty::Alliance => "mutual defence, shared vision",
            Treaty::TradePact => "better trade terms",
            Treaty::Ceasefire => "fighting has stopped",
            Treaty::NonAggression => "neither aids the other's enemies",
            Treaty::Tributary => "tribute flows one way",
        }
    }

    #[must_use]
    pub fn betrayal_penalty(self) -> f32 {
        match self {
            Treaty::Alliance => 70.0,
            Treaty::TradePact => 25.0,
            Treaty::Ceasefire => 40.0,
            Treaty::NonAggression => 35.0,
            Treaty::Tributary => 55.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiplomaticAction {

    Trade,

    Alliance,

    Peace,

    War,

    Tribute,

    None,
}

impl DiplomaticAction {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            DiplomaticAction::Trade => "TRADE PACT",
            DiplomaticAction::Alliance => "ALLIANCE",
            DiplomaticAction::Peace => "PEACE",
            DiplomaticAction::War => "DECLARE WAR",
            DiplomaticAction::Tribute => "TRIBUTE",
            DiplomaticAction::None => "TALKS",
        }
    }
}

#[derive(Clone)]
pub struct Relation {

    pub a: u32,

    pub b: u32,
    pub state: RelationState,

    pub trust: f32,

    pub treaties: Vec<Treaty>,

    pub wars: u32,

    pub betrayals: u32,

    pub last_action: u64,

    pub tribute_paid: f32,

    pub war_weariness: f32,
}

impl Relation {
    #[must_use]
    pub fn new(a: u32, b: u32) -> Self {
        Relation {
            a: a.min(b),
            b: a.max(b),
            state: RelationState::Neutral,
            trust: 0.0,
            treaties: Vec::new(),
            wars: 0,
            betrayals: 0,
            last_action: 0,
            tribute_paid: 0.0,
            war_weariness: 0.0,
        }
    }

    #[must_use]
    pub fn has(&self, t: Treaty) -> bool {
        self.treaties.contains(&t)
    }
}

#[derive(Clone)]
pub struct DiploEvent {
    pub text: String,
    pub color: [u8; 3],
    pub tick: u64,
}

#[derive(Clone, Copy, Default)]
pub struct DiplomacyStats {
    pub alliances: u32,
    pub wars: u32,
    pub treaties: u32,
    pub tributes: u32,
    pub envoys: u32,
    pub negotiations: u32,
    pub betrayals: u32,
    pub raids: u32,

    pub truces: u32,
}

#[derive(Default)]
pub struct Diplomacy {

    pub relations: Vec<Relation>,

    pub log: Vec<DiploEvent>,
    pub stats: DiplomacyStats,

    pub tribute_timer: u32,
}

impl Diplomacy {

    #[must_use]
    pub fn get(&self, a: u32, b: u32) -> Option<&Relation> {
        let (lo, hi) = (a.min(b), a.max(b));
        self.relations
            .iter()
            .find(|r| r.a == lo && r.b == hi)
    }

    #[must_use]
    pub fn get_mut(&mut self, a: u32, b: u32) -> &mut Relation {
        let (lo, hi) = (a.min(b), a.max(b));
        if let Some(pos) = self.relations.iter().position(|r| r.a == lo && r.b == hi) {
            return &mut self.relations[pos];
        }
        self.relations.push(Relation::new(lo, hi));
        self.relations.last_mut().unwrap()
    }

    #[must_use]
    pub fn are_allied(&self, a: u32, b: u32) -> bool {

        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return false;
        }
        self.get(a, b).map_or(false, |r| r.has(Treaty::Alliance))
    }

    #[must_use]
    pub fn at_war(&self, a: u32, b: u32) -> bool {

        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return a != b;
        }
        self.get(a, b).map_or(false, |r| r.state == RelationState::War)
    }

    #[must_use]
    pub fn has_treaty(&self, a: u32, b: u32, t: Treaty) -> bool {
        self.get(a, b).map_or(false, |r| r.has(t))
    }

    #[must_use]
    pub fn allies_of(&self, species: u32) -> Vec<u32> {
        self.relations
            .iter()
            .filter(|r| r.has(Treaty::Alliance))
            .filter_map(|r| {
                if r.a == species {
                    Some(r.b)
                } else if r.b == species {
                    Some(r.a)
                } else {
                    None
                }
            })
            .collect()
    }

    #[must_use]
    pub fn trust(&self, a: u32, b: u32) -> f32 {
        self.get(a, b).map_or(0.0, |r| r.trust)
    }

    pub fn adjust_trust(&mut self, a: u32, b: u32, delta: f32) {
        let rel = self.get_mut(a, b);
        rel.trust = (rel.trust + delta).clamp(-100.0, 100.0);
    }

    pub fn log_event(&mut self, text: String, color: [u8; 3], tick: u64) {
        self.log.push(DiploEvent { text, color, tick });
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }
}

#[must_use]
pub fn negotiation_rate(role: Role) -> f32 {
    match role {
        Role::Envoy => 1.0,
        Role::Queen => 0.55,
        Role::Merchant => 0.45,
        Role::Priest => 0.4,
        Role::Soldier => 0.15,
        Role::Explorer => 0.3,
        Role::Builder => 0.2,
        Role::Worker => 0.18,
        Role::Shaman => 0.25,
    }
}

pub const DIPLO_COST: f32 = 0.2;

#[derive(Clone, Copy, Default)]
pub struct DiplomaticSense {

    pub partner: Option<u32>,

    pub dist: f32,

    pub trust: f32,

    pub state: f32,

    pub alliances: f32,

    pub wars: f32,

    pub treaties: f32,

    pub balance: f32,

    pub is_envoy: f32,

    pub at_war: f32,

    pub allied: f32,

    pub talking: f32,
}

impl DiplomaticSense {

    #[must_use]
    pub fn state_value(state: RelationState) -> f32 {
        match state {
            RelationState::War => -1.0,
            RelationState::Hostile => -0.5,
            RelationState::Neutral => 0.0,
            RelationState::Cordial => 0.4,
            RelationState::Allied => 0.9,
            RelationState::Vassal => -0.3,
            RelationState::Overlord => 0.3,
        }
    }
}

#[must_use]
pub fn diplomacy_reach(role: Role) -> f32 {
    match role {
        Role::Envoy => 22.0,
        Role::Queen => 14.0,
        Role::Merchant => 14.0,
        Role::Priest => 12.0,
        Role::Explorer => 12.0,
        Role::Shaman => 10.0,
        Role::Soldier => 8.0,
        Role::Builder => 8.0,
        Role::Worker => 8.0,
    }
}

pub fn diplomatic_sense(
    diplomacy: &Diplomacy,
    c: &Creature,
    centroids: &[(u32, f32, f32)],
    pops: &[(u32, u32)],
) -> DiplomaticSense {
    let mut ds = DiplomaticSense {
        dist: 999.0,
        balance: 1.0,
        is_envoy: if c.role == Role::Envoy { 1.0 } else { 0.0 },
        talking: if c.diplo_with.is_some() { 1.0 } else { 0.0 },
        ..Default::default()
    };

    let mut best: Option<(u32, f32)> = None;
    for (sp, sx, sy) in centroids {
        if *sp == c.species {
            continue;
        }
        let d = ((sx - c.x).powi(2) + (sy - c.y).powi(2)).sqrt();
        if best.map_or(true, |(_, bd)| d < bd) {
            best = Some((*sp, d));
        }
    }
    let Some((partner, dist)) = best else {
        return ds;
    };
    ds.partner = Some(partner);
    ds.dist = dist;
    if let Some(rel) = diplomacy.get(c.species, partner) {
        ds.trust = rel.trust;
        ds.state = DiplomaticSense::state_value(rel.state);
        ds.at_war = if rel.state == RelationState::War { 1.0 } else { 0.0 };
        ds.allied = if rel.has(Treaty::Alliance) { 1.0 } else { 0.0 };
        ds.treaties = rel.treaties.len() as f32;
    }

    for rel in &diplomacy.relations {
        if rel.a == c.species || rel.b == c.species {
            if rel.state == RelationState::Allied {
                ds.alliances += 1.0;
            }
            if rel.state == RelationState::War {
                ds.wars += 1.0;
            }
        }
    }
    let my_pop = pops
        .iter()
        .find(|(s, _)| *s == c.species)
        .map(|(_, n)| *n)
        .unwrap_or(1)
        .max(1);
    let their_pop = pops
        .iter()
        .find(|(s, _)| *s == partner)
        .map(|(_, n)| *n)
        .unwrap_or(1)
        .max(1);
    ds.balance = my_pop as f32 / their_pop as f32;
    ds
}
