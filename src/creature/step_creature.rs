use super::{CreatureSystem, Creature, World, SpatialGrid, Stage, Bomb, Faction, Culture, FearTable, Diplomacy, DiplomaticAction, Structure, SimEvent, Projectile, Rng, discovery_bits, trait_bits, Disease, Plague, Discovery, Mood, structure_kind, STAMINA_REGEN, MANA_REGEN, Elem, grow_and_learn, diplomatic_sense, zorg_carried_disease, DiplomaticSense};

use crate::world::LandmarkBonus;

/// The lessons a mind takes from one tick.
pub(super) type Lessons = [(usize, f32); 6];

/// The world and the neighbours, as they are this tick.
pub(super) struct StepCtx<'a> {
    pub world: &'a mut World,
    pub grid: &'a SpatialGrid,
    pub agents: &'a [(u32, f32, f32, u32, u8, f32)],
    pub lives: &'a [(Stage, u16)],
    pub centroids: &'a [(u32, f32, f32)],
    pub species_counts: &'a [(u32, u32)],
    pub structures_by_species: &'a [Vec<usize>],
    pub fire_lit: &'a [bool],
    pub fear: &'a FearTable,
    pub diplomacy: &'a mut Diplomacy,
    pub discoveries: &'a [u32],
    pub day_night: f32,
    pub temperature: f32,
    pub tick: u64,
    /// Whether the minds are allowed to learn.
    pub learn: bool,
}

/// What the colony looks like from the outside.
pub(super) struct View {
    pub agents: Vec<(u32, f32, f32, u32, u8, f32)>,
    pub lives: Vec<(Stage, u16)>,
    pub species_counts: Vec<(u32, u32)>,
    pub centroids: Vec<(u32, f32, f32)>,
    pub discoveries: Vec<u32>,
    pub fear: FearTable,
}

/// The colony as the creatures see it.
pub(super) fn build_view(sys: &CreatureSystem) -> View {
    let agents: Vec<(u32, f32, f32, u32, u8, f32)> = sys
        .list
        .iter()
        .map(|c| (c.id, c.x, c.y, c.species, c.role as u8, c.health))
        .collect();

    let lives: Vec<(Stage, u16)> = sys.list.iter().map(|c| (c.stage, c.culture)).collect();

    let mut species_counts: Vec<(u32, u32)> = Vec::new();
    for c in &sys.list {
        match species_counts.iter_mut().find(|(id, _)| *id == c.species) {
            Some(e) => e.1 += 1,
            None => species_counts.push((c.species, 1)),
        }
    }

    let mut centroids: Vec<(u32, f32, f32)> = Vec::new();
    {
        let mut acc: Vec<(u32, f32, f32, u32)> = Vec::new();
        for c in &sys.list {
            match acc.iter_mut().find(|(id, _, _, _)| *id == c.species) {
                Some(e) => {
                    e.1 += c.x;
                    e.2 += c.y;
                    e.3 += 1;
                }
                None => acc.push((c.species, c.x, c.y, 1)),
            }
        }
        for (id, sx, sy, n) in acc {
            centroids.push((id, sx / n as f32, sy / n as f32));
        }
    }

    let mut discoveries: Vec<u32> = vec![0; sys.next_species as usize + 8];
    for (sp, d) in &sys.discoveries {
        if (*sp as usize) < discoveries.len() {
            discoveries[*sp as usize] = *d;
        }
    }

    let fear = FearTable::build(sys.predation.entries(), sys.next_species as usize + 8);

    View {
        agents,
        lives,
        species_counts,
        centroids,
        discoveries,
        fear,
    }
}

/// The buffers and the counters one tick writes into.
pub(super) struct Tick {
    pub spell_targets: Vec<(usize, usize, f32)>,
    pub disease_targets: Vec<usize>,
    pub heal_targets: Vec<(u32, f32, f32)>,
    pub proposals: Vec<(usize, usize, DiplomaticAction)>,
    pub learned: Vec<(usize, Culture)>,
    pub virus_spawns: Vec<(i32, i32)>,
    pub shots: Vec<Projectile>,
    pub births: Vec<(f32, f32, Creature)>,
    pub attackers: Vec<usize>,
    pub dead: Vec<usize>,
    pub diplo_willing: Vec<f32>,
    pub mate_willing: Vec<bool>,
    pub bombs: Vec<Bomb>,
    pub input_buf: Vec<f32>,
    pub net_act: Vec<f32>,
    pub net_next: Vec<f32>,
    pub structures_total: u32,
    pub mounts_total: u32,
    pub spells_cast_total: u32,
    pub shots_fired: u32,
    pub traded_total: u64,
    pub pop_now: usize,
}

/// The buffers and the counters a step writes into.
pub(super) struct StepOut<'a> {
    pub rng: &'a mut Rng,
    pub events: &'a mut Vec<SimEvent>,
    pub structures: &'a mut Vec<Structure>,
    pub activity: &'a mut Vec<(u32, u32)>,
    pub deepest: &'a mut Vec<(u32, u32)>,
    pub plagues_started: &'a mut u32,
    pub tick: Tick,
}

/// What one creature has done this tick.
#[derive(Default)]
pub(super) struct Turn {
    pub flags: u32,
    pub lifespan: u32,
    pub ds: DiplomaticSense,
    pub kin_near: u32,
    pub company: f32,
    pub crowd: u32,
    pub crowding: f32,
    pub place: LandmarkBonus,
    pub hardship: f32,
    pub debility: f32,
    pub discoveries: u32,
    pub crisis: f32,
    pub daylight: f32,
    pub child_count: u32,
    pub child_value: f32,
}

impl Turn {
    /// The creature has been hurt.
    pub(super) const HURT: u32 = 1 << 0;
    /// The creature has killed.
    pub(super) const KILLED: u32 = 1 << 1;
    /// The creature has eaten.
    pub(super) const ATE: u32 = 1 << 2;
    /// The creature has built.
    pub(super) const BUILT: u32 = 1 << 3;
    /// The creature has attacked.
    pub(super) const ATTACKED: u32 = 1 << 4;
    /// The creature has cast a spell.
    pub(super) const CAST: u32 = 1 << 5;
    /// The creature has traded.
    pub(super) const TRADED: u32 = 1 << 6;
    /// The creature has died.
    pub(super) const DIED: u32 = 1 << 7;
    /// The creature wants a child.
    pub(super) const WANTS_CHILD: u32 = 1 << 8;
    /// The creature stands beside a market.
    pub(super) const NEAR_MARKET: u32 = 1 << 9;
    /// The creature stands beside a workshop.
    pub(super) const NEAR_WORKSHOP: u32 = 1 << 10;

    /// Has the creature done this?
    pub(super) fn has(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }

    pub(super) fn new() -> Self {
        Turn {
            child_count: 1,
            child_value: 1.0,
            ..Turn::default()
        }
    }
}

/// The first of the tick: the creature grows, counts its neighbours and
/// may catch the plague.
pub(super) fn wake(c: &mut Creature, ctx: &StepCtx<'_>, out: &mut StepOut<'_>, turn: &mut Turn) {
    c.age += 1;

    if c.health < c.health_seen - 2.0 {
        turn.flags |= Turn::HURT;
    }
    if c.kills > c.kills_seen {
        turn.flags |= Turn::KILLED;
    }

    turn.lifespan = c.lifespan();
    if ctx.learn {
        grow_and_learn(c, ctx.grid, ctx.agents, ctx.lives);
    }

    turn.ds = diplomatic_sense(&*ctx.diplomacy, c, ctx.centroids, ctx.species_counts);

    let mut kin_near = 0u32;
    ctx.grid.near(c.x, c.y, 30, |idx| {
        let a = &ctx.agents[idx as usize];
        if a.0 != c.id && a.3 == c.species {
            kin_near += 1;
        }
    });
    turn.kin_near = kin_near;
    turn.company = if c.traits & trait_bits::GREGARIOUS != 0 {
        1.0 + (kin_near as f32 / 6.0).min(1.0) * 0.5
    } else {
        1.0
    };

    let mut crowd = 0u32;
    ctx.grid.near(c.x, c.y, 4, |idx| {
        let a = &ctx.agents[idx as usize];
        if a.0 == c.id {
            return;
        }
        let dx = a.1 - c.x;
        let dy = a.2 - c.y;
        if dx * dx + dy * dy <= 16.0 {
            crowd += 1;
        }
    });
    turn.crowd = crowd;
    turn.crowding = 1.0 + 0.08 * crowd.min(16) as f32;

    if crowd >= 10 && c.disease == Disease::None {
        let risk = 0.00003 * (crowd - 9) as f32;
        if out.rng.chance(risk) {
            c.disease = Disease::Plague;
            c.disease_timer = Plague::from(Disease::Plague).duration();
            *out.plagues_started += 1;

            out.tick.virus_spawns.push((c.x.round() as i32, c.y.round() as i32));
        }
    }
}

/// The body: hunger, thirst, the cold and the ground it stands on.
pub(super) fn vitals(c: &mut Creature, ctx: &StepCtx<'_>, turn: &mut Turn) {
    let place = ctx.world.landmark_bonus(c.x, c.y);
    turn.place = place;

    let hardy = if c.traits & trait_bits::HARDY != 0 { 0.7 } else { 1.0 };

    let stores = if c.knows_tech(Discovery::Pottery) { 0.85 } else { 1.0 };

    let hardship = ctx.world.hardship();
    turn.hardship = hardship;

    let debility = if c.disease == Disease::None {
        0.0
    } else {
        Plague::from(c.disease).debility()
    };
    turn.debility = debility;

    let hunger_rate = 0.06 * hardy * stores * (0.8 + 0.55 * hardship) * (1.0 + 0.45 * debility);
    c.hunger = (c.hunger + hunger_rate).clamp(0.0, 100.0);

    if turn.crisis > 0.0 {
        c.energy = (c.energy - turn.crisis).max(0.0);
        c.mood = Mood::Depressed;
    }

    if place.has(crate::world::Landmark::FertileSoil) {
        c.hunger = (c.hunger - 0.035).clamp(0.0, 100.0);
    }

    if place.has(crate::world::Landmark::SaltFlat) {
        c.immunity[Disease::Plague as usize] = c.immunity[Disease::Plague as usize].max(1);
        c.immunity[Disease::Infection as usize] = c.immunity[Disease::Infection as usize].max(1);
    }

    let discoveries = ctx.discoveries.get(c.species as usize).copied().unwrap_or(0);
    turn.discoveries = discoveries;
    if discoveries & discovery_bits::HUSBANDRY != 0 {
        c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
    }

    let thirst_rate =
        0.045 * hardy / ctx.world.weather.thirst_factor() * (0.25 + 0.75 * hardship);
    c.thirst = (c.thirst + thirst_rate).clamp(0.0, 100.0);

    if c.faction == Faction::Zorg {
        c.thirst = 0.0;
    }

    if c.stage == Stage::Juvenile {
        c.stamina = (c.stamina - 0.02 * (1.0 + 0.7 * debility)).clamp(0.0, c.max_stamina);
        c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
    } else {
        c.stamina = (c.stamina - 0.05 * (1.0 + 0.7 * debility)).clamp(0.0, c.max_stamina);
    }

    if c.stage == Stage::Elder {
        c.stamina = (c.stamina - 0.03).clamp(0.0, c.max_stamina);
    }
}

/// The buildings of the colony: what they give the creature that
/// stands beside them.
pub(super) fn surroundings(c: &mut Creature, ctx: &StepCtx<'_>, out: &StepOut<'_>, turn: &mut Turn) {
    let mut morale = false;
    let mut near_market = false;
    let mut near_workshop = false;
    let mut near_granary = false;

    let sidx = (c.species as usize).min(ctx.structures_by_species.len() - 1);
    for &si in &ctx.structures_by_species[sidx] {
        let s = &out.structures[si];
        let d2 = (s.x - c.x).powi(2) + (s.y - c.y).powi(2);
        match s.kind {
            structure_kind::TEMPLE => {
                if d2 < 625.0 {
                    morale = true;
                }
            }
            structure_kind::MARKET => {
                if d2 < 900.0 {
                    near_market = true;
                }
            }
            structure_kind::WORKSHOP => {
                if d2 < 900.0 {
                    near_workshop = true;
                }
            }
            structure_kind::GRANARY => {
                if d2 < 900.0 {
                    near_granary = true;
                }
            }
            structure_kind::BEACON if d2 < 2500.0 => {}
            _ => {}
        }
    }
    if near_market {
        turn.flags |= Turn::NEAR_MARKET;
    }
    if near_workshop {
        turn.flags |= Turn::NEAR_WORKSHOP;
    }

    if morale {
        c.hunger = (c.hunger - 0.03).clamp(0.0, 100.0);
    }

    if near_granary {
        c.hunger = (c.hunger - 0.05).clamp(0.0, 100.0);
    }

    if turn.ds.alliances > 0.0 {
        c.health = (c.health + 0.04).min(100.0);
        c.stamina = (c.stamina + 0.01).min(c.max_stamina);
    }
}

/// The air it breathes and the fire it sits beside.
pub(super) fn breath_and_warmth(c: &mut Creature, ctx: &StepCtx<'_>, turn: &mut Turn) {
    let place = turn.place;
    let hardship = turn.hardship;
    let debility = turn.debility;

    let mut air = ctx.world.fresh_air(c.x.round() as i32, c.y.round() as i32, 2);

    if place.has(crate::world::Landmark::Undergrove) {
        air = 1.0;
    }

    let air_factor = 0.7 + 0.6 * air;
    let breath = 0.05 * (1.0 - air);
    if c.resting {
        let heal = (1.0 - 0.4 * debility).max(0.2);
        c.stamina = (c.stamina + STAMINA_REGEN * air_factor * heal).clamp(0.0, c.max_stamina);

        c.energy = (c.energy + (c.max_energy - c.energy) * 0.01 - breath).min(c.max_energy);
    } else {
        c.energy -= 0.08
            * ctx.world.weather.energy_factor()
            * (1.4 - 0.4 * air)
            * (0.55 + 0.45 * hardship)
            * turn.crowding
            + breath;
    }

    if c.bond.is_some() {
        c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
        c.energy = (c.energy + 0.01).min(c.max_energy);
    }

    let cap = c.max_health();
    if c.health < cap {
        let mut mend = 0.05;
        if c.knows_tech(Discovery::Surgery) {
            mend *= 1.4;
        }
        if c.knows_tech(Discovery::Alchemy) {
            mend *= 1.3;
        }
        c.health = (c.health + mend).min(cap);
    }

    if c.faction == Faction::Zorg
        && ctx.world.in_zorg_cave(c.x.round() as i32, c.y.round() as i32)
    {
        c.health = (c.health + 0.08).min(cap);
        c.stamina = (c.stamina + 0.05).min(c.max_stamina);
    }
}

/// The hour of the day, the cold and the light of the fire.
pub(super) fn day_and_fire(c: &mut Creature, ctx: &StepCtx<'_>, turn: &mut Turn) {
    let place = turn.place;
    let hardship = turn.hardship;

    let crepuscular = c.traits & trait_bits::CREPUSCULAR != 0;
    let hour = ctx.day_night * 24.0;

    let daylight = if crepuscular {
        let dawn = (hour - 6.0).abs().min(12.0);
        let dusk = (hour - 18.0).abs().min(12.0);
        1.0 - (dawn.min(dusk) / 6.0).min(1.0) * 0.6
    } else {
        1.0
    };
    turn.daylight = daylight;

    if c.traits & trait_bits::PROVISIONER != 0 && turn.kin_near > 0 {
        c.hunger = (c.hunger - 0.02).clamp(0.0, 100.0);
    }

    let fire_warmth = ctx
        .fire_lit
        .get((c.species as usize).min(ctx.fire_lit.len().saturating_sub(1)))
        .copied()
        .unwrap_or(false)
        && (c.x - c.base_x).abs() < 14.0
        && (c.y - c.base_y).abs() < 14.0;
    if ctx.temperature < 4.0 && !place.has(crate::world::Landmark::HotSpring) && !fire_warmth {
        let chill = (4.0 - ctx.temperature) * c.affinity.cold_vulnerability();
        c.health -= chill * 0.012 * (0.3 + 0.7 * hardship);
    }
    if fire_warmth {
        c.stamina = (c.stamina + 0.015).min(c.max_stamina);
    }

    let (fx, fy) = (c.x.round() as i32, c.y.round() as i32);
    if !c.affinity.fire_immune() {
        let here = ctx.world.get(fx, fy);
        if here == Elem::Fire || here == Elem::Lava || here == Elem::Ember {
            c.health -= 0.6;
        }
    }
    c.mana = (c.mana + MANA_REGEN * c.affinity.mana_factor() * place.mana).clamp(0.0, c.max_mana);

    if place.has(crate::world::Landmark::Stormspire) {
        c.mana = (c.mana + MANA_REGEN * 1.2).clamp(0.0, c.max_mana);
        c.stamina = (c.stamina + STAMINA_REGEN * 1.5).min(c.max_stamina);
    }

    if turn.discoveries & discovery_bits::SANCTUARY != 0 {
        c.mana = (c.mana + MANA_REGEN * 0.8).clamp(0.0, c.max_mana);
    }

    if c.knows_tech(Discovery::Radiance) {
        c.mana = (c.mana + MANA_REGEN * 0.6).clamp(0.0, c.max_mana);
    }
}

/// The cooldowns, the sickness and the mood.
pub(super) fn cooldowns_and_mood(c: &mut Creature, out: &mut StepOut<'_>) {
    if c.cooldown > 0 {
        c.cooldown -= 1;
    }
    if c.fire_cooldown > 0 {
        c.fire_cooldown -= 1;
    }
    if c.build_cooldown > 0 {
        c.build_cooldown -= 1;
    }
    if c.craft_cooldown > 0 {
        c.craft_cooldown -= 1;
    }
    if c.spell_cooldown > 0 {
        c.spell_cooldown -= 1;
    }
    if c.diplo_cooldown > 0 {
        c.diplo_cooldown -= 1;
    }
    for cd in &mut c.spell_cd {
        if *cd > 0 {
            *cd -= 1;
        }
    }
    if c.disease_timer > 0 {
        c.disease_timer -= 1;
    } else {
        c.disease = Disease::None;
    }

    if c.faction == Faction::Zorg {
        if c.disease == Disease::None {
            c.disease = zorg_carried_disease(out.rng.range(5));
        }
        c.disease_timer = Plague::from(c.disease).duration();
    }

    if c.mood_timer == 0 {
        c.mood = match out.rng.range(10) {
            0 => Mood::Happy,
            1 => Mood::Angry,
            2 => Mood::Fearful,
            3 => Mood::Curious,
            4 => Mood::Loyal,
            5 => Mood::Rebellious,
            6 => Mood::Depressed,
            7 => Mood::Zealous,
            _ => Mood::Neutral,
        };
        c.mood_timer = (out.rng.range(120) + 60) as u16;
    } else {
        c.mood_timer -= 1;
    }
}

