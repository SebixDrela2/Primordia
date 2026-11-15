use super::{CreatureSystem, Rng, CreatureStats, PredationMemory, Diplomacy, World, Role, in_crisis, Elem, ZORG_START, RevivalGenome, Faction, MAX_POP, CreatureGenesis, ZORG_MIN, ZORG_POP_CAP, discovery_bits, culture_bits, gene_distance, Creature, Personality, Affinity, decode_traits, Model};

impl CreatureSystem {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        CreatureSystem {
            list: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5_1234_9E37_79B9),
            next_id: 1,

            // The minds learn by default. The run turns it off, so
            // that what is seen is the doing of the weights that were
            // given and not of a mind that is still learning.
            learn: true,

            next_species: 0,

            fire_lit: vec![false; 64],
            tick: 0,
            stats: CreatureStats::default(),
            births_total: 0,
            deaths_total: 0,
            eats_total: 0,
            eats_by_elem: [0; 128],
            builds_total: 0,
            trades_total: 0,
            spells_cast: 0,
            matings_total: 0,
            two_parent_births: 0,
            next_squad: 1,
            day_night: 0.25,
            temperature: 20.0,
            season: 0,
            season_timer: 0,
            projectiles: Vec::new(),
            structures: Vec::new(),
            claims: Vec::new(),
            predation: PredationMemory::default(),
            succession: Vec::new(),
            events: Vec::new(),
            bombs: Vec::new(),
            diplomacy: Diplomacy::default(),
            culture_pool: Vec::new(),
            migrations: Vec::new(),
            splits_total: 0,
            migrations_started: 0,
            plagues_started: 0,
            discoveries: Vec::new(),
            deepest: Vec::new(),
            activity: Vec::new(),

            last_discovery: None,
        }
    }

    #[must_use]
    pub fn species_culture(&self, species: u32) -> u16 {
        self.culture_pool
            .iter()
            .find(|(id, _)| *id == species)
            .map_or(0, |(_, c)| *c)
    }

    pub fn note_culture(&mut self, species: u32, bit: u16) {
        if let Some(entry) = self.culture_pool.iter_mut().find(|(id, _)| *id == species) {
            entry.1 |= bit;
        } else {
            self.culture_pool.push((species, bit));
        }
    }

    pub(super) fn find_landmark_spot(&mut self, world: &World, claimer: u32) -> Option<(f32, f32)> {
        if !self.rng.chance(0.6) {
            return None;
        }
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(1));
        let mut found = None;
        let mut taken: Vec<(u32, f32, f32)> = Vec::new();
        for l in &world.landmarks {

            let holder = self.list.iter().find(|c| {
                c.role == Role::Queen
                    && ((c.base_x - l.x).powi(2) + (c.base_y - l.y).powi(2)).sqrt()
                        < l.kind.radius() * 1.6
            });
            if let Some(h) = holder {
                if h.species != claimer {
                    taken.push((h.species, l.x, l.y));
                }
                continue;
            }

            for _ in 0..40 {
                let a = rng.f32() * std::f32::consts::TAU;
                let d = rng.f32() * l.kind.radius() * 0.9;
                let px = (l.x + a.cos() * d) as i32;
                let py = (l.y + a.sin() * d * 0.6) as i32;
                if world.get(px, py).is_passable() {
                    found = Some((px as f32, py as f32));
                    break;
                }
            }
        }
        self.rng = rng;

        if found.is_none() && !taken.is_empty() && self.rng.chance(0.35) {
            let i = self.rng.range(taken.len() as u32) as usize;
            let (holder, _, _) = taken[i];
            if !self.diplomacy.at_war(holder, claimer) {
                self.declare_war(holder, claimer, "disputed landmark", self.tick);
            }
        }
        found
    }

    #[must_use]
    pub fn claim_of(&self, species: u32) -> Option<(f32, f32, f32)> {
        self.claims
            .iter()
            .find(|(id, _, _, _)| *id == species)
            .map(|(_, cx, cy, r)| (*cx, *cy, *r))
    }

    pub fn rebuild_claims(&mut self) {
        self.claims.clear();
        let mut centres: Vec<(u32, f32, f32)> = Vec::new();
        let mut builders: Vec<(u32, u32)> = Vec::new();
        for c in &self.list {
            if c.role == Role::Queen {

                if in_crisis(&self.succession, c.species, self.tick) {
                    continue;
                }
                centres.push((c.species, c.base_x, c.base_y));
            }
            if c.role == Role::Queen || c.role == Role::Builder {
                if let Some(e) = builders.iter_mut().find(|(id, _)| *id == c.species) {
                    e.1 += 1;
                } else {
                    builders.push((c.species, 1));
                }
            }
        }
        for (id, cx, cy) in centres {
            let buildings = builders
                .iter()
                .find(|(bid, _)| *bid == id)
                .map_or(1, |(_, n)| *n);
            let radius = (18.0 + buildings as f32 * 4.0).min(46.0);
            self.claims.push((id, cx, cy, radius));
        }
    }

    pub fn spawn_energy_burst(&mut self, world: &mut World, x: i32, y: i32, count: u32, color: [u8; 3]) {
        let _ = color;
        for _ in 0..count {
            let ang = self.rng.f32() * std::f32::consts::TAU;
            let dist = 0.5 + self.rng.f32() * 2.0;
            let px = (x as f32 + ang.cos() * dist).round() as i32;
            let py = (y as f32 + ang.sin() * dist).round() as i32;
            if world.get(px, py) != Elem::Empty {
                continue;
            }

            world.set_xy(px, py, Elem::Smoke);
        }
    }

    pub fn spawn_impact_puff(&mut self, world: &mut World, x: i32, y: i32, count: u32) {
        for _ in 0..count {
            let px = x + self.rng.range_i(-1, 2);
            let py = y + self.rng.range_i(-1, 2);
            if world.get(px, py) != Elem::Empty { continue; }
            world.set_xy(px, py, Elem::Smoke);
        }
    }

    pub fn populate(&mut self, world: &World, count: usize) {
        for _ in 0..count {

            let _ = self.spawn_near_surface(world);
        }

        self.spawn_zorgs(world, ZORG_START);
        self.update_stats();
    }

    /// Give the colony the mind of a model: the same weights, the same
    /// genes and the same temperament for every one of them. The ZORGs
    /// are left alone: they are not the colony.
    pub fn give_the_mind(&mut self, model: &Model) {
        let net = model.to_net();
        for c in &mut self.list {
            if c.faction == Faction::Zorg {
                continue;
            }
            c.net = net.clone_net();
            c.genes = model.genes;
            c.traits = model.traits;
            c.culture = model.culture;
            c.size = model.size;
            c.personality = model.personality;
        }
    }

    #[must_use]
    pub fn champion_from(&self, species: u32) -> Option<RevivalGenome> {
        self.list
            .iter()
            .filter(|c| c.species == species)
            .max_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| RevivalGenome {
                net: c.net.clone_net(),
                color: c.color,
                genes: c.genes,
                size: c.size,
                generation: c.generation,

                culture: c.culture,
                personality: c.personality,
                fitness: c.fitness,
            })
    }

    #[must_use]
    pub fn champion_overall(&self) -> Option<RevivalGenome> {
        self.list
            .iter()
            .filter(|c| c.faction != Faction::Zorg)
            .max_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| RevivalGenome {
                net: c.net.clone_net(),
                color: c.color,
                genes: c.genes,
                size: c.size,
                generation: c.generation,
                culture: c.culture,
                personality: c.personality,
                fitness: c.fitness,
            })
    }

    #[must_use]
    pub fn pick_revival_genome(
        won: bool,
        champion: Option<RevivalGenome>,
        fallback: Option<RevivalGenome>,
    ) -> Option<RevivalGenome> {
        if won {
            champion
        } else {

            fallback.or(champion)
        }
    }

    pub fn populate_revival(&mut self, world: &World, count: usize, genome: &RevivalGenome) {
        for _ in 0..count {
            if self.list.len() >= MAX_POP {
                break;
            }
            let x = self.rng.range_i(3, world.w as i32 - 3);
            let surface = world.surface_y(x);
            let y = surface - 2;
            let (sx, sy) = world
                .find_empty_passable_near(x, y, 8)
                .unwrap_or((x, y));
            let before = self.list.len();
            let _ = self.spawn(world, sx as f32, sy as f32, &CreatureGenesis::Revival(genome));
            if self.list.len() > before {
                let last = self.list.len() - 1;
                self.list[last].culture = genome.culture;
            }
        }

        self.spawn_zorgs(world, ZORG_START);
        self.update_stats();
    }

    #[must_use]
    pub fn spawn_near_surface(&mut self, world: &World) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let x = self.rng.range_i(3, world.w as i32 - 3);
        let surface = world.surface_y(x);
        let y = surface - 2;

        let (sx, sy) = world
            .find_empty_passable_near(x, y, 8)
            .unwrap_or((x, y));
        self.spawn(world, sx as f32, sy as f32, &CreatureGenesis::Random)
    }

    pub(super) fn find_zorg_spot(&mut self, world: &World) -> Option<(f32, f32)> {

        if let Some((cx, cy)) = world.zorg_cave_center() {

            let (cx, cy) = if self.rng.chance(0.35) {
                (cx, cy)
            } else {
                let jitter_x = self.rng.range_i(-10, 11);
                let jitter_y = self.rng.range_i(-6, 7);
                (cx + jitter_x, cy + jitter_y)
            };
            if let Some((x, y)) = world.find_zorg_cave_cell(cx, cy) {
                return Some((x as f32, y as f32));
            }
        }

        let w = world.w as i32;
        let h = world.h as i32;

        let deep_start = h * 2 / 3;
        for _ in 0..200 {
            let x = self.rng.range_i(4, w - 4);
            let y = self.rng.range_i(deep_start, h - 4);
            if world.get(x, y) != Elem::Empty {
                continue;
            }

            if world.get(x, y - 1).is_solid() && world.get(x, y + 1) != Elem::Empty {
                return Some((x as f32, y as f32));
            }
        }

        for _ in 0..200 {
            let x = self.rng.range_i(4, w - 4);
            let y = self.rng.range_i(deep_start, h - 4);
            if world.get(x, y) == Elem::Empty {
                return Some((x as f32, y as f32));
            }
        }
        None
    }

    pub fn spawn_zorgs(&mut self, world: &World, count: usize) {
        for _ in 0..count {
            if self.list.len() >= MAX_POP {
                return;
            }
            let Some((x, y)) = self.find_zorg_spot(world) else {
                return;
            };
            let _ = self.spawn(world, x, y, &CreatureGenesis::Zorg);
        }
    }

    pub fn reseed_the_deep(&mut self, world: &World) {
        if !self.tick.is_multiple_of(120) {
            return;
        }

        let mut floor = ZORG_MIN + (world.hardship() * (ZORG_POP_CAP / 2) as f32) as u32;
        let mut slow = false;
        for (_, d) in &self.discoveries {
            if *d & discovery_bits::DOMINION != 0 {
                slow = true;
            }
        }
        if slow {
            floor = 2;
        }
        if self.zorg_count() >= floor {
            return;
        }

        if world.zorg_cave_sealed {
            return;
        }
        if world.zorg_cave_center().is_none() {
            return;
        }
        let Some((x, y)) = self.find_zorg_spot(world) else {
            return;
        };
        let _ = self.spawn(world, x, y, &CreatureGenesis::Zorg);
    }

}
impl Creature {
    pub(super) fn inherited_culture(&self, rng: &mut Rng) -> u16 {
        let mut culture = self.culture;

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
            if culture & bit != 0 && !rng.chance(0.34) {
                culture &= !bit;
            }
        }

        if rng.chance(0.06) {
            let all = [
                culture_bits::CULTIVATION,
                culture_bits::MINING,
                culture_bits::WARFARE,
                culture_bits::COMMERCE,
                culture_bits::CRAFT,
                culture_bits::RITUAL,
                culture_bits::ARCHITECTURE,
                culture_bits::HUNTING,
            ];
            let pick = all[rng.range(all.len() as u32) as usize];
            culture |= pick;
        }
        culture
    }

    #[must_use]
    pub fn child_blueprint_with(&self, mate: &Creature) -> Creature {
        let mut child = self.child_blueprint();
        let mut rng = Rng::new(
            self.id.wrapping_mul(0x9E37_79B9) as u64
                ^ mate.id.wrapping_mul(0x85EB_CA6B) as u64
                ^ self.age as u64,
        );

        for i in 0..32 {
            let mid = (self.genes[i] + mate.genes[i]) * 0.5;
            let blend = mid + (rng.f32() * 2.0 - 1.0) * 0.12;
            child.genes[i] = blend.clamp(0.0, 1.0);
        }

        child.personality = Personality::blend(&self.personality, &mate.personality, &mut rng);

        child.traits = decode_traits(&child.genes);
        child.affinity = Affinity::from_roll(child.genes[16]);

        child.culture = self.inherited_culture(&mut rng) | mate.culture;

        child.size = ((self.size + mate.size) * 0.5 + rng.f32() * 0.2 - 0.1).clamp(1.0, 2.4);

        let distance = gene_distance(&self.genes, &mate.genes);
        let vigour = (distance - 0.06).clamp(-0.25, 0.30);
        child.max_energy = (child.max_energy * (1.0 + vigour)).max(200.0);
        child.health = (child.health * (1.0 + vigour)).max(20.0);
        child
    }
}