use super::{CreatureSystem, Rng, Creature, World, Role, in_crisis, Elem, Faction, MAX_POP, CreatureGenesis, ZORG_SPECIES, species_team_color, decode_traits, trait_bits, Personality, Affinity, zorg_carried_disease, Disease, Plague, OUTPUTS, Percept, generate_name, Tool, Belief, Mood, Spell, MountKind, Stage, brain_sizes, Net, ZORG_COLOR, RevivalGenome};

/// What a creature is made of when it is born.
struct Born {
    net: Net,
    color: [u8; 3],
    species: u32,
    generation: u32,
    size: f32,
    genes: [f32; 32],
    mutations: u16,
}

/// Where a creature is heading when it is born.
struct Birthplace {
    species: u32,
    is_child: bool,
    is_zorg: bool,
    role: Role,
    x: f32,
    y: f32,
    parent_base_x: f32,
    parent_base_y: f32,
}

/// Everything that makes one newborn different from another.
struct Newborn {
    id: u32,
    x: f32,
    y: f32,
    net: Net,
    color: [u8; 3],
    team_color: [u8; 3],
    name: [u8; 10],
    role: Role,
    hardy: bool,
    energy: f32,
    species: u32,
    faction: Faction,
    generation: u32,
    size: f32,
    cooldown: u32,
    inventory: [u16; 16],
    disease: Disease,
    disease_timer: u16,
    base_x: f32,
    base_y: f32,
    target_x: f32,
    target_y: f32,
    known_spells: u16,
    active_spell: Spell,
    genes: [f32; 32],
    mutations: u16,
    traits: u32,
    personality: Personality,
    affinity: Affinity,
    culture: u16,
}

/// The body a newborn starts from.
fn newborn_body(n: Newborn) -> Creature {
    Creature {
        id: n.id,
        x: n.x,
        y: n.y,
        vx: 0.0,
        vy: 0.0,
        energy: n.energy,
        max_energy: if n.role == Role::Soldier { 1500.0 } else { 1200.0 },
        health: if n.role == Role::Soldier { 150.0 } else { 100.0 } * if n.hardy { 1.4 } else { 1.0 },
        age: 0,
        net: n.net,
        color: n.color,
        species: n.species,
        faction: n.faction,
        generation: n.generation,
        fitness: 0.0,
        offspring: 0,
        kills: 0,
        cooldown: n.cooldown,
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
        size: n.size,
        last_x: n.x,
        last_y: n.y,
        name: n.name,
        team_color: n.team_color,
        role: n.role,
        base_x: n.base_x,
        base_y: n.base_y,
        carried: 0.0,
        build_cooldown: 0,
        target_x: n.target_x,
        target_y: n.target_y,
        tool: Tool::None,
        tool_durability: 0,
        bombs: 0,
        salves: 0,
        tech_level: 0,
        inventory: n.inventory,
        craft_cooldown: 0,
        disease: n.disease,
        disease_timer: n.disease_timer,
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
        known_spells: n.known_spells,
        active_spell: n.active_spell,
        spell_cd: [0; 10],
        mount: None,
        vehicle: None,
        mount_kind: MountKind::None,
        air_time: 0,
        mount_stamina: 100.0,
        genes: n.genes,
        mutations: n.mutations,
        traits: n.traits,
        personality: n.personality,
        stage: Stage::Juvenile,
        affinity: n.affinity,
        culture: n.culture,
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

/// A ZORG: born in the deep, with no name and no culture.
fn zorg_being(rng: &mut Rng) -> Born {
    let mut g = [0.0f32; 32];
    for gi in &mut g {
        *gi = rng.f32();
    }

    g[0] = 0.9;
    g[1] = 0.7;
    g[3] = 0.6;
    g[5] = 0.8;
    let net = Net::random(&brain_sizes(), rng);
    let size = 1.4 + rng.f32() * 0.8;
    Born {
        net,
        color: ZORG_COLOR,
        species: ZORG_SPECIES,
        generation: 0,
        size,
        genes: g,
        mutations: 0,
    }
}

/// A settler: a brand new species on an empty map.
fn random_being(rng: &mut Rng, species: u32) -> Born {
    let mut g = [0.0f32; 32];
    for gi in &mut g {
        *gi = rng.f32();
    }

    let color = species_team_color(rng, species);
    let net = Net::random(&brain_sizes(), rng);
    let size = 1.0 + rng.f32() * 1.3;
    Born {
        net,
        color,
        species,
        generation: 0,
        size,
        genes: g,
        mutations: 0,
    }
}

/// A child: the parent's shape, a little changed.
fn child_being(parent: &Creature, rng: &mut Rng) -> Born {
    let mut net = parent.net.clone_net();

    net.mutate(rng, 0.085, 0.15);

    if rng.chance(0.12) {
        let _ = net.mutate_shape(rng);
    }

    for s in &mut net.skip {
        *s += (rng.f32() * 2.0 - 1.0) * 0.04;
        *s = s.clamp(-1.5, 1.5);
    }

    let color = species_team_color(rng, parent.species);

    let size = (parent.size + rng.f32() * 0.3 - 0.15).clamp(1.0, 2.4);
    let mut g = parent.genes;
    for gi in &mut g {
        *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.08).clamp(0.0, 1.0);
    }
    Born {
        net,
        color,
        species: parent.species,
        generation: parent.generation + 1,
        size,
        genes: g,
        mutations: parent.mutations.saturating_add(1),
    }
}

/// A revival: an old genome woken up in a new world.
fn revival_being(base: &RevivalGenome, species: u32, rng: &mut Rng) -> Born {
    let mut net = base.net.clone_net();

    net.mutate(rng, 0.14, 0.22);

    let color = species_team_color(rng, species);
    let size = (base.size + rng.f32() * 0.4 - 0.2).clamp(1.0, 2.4);
    let mut g = base.genes;
    for gi in &mut g {
        *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.12).clamp(0.0, 1.0);
    }
    Born {
        net,
        color,
        species,
        generation: base.generation + 1,
        size,
        genes: g,
        mutations: 1,
    }
}

impl CreatureSystem {
    /// The role a child of a queen is given.
    fn child_of_queen(&mut self, crisis: bool) -> Role {
        let r = self.rng.f32();
        if crisis {
            if r < 0.35 {
                Role::Queen
            } else if r < 0.65 {
                Role::Worker
            } else if r < 0.85 {
                Role::Soldier
            } else {
                Role::Envoy
            }
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

    /// The role a child of a worker is given.
    fn child_of_worker(&mut self, crisis: bool) -> Role {
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

    /// The role a child of a soldier is given.
    fn child_of_soldier(&mut self, crisis: bool) -> Role {
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

    /// The role a child of an envoy is given.
    fn child_of_envoy(&mut self, crisis: bool) -> Role {
        let r = self.rng.f32();
        if crisis && r < 0.12 {
            Role::Queen
        } else if r < 0.60 {
            Role::Envoy
        } else if r < 0.70 {
            Role::Merchant
        } else if r < 0.80 {
            Role::Priest
        } else {
            Role::Worker
        }
    }

    /// The role a child is given, which mostly follows the mother.
    fn child_role(&mut self, parent_role: Role, crisis: bool) -> Role {
        match parent_role {
            Role::Queen => self.child_of_queen(crisis),
            Role::Worker => self.child_of_worker(crisis),
            Role::Soldier => self.child_of_soldier(crisis),
            Role::Envoy => self.child_of_envoy(crisis),
            Role::Priest => {
                if self.rng.chance(0.05) {
                    Role::Shaman
                } else {
                    Role::Priest
                }
            }
            Role::Merchant => Role::Merchant,
            Role::Builder => Role::Builder,
            Role::Explorer => Role::Explorer,
            Role::Shaman => Role::Shaman,
        }
    }

    /// The role a creature is given when it is born.
    fn choose_role(
        &mut self,
        species: u32,
        is_child: bool,
        is_zorg: bool,
        crisis: bool,
        parent_role: Role,
    ) -> Role {
        if is_zorg && !is_child {
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
            self.child_role(parent_role, crisis)
        } else {
            let species_count = self.list.iter().filter(|c| c.species == species).count();
            if species_count == 0 {
                Role::Queen
            } else if self.rng.chance(0.12) {
                Role::Soldier
            } else {
                Role::Worker
            }
        }
    }

    /// Where a ZORG is born: beside the queen of the deep, or in the cave.
    fn zorg_base(&mut self, world: &World, x: f32, y: f32) -> (f32, f32) {
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
    }

    /// Where a creature makes its home.
    fn choose_base(&mut self, world: &World, b: &Birthplace) -> (f32, f32) {
        if b.is_child {
            (b.parent_base_x, b.parent_base_y)
        } else if b.is_zorg {
            self.zorg_base(world, b.x, b.y)
        } else if b.role == Role::Queen {
            match self.find_landmark_spot(world, b.species) {
                Some(sp) => sp,
                None => (b.x, b.y),
            }
        } else {
            self.list
                .iter()
                .find(|c| c.species == b.species && c.role == Role::Queen)
                .map_or((b.x, b.y), |q| (q.x, q.y))
        }
    }

    /// The spells a creature is born knowing.
    fn starting_spells(&mut self, role: Role) -> u32 {
        match role {
            Role::Shaman => self.rng.range(2) + 1,
            Role::Priest => u32::from(self.rng.chance(0.5)),
            _ => u32::from(self.rng.chance(0.08)),
        }
    }

    /// The temperament a creature is born with.
    fn starting_personality(&mut self, genesis: &CreatureGenesis<'_>) -> Personality {
        match genesis {
            CreatureGenesis::Child(parent) => parent.personality,
            CreatureGenesis::Revival(base) => Personality::inherit(&base.personality, &mut self.rng),
            _ => Personality::random(&mut self.rng),
        }
    }

    /// The culture a creature is born into.
    fn starting_culture(&mut self, genesis: &CreatureGenesis<'_>, species: u32) -> u16 {
        match genesis {
            CreatureGenesis::Child(parent) => parent.culture | self.species_culture(species),
            _ => self.species_culture(species),
        }
    }

    /// What a creature is carrying when it is born.
    fn starting_inventory(&mut self, role: Role) -> [u16; 16] {
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
        starting_inv
    }

    /// The sickness a creature is born with.
    fn starting_disease(&mut self, is_zorg: bool) -> (Disease, u16) {
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
        (carried, carried_timer)
    }

    #[must_use]
    pub fn spawn(&mut self, world: &World, x: f32, y: f32, genesis: &CreatureGenesis<'_>) -> bool {
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

        let born = match genesis {
            CreatureGenesis::Zorg => zorg_being(&mut rng),
            CreatureGenesis::Random => {
                self.next_species += 1;
                random_being(&mut rng, self.next_species)
            }
            CreatureGenesis::Child(parent) => child_being(parent, &mut rng),
            CreatureGenesis::Revival(base) => {
                self.next_species += 1;
                revival_being(base, self.next_species, &mut rng)
            }
        };

        self.rng = rng;

        let crisis = in_crisis(&self.succession, born.species, self.tick);
        let is_zorg = born.species == ZORG_SPECIES;
        let role = self.choose_role(born.species, is_child, is_zorg, crisis, parent_role);
        let (base_x, base_y) = self.choose_base(
            world,
            &Birthplace {
                species: born.species,
                is_child,
                is_zorg,
                role,
                x,
                y,
                parent_base_x,
                parent_base_y,
            },
        );

        let initial_spells = self.starting_spells(role);
        let traits = decode_traits(&born.genes);
        let hardy = traits & trait_bits::HARDY != 0;
        let personality = self.starting_personality(genesis);
        let affinity = Affinity::from_roll(born.genes[16]);
        let culture = self.starting_culture(genesis, born.species);
        let starting_inv = self.starting_inventory(role);
        let (carried, carried_timer) = self.starting_disease(is_zorg);

        let name = generate_name(&mut self.rng, role, born.species);
        let team_color = species_team_color(&mut self.rng, born.species);

        let creature = newborn_body(Newborn {
            id: self.next_id,
            x: x.clamp(2.0, world.w as f32 - 3.0),
            y: y.clamp(2.0, world.h as f32 - 3.0),
            net: born.net,
            color: born.color,
            team_color,
            name,
            role,
            hardy,
            energy: start_energy,
            species: born.species,
            faction: if born.species == ZORG_SPECIES {
                Faction::Zorg
            } else {
                Faction::Player
            },
            generation: born.generation,
            size: born.size,
            cooldown: 150,
            inventory: starting_inv,
            disease: carried,
            disease_timer: carried_timer,
            base_x,
            base_y,
            target_x: x,
            target_y: y,
            known_spells: initial_spells as u16,
            active_spell: if initial_spells > 0 {
                Spell::Fireball
            } else {
                Spell::None
            },
            genes: born.genes,
            mutations: born.mutations,
            traits,
            personality,
            affinity,
            culture,
        });
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
        let traits = decode_traits(&child_genes);
        let culture = self.inherited_culture(&mut rng);
        let name = generate_name(
            &mut Rng::new(self.id.wrapping_mul(0x9E37_79B9) as u64),
            self.role,
            self.species,
        );
        newborn_body(Newborn {
            id: 0,
            x: self.x,
            y: self.y,
            net: self.net.clone_net(),
            color: self.color,
            team_color: self.team_color,
            name,
            role: self.role,
            hardy: self.traits & trait_bits::HARDY != 0,
            energy: 380.0,
            species: self.species,
            faction: self.faction,
            generation: self.generation,
            size: self.size,
            cooldown: 0,
            inventory: [0; 16],
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
            base_x: self.base_x,
            base_y: self.base_y,
            target_x: self.base_x,
            target_y: self.base_y,
            known_spells: self.known_spells,
            active_spell: self.active_spell,
            genes: child_genes,
            mutations: self.mutations + 1,
            traits,
            personality,
            affinity,
            culture,
        })
    }
}
