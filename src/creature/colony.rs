use super::*;

impl CreatureSystem {
    pub fn new(seed: u64) -> Self {
        CreatureSystem {
            list: Vec::new(),
            rng: Rng::new(seed ^ 0xA5A5_1234_9E37_79B9),
            next_id: 1,
            // The ZORGs are species zero; the first colony of the run is
            // species one, and it is the first colour of the wheel.
            next_species: 0,
            // The colony fires start cold: somebody has to bring the wood.
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
            vehicles: Vec::new(),
            next_vehicle: 1,
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

            last_discovery: None,
        }
    }

    /// What a species has collectively learned.
    pub fn species_culture(&self, species: u32) -> u16 {
        self.culture_pool
            .iter()
            .find(|(id, _)| *id == species)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    /// Record that a species has learned a new practice.
    pub fn note_culture(&mut self, species: u32, bit: u16) {
        if let Some(entry) = self.culture_pool.iter_mut().find(|(id, _)| *id == species) {
            entry.1 |= bit;
        } else {
            self.culture_pool.push((species, bit));
        }
    }

    /// A place for a new colony to settle: a landmark that nobody has
    /// claimed yet.
    ///
    /// A landmark is worth having, so a colony that can have one takes it.
    /// The second colony to arrive has to make do with ordinary ground - and
    /// the two colonies end up fighting over the place.
    fn find_landmark_spot(&mut self, world: &World, claimer: u32) -> Option<(f32, f32)> {
        if !self.rng.chance(0.6) {
            return None;
        }
        let mut rng = std::mem::replace(&mut self.rng, Rng::new(1));
        let mut found = None;
        let mut taken: Vec<(u32, f32, f32)> = Vec::new();
        for l in world.landmarks.iter() {
            // Is the landmark already taken? If it belongs to another species
            // this is a territorial dispute, not a free spot.
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
            // Find a spot in the landmark that a colony could stand in.
            // The middle of a landmark is full of crystal, bone or water,
            // so the search looks for somewhere passable.
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
        // There was no free landmark left, so this colony has to do with
        // ordinary ground - and the colony that holds the place is not going
        // to share it. That is a territorial dispute and it turns into a war.
        if found.is_none() && !taken.is_empty() && self.rng.chance(0.35) {
            let i = self.rng.range(taken.len() as u32) as usize;
            let (holder, _, _) = taken[i];
            if !self.diplomacy.at_war(holder, claimer) {
                self.declare_war(holder, claimer, "disputed landmark", self.tick);
            }
        }
        found
    }

    /// The ground a species holds: the middle of its colony and how far its
    /// claim reaches.
    ///
    /// A colony with buildings holds more ground, so a rich colony pushes the
    /// others out.
    pub fn claim_of(&self, species: u32) -> Option<(f32, f32, f32)> {
        self.claims
            .iter()
            .find(|(id, _, _, _)| *id == species)
            .map(|(_, cx, cy, r)| (*cx, *cy, *r))
    }

    /// Work out the ground every colony holds.
    ///
    /// A colony with buildings holds more ground, so a rich colony pushes
    /// the others out. The result is cached for the tick: the claim is asked
    /// for by every soldier that looks for a trespasser, and walking the
    /// whole population each time would be far too slow.
    pub fn rebuild_claims(&mut self) {
        self.claims.clear();
        let mut centres: Vec<(u32, f32, f32)> = Vec::new();
        let mut builders: Vec<(u32, u32)> = Vec::new();
        for c in self.list.iter() {
            if c.role == Role::Queen {
                // A colony in crisis has nobody to defend it, so it holds
                // no ground at all.
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
                .map(|(_, n)| *n)
                .unwrap_or(1);
            let radius = (18.0 + buildings as f32 * 4.0).min(46.0);
            self.claims.push((id, cx, cy, radius));
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
        // The ZORGs are already down there, waiting.
        self.spawn_zorgs(world, ZORG_START);
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
        // The ZORGs survive the revival: they are always down there.
        self.spawn_zorgs(world, ZORG_START);
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

    /// A place in the deep caves for the ZORGs to hold.
    ///
    /// The ZORGs have a cave of their own: a chamber carved into the deep
    /// rock, far below the surface, where the poison pools on the floor.
    /// Every one of them starts there, so the deep has a heart instead of
    /// a scattering of animals across the bottom of the map.
    ///
    /// If the world has no such cave (it is carved during the seed, so it
    /// normally always has one), the search falls back to the deep caves.
    fn find_zorg_spot(&mut self, world: &World) -> Option<(f32, f32)> {
        // The cave of the ZORGs comes first.
        if let Some((cx, cy)) = world.zorg_cave_center() {
            // The heart of the chamber is where the queen of the ZORGs
            // belongs; the rest of them spread out through it.
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
        // No cave: fall back to the deep caves.
        let w = world.w as i32;
        let h = world.h as i32;
        // The deep: the last third of the world, below the caves.
        let deep_start = h * 2 / 3;
        for _ in 0..200 {
            let x = self.rng.range_i(4, w - 4);
            let y = self.rng.range_i(deep_start, h - 4);
            if world.get(x, y) != Elem::Empty {
                continue;
            }
            // It has to be a place they can stand: room above and below.
            if world.get(x, y - 1).is_solid() && world.get(x, y + 1) != Elem::Empty {
                return Some((x as f32, y as f32));
            }
        }
        // Nothing found: the deep is solid here, so take any empty cell.
        for _ in 0..200 {
            let x = self.rng.range_i(4, w - 4);
            let y = self.rng.range_i(deep_start, h - 4);
            if world.get(x, y) == Elem::Empty {
                return Some((x as f32, y as f32));
            }
        }
        None
    }

    /// Found the first of the ZORGs in their own cave.
    ///
    /// The ZORGs are there from the very beginning of the run: they are
    /// the thing the players have to beat, and they start growing from the
    /// first tick. They all start together, in the one place in the world
    /// that belongs to them.
    pub fn spawn_zorgs(&mut self, world: &World, count: usize) {
        for _ in 0..count {
            if self.list.len() >= MAX_POP {
                return;
            }
            let Some((x, y)) = self.find_zorg_spot(world) else {
                return;
            };
            self.spawn(world, x, y, CreatureGenesis::Zorg);
        }
    }

    /// Keep the deep from ever being emptied.
    ///
    /// The ZORGs are not a colony that can be wiped out for good: as long
    /// as their cave is there, the deep breeds more of them. A player can
    /// thin them out and beat them back, but the deep always has a few
    /// left in it.
    pub(crate) fn reseed_the_deep(&mut self, world: &World) {
        if self.tick % 120 != 0 {
            return;
        }
        // Dominion: a colony that has worked out how to close the deep
        // holds it closed. The ZORGs breed far more slowly, and the floor
        // of their numbers is lower.
        //
        // The floor also rises with the difficulty: the deep starts as a
        // handful of beasts and becomes a proper threat as the run goes
        // on. A colony has to be able to learn to live with the deep
        // before the deep is worth fighting.
        let mut floor = ZORG_MIN + (world.hardship() * (ZORG_POP_CAP / 2) as f32) as u32;
        let mut slow = false;
        for (_, d) in self.discoveries.iter() {
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
        // A sealed cave breeds nothing. That is the point of sealing it.
        if world.zorg_cave_sealed {
            return;
        }
        if world.zorg_cave_center().is_none() {
            return;
        }
        let Some((x, y)) = self.find_zorg_spot(world) else {
            return;
        };
        self.spawn(world, x, y, CreatureGenesis::Zorg);
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
            CreatureGenesis::Zorg => {
                // The first ZORG. Its genes are as good a roll as any other
                // species gets, so it evolves on the same terms as everyone
                // else: it has to earn its traits.
                let mut g = [0.0f32; 32];
                for gi in g.iter_mut() {
                    *gi = rng.f32();
                }
                // The genes of a ZORG are pushed towards what it needs to
                // live on rock and poison.
                g[0] = 0.9; // sturdy
                g[1] = 0.7; // industrious
                g[3] = 0.6; // fertile
                g[5] = 0.8; // scavenger
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
                for gi in g.iter_mut() {
                    *gi = rng.f32();
                }
                // The colour of a species is the colour of the palette:
                // every colony in a run is a different colour of the
                // wheel, and the ZORGs are always purple.
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
                // The brain is deeper now, so there are more weights to
                // shake. The rate is scaled down to keep the total amount of
                // change in a generation about the same: a young animal
                // should be its mother's breed with a little of its own.
                net.mutate(&mut rng, 0.085, 0.15);
                // The shape of the mind is bred as well. Now and then a
                // young animal is born with one neuron more or less than
                // its mother had, and the shape that turns out to work is
                // the one that gets bred.
                if rng.chance(0.12) {
                    net.mutate_shape(&mut rng);
                }
                // The instincts are bred too, and this is where evolution
                // gets its teeth: the bias towards the sweet smell is
                // stronger in a line that survived on it, and weaker in a
                // line that starved. Over the generations the whole
                // species drifts towards whatever actually fed it.
                for s in net.skip.iter_mut() {
                    *s += (rng.f32() * 2.0 - 1.0) * 0.04;
                    *s = s.clamp(-1.5, 1.5);
                }
                // A child is of its mother's species, so it is of her
                // colour exactly: the colour belongs to the species, not
                // to the individual, and a ZORG's young are purple like
                // their mother.
                let color = species_team_color(&mut rng, parent.species);
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
            }            CreatureGenesis::Revival(base) => {
                let mut net = base.net.clone_net();
                // Stronger shake-up: the winning lineage diverges into several
                // new species that then compete again.
                net.mutate(&mut rng, 0.14, 0.22);
                // A revived lineage is a new species, so it takes a new
                // colour from the wheel.
                self.next_species += 1;
                let species = self.next_species;
                let color = species_team_color(&mut rng, species);
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
        //
        // A colony in crisis raises queens far more readily: without a
        // queen there is nobody to organise the colony, so the animals
        // put their effort into finding one. This is what ends the crisis.
        let crisis = in_crisis(&self.succession, species, self.tick);
        let is_zorg = species == ZORG_SPECIES;
        let role = if is_zorg && !is_child {
            // The first of the ZORGs is the queen of the cave: it holds
            // the heart of it as its base. The rest of them are soldiers
            // and workers: a ZORG colony is a war party.
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
            // The ZORGs breed soldiers: every one of them is a weapon.
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
                    // Queen's offspring: 78% worker, 10% soldier, 7% envoy,
                    // 5% queen (a rare princess). Envoys are the diplomats
                    // that carry treaties between species.
                    let r = self.rng.f32();
                    if crisis {
                        // The colony is looking for a queen.
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
                    // Worker offspring: mostly workers, rare soldier or a new
                    // princess that can found a fresh colony/lineage.
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
                    // Soldiers mostly breed soldiers, occasionally a queen.
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
                    // Envoys breed envoys most of the time, but they can also
                    // produce ordinary workers.
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

        // Base position: Queen sets it; children inherit; random spawns pick a spot
        //
        // A founding colony settles where there is something worth having:
        // near a landmark if there is one free. Two colonies that both want
        // the same crystal grove end up fighting over it.
        let (base_x, base_y) = if is_child {
            (parent_base_x, parent_base_y)
        } else if is_zorg {
            // The ZORGs have their own ground: the cave in the deep. They
            // do not settle beside a landmark, because a landmark is a
            // thing of the surface.
            //
            // The queen holds the heart of the cave; the rest keep to
            // their own patch of it.
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
                        // The heart of the cave is where the ZORG colony
                        // makes its home, with the larder a step below it.
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

        // Every founder starts with its own temperament. Two animals of the
        // same species with the same genes still behave differently.
        //
        // A child inherits the temperament of its blueprint, which is
        // already the blend of its parents.
        let personality = match genesis {
            CreatureGenesis::Child(parent) => parent.personality,
            _ => Personality::random(&mut self.rng),
        };
        // The affinity is read from the genes, so it is inherited like any
        // other trait and can change over generations.
        let affinity = Affinity::from_roll(genes[16]);
        // The young inherit what the colony has learned, but a colony
        // that keeps its traditions can also innovate: a small chance
        // of picking up a practice the parents did not have.
        let culture = match genesis {
            CreatureGenesis::Child(parent) => {
                parent.culture | self.species_culture(species)
            }
            _ => self.species_culture(species),
        };

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

        // The resistance this animal has to the diseases, worked out from
        // its genes. It is inherited, so a lineage that has been through a
        // plague becomes a plague-proof lineage.
        let immunity = immunity_from_genes(&genes);

        // A ZORG is born filthy: it carries the sickness of the deep from
        // the moment it opens its eyes. It does not suffer from it, but
        // everything that stands close to it catches it.
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
        // A newborn is a juvenile: small, slow and unable to fight. It only
        // grows into an adult as it ages.
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
            // A ZORG young is born with the sickness of the deep already
            // in it, the same one its mother carried.
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
            mount_stamina: 100.0,
            genes: child_genes,
            mutations: self.mutations + 1,
            // The traits are decoded from the mutated genes, so a species can
            // evolve a new trait between generations.
            traits: decode_traits(&child_genes),
            personality,
            stage: Stage::Juvenile,
            affinity,
            // The young inherit what the colony has learned, but a colony
            // that keeps its traditions can also innovate: a small chance
            // of picking up a practice the parents did not have.
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

    /// What the young inherit of the colony's culture.
    ///
    /// Most of the colony's practices are passed on, and there is a small
    /// chance the young pick up something new: a colony that farms for a
    /// thousand generations will eventually find a second way to live.
    fn inherited_culture(&self, rng: &mut Rng) -> u16 {
        let mut culture = self.culture;
        // Every practice is one chance in three of being lost.
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
        // A small chance of innovation: the young learn something the
        // parents did not know.
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

    /// The blueprint for a child sired with a mate.
    ///
    /// The child inherits from *both* parents: the genes are blended, the
    /// temperament is a mix of the two, and the offspring of two different
    /// animals is usually more vigorous than either of them.
    pub(crate) fn child_blueprint_with(&self, mate: &Creature) -> Creature {
        let mut child = self.child_blueprint();
        let mut rng = Rng::new(
            self.id.wrapping_mul(0x9E37_79B9) as u64
                ^ mate.id.wrapping_mul(0x85EB_CA6B) as u64
                ^ self.age as u64,
        );
        // Blend the genes of the two parents.
        for i in 0..32 {
            let mid = (self.genes[i] + mate.genes[i]) * 0.5;
            let blend = mid + (rng.f32() * 2.0 - 1.0) * 0.12;
            child.genes[i] = blend.clamp(0.0, 1.0);
        }
        // The temperament is a mix of the two.
        child.personality = Personality::blend(&self.personality, &mate.personality, &mut rng);
        // The traits are re-decoded from the blended genes.
        child.traits = decode_traits(&child.genes);
        child.affinity = Affinity::from_roll(child.genes[16]);
        // The culture of both parents is merged.
        child.culture = self.inherited_culture(&mut rng) | mate.culture;
        // The size sits between the parents, with a little drift.
        child.size = ((self.size + mate.size) * 0.5 + rng.f32() * 0.2 - 0.1).clamp(1.0, 2.4);

        // Hybrid vigour: a child of two different parents is usually
        // stronger than either of them. Two near-identical parents produce
        // a weak child, which is the price of inbreeding.
        let distance = gene_distance(&self.genes, &mate.genes);
        let vigour = (distance - 0.06).clamp(-0.25, 0.30);
        child.max_energy = (child.max_energy * (1.0 + vigour)).max(200.0);
        child.health = (child.health * (1.0 + vigour)).max(20.0);
        child
    }
}






