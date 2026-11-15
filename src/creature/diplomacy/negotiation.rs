use super::{CreatureSystem, Tool, Role, RelationState, Mood, Treaty, ZORG_SPECIES, species_name_for, Creature, SpatialGrid, diplomacy_reach, Diplomacy, DiplomaticAction, outputs, SimEvent, negotiation_rate, DIPLO_COST, trait_bits};

impl CreatureSystem {

    pub fn prune_diplomacy(&mut self) {
        let alive: Vec<u32> = self.list.iter().map(|c| c.species).collect();
        self.diplomacy
            .relations
            .retain(|r| alive.contains(&r.a) && alive.contains(&r.b));
    }

    #[must_use]
    pub fn species_population(&self, species: u32) -> u32 {
        self.list.iter().filter(|c| c.species == species).count() as u32
    }

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

    pub fn declare_war(&mut self, a: u32, b: u32, reason: &str, tick: u64) {
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

        for c in &mut self.list {
            if c.species == a || c.species == b {
                c.mood = Mood::Angry;
                c.mood_timer = 600;
            }
        }
    }

    #[must_use]
    pub fn sign_treaty(&mut self, a: u32, b: u32, treaty: Treaty, tick: u64) -> bool {

        if a == ZORG_SPECIES || b == ZORG_SPECIES {
            return false;
        }
        let color_a = self.species_color(a);

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

    pub fn break_treaty(&mut self, a: u32, b: u32, treaty: Treaty, tick: u64) {
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

    #[must_use]
    pub fn species_color(&self, species: u32) -> [u8; 3] {
        self.list
            .iter()
            .find(|c| c.species == species)
            .map_or([200, 200, 200], |c| c.team_color)
    }

    #[must_use]
    pub fn species_name(&self, species: u32) -> String {
        self.list
            .iter()
            .find(|c| c.species == species)
            .map_or_else(|| format!("species {}", species), |c| {
                let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
                String::from_utf8_lossy(&c.name[..end]).to_string()
            })
    }

    #[must_use]
    pub fn lineage_name(&self, species: u32) -> String {
        species_name_for(species)
    }
}

#[must_use]
pub fn find_diplomatic_partner(
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
        if d2 <= reach2 && best.is_none_or(|(_, bd)| d2 < bd) {
            best = Some((idx as usize, d2));
        }
    });
    best.map(|(i, _)| i)
}

#[must_use]
pub fn desired_action(
    diplomacy: &Diplomacy,
    out: &[f32],
    own_species: u32,
    partner_species: u32,
) -> DiplomaticAction {

    if own_species == ZORG_SPECIES || partner_species == ZORG_SPECIES {
        return DiplomaticAction::War;
    }
    let at_war = diplomacy.at_war(own_species, partner_species);
    let allied = diplomacy.are_allied(own_species, partner_species);
    let trust = diplomacy.trust(own_species, partner_species);
    let hostile = out[outputs::HOSTILE];
    let ally = out[outputs::ALLY];

    if at_war {
        return DiplomaticAction::Peace;
    }
    if !allied {

        if ally > 0.42 && hostile < 0.62 && trust > -25.0 {
            return DiplomaticAction::Alliance;
        }

        if hostile > 0.55 && hostile >= ally {
            return DiplomaticAction::War;
        }
        if trust < -45.0 && hostile > 0.15 {
            return DiplomaticAction::War;
        }

        if out[outputs::TRIBUTE] > 0.45
            && !diplomacy.has_treaty(own_species, partner_species, Treaty::Tributary)
        {
            return DiplomaticAction::Tribute;
        }
    }

    if diplomacy.has_treaty(own_species, partner_species, Treaty::TradePact)
        && diplomacy.has_treaty(own_species, partner_species, Treaty::Ceasefire)
        && diplomacy.has_treaty(own_species, partner_species, Treaty::NonAggression)
    {
        return DiplomaticAction::None;
    }
    DiplomaticAction::Trade
}

impl CreatureSystem {

    pub fn conclude_negotiations(
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

    pub fn advance_negotiation(
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

            let cost = DIPLO_COST * (1.0 / rate.max(0.25)).min(4.0);
            self.list[index].energy = (self.list[index].energy - cost).max(0.0);
            self.list[index].diplo_progress >= 1.0
        };
        if !ready {
            self.list[index].diplo_with = Some(partner_id);
            self.list[index].diplo_partner = Some(partner_species);
            return None;
        }

        self.list[index].diplo_progress = 0.0;
        self.list[index].diplo_with = None;
        self.list[index].diplo_partner = None;
        self.list[index].diplo_cooldown = if role == Role::Envoy { 200 } else { 600 };

        self.list[partner_index].diplo_progress = 0.0;
        self.list[partner_index].diplo_with = None;
        self.list[partner_index].diplo_partner = None;

        if !partner_willing {

            self.adjust_trust(my_species, partner_species, -3.0);
            return None;
        }
        self.list[index].pacts_signed += 1;
        Some(self.resolve_negotiation(index, partner_index, action))
    }

    pub fn resolve_negotiation(
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
            .map_or(RelationState::Neutral, |r| r.state);
        let actor_rep = self.list[actor].reputation as f32;
        let partner_rep = self.list[partner].reputation as f32;
        let actor_peaceful = self.list[actor].traits & trait_bits::PEACEFUL != 0;
        let partner_peaceful = self.list[partner].traits & trait_bits::PEACEFUL != 0;

        let mut chance: f32;
        match action {
            DiplomaticAction::Trade => {
                chance = 0.25 + trust / 260.0 + state.bias() * 0.3;
            }
            DiplomaticAction::Alliance => {

                let power_ratio = power_a / power_b.max(0.001);
                let balance = 1.0 - (power_ratio - 1.0).abs().min(1.0);
                chance = 0.08 + trust / 130.0 + balance * 0.30 + state.bias() * 0.25;
            }
            DiplomaticAction::Peace => {

                let weariness = self
                    .diplomacy
                    .get(sp_a, sp_b)
                    .map_or(0.0, |r| r.war_weariness);
                chance = 0.08 + weariness * 0.45 + trust / 300.0;
            }
            DiplomaticAction::Tribute => {

                let ratio = power_a / power_b.max(0.001);
                if ratio < 1.5 {
                    chance = -1.0;
                } else {
                    chance = 0.05 + (ratio - 1.5).min(2.0) * 0.18 - trust / 200.0;
                }
            }
            DiplomaticAction::War => {

                chance = 0.35 + (1.0 - trust / 100.0) * 0.4;
            }
            DiplomaticAction::None => chance = 0.5,
        }

        if partner_peaceful && action != DiplomaticAction::War {
            chance += 0.12;
        }
        if actor_peaceful && action != DiplomaticAction::War {
            chance += 0.06;
        }

        chance += (actor_rep - partner_rep) / 400.0;
        chance = chance.clamp(0.02, 0.95);

        if action == DiplomaticAction::Peace && power_b < power_a * 0.5 {
            chance += 0.25;
        }

        let accepted = self.rng.chance(chance);

        match action {
            DiplomaticAction::Trade => {
                if accepted {

                    let _ = self.sign_treaty(sp_a, sp_b, Treaty::TradePact, tick);
                } else {
                    self.adjust_trust(sp_a, sp_b, -4.0);
                }
            }
            DiplomaticAction::Alliance => {
                if accepted {
                    if self.sign_treaty(sp_a, sp_b, Treaty::Alliance, tick) {
                        let _ = self.sign_treaty(sp_a, sp_b, Treaty::NonAggression, tick);
                        let _ = self.sign_treaty(sp_a, sp_b, Treaty::TradePact, tick);
                        self.diplomacy.stats.alliances += 1;
                    }
                } else {
                    self.adjust_trust(sp_a, sp_b, -8.0);
                }
            }
            DiplomaticAction::Peace => {
                if accepted {
                    self.break_treaty(sp_a, sp_b, Treaty::Alliance, tick);
                    let _ = self.sign_treaty(sp_a, sp_b, Treaty::Ceasefire, tick);
                    let _ = self.sign_treaty(sp_a, sp_b, Treaty::NonAggression, tick);
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
                    let _ = self.sign_treaty(sp_a, sp_b, Treaty::Tributary, tick);
                    self.diplomacy.stats.tributes += 1;
                } else {
                    self.adjust_trust(sp_a, sp_b, -20.0);

                    let hostile = self.diplomacy.trust(sp_a, sp_b) < -40.0;
                    if hostile {
                        self.declare_war(sp_a, sp_b, "tribute refused", tick);
                    }
                }
            }
            DiplomaticAction::War => {
                self.declare_war(sp_a, sp_b, "envoys expelled", tick);

                self.adjust_trust(sp_a, sp_b, -30.0);
            }
            DiplomaticAction::None => {
                if accepted {

                    self.adjust_trust(sp_a, sp_b, 3.0);
                }
            }
        }

        self.diplomacy.stats.negotiations += 1;
        (action, accepted)
    }

    pub fn adjust_trust(&mut self, a: u32, b: u32, delta: f32) {
        let rel = self.diplomacy.get_mut(a, b);
        rel.trust = (rel.trust + delta).clamp(-100.0, 100.0);
    }

    pub fn tick_diplomacy(&mut self) {

        if self.tick.is_multiple_of(120) {
            self.prune_diplomacy();
        }

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
                for c in &mut self.list {
                    if c.species == vassal && c.energy > 120.0 {
                        c.energy -= amount;
                        paid = true;
                        break;
                    }
                }
                if paid {
                    for c in &mut self.list {
                        if c.species == overlord {
                            c.energy = (c.energy + amount).min(c.max_energy);
                            break;
                        }
                    }
                    let rel = self.diplomacy.get_mut(a, b);
                    rel.tribute_paid += amount;
                    self.diplomacy.stats.tributes += 1;
                } else {

                    self.adjust_trust(a, b, -3.0);
                }
            }
        }

        if self.tick.is_multiple_of(60) {

            let mut homes: Vec<(u32, f32, f32)> = Vec::new();
            for c in &self.list {
                if c.role == Role::Queen && !homes.iter().any(|(id, _, _)| *id == c.species) {
                    homes.push((c.species, c.base_x, c.base_y));
                }
            }

            let mut claims: Vec<(u32, f32, f32, f32)> = Vec::new();
            for (id, _hx, _hy) in &homes {
                if let Some((cx, cy, radius)) = self.claim_of(*id) {
                    claims.push((*id, cx, cy, radius));
                }
            }

            let mut trespass: Vec<(u32, u32, u32)> = Vec::new();
            for (owner, x, y, radius) in claims {
                for c in &self.list {
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

            let mut exhausted: Vec<(u32, u32)> = Vec::new();
            for r in &mut self.diplomacy.relations {
                if r.state == RelationState::War {

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

                    for (owner, intruder, count) in &trespass {
                        let pair = (*owner == r.a && *intruder == r.b)
                            || (*owner == r.b && *intruder == r.a);
                        if pair && !r.has(Treaty::Alliance) {
                            r.trust -= (*count as f32).min(6.0) * 0.12;
                        }
                    }

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

            for (a, b) in exhausted {
                if self.diplomacy.get(a, b).map(|r| r.state) != Some(RelationState::War) {
                    continue;
                }
                let power_a = self.species_power(a);
                let power_b = self.species_power(b);
                let winner = if power_a > power_b { a } else { b };
                let loser = if winner == a { b } else { a };
                self.break_treaty(a, b, Treaty::Alliance, self.tick);
                let _ = self.sign_treaty(a, b, Treaty::Ceasefire, self.tick);
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

                self.adjust_trust(loser, winner, -12.0);
            }
        }

        if self.tick.is_multiple_of(30) {
            let mut alliances = 0u32;
            let mut wars = 0u32;
            let mut treaties = 0u32;
            for r in &self.diplomacy.relations {
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
