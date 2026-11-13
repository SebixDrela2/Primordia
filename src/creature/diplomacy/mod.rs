//! Diplomacy: the relations between species.
//!
//! Split by responsibility:
//! - mod.rs        the shared types (RelationState, Treaty, Relation,
//!                 Diplomacy, DiplomaticSense) and the free helpers
//! - negotiation.rs the CreatureSystem side: signing treaties, declaring
//!                 wars, resolving a negotiation and collecting tribute
//!
//! Diplomacy is a *sector* of the simulation, not a scripted event. Every
//! species keeps a relation with every other species it has met, and that
//! relation is made of:
//! - a state   (at war, neutral, allied, tributary, ...)
//! - a trust   (-100 .. +100) that decides whether proposals are accepted
//! - a set of treaties that are signed, kept or betrayed
//!
//! Creatures negotiate the relations, they do not just read them: an envoy
//! walks to a foreign colony, talks, and the outcome depends on the trust
//! that has built up, the relative power of the two species, the mood of the
//! negotiator and the traits it has evolved.

mod negotiation;

pub use negotiation::*;

use super::*;

/// How two species currently feel about each other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelationState {
    /// Open war. No diplomacy is possible until a peace is signed.
    War,
    /// Cold. Treaties have been broken or insults exchanged.
    Hostile,
    /// The default: no opinion, no treaty.
    Neutral,
    /// Warming up. Trust is growing, talks are possible.
    Cordial,
    /// A formal alliance is in force.
    Allied,
    /// This species pays tribute to the other one.
    Vassal,
    /// This species collects tribute from the other one.
    Overlord,
}

impl RelationState {
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

    /// Short blurb shown in the diplomacy panel.
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

    /// A colour for the diplomacy panel.
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

    /// How much this state raises or lowers the chance of accepting a
    /// peaceful proposal.
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

/// The treaties two species can sign.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Treaty {
    /// A formal alliance: mutual defence, shared vision, no attacks.
    Alliance,
    /// A trade agreement: better terms on every trade.
    TradePact,
    /// Neither side attacks the other, but no binding promises are made.
    Ceasefire,
    /// Neither side helps the other's enemies.
    NonAggression,
    /// The weaker side pays tribute to the stronger one.
    Tributary,
}

impl Treaty {
    pub fn name(self) -> &'static str {
        match self {
            Treaty::Alliance => "Alliance",
            Treaty::TradePact => "Trade Pact",
            Treaty::Ceasefire => "Ceasefire",
            Treaty::NonAggression => "Non-Aggression",
            Treaty::Tributary => "Tributary",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Treaty::Alliance => "mutual defence, shared vision",
            Treaty::TradePact => "better trade terms",
            Treaty::Ceasefire => "fighting has stopped",
            Treaty::NonAggression => "neither aids the other's enemies",
            Treaty::Tributary => "tribute flows one way",
        }
    }

    /// How badly breaking this treaty is punished.
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

/// What a diplomat is trying to achieve during a negotiation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiplomaticAction {
    /// Sign a trade pact.
    Trade,
    /// Sign a formal alliance.
    Alliance,
    /// Offer a ceasefire (only meaningful during a war).
    Peace,
    /// Declare war.
    War,
    /// Demand that the weaker side pays tribute.
    Tribute,
    /// Nothing was agreed; the talks were a formality.
    None,
}

impl DiplomaticAction {
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

/// The full relation between two species.
#[derive(Clone)]
pub struct Relation {
    /// The lower species id (the pair is stored once).
    pub a: u32,
    /// The higher species id.
    pub b: u32,
    pub state: RelationState,
    /// -100 (hated) .. +100 (devoted).
    pub trust: f32,
    /// Treaties currently in force.
    pub treaties: Vec<Treaty>,
    /// How many wars the pair has fought.
    pub wars: u32,
    /// How many treaties the pair has broken.
    pub betrayals: u32,
    /// The tick of the last diplomatic action between the pair.
    pub last_action: u64,
    /// Tribute collected by the overlord since the treaty was signed.
    pub tribute_paid: f32,
    /// How tired both sides are of the war. At 1.0 a war is exhausted and
    /// peace becomes much easier to sell.
    pub war_weariness: f32,
}

impl Relation {
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

    pub fn has(&self, t: Treaty) -> bool {
        self.treaties.contains(&t)
    }
}

/// A diplomatic event, shown in the feed and animated on screen.
#[derive(Clone)]
pub struct DiploEvent {
    pub text: String,
    pub color: [u8; 3],
    pub tick: u64,
}

/// Aggregate diplomacy counters, shown in the HUD.
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
    /// How many wars have ended in a tired truce.
    pub truces: u32,
}

/// The whole diplomatic state of the world.
#[derive(Default)]
pub struct Diplomacy {
    /// One relation per species pair that has ever met.
    pub relations: Vec<Relation>,
    /// A rolling feed of diplomatic events.
    pub log: Vec<DiploEvent>,
    pub stats: DiplomacyStats,
    /// Ticks until the next tribute collection.
    pub tribute_timer: u32,
}

impl Diplomacy {
    /// Find the relation between two species, if it exists.
    pub fn get(&self, a: u32, b: u32) -> Option<&Relation> {
        let (lo, hi) = (a.min(b), a.max(b));
        self.relations
            .iter()
            .find(|r| r.a == lo && r.b == hi)
    }

    /// Find the relation between two species, creating it if needed.
    pub fn get_mut(&mut self, a: u32, b: u32) -> &mut Relation {
        let (lo, hi) = (a.min(b), a.max(b));
        if let Some(pos) = self.relations.iter().position(|r| r.a == lo && r.b == hi) {
            return &mut self.relations[pos];
        }
        self.relations.push(Relation::new(lo, hi));
        self.relations.last_mut().unwrap()
    }

    /// Are the two species formally allied?
    pub fn are_allied(&self, a: u32, b: u32) -> bool {
        // A ZORG never allies with anything. There is no treaty with the
        // deep.
        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return false;
        }
        self.get(a, b).map_or(false, |r| r.has(Treaty::Alliance))
    }

    /// Are the two species at war?
    pub fn at_war(&self, a: u32, b: u32) -> bool {
        // A ZORG is at war with everything. It is the deep's own hostile
        // faction: there is no peace to be had with it.
        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return a != b;
        }
        self.get(a, b).map_or(false, |r| r.state == RelationState::War)
    }

    /// Does a specific treaty hold between the two species?
    pub fn has_treaty(&self, a: u32, b: u32, t: Treaty) -> bool {
        self.get(a, b).map_or(false, |r| r.has(t))
    }

    /// The species this one is formally allied with.
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

    /// The trust the two species have in each other (0 when they never met).
    pub fn trust(&self, a: u32, b: u32) -> f32 {
        self.get(a, b).map_or(0.0, |r| r.trust)
    }

    /// Nudge the trust between two species.
    pub fn adjust_trust(&mut self, a: u32, b: u32, delta: f32) {
        let rel = self.get_mut(a, b);
        rel.trust = (rel.trust + delta).clamp(-100.0, 100.0);
    }

    /// Record a diplomatic event in the feed.
    pub fn log_event(&mut self, text: String, color: [u8; 3], tick: u64) {
        self.log.push(DiploEvent { text, color, tick });
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }
}

// ---------------------------------------------------------------------------
// CreatureSystem side of the diplomacy
// ---------------------------------------------------------------------------

/// How fast a role can build rapport with a foreign creature.
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

/// The energy cost of a creature that is out on diplomatic business.
///
/// It is charged per tick of an ongoing negotiation, so a long negotiation is
/// expensive. Envoys pay the least, because diplomacy is their job: a
/// worker that tries to play the diplomat spends far more energy for far
/// less effect.
pub const DIPLO_COST: f32 = 0.2;

/// What a creature knows about its diplomatic situation.
///
/// This is the slice of the world a creature perceives about *other species*:
/// who the nearest foreigner is, how it feels about them and how the two
/// colonies compare in strength. The network turns this into a stance.
#[derive(Clone, Copy, Default)]
pub struct DiplomaticSense {
    /// The species of the nearest foreign creature.
    pub partner: Option<u32>,
    /// Distance to that creature.
    pub dist: f32,
    /// Trust with that species, -100 .. 100.
    pub trust: f32,
    /// The relation state, mapped to -1 (war) .. 1 (allied).
    pub state: f32,
    /// How many alliances this species holds.
    pub alliances: f32,
    /// How many wars this species is fighting.
    pub wars: f32,
    /// How many treaties this species holds in total.
    pub treaties: f32,
    /// Own strength relative to the partner's, 1 = even.
    pub balance: f32,
    /// 1 when the creature is an envoy.
    pub is_envoy: f32,
    /// 1 when the partner is at war with us.
    pub at_war: f32,
    /// 1 when the partner is a formal ally.
    pub allied: f32,
    /// 1 when the creature is currently talking to someone.
    pub talking: f32,
}

impl DiplomaticSense {
    /// Map a relation state onto the -1 .. 1 scale the network reads.
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

/// How far a diplomat will travel to open talks.
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

/// Work out what a creature knows about the other species.
///
/// The nearest foreign species is found from the per-species centroids, so
/// this costs one short scan instead of a full sweep over every creature.
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
    // The nearest other species, measured from its centroid.
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
    // How the colony as a whole is doing: alliances and wars it is in.
    for rel in diplomacy.relations.iter() {
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
