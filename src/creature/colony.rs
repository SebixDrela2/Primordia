use super::{CreatureSystem, Rng, CreatureStats, PredationMemory, Diplomacy, World, Role, in_crisis, Elem, ZORG_START, RevivalGenome, Faction, MAX_POP, CreatureGenesis, ZORG_MIN, ZORG_POP_CAP, discovery_bits, Net, brain_sizes, ZORG_COLOR, ZORG_SPECIES, species_team_color, decode_traits, trait_bits, Personality, Affinity, immunity_from_genes, zorg_carried_disease, Disease, Plague, Creature, OUTPUTS, Percept, generate_name, Tool, Belief, Mood, Spell, MountKind, Stage, culture_bits, gene_distance};

impl CreatureSystem {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        CreatureSystem {
            list: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5_1234_9E37_79B9),
            next_id: 1,

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

    fn find_landmark_spot(&mut self, world: &World, claimer: u32) -> Option<(f32, f32)> {
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
            let _ = self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Revival(genome));
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
        self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Random)
    }

    fn find_zorg_spot(&mut self, world: &World) -> Option<(f32, f32)> {

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
            let _ = self.spawn(world, x, y, CreatureGenesis::Zorg);
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
        let _ = self.spawn(world, x, y, CreatureGenesis::Zorg);
    }

    #[must_use]
    pub fn spawn(&mut self, world: &World, x: f32, y: f32, genesis: CreatureGenesis<'_>) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(1));

        let (parent_role, parent_base_x, parent_base_y) = match genesis {
            CreatureGenesis::Child(parent) => (parent.role, parent.base_x, parent.base_y),
            _ => (Role::Worker, 0.0, 0.0),
        };
        let is_child = matches!(genesis, CreatureGenesis::Child(_));
        let start_energy = if is_child { 300.0 } else { 520.0 };

        let (net, color, species, generation, size, genes, mutations) = match genesis {
            CreatureGenesis::Zorg => {

                let mut g = [0.0f32; 32];
                for gi in &mut g {
                    *gi = rng.f32();
                }

                g[0] = 0.9;
                g[1] = 0.7;
                g[3] = 0.6;
                g[5] = 0.8;
                (
                    Net::random(&brain_sizes(), &mut rng),
                    ZORG_COLOR,
                    ZORG_SPECIES,
                    0,
                    1.4 + rng.f32() * 0.8,
                    g,
                    0,
                )
            }
            CreatureGenesis::Random => {
                let mut g = [0.0f32; 32];
                for gi in &mut g {
                    *gi = rng.f32();
                }

                self.next_species += 1;
                let species = self.next_species;
                let color = species_team_color(&mut rng, species);
                (
                    Net::random(&brain_sizes(), &mut rng),
                    color,
                    species,
                    0,
                    1.0 + rng.f32() * 1.3,
                    g,
                    0,
                )
            }
            CreatureGenesis::Child(parent) => {
                let mut net = parent.net.clone_net();

                net.mutate(&mut rng, 0.085, 0.15);

                if rng.chance(0.12) {
                    let _ = net.mutate_shape(&mut rng);
                }

                for s in &mut net.skip {
                    *s += (rng.f32() * 2.0 - 1.0) * 0.04;
                    *s = s.clamp(-1.5, 1.5);
                }

                let color = species_team_color(&mut rng, parent.species);

                let species = parent.species;
                let size = (parent.size + rng.f32() * 0.3 - 0.15).clamp(1.0, 2.4);
                let mut g = parent.genes;
                for gi in &mut g {
                    *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.08).clamp(0.0, 1.0);
                }
                (
                    net,
                    color,
                    species,
                    parent.generation + 1,
                    size,
                    g,
                    parent.mutations.saturating_add(1),
                )
            }            CreatureGenesis::Revival(base) => {
                let mut net = base.net.clone_net();

                net.mutate(&mut rng, 0.14, 0.22);

                self.next_species += 1;
                let species = self.next_species;
                let color = species_team_color(&mut rng, species);
                let size = (base.size + rng.f32() * 0.4 - 0.2).clamp(1.0, 2.4);
                let mut g = base.genes;
                for gi in &mut g {
                    *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.12).clamp(0.0, 1.0);
                }
                (
                    net,
                    color,
                    species,
                    base.generation + 1,
                    size,
                    g,
                    1,
                )
            }
        };

        self.rng = rng;

        let crisis = in_crisis(&self.succession, species, self.tick);
        let is_zorg = species == ZORG_SPECIES;
        let role = if is_zorg && !is_child {

            let has_queen = self
                .list
                .iter()
                .any(|c| c.species == ZORG_SPECIES && c.role == Role::Queen);
            if !has_queen {
                Role::Queen
            } else if self.rng.chance(0.55) {
                Role::Soldier
            } else {
                Role::Worker
            }
        } else if is_zorg && is_child {

            let r = self.rng.f32();
            if r < 0.12 {
                Role::Queen
            } else if r < 0.75 {
                Role::Soldier
            } else {
                Role::Worker
            }
        } else if is_child {
            match parent_role {
                Role::Queen => {

                    let r = self.rng.f32();
                    if crisis {

                        if r < 0.35 { Role::Queen }
                        else if r < 0.65 { Role::Worker }
                        else if r < 0.85 { Role::Soldier }
                        else { Role::Envoy }
                    } else if r < 0.78 {
                        Role::Worker
                    } else if r < 0.88 {
                        Role::Soldier
                    } else if r < 0.95 {
                        Role::Envoy
                    } else {
                        Role::Queen
                    }
                }
                Role::Worker => {

                    let r = self.rng.f32();
                    let queen_chance = if crisis { 0.30 } else { 0.05 };
                    if r < queen_chance {
                        Role::Queen
                    } else if r < queen_chance + 0.01 {
                        Role::Soldier
                    } else if r < queen_chance + 0.05 {
                        Role::Envoy
                    } else {
                        Role::Worker
                    }
                }
                Role::Soldier => {

                    let r = self.rng.f32();
                    let queen_chance = if crisis { 0.22 } else { 0.03 };
                    if r < queen_chance {
                        Role::Queen
                    } else if r < queen_chance + 0.01 {
                        Role::Worker
                    } else if r < queen_chance + 0.03 {
                        Role::Envoy
                    } else {
                        Role::Soldier
                    }
                }
                Role::Envoy => {

                    let r = self.rng.f32();
                    if crisis && r < 0.12 { Role::Queen }
                    else if r < 0.60 { Role::Envoy }
                    else if r < 0.70 { Role::Merchant }
                    else if r < 0.80 { Role::Priest }
                    else { Role::Worker }
                }
                Role::Priest => {
                    if self.rng.chance(0.05) { Role::Shaman } else { Role::Priest }
                }
                Role::Merchant => { Role::Merchant }
                Role::Builder => { Role::Builder }
                Role::Explorer => { Role::Explorer }
                Role::Shaman => { Role::Shaman }
            }
        } else {
            let species_count = self.list.iter().filter(|c| c.species == species).count();
            if species_count == 0 {
                Role::Queen
            } else if self.rng.chance(0.12) {
                Role::Soldier
            } else {
                Role::Worker
            }
        };

        let (base_x, base_y) = if is_child {
            (parent_base_x, parent_base_y)
        } else if is_zorg {

            let queen_base = self
                .list
                .iter()
                .find(|c| c.species == ZORG_SPECIES && c.role == Role::Queen)
                .map(|q| (q.base_x, q.base_y));
            match queen_base {
                Some(q) => q,
                None => world
                    .zorg_cave_center()
                    .and_then(|(cx, cy)| {

                        let fy = world.zorg_cave_floor_row();
                        world
                            .find_zorg_cave_cell(cx, fy - 2)
                            .or_else(|| world.find_zorg_cave_cell(cx, cy))
                    })
                    .map(|(px, py)| (px as f32, py as f32))
                    .or_else(|| self.find_zorg_spot(world))
                    .unwrap_or((x, y)),
            }
        } else if role == Role::Queen {
            match self.find_landmark_spot(world, species) {
                Some(sp) => sp,
                None => (x, y),
            }
        } else {

            self.list.iter()
                .find(|c| c.species == species && c.role == Role::Queen)
                .map_or((x, y), |q| (q.x, q.y))
        };

        let initial_spells = match role {
            Role::Shaman => self.rng.range(2) + 1,
            Role::Priest => u32::from(self.rng.chance(0.5)),
            _ => u32::from(self.rng.chance(0.08)),
        };

        let traits = decode_traits(&genes);
        let hardy = traits & trait_bits::HARDY != 0;

        let personality = match genesis {
            CreatureGenesis::Child(parent) => parent.personality,

            CreatureGenesis::Revival(base) => {
                Personality::inherit(&base.personality, &mut self.rng)
            }
            _ => Personality::random(&mut self.rng),
        };

        let affinity = Affinity::from_roll(genes[16]);

        let culture = match genesis {
            CreatureGenesis::Child(parent) => {
                parent.culture | self.species_culture(species)
            }
            _ => self.species_culture(species),
        };

        let mut starting_inv = [0u16; 16];
        starting_inv[0] = (self.rng.range(10) + 5) as u16;
        starting_inv[2] = (self.rng.range(4) + 1) as u16;
        starting_inv[6] = (self.rng.range(6) + 2) as u16;
        if role == Role::Builder {
            starting_inv[0] = starting_inv[0].saturating_add(20u16);
            starting_inv[1] = self.rng.range(5) as u16;
        }
        if role == Role::Shaman || role == Role::Priest {
            starting_inv[4] = self.rng.range(3) as u16;
        }

        let _ = immunity_from_genes(&genes);

        let carried = if is_zorg {
            zorg_carried_disease(self.rng.range(5))
        } else {
            Disease::None
        };
        let carried_timer = if carried == Disease::None {
            0
        } else {
            Plague::from(carried).duration()
        };

        let creature = Creature {
            id: self.next_id,
            x: x.clamp(2.0, world.w as f32 - 3.0),
            y: y.clamp(2.0, world.h as f32 - 3.0),
            vx: 0.0,
            vy: 0.0,
            energy: start_energy,
            max_energy: if role == Role::Soldier { 1500.0 } else { 1200.0 },
            health: if role == Role::Soldier {
                150.0
            } else {
                100.0
            } * if hardy { 1.4 } else { 1.0 },
            age: 0,
            net,
            color,
            species,
            faction: if species == ZORG_SPECIES {
                Faction::Zorg
            } else {
                Faction::Player
            },
            generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 150,
            fire_cooldown: 0,
            out_cache: [0.0; OUTPUTS],
            brain_ready: false,
            percept_cache: Percept::empty(),
            desire_dir: 0,
            hurt_dir: -1,
            kill_dir: -1,
            health_seen: 100.0,
            kills_seen: 0,
            last_meal: Elem::Empty,
            size,
            last_x: x,
            last_y: y,
            name: generate_name(&mut self.rng, role, species),
            team_color: species_team_color(&mut self.rng, species),
            role,
            base_x,
            base_y,
            carried: 0.0,
            build_cooldown: 0,
            target_x: x,
            target_y: y,
            tool: Tool::None,
            tool_durability: 0,
            bombs: 0,
            salves: 0,
            tech_level: 0,
            inventory: starting_inv,
            craft_cooldown: 0,
            disease: carried,
            disease_timer: carried_timer,
            immunity: [0; 8],
            belief: Belief::None,
            devotion: 0,
            mood: Mood::Neutral,
            mood_timer: 0,
            loyalty: 0,
            reputation: 0,
            trade_partner: None,
            squad_id: 0,
            squad_role: 0,
            mana: 0.0,
            max_mana: 50.0,
            spell_cooldown: 0,
            known_spells: initial_spells as u16,
            active_spell: if initial_spells > 0 { Spell::Fireball } else { Spell::None },
            spell_cd: [0; 10],
            mount: None,
            vehicle: None,
            mount_kind: MountKind::None,
            air_time: 0,
            mount_stamina: 100.0,
            genes,
            mutations,
            traits,
            personality,
            stage: Stage::Juvenile,
            affinity,
            culture,
            mate: None,
            sired: 0,
            migrating: false,
            bond: None,
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
            tech: 0,
            diplo_cooldown: 0,
            diplo_progress: 0.0,
            diplo_partner: None,
            diplo_with: None,
            pacts_signed: 0,
        };
        self.next_id += 1;
        self.list.push(creature);
        self.births_total += 1;
        true
    }

}
impl Creature {
    #[must_use]
    pub fn child_blueprint(&self) -> Creature {

        let mut child_genes = self.genes;
        for (i, g) in child_genes.iter_mut().enumerate() {
            let mut seed = self.id.wrapping_mul(i as u32 + 1).wrapping_add(self.age);
            seed ^= seed >> 13;
            seed ^= seed << 17;
            seed ^= seed >> 5;
            let rand_val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            if seed.is_multiple_of(20) {
                *g = (*g + rand_val * 0.1).clamp(0.0, 1.0);
            }
        }

        let mut rng = Rng::new(self.id.wrapping_mul(0x9E37_79B9) as u64 ^ self.age as u64);
        let personality = Personality::inherit(&self.personality, &mut rng);
        let affinity = Affinity::from_roll(child_genes[16]);
        Creature {
            id: 0,
            x: self.x,
            y: self.y,
            vx: 0.0,
            vy: 0.0,
            energy: 380.0,
            max_energy: if self.role == Role::Soldier { 1500.0 } else { 1200.0 },
            health: if self.role == Role::Soldier {
                150.0
            } else {
                100.0
            } * if self.traits & trait_bits::HARDY != 0 {
                1.4
            } else {
                1.0
            },
            age: 0,
            net: self.net.clone_net(),
            color: self.color,
            species: self.species,
            faction: self.faction,
            generation: self.generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 0,
            fire_cooldown: 0,
            out_cache: [0.0; OUTPUTS],
            brain_ready: false,
            percept_cache: Percept::empty(),
            desire_dir: 0,
            hurt_dir: -1,
            kill_dir: -1,
            health_seen: 100.0,
            kills_seen: 0,
            last_meal: Elem::Empty,
            size: self.size,
            last_x: self.x,
            last_y: self.y,
            name: generate_name(&mut Rng::new(self.id.wrapping_mul(0x9E37_79B9) as u64), self.role, self.species),
            team_color: self.team_color,
            role: self.role,
            base_x: self.base_x,
            base_y: self.base_y,
            carried: 0.0,
            build_cooldown: 0,
            target_x: self.base_x,
            target_y: self.base_y,
            tool: Tool::None,
            tool_durability: 0,
            bombs: self.bombs,
            salves: self.salves,
            tech_level: self.tech_level,
            inventory: [0; 16],
            craft_cooldown: 0,

            disease: if self.faction == Faction::Zorg {
                self.disease
            } else {
                Disease::None
            },
            disease_timer: if self.faction == Faction::Zorg {
                self.disease_timer
            } else {
                0
            },
            immunity: [0; 8],
            belief: self.belief,
            devotion: 0,
            mood: Mood::Neutral,
            mood_timer: 0,
            loyalty: 0,
            reputation: 0,
            trade_partner: None,
            squad_id: 0,
            squad_role: 0,
            mana: 0.0,
            max_mana: 50.0,
            spell_cooldown: 0,
            known_spells: self.known_spells,
            active_spell: self.active_spell,
            spell_cd: [0; 10],
            mount: None,
            vehicle: None,
            mount_kind: MountKind::None,
            air_time: 0,
            mount_stamina: 100.0,
            genes: child_genes,
            mutations: self.mutations + 1,

            traits: decode_traits(&child_genes),
            personality,
            stage: Stage::Juvenile,
            affinity,

            culture: self.inherited_culture(&mut rng),
            mate: None,
            sired: 0,
            migrating: false,
            bond: None,
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
            tech: 0,
            diplo_cooldown: 0,
            diplo_progress: 0.0,
            diplo_partner: None,
            diplo_with: None,
            pacts_signed: 0,
        }
    }

    fn inherited_culture(&self, rng: &mut Rng) -> u16 {
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
