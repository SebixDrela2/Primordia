//! The CreatureSystem side of the diplomacy: signing treaties, declaring
//! wars, resolving a negotiation and collecting the tribute.

use super::*;

impl CreatureSystem {
    /// Drop every relation that involves a species which no longer exists.
    pub(crate) fn prune_diplomacy(&mut self) {
        let alive: Vec<u32> = self.list.iter().map(|c| c.species).collect();
        self.diplomacy
            .relations
            .retain(|r| alive.contains(&r.a) && alive.contains(&r.b));
    }

    /// The combined population of a species, used for the power comparison.
    pub fn species_population(&self, species: u32) -> u32 {
        self.list.iter().filter(|c| c.species == species).count() as u32
    }

    /// The combined strength of a species: numbers, tech and weapons.
    fn species_power(&self, species: u32) -> f32 {
        let mut power = 0.0;
        let mut members = 0u32;
        for c in self.list.iter().filter(|c| c.species == species) {
            members += 1;
            power += 1.0;
            power += c.tech_level as f32 * 0.35;
            power += match c.tool {
                Tool::Sword | Tool::Bow | Tool::Spear | Tool::Staff => 0.5,
                Tool::Pickaxe | Tool::Drill => 0.25,
                _ => 0.0,
            };
            if c.role == Role::Soldier {
                power += 0.4;
            }
            if c.role == Role::Queen {
                power += 0.8;
            }
        }
        if members == 0 {
            return 0.0;
        }
        power
    }

    /// Declare war between two species. Any treaty they held is destroyed.
    pub(crate) fn declare_war(&mut self, a: u32, b: u32, reason: &str, tick: u64) {
        let color_a = self.species_color(a);
        let broke_treaty;
        {
            let rel = self.diplomacy.get_mut(a, b);
            broke_treaty = !rel.treaties.is_empty();
            rel.treaties.clear();
            rel.state = RelationState::War;
            rel.trust = (rel.trust - 45.0).max(-100.0);
            rel.wars += 1;
            rel.war_weariness = 0.0;
            rel.last_action = tick;
            if broke_treaty {
                rel.betrayals += 1;
            }
        }
        if broke_treaty {
            self.diplomacy.stats.betrayals += 1;
        }
        self.diplomacy.stats.wars += 1;
        self.diplomacy.log_event(
            format!("WAR: species {} vs species {} ({})", a, b, reason),
            color_a,
            tick,
        );
        // Everyone on both sides now has a reason to fight, so their mood
        // sharpens towards anger.
        for c in self.list.iter_mut() {
            if c.species == a || c.species == b {
                c.mood = Mood::Angry;
                c.mood_timer = 600;
            }
        }
    }

    /// Sign a treaty between two species.
    ///
    /// Returns true when the treaty was new. Signing a treaty that is
    /// already in force does nothing, so a pair cannot keep "renewing" the
    /// same pact forever.
    pub(crate) fn sign_treaty(&mut self, a: u32, b: u32, treaty: Treaty, tick: u64) -> bool {
        // A ZORG signs nothing. There is no treaty with the deep.
        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return false;
        }
        let color_a = self.species_color(a);
        // The power comparison decides who ends up above whom in a tribute
        // treaty. It has to be computed before the relation is borrowed.
        let pa = self.species_power(a);
        let pb = self.species_power(b);
        let mut fresh = false;
        {
            let rel = self.diplomacy.get_mut(a, b);
            if !rel.has(treaty) {
                rel.treaties.push(treaty);
                fresh = true;
                rel.trust = (rel.trust + 18.0).clamp(-100.0, 100.0);
            }
            rel.last_action = tick;
            match treaty {
                Treaty::Alliance => rel.state = RelationState::Allied,
                Treaty::TradePact => {
                    if rel.state == RelationState::Neutral {
                        rel.state = RelationState::Cordial;
                    }
                }
                Treaty::Ceasefire => {
                    if rel.state == RelationState::War {
                        rel.state = RelationState::Cordial;
                    }
                }
                Treaty::NonAggression => {
                    if rel.state == RelationState::Hostile {
                        rel.state = RelationState::Neutral;
                    }
                }
                Treaty::Tributary => {
                    // The stronger species becomes the overlord.
                    if pa >= pb {
                        rel.state = RelationState::Overlord;
                    } else {
                        rel.state = RelationState::Vassal;
                    }
                }
            }
        }
        if fresh {
            self.diplomacy.stats.treaties += 1;
            self.diplomacy.log_event(
                format!("TREATY: {} and {} sign {}", a, b, treaty.name()),
                color_a,
                tick,
            );
        }
        fresh
    }

    /// Break a treaty. This is the most destructive act in diplomacy: the
    /// other side remembers it for a long time.
    pub(crate) fn break_treaty(&mut self, a: u32, b: u32, treaty: Treaty, tick: u64) {
        let mut broke = false;
        {
            let rel = self.diplomacy.get_mut(a, b);
            if let Some(pos) = rel.treaties.iter().position(|t| *t == treaty) {
                rel.treaties.remove(pos);
                broke = true;
                rel.trust = (rel.trust - treaty.betrayal_penalty()).max(-100.0);
                rel.betrayals += 1;
                rel.last_action = tick;
                if rel.state == RelationState::Allied || rel.state == RelationState::Overlord
                    || rel.state == RelationState::Vassal
                {
                    rel.state = RelationState::Hostile;
                }
            }
        }
        if broke {
            self.diplomacy.stats.betrayals += 1;
            self.diplomacy.log_event(
                format!("BETRAYAL: {} breaks {} with {}", a, treaty.name(), b),
                [255, 110, 110],
                tick,
            );
        }
    }

    /// The colour a species is drawn in.
    pub fn species_color(&self, species: u32) -> [u8; 3] {
        self.list
            .iter()
            .find(|c| c.species == species)
            .map(|c| c.team_color)
            .unwrap_or([200, 200, 200])
    }

    /// The name of a species, taken from one of its members.
    pub fn species_name(&self, species: u32) -> String {
        self.list
            .iter()
            .find(|c| c.species == species)
            .map(|c| {
                let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
                String::from_utf8_lossy(&c.name[..end]).to_string()
            })
            .unwrap_or_else(|| format!("species {}", species))
    }

    /// The name of a *lineage*: the name the species is known by, which is
    /// the same for every member of it.
    pub fn lineage_name(&self, species: u32) -> String {
        species_name_for(species)
    }
}

/// Find the nearest creature of another species that a creature could talk
/// to. Returns its index in the creature list.
///
/// The search goes through the spatial grid, so it never scans the whole
/// colony. This is a free function rather than a method because the caller
/// holds a mutable borrow of a single creature while it looks for a
/// partner.
pub(crate) fn find_diplomatic_partner(
    c: &Creature,
    grid: &SpatialGrid,
    agents: &[(u32, f32, f32, u32, u8, f32)],
) -> Option<usize> {
    let reach = diplomacy_reach(c.role);
    let reach2 = reach * reach;
    let mut best: Option<(usize, f32)> = None;
    grid.near(c.x, c.y, reach as usize, |idx| {
        let a = &agents[idx as usize];
        if a.0 == c.id || a.3 == c.species {
            return;
        }
        let dx = a.1 - c.x;
        let dy = a.2 - c.y;
        let d2 = dx * dx + dy * dy;
        if d2 <= reach2 && best.map_or(true, |(_, bd)| d2 < bd) {
            best = Some((idx as usize, d2));
        }
    });
    best.map(|(i, _)| i)
}

/// Work out what a creature would like to negotiate about, given the
/// outputs of its network and the current state of the relations.
pub fn desired_action(
    diplomacy: &Diplomacy,
    out: &[f32],
    own_species: u32,
    partner_species: u32,
) -> DiplomaticAction {
    // A ZORG has no diplomacy. It is always at war with everything and it
    // never proposes anything.
    if own_species == ZORG_SPECIES || partner_species == ZORG_SPECIES {
        return DiplomaticAction::War;
    }
    let at_war = diplomacy.at_war(own_species, partner_species);
    let allied = diplomacy.are_allied(own_species, partner_species);
    let trust = diplomacy.trust(own_species, partner_species);
    let hostile = out[outputs::HOSTILE];
    let ally = out[outputs::ALLY];

    // The war is already on: the only question left is whether to make
    // peace. Declaring it again would just be noise.
    if at_war {
        return DiplomaticAction::Peace;
    }
    if !allied {
        // The alliance and hostility outputs sit next to each other in the
        // network, so they are strongly correlated: a creature that wants
        // an alliance often looks hostile as well. The alliance is only
        // proposed when the hostility is moderate, and the war only when
        // the hostility clearly wins.
        if ally > 0.42 && hostile < 0.62 && trust > -25.0 {
            return DiplomaticAction::Alliance;
        }
        // A war is declared when the creature wants it, or when the other
        // side is badly mistrusted and the creature is already inclined
        // that way. Without the second rule a run never turns into a war
        // and the whole peace treaty machinery stays unused.
        if hostile > 0.55 && hostile >= ally {
            return DiplomaticAction::War;
        }
        if trust < -45.0 && hostile > 0.15 {
            return DiplomaticAction::War;
        }
        // A tribute demand only makes sense against a species that is not
        // already paying.
        if out[outputs::TRIBUTE] > 0.45
            && !diplomacy.has_treaty(own_species, partner_species, Treaty::Tributary)
        {
            return DiplomaticAction::Tribute;
        }
    }
    // Everything that could be signed is already signed: the talks are a
    // formality now, but they still warm the relations up.
    if diplomacy.has_treaty(own_species, partner_species, Treaty::TradePact)
        && diplomacy.has_treaty(own_species, partner_species, Treaty::Ceasefire)
        && diplomacy.has_treaty(own_species, partner_species, Treaty::NonAggression)
    {
        return DiplomaticAction::None;
    }
    DiplomaticAction::Trade
}

impl CreatureSystem {
    /// Conclude all the talks that were under way this tick.
    ///
    /// The talks are resolved after the creature loop, because signing a
    /// treaty changes the relations of two species, and that would
    /// otherwise change what the other creatures decide to do in the very
    /// same tick.
    pub(crate) fn conclude_negotiations(
        &mut self,
        proposals: &[(usize, usize, DiplomaticAction)],
        willing: &[f32],
    ) {
        for (actor, partner, action) in proposals {
            let ready = willing.get(*partner).copied().unwrap_or(0.0) > 0.2;
            let Some((act, accepted)) =
                self.advance_negotiation(*actor, *partner, *action, ready)
            else {
                continue;
            };
            if *actor >= self.list.len() || *partner >= self.list.len() {
                continue;
            }
            let (x, y, color) = {
                let a = &self.list[*actor];
                let p = &self.list[*partner];
                ((a.x + p.x) * 0.5, (a.y + p.y) * 0.5, a.team_color)
            };
            match act {
                DiplomaticAction::War => {
                    self.events.push(SimEvent::War { x, y, color });
                }
                DiplomaticAction::None => {}
                other => {
                    let label = if accepted {
                        format!("{} SIGNED", other.name())
                    } else {
                        format!("{} REFUSED", other.name())
                    };
                    self.events.push(SimEvent::Treaty { x, y, color, label });
                }
            }
        }
    }

    /// Advance the negotiation between two creatures.
    pub(crate) fn advance_negotiation(
        &mut self,
        index: usize,
        partner_index: usize,
        action: DiplomaticAction,
        partner_willing: bool,
    ) -> Option<(DiplomaticAction, bool)> {
        if index >= self.list.len() || partner_index >= self.list.len() {
            return None;
        }
        let partner_id = self.list[partner_index].id;
        let partner_species = self.list[partner_index].species;
        let my_species = self.list[index].species;
        if partner_species == my_species {
            return None;
        }
        let role = self.list[index].role;
        let rate = negotiation_rate(role);
        let switched = self.list[index].diplo_with != Some(partner_id);
        let ready = if switched {
            self.list[index].diplo_progress = 0.0;
            false
        } else {
            self.list[index].diplo_progress += rate * 0.035;
            // Diplomacy is work: a creature that spends its life at the
            // negotiating table is not foraging or fighting. The
            // specialists pay less, because that is what they are for.
            let cost = DIPLO_COST * (1.0 / rate.max(0.25)).min(4.0);
            self.list[index].energy = (self.list[index].energy - cost).max(0.0);
            self.list[index].diplo_progress >= 1.0
        };
        if !ready {
            self.list[index].diplo_with = Some(partner_id);
            self.list[index].diplo_partner = Some(partner_species);
            return None;
        }

        // The talks conclude.
        self.list[index].diplo_progress = 0.0;
        self.list[index].diplo_with = None;
        self.list[index].diplo_partner = None;
        self.list[index].diplo_cooldown = if role == Role::Envoy { 200 } else { 600 };
        // The other side starts over as well, so a treaty is not concluded
        // twice in a row by both parties.
        self.list[partner_index].diplo_progress = 0.0;
        self.list[partner_index].diplo_with = None;
        self.list[partner_index].diplo_partner = None;

        if !partner_willing {
            // The other side walks away from the table.
            self.adjust_trust(my_species, partner_species, -3.0);
            return None;
        }
        self.list[index].pacts_signed += 1;
        Some(self.resolve_negotiation(index, partner_index, action))
    }

    /// Resolve a single negotiation between two creatures of different
    /// species. Returns the action that was attempted and whether it worked.
    pub(crate) fn resolve_negotiation(
        &mut self,
        actor: usize,
        partner: usize,
        action: DiplomaticAction,
    ) -> (DiplomaticAction, bool) {
        if actor >= self.list.len() || partner >= self.list.len() {
            return (DiplomaticAction::None, false);
        }
        let (sp_a, sp_b) = (self.list[actor].species, self.list[partner].species);
        if sp_a == sp_b {
            return (DiplomaticAction::None, false);
        }
        let tick = self.tick;
        let power_a = self.species_power(sp_a);
        let power_b = self.species_power(sp_b);
        let trust = self.diplomacy.trust(sp_a, sp_b);
        let state = self
            .diplomacy
            .get(sp_a, sp_b)
            .map(|r| r.state)
            .unwrap_or(RelationState::Neutral);
        let actor_rep = self.list[actor].reputation as f32;
        let partner_rep = self.list[partner].reputation as f32;
        let actor_peaceful = self.list[actor].traits & trait_bits::PEACEFUL != 0;
        let partner_peaceful = self.list[partner].traits & trait_bits::PEACEFUL != 0;

        // The chance the partner accepts. Trust dominates; power, reputation
        // and the temperament of both sides bend the odds.
        let mut chance: f32;
        match action {
            DiplomaticAction::Trade => {
                chance = 0.25 + trust / 260.0 + state.bias() * 0.3;
            }
            DiplomaticAction::Alliance => {
                // An alliance is a big ask: it needs real trust and a belief
                // that the other side is not about to be overwhelmed.
                let power_ratio = power_a / power_b.max(0.001);
                let balance = 1.0 - (power_ratio - 1.0).abs().min(1.0);
                chance = 0.08 + trust / 130.0 + balance * 0.30 + state.bias() * 0.25;
            }
            DiplomaticAction::Peace => {
                // A war-weary side is much easier to talk to.
                let weariness = self
                    .diplomacy
                    .get(sp_a, sp_b)
                    .map(|r| r.war_weariness)
                    .unwrap_or(0.0);
                chance = 0.08 + weariness * 0.45 + trust / 300.0;
            }
            DiplomaticAction::Tribute => {
                // A tribute demand only works if the asker is clearly stronger
                // and the victim has little trust to fall back on.
                let ratio = power_a / power_b.max(0.001);
                if ratio < 1.5 {
                    chance = -1.0;
                } else {
                    chance = 0.05 + (ratio - 1.5).min(2.0) * 0.18 - trust / 200.0;
                }
            }
            DiplomaticAction::War => {
                // Declaring war does not need consent: it is a fait accompli.
                // The only question is whether the other side was ready for it.
                chance = 0.35 + (1.0 - trust / 100.0) * 0.4;
            }
            DiplomaticAction::None => chance = 0.5,
        }
        // Temperament: peaceful species are more willing to sign and less
        // willing to be pushed around.
        if partner_peaceful && action != DiplomaticAction::War {
            chance += 0.12;
        }
        if actor_peaceful && action != DiplomaticAction::War {
            chance += 0.06;
        }
        // Reputation travels with the envoy: a well-liked diplomat is believed.
        chance += (actor_rep - partner_rep) / 400.0;
        chance = chance.clamp(0.02, 0.95);

        // A species that is being crushed accepts anything.
        if action == DiplomaticAction::Peace && power_b < power_a * 0.5 {
            chance += 0.25;
        }

        let accepted = self.rng.chance(chance);

        match action {
            DiplomaticAction::Trade => {
                if accepted {
                    // A trade pact on its own. A ceasefire is only signed
                    // when a war actually ends, so the world does not fall
                    // permanently quiet.
                    self.sign_treaty(sp_a, sp_b, Treaty::TradePact, tick);
                } else {
                    self.adjust_trust(sp_a, sp_b, -4.0);
                }
            }
            DiplomaticAction::Alliance => {
                if accepted {
                    if self.sign_treaty(sp_a, sp_b, Treaty::Alliance, tick) {
                        self.sign_treaty(sp_a, sp_b, Treaty::NonAggression, tick);
                        self.sign_treaty(sp_a, sp_b, Treaty::TradePact, tick);
                        self.diplomacy.stats.alliances += 1;
                    }
                } else {
                    self.adjust_trust(sp_a, sp_b, -8.0);
                }
            }
            DiplomaticAction::Peace => {
                if accepted {
                    self.break_treaty(sp_a, sp_b, Treaty::Alliance, tick);
                    self.sign_treaty(sp_a, sp_b, Treaty::Ceasefire, tick);
                    self.sign_treaty(sp_a, sp_b, Treaty::NonAggression, tick);
                    {
                        let rel = self.diplomacy.get_mut(sp_a, sp_b);
                        rel.state = RelationState::Cordial;
                        rel.war_weariness = 0.0;
                    }
                } else {
                    self.adjust_trust(sp_a, sp_b, -6.0);
                }
            }
            DiplomaticAction::Tribute => {
                if accepted {
                    self.sign_treaty(sp_a, sp_b, Treaty::Tributary, tick);
                    self.diplomacy.stats.tributes += 1;
                } else {
                    self.adjust_trust(sp_a, sp_b, -20.0);
                    // A refused tribute demand is a serious insult and can
                    // start a war on the spot.
                    let hostile = self.diplomacy.trust(sp_a, sp_b) < -40.0;
                    if hostile {
                        self.declare_war(sp_a, sp_b, "tribute refused", tick);
                    }
                }
            }
            DiplomaticAction::War => {
                self.declare_war(sp_a, sp_b, "envoys expelled", tick);
                // The aggressor looks bad: trust with everyone else drops a
                // little, and other species start to distrust the aggressor.
                self.adjust_trust(sp_a, sp_b, -30.0);
            }
            DiplomaticAction::None => {
                if accepted {
                    // Courtesy talks: nothing is signed, but the two sides
                    // get to know each other a little better.
                    self.adjust_trust(sp_a, sp_b, 3.0);
                }
            }
        }

        self.diplomacy.stats.negotiations += 1;
        (action, accepted)
    }

    /// Nudge the trust between two species.
    pub(crate) fn adjust_trust(&mut self, a: u32, b: u32, delta: f32) {
        let rel = self.diplomacy.get_mut(a, b);
        rel.trust = (rel.trust + delta).clamp(-100.0, 100.0);
    }

    /// Collect tribute from vassals and let trust decay towards zero.
    ///
    /// This runs on a slow timer, not every tick, so the numbers move at a
    /// pace a player can actually read on the diplomacy panel.
    pub(crate) fn tick_diplomacy(&mut self) {
        // Relations with a species that no longer exists are dropped: the
        // history of a dead lineage is of no interest to anybody.
        if self.tick % 120 == 0 {
            self.prune_diplomacy();
        }
        // Tribute collection.
        self.diplomacy.tribute_timer += 1;
        if self.diplomacy.tribute_timer >= 120 {
            self.diplomacy.tribute_timer = 0;
            let pairs: Vec<(u32, u32, bool)> = self
                .diplomacy
                .relations
                .iter()
                .filter(|r| r.has(Treaty::Tributary))
                .map(|r| {
                    let overlord_is_a = r.state == RelationState::Overlord;
                    (r.a, r.b, overlord_is_a)
                })
                .collect();
            for (a, b, overlord_is_a) in pairs {
                let (overlord, vassal) = if overlord_is_a { (a, b) } else { (b, a) };
                let amount = 40.0;
                let mut paid = false;
                for c in self.list.iter_mut() {
                    if c.species == vassal && c.energy > 120.0 {
                        c.energy -= amount;
                        paid = true;
                        break;
                    }
                }
                if paid {
                    for c in self.list.iter_mut() {
                        if c.species == overlord {
                            c.energy = (c.energy + amount).min(c.max_energy);
                            break;
                        }
                    }
                    let rel = self.diplomacy.get_mut(a, b);
                    rel.tribute_paid += amount;
                    self.diplomacy.stats.tributes += 1;
                } else {
                    // The vassal cannot pay: the tribute is overdue and the
                    // overlord may turn it into a war.
                    self.adjust_trust(a, b, -3.0);
                }
            }
        }

        // Slow drift: trust decays towards zero unless treaties keep it up.
        if self.tick % 60 == 0 {
            // Where each species has its home. Two colonies that live too
            // close together resent each other: the ground between them can
            // only feed one of them.
            let mut homes: Vec<(u32, f32, f32)> = Vec::new();
            for c in self.list.iter() {
                if c.role == Role::Queen && !homes.iter().any(|(id, _, _)| *id == c.species) {
                    homes.push((c.species, c.base_x, c.base_y));
                }
            }
            // The ground each colony holds.
            let mut claims: Vec<(u32, f32, f32, f32)> = Vec::new();
            for (id, hx, hy) in homes.iter() {
                if let Some((cx, cy, radius)) = self.claim_of(*id) {
                    claims.push((*id, cx, cy, radius));
                }
            }
            // How many animals of each species are standing on the ground
            // another one holds. A colony that is trespassed on takes it
            // personally.
            let mut trespass: Vec<(u32, u32, u32)> = Vec::new();
            for (owner, x, y, radius) in claims {
                for c in self.list.iter() {
                    if c.species == owner {
                        continue;
                    }
                    if (c.x - x).powi(2) + (c.y - y).powi(2) > radius * radius {
                        continue;
                    }
                    if let Some(e) = trespass
                        .iter_mut()
                        .find(|(o, v, _)| *o == owner && *v == c.species)
                    {
                        e.2 += 1;
                    } else {
                        trespass.push((owner, c.species, 1));
                    }
                }
            }
            // Wars that neither side can afford any more simply stop. The
            // weariness climbs with the length of the war, so a long war
            // ends in a tired truce rather than a bloodbath.
            let mut exhausted: Vec<(u32, u32)> = Vec::new();
            for r in self.diplomacy.relations.iter_mut() {
                if r.state == RelationState::War {
                    // The longer the war goes on, the less either side wants
                    // it. A truce comes after a few thousand ticks of it.
                    r.war_weariness = (r.war_weariness + 0.03).min(1.0);
                    r.trust = (r.trust - 0.15).max(-100.0);
                    if r.war_weariness >= 0.85 {
                        exhausted.push((r.a, r.b));
                    }
                } else {
                    let anchor = if r.has(Treaty::Alliance) {
                        45.0
                    } else if r.has(Treaty::TradePact) {
                        20.0
                    } else if r.has(Treaty::Ceasefire) {
                        10.0
                    } else if r.has(Treaty::NonAggression) {
                        0.0
                    } else {
                        0.0
                    };
                    r.trust += (anchor - r.trust) * 0.02;
                    // Trespassing: an animal of one species on the ground of
                    // another is an insult that is remembered.
                    for (owner, intruder, count) in trespass.iter() {
                        let pair = (*owner == r.a && *intruder == r.b)
                            || (*owner == r.b && *intruder == r.a);
                        if pair && !r.has(Treaty::Alliance) {
                            r.trust -= (*count as f32).min(6.0) * 0.12;
                        }
                    }
                    // Neighbours: the closer two colonies live, the less they
                    // like each other, unless they have signed a treaty.
                    if !r.has(Treaty::Alliance) && !r.has(Treaty::NonAggression) {
                        let home_a = homes.iter().find(|(id, _, _)| *id == r.a);
                        let home_b = homes.iter().find(|(id, _, _)| *id == r.b);
                        if let (Some((_, ax, ay)), Some((_, bx, by))) = (home_a, home_b) {
                            let d = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
                            if d < 55.0 {
                                r.trust -= (55.0 - d) * 0.06;
                            }
                        }
                    }
                    r.trust = r.trust.clamp(-100.0, 100.0);
                }
            }

            // A war that has gone on too long ends in a tired truce. The
            // colony that is losing takes it as a defeat and the winner
            // takes it as a win, but nobody is wiped out.
            for (a, b) in exhausted {
                if self.diplomacy.get(a, b).map(|r| r.state) != Some(RelationState::War) {
                    continue;
                }
                let power_a = self.species_power(a);
                let power_b = self.species_power(b);
                let winner = if power_a > power_b { a } else { b };
                let loser = if winner == a { b } else { a };
                self.break_treaty(a, b, Treaty::Alliance, self.tick);
                self.sign_treaty(a, b, Treaty::Ceasefire, self.tick);
                {
                    let rel = self.diplomacy.get_mut(a, b);
                    rel.state = RelationState::Cordial;
                    rel.war_weariness = 0.0;
                    rel.trust = (rel.trust + 8.0).clamp(-100.0, 100.0);
                }
                self.diplomacy.stats.truces += 1;
                let name_a = self.species_name(a);
                let name_b = self.species_name(b);
                self.diplomacy.log_event(
                    format!("{} and {} lay down arms, worn out by the war", name_a, name_b),
                    self.species_color(winner),
                    self.tick,
                );
                // The side that lost has to pay for the ruin.
                self.adjust_trust(loser, winner, -12.0);
            }
        }

        // Recount the treaty statistics.
        if self.tick % 30 == 0 {
            let mut alliances = 0u32;
            let mut wars = 0u32;
            let mut treaties = 0u32;
            for r in self.diplomacy.relations.iter() {
                if r.state == RelationState::Allied {
                    alliances += 1;
                }
                if r.state == RelationState::War {
                    wars += 1;
                }
                treaties += r.treaties.len() as u32;
            }
            self.diplomacy.stats.alliances = alliances;
            self.diplomacy.stats.wars = wars;
            self.diplomacy.stats.treaties = treaties;
            self.diplomacy.stats.envoys =
                self.list.iter().filter(|c| c.role == Role::Envoy).count() as u32;
        }
    }
}
