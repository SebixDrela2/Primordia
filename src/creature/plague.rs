use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plague {

    Flu,

    Plague,

    Poison,

    Curse,

    Infection,

    Parasite,

    Madness,
}

impl Plague {

    pub fn all() -> [Plague; 7] {
        [
            Plague::Flu,
            Plague::Plague,
            Plague::Poison,
            Plague::Curse,
            Plague::Infection,
            Plague::Parasite,
            Plague::Madness,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Plague::Flu => "flu",
            Plague::Plague => "plague",
            Plague::Poison => "poison",
            Plague::Curse => "curse",
            Plague::Infection => "infection",
            Plague::Parasite => "parasite",
            Plague::Madness => "madness",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Plague::Flu => "catches easily, weakens and burns out",
            Plague::Plague => "catches easily and kills",
            Plague::Poison => "enters through wounds",
            Plague::Curse => "wears the victim down",
            Plague::Infection => "spreads through the flesh and kills slowly",
            Plague::Parasite => "drinks the host's energy",
            Plague::Madness => "drives the host to attack its own kind",
        }
    }

    pub fn contagiousness(self) -> f32 {
        match self {
            Plague::Flu => 0.010,
            Plague::Plague => 0.014,
            Plague::Poison => 0.0,
            Plague::Curse => 0.0,
            Plague::Infection => 0.004,
            Plague::Parasite => 0.002,
            Plague::Madness => 0.003,
        }
    }

    pub fn lethality(self) -> f32 {
        match self {
            Plague::Flu => 0.0,
            Plague::Plague => 0.30,
            Plague::Poison => 0.12,
            Plague::Curse => 0.08,
            Plague::Infection => 0.10,
            Plague::Parasite => 0.0,
            Plague::Madness => 0.0,
        }
    }

    pub fn drain(self) -> f32 {
        match self {
            Plague::Flu => 0.10,
            Plague::Plague => 0.22,
            Plague::Poison => 0.14,
            Plague::Curse => 0.12,
            Plague::Infection => 0.10,
            Plague::Parasite => 0.30,
            Plague::Madness => 0.08,
        }
    }

    pub fn slowness(self) -> f32 {
        match self {
            Plague::Flu => 0.7,
            Plague::Plague => 0.8,
            Plague::Poison => 0.85,
            Plague::Curse => 0.9,
            Plague::Infection => 0.8,
            Plague::Parasite => 0.9,
            Plague::Madness => 0.85,
        }
    }

    pub fn maddens(self) -> bool {
        matches!(self, Plague::Madness)
    }

    pub fn debility(self) -> f32 {
        match self {
            Plague::Flu => 0.35,
            Plague::Plague => 1.0,
            Plague::Poison => 0.55,
            Plague::Curse => 0.45,
            Plague::Infection => 0.7,
            Plague::Parasite => 0.8,
            Plague::Madness => 0.5,
        }
    }

    pub fn duration(self) -> u16 {
        match self {
            Plague::Flu => 420,
            Plague::Plague => 900,
            Plague::Poison => 300,
            Plague::Curse => 360,
            Plague::Infection => 700,
            Plague::Parasite => 1200,
            Plague::Madness => 480,
        }
    }

    pub fn is_active(self) -> bool {
        true
    }
}

impl Creature {

    pub fn apply_disease(&mut self) -> f32 {
        if self.disease == Disease::None {
            return 0.0;
        }

        if self.faction == Faction::Zorg {
            return 0.0;
        }
        let plague = Plague::from(self.disease);
        let harm = plague.lethality();
        self.health -= harm;
        self.energy = (self.energy - plague.drain()).max(0.0);
        harm
    }

    pub fn disease_speed_factor(&self) -> f32 {
        if self.disease == Disease::None {
            return 1.0;
        }
        if self.faction == Faction::Zorg {
            return 1.0;
        }
        Plague::from(self.disease).slowness()
    }

    pub fn is_mad(&self) -> bool {
        self.disease != Disease::None && Plague::from(self.disease).maddens()
    }

    pub fn resistance(&self, disease: Disease) -> u8 {
        self.immunity[disease as usize]
    }

    pub fn catch_chance(&self, disease: Disease) -> f32 {
        match self.resistance(disease) {
            0 => 1.0,
            1 => 0.35,
            _ => 0.0,
        }
    }
}

pub const IMMUNITY_GENES: [(usize, Disease); 4] = [
    (23, Disease::Flu),
    (24, Disease::Plague),
    (25, Disease::Infection),
    (26, Disease::Madness),
];

pub fn immunity_from_genes(genes: &[f32; 32]) -> [u8; 8] {
    let mut immunity = [0u8; 8];
    for (gene, disease) in IMMUNITY_GENES {
        let g = genes[gene];
        if g > 0.74 {
            immunity[disease as usize] = 2;
        } else if g > 0.58 {
            immunity[disease as usize] = 1;
        }
    }
    immunity
}

impl From<Disease> for Plague {
    fn from(d: Disease) -> Plague {
        match d {
            Disease::Flu => Plague::Flu,
            Disease::Plague => Plague::Plague,
            Disease::Poison => Plague::Poison,
            Disease::Curse => Plague::Curse,
            Disease::Infection => Plague::Infection,
            Disease::Parasite => Plague::Parasite,
            Disease::Madness => Plague::Madness,
            Disease::None => Plague::Flu,
        }
    }
}

impl CreatureSystem {

    pub(crate) fn maybe_outbreak(&mut self, world: &World) {
        if self.tick % 400 != 0 || self.list.len() <= 20 {
            return;
        }
        let sick = self
            .list
            .iter()
            .filter(|c| c.disease != Disease::None)
            .count();

        let mut warren = 0.0f32;
        for c in self.list.iter() {
            if world.landmark_bonus(c.x, c.y).plague {
                warren += 1.0;
            }
        }
        let breed = 0.25 + warren / (self.list.len() as f32).max(1.0) * 1.5;

        if sick * 6 >= self.list.len() || !self.rng.chance(breed) {
            return;
        }

        let victim = if warren > 0.0 && self.rng.chance(0.6) {
            let mut best: Option<usize> = None;
            for (i, c) in self.list.iter().enumerate() {
                if c.disease != Disease::None {
                    continue;
                }
                if world.landmark_bonus(c.x, c.y).plague {
                    best = Some(i);
                    break;
                }
            }
            match best {
                Some(i) => i,
                None => self.rng.range(self.list.len() as u32) as usize,
            }
        } else {
            self.rng.range(self.list.len() as u32) as usize
        };
        if self.list[victim].disease != Disease::None {
            return;
        }
        const OPTIONS: [Disease; 5] = [
            Disease::Flu,
            Disease::Plague,
            Disease::Infection,
            Disease::Parasite,
            Disease::Madness,
        ];
        let pick = OPTIONS[self.rng.range(OPTIONS.len() as u32) as usize];
        self.list[victim].disease = pick;
        self.list[victim].disease_timer = Plague::from(pick).duration();
        self.plagues_started += 1;
        self.events.push(SimEvent::Plague {
            x: self.list[victim].x,
            y: self.list[victim].y,
            color: self.list[victim].team_color,
        });
    }

    pub(crate) fn spread_diseases(&mut self, grid: &SpatialGrid) {
        if self.tick % 3 != 0 {
            return;
        }

        let sick: Vec<(usize, Disease, f32)> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, c)| c.disease != Disease::None)
            .map(|(i, c)| {
                (
                    i,
                    c.disease,
                    Plague::from(c.disease).contagiousness() * 3.0,
                )
            })
            .collect();
        if sick.is_empty() {
            return;
        }

        let mut infected: Vec<(usize, Disease)> = Vec::new();
        for (i, disease, chance) in sick {
            if chance <= 0.0 {
                continue;
            }
            let (sx, sy, sspecies) = {
                let c = &self.list[i];
                (c.x, c.y, c.species)
            };
            grid.near(sx, sy, 14, |idx| {
                let j = idx as usize;
                if j >= self.list.len() || j == i {
                    return;
                }
                let other = &self.list[j];

                if other.species != sspecies && chance < 0.008 {
                    return;
                }
                if other.disease != Disease::None || other.health <= 0.0 {
                    return;
                }
                let d = ((other.x - sx).powi(2) + (other.y - sy).powi(2)).sqrt();
                if d > 14.0 {
                    return;
                }

                let catch = other.catch_chance(disease);
                if catch <= 0.0 {
                    return;
                }
                if self.rng.chance(chance * catch) {
                    infected.push((j, disease));
                }
            });
        }
        for (j, disease) in infected {
            self.list[j].disease = disease;
            self.list[j].disease_timer = Plague::from(disease).duration();
        }
    }

    pub(crate) fn spread_zorg_plague(&mut self, grid: &SpatialGrid) {
        if self.tick % 10 != 0 {
            return;
        }

        let carriers: Vec<(f32, f32, Disease)> = self
            .list
            .iter()
            .filter(|c| c.faction == Faction::Zorg && c.disease != Disease::None)
            .map(|c| (c.x, c.y, c.disease))
            .collect();
        if carriers.is_empty() {
            return;
        }
        let mut infected: Vec<(usize, Disease)> = Vec::new();

        let mut purge = 1.0f32;
        for (_, d) in self.discoveries.iter() {
            if *d & discovery_bits::PURGE != 0 {
                purge *= 0.5;
            }
        }

        let mut near: Vec<usize> = Vec::new();
        for (sx, sy, disease) in carriers {
            near.clear();
            grid.near(sx, sy, 9, |idx| near.push(idx as usize));
            for &j in near.iter() {
                if j >= self.list.len() {
                    continue;
                }
                let other = &self.list[j];

                if other.faction == Faction::Zorg
                    || other.disease != Disease::None
                    || other.health <= 0.0
                {
                    continue;
                }
                let d = ((other.x - sx).powi(2) + (other.y - sy).powi(2)).sqrt();
                if d > 9.0 {
                    continue;
                }

                let catch = other.catch_chance(disease);
                if catch <= 0.0 {
                    continue;
                }
                if self.rng.chance(0.05 * catch * purge) {
                    infected.push((j, disease));
                }
            }
        }
        for (j, disease) in infected {
            self.list[j].disease = disease;
            self.list[j].disease_timer = Plague::from(disease).duration();
            self.plagues_started += 1;
        }
    }
}
