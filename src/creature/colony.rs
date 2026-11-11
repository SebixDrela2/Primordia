use super::*;

impl CreatureSystem {
    pub fn new(seed: u64) -> Self {
        CreatureSystem {
            list: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5_1234_9E37_79B9),
            next_id: 1,
            next_species: 1,
            tick: 0,
            stats: CreatureStats::default(),
            births_total: 0,
            deaths_total: 0,
            eats_total: 0,
            builds_total: 0,
            trades_total: 0,
            spells_cast: 0,
            next_squad: 1,
            vehicles: Vec::new(),
            next_vehicle: 1,
            day_night: 0.25,
            temperature: 20.0,
            season: 0,
            season_timer: 0,
            projectiles: Vec::new(),
            structures: Vec::new(),
            events: Vec::new(),
            bombs: Vec::new(),
        }
    }

    /// Scatter a burst of short-lived energy motes into the world.
    ///
    /// The motes are deliberately *not* light sources: an explosion would
    /// otherwise dump a dozen glowing cells into the light field and flash
    /// the whole cave white.
    pub(crate) fn spawn_energy_burst(&mut self, world: &mut World, x: i32, y: i32, count: u32, color: [u8; 3]) {
        let _ = color;
        for _ in 0..count {
            let ang = self.rng.f32() * std::f32::consts::TAU;
            let dist = 0.5 + self.rng.f32() * 2.0;
            let px = (x as f32 + ang.cos() * dist).round() as i32;
            let py = (y as f32 + ang.sin() * dist).round() as i32;
            if world.get(px, py) != Elem::Empty {
                continue;
            }
            // Smoke spreads out and dies quickly without lighting anything up.
            world.set_xy(px, py, Elem::Smoke);
        }
    }

    /// Scatter a puff of smoke/steam alongside an impact (used when a shot
    /// hits terrain).
    pub(crate) fn spawn_impact_puff(&mut self, world: &mut World, x: i32, y: i32, count: u32) {
        for _ in 0..count {
            let px = x + self.rng.range_i(-1, 2);
            let py = y + self.rng.range_i(-1, 2);
            if world.get(px, py) != Elem::Empty { continue; }
            world.set_xy(px, py, Elem::Smoke);
        }
    }

    pub fn populate(&mut self, world: &World, count: usize) {
        for _ in 0..count {
            // Every founder starts alone. Handing each one a guaranteed
            // partner gave the starting teams a free ally, which meant a
            // species that spawned badly still survived its first fights.
            self.spawn_near_surface(world);
        }
        self.update_stats();
    }

    /// Snapshot the fittest living member of `species` so the next run can be
    /// seeded from the winning lineage.
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
            })
    }

    /// Seed a fresh world with mutated descendants of a previous champion.
    /// Every spawn becomes a new species (and therefore starts as a queen), so
    /// the winning lineage immediately diversifies into competing clades.
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
            self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Revival(genome));
        }
        self.update_stats();
    }

    pub fn spawn_near_surface(&mut self, world: &World) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let x = self.rng.range_i(3, world.w as i32 - 3);
        let surface = world.surface_y(x);
        let y = surface - 2;
        // Find a nearby empty/passable spot so creatures never spawn stuck in solids or deep water.
        let (sx, sy) = world
            .find_empty_passable_near(x, y, 8)
            .unwrap_or((x, y));
        self.spawn(world, sx as f32, sy as f32, CreatureGenesis::Random)
    }

    pub(crate) fn spawn(&mut self, world: &World, x: f32, y: f32, genesis: CreatureGenesis<'_>) -> bool {
        if self.list.len() >= MAX_POP {
            return false;
        }
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(1));

        // Extract parent data if child
        let (parent_role, parent_base_x, parent_base_y) = match genesis {
            CreatureGenesis::Child(parent) => (parent.role, parent.base_x, parent.base_y),
            _ => (Role::Worker, 0.0, 0.0),
        };
        let is_child = matches!(genesis, CreatureGenesis::Child(_));
        let start_energy = if is_child { 300.0 } else { 520.0 };

        let (net, color, species, generation, size, genes, mutations) = match genesis {
            CreatureGenesis::Random => {
                let mut g = [0.0f32; 32];
                for gi in g.iter_mut() {
                    *gi = rng.f32();
                }
                (
                    Net::random(&[INPUTS, 24, 12, OUTPUTS], &mut rng),
                    [
                        rng.range_i(90, 256) as u8,
                        rng.range_i(100, 256) as u8,
                        rng.range_i(120, 256) as u8,
                    ],
                    {
                        self.next_species += 1;
                        self.next_species
                    },
                    0,
                    1.0 + rng.f32() * 1.3,
                    g,
                    0,
                )
            }
            CreatureGenesis::Child(parent) => {
                let mut net = parent.net.clone_net();
                net.mutate(&mut rng, 0.12, 0.22);
                let color = [
                    (parent.color[0] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                    (parent.color[1] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                    (parent.color[2] as i32 + rng.range_i(-18, 19)).clamp(30, 255) as u8,
                ];
                // No ongoing speciation: species only disappear through
                // extinction, so a run can actually converge on one survivor.
                // New diversity is injected between runs by `Revival`.
                let species = parent.species;
                let size = (parent.size + rng.f32() * 0.3 - 0.15).clamp(1.0, 2.4);
                let mut g = parent.genes;
                for gi in g.iter_mut() {
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
            }
            CreatureGenesis::Revival(base) => {
                let mut net = base.net.clone_net();
                // Stronger shake-up: the winning lineage diverges into several
                // new species that then compete again.
                net.mutate(&mut rng, 0.20, 0.30);
                let color = [
                    (base.color[0] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                    (base.color[1] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                    (base.color[2] as i32 + rng.range_i(-32, 33)).clamp(40, 255) as u8,
                ];
                self.next_species += 1;
                let species = self.next_species;
                let size = (base.size + rng.f32() * 0.4 - 0.2).clamp(1.0, 2.4);
                let mut g = base.genes;
                for gi in g.iter_mut() {
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

        // Determine role: first of species = Queen, then ~10% Soldiers, rest Workers
        // Children of queens become workers/soldiers; children of workers/soldiers inherit with small mutation
        let role = if is_child {
            match parent_role {
                Role::Queen => {
                    // Queen's offspring: 85% worker, 10% soldier, 5% queen (rare princess)
                    let r = self.rng.f32();
                    if r < 0.85 { Role::Worker }
                    else if r < 0.95 { Role::Soldier }
                    else { Role::Queen }
                }
                Role::Worker => {
                    // Worker offspring: mostly workers, rare soldier or a new
                    // princess that can found a fresh colony/lineage.
                    let r = self.rng.f32();
                    if r < 0.05 { Role::Queen }
                    else if r < 0.06 { Role::Soldier }
                    else { Role::Worker }
                }
                Role::Soldier => {
                    // Soldiers mostly breed soldiers, occasionally a queen.
                    let r = self.rng.f32();
                    if r < 0.03 { Role::Queen }
                    else if r < 0.04 { Role::Worker }
                    else { Role::Soldier }
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

        // Base position: Queen sets it; children inherit; random spawns pick a spot
        let (base_x, base_y) = if is_child {
            (parent_base_x, parent_base_y)
        } else if role == Role::Queen {
            (x, y)
        } else {
            // Find species base or use spawn pos
            self.list.iter()
                .find(|c| c.species == species && c.role == Role::Queen)
                .map(|q| (q.x, q.y))
                .unwrap_or((x, y))
        };

        // Spell learning at birth: shamans/priests start with spells, others rarely
        let initial_spells = match role {
            Role::Shaman => self.rng.range(2) + 1,
            Role::Priest => if self.rng.chance(0.5) { 1 } else { 0 },
            _ => if self.rng.chance(0.08) { 1 } else { 0 },
        };

        // Traits are decoded from the genes, so they are inherited and evolve.
        let traits = decode_traits(&genes);
        let hardy = traits & trait_bits::HARDY != 0;

        // Starting resources so creatures can immediately craft and build
        let mut starting_inv = [0u16; 16];
        starting_inv[0] = (self.rng.range(10) + 5) as u16;  // stone
        starting_inv[2] = (self.rng.range(4) + 1) as u16;   // copper - enables tech progression
        starting_inv[6] = (self.rng.range(6) + 2) as u16;   // wood/plant
        if role == Role::Builder {
            starting_inv[0] = starting_inv[0].saturating_add(20u16);
            starting_inv[1] = self.rng.range(5) as u16;   // metal
        }
        if role == Role::Shaman || role == Role::Priest {
            starting_inv[4] = self.rng.range(3) as u16;   // crystal
        }

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
            generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 150,
            fire_cooldown: 0,
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
            disease: Disease::None,
            disease_timer: 0,
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
            mount_stamina: 100.0,
            genes,
            mutations,
            traits,
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
        };
        self.next_id += 1;
        self.list.push(creature);
        self.births_total += 1;
        true
    }

}
impl Creature {
    pub(crate) fn child_blueprint(&self) -> Creature {
        // Mutate the genes first so the inherited traits can be decoded.
        let mut child_genes = self.genes;
        for i in 0..32 {
            // Deterministic mutation based on id, age, and gene index
            let mut seed = self.id.wrapping_mul(i as u32 + 1).wrapping_add(self.age);
            seed ^= seed >> 13;
            seed ^= seed << 17;
            seed ^= seed >> 5;
            let rand_val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            if (seed % 20) == 0 {
                // 5% chance
                child_genes[i] = (child_genes[i] + rand_val * 0.1).clamp(0.0, 1.0);
            }
        }
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
            generation: self.generation,
            fitness: 0.0,
            offspring: 0,
            kills: 0,
            cooldown: 0,
            fire_cooldown: 0,
            size: self.size,
            last_x: self.x,
            last_y: self.y,
            name: generate_name(&mut Rng::new(self.id.wrapping_mul(0x9E3779B9) as u64), self.role, self.species),
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
            disease: Disease::None,
            disease_timer: 0,
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
            mount_stamina: 100.0,
            genes: child_genes,
            mutations: self.mutations + 1,
            // The traits are decoded from the mutated genes, so a species can
            // evolve a new trait between generations.
            traits: decode_traits(&child_genes),
            hunger: 50.0,
            thirst: 50.0,
            stamina: 100.0,
            max_stamina: 100.0,
            resting: false,
            memory: [0.0; 8],
        }
    }
}

