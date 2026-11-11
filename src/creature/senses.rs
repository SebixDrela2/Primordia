use super::*;

pub(crate) struct SpatialGrid {
    cell: usize,
    gw: usize,
    gh: usize,
    buckets: Vec<Vec<u32>>,
}

impl SpatialGrid {
    pub(crate) fn new(w: usize, h: usize, cell: usize) -> Self {
        let gw = w / cell + 2;
        let gh = h / cell + 2;
        SpatialGrid {
            cell,
            gw,
            gh,
            buckets: (0..gw * gh).map(|_| Vec::new()).collect(),
        }
    }

    pub(crate) fn clear(&mut self) {
        for b in self.buckets.iter_mut() {
            b.clear();
        }
    }

    pub(crate) fn insert(&mut self, x: f32, y: f32, idx: u32) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        if gx < self.gw && gy < self.gh {
            self.buckets[gy * self.gw + gx].push(idx);
        }
    }

    /// Visit every creature whose bucket is within `radius` cells.
    pub(crate) fn near(&self, x: f32, y: f32, radius: usize, mut f: impl FnMut(u32)) {
        let gx = (x.max(0.0) as usize) / self.cell + 1;
        let gy = (y.max(0.0) as usize) / self.cell + 1;
        let r = radius / self.cell + 1;
        let y0 = gy.saturating_sub(r);
        let y1 = (gy + r + 1).min(self.gh);
        let x0 = gx.saturating_sub(r);
        let x1 = (gx + r + 1).min(self.gw);
        for by in y0..y1 {
            for bx in x0..x1 {
                for &i in self.buckets[by * self.gw + bx].iter() {
                    f(i);
                }
            }
        }
    }
}

/// A cheap local scan of the world around a creature (radius 8, every other
/// cell). Shared between the neural senses and the steering behaviours so the
/// world is only scanned once per creature per step.
#[derive(Clone, Copy)]
pub(crate) struct LocalScan {
    /// (dx, dy, distance) of the closest interesting thing.
    pub(crate) food: (f32, f32, f32),
    pub(crate) water: (f32, f32, f32),
    pub(crate) wood: (f32, f32, f32),
    pub(crate) ore: (f32, f32, f32),
    pub(crate) hazard: (f32, f32, f32),
    pub(crate) poison: (f32, f32, f32),
    /// wood, metal, crystal, food, water, solid, empty, lava/fire, acid, plants, structures
    pub(crate) counts: [f32; 11],
}

/// Element lookup tables, so the hot scan does not match on the enum.
pub(crate) struct ElemTables {
    solid: [bool; 80],
    edible: [bool; 80],
    nutrition: [f32; 80],
    toxicity: [f32; 80],
    /// Bit flags: wood | metal | crystal | hazard | poison | plant
    classes: [u8; 80],
}

const CLASS_WOOD: u8 = 1;
const CLASS_METAL: u8 = 2;
const CLASS_CRYSTAL: u8 = 4;
const CLASS_HAZARD: u8 = 8;
const CLASS_POISON: u8 = 16;
const CLASS_PLANT: u8 = 32;

impl ElemTables {
    pub(crate) fn new() -> Self {
        let mut classes = [0u8; 80];
        let mut i = 0;
        while i < 80 {
            classes[i] = match Elem::from_u8(i as u8) {
                Elem::Wood | Elem::Roots => CLASS_WOOD,
                Elem::Metal
                | Elem::Copper
                | Elem::Tin
                | Elem::Bronze
                | Elem::Iron
                | Elem::Steel
                | Elem::Gold
                | Elem::Silver => CLASS_METAL,
                Elem::Crystal | Elem::Gem | Elem::Diamond => CLASS_CRYSTAL,
                Elem::Lava | Elem::Magma | Elem::Fire => CLASS_HAZARD,
                Elem::Acid | Elem::Poison | Elem::Chlorine => CLASS_POISON,
                Elem::Plant | Elem::Moss | Elem::Spore | Elem::Fungus => CLASS_PLANT,
                _ => 0,
            };
            i += 1;
        }
        ElemTables {
            solid: Elem::solid_table(),
            edible: Elem::edible_table(),
            nutrition: Elem::nutrition_table(),
            toxicity: Elem::toxicity_table(),
            classes,
        }
    }
}

thread_local! {
    static TABLES: ElemTables = ElemTables::new();
}

impl LocalScan {
    pub(crate) fn empty() -> Self {
        LocalScan {
            food: (0.0, 0.0, f32::MAX),
            water: (0.0, 0.0, f32::MAX),
            wood: (0.0, 0.0, f32::MAX),
            ore: (0.0, 0.0, f32::MAX),
            hazard: (0.0, 0.0, f32::MAX),
            poison: (0.0, 0.0, f32::MAX),
            counts: [0.0; 11],
        }
    }
}

pub(crate) fn scan_local(world: &World, cx: i32, cy: i32) -> LocalScan {
    TABLES.with(|t| scan_local_with(world, cx, cy, t))
}

pub(crate) fn scan_local_with(world: &World, cx: i32, cy: i32, tab: &ElemTables) -> LocalScan {
    let mut s = LocalScan::empty();
    let w = world.w as i32;
    let h = world.h as i32;
    for dy in (-8..=8).step_by(2) {
        let y = cy + dy;
        if y < 0 || y >= h {
            continue;
        }
        for dx in (-8..=8).step_by(2) {
            let x = cx + dx;
            if x < 0 || x >= w {
                continue;
            }
            let k = world.kind[(y as usize) * world.w + x as usize];
            if k == Elem::Empty as u8 {
                s.counts[6] += 1.0;
                continue;
            }
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let cls = tab.classes[k as usize];
            if tab.solid[k as usize] {
                s.counts[5] += 1.0;
            }
            if tab.nutrition[k as usize] > 0.0 && tab.edible[k as usize] {
                s.counts[3] += 1.0;
                if dist < s.food.2 {
                    s.food = (dx as f32, dy as f32, dist);
                }
            }
            if k == Elem::Water as u8 {
                s.counts[4] += 1.0;
                if dist < s.water.2 {
                    s.water = (dx as f32, dy as f32, dist);
                }
            }
            if cls & CLASS_WOOD != 0 {
                s.counts[0] += 1.0;
                if dist < s.wood.2 {
                    s.wood = (dx as f32, dy as f32, dist);
                }
            }
            if cls & (CLASS_METAL | CLASS_CRYSTAL) != 0 {
                if cls & CLASS_METAL != 0 {
                    s.counts[1] += 1.0;
                }
                if cls & CLASS_CRYSTAL != 0 {
                    s.counts[2] += 1.0;
                }
                if dist < s.ore.2 {
                    s.ore = (dx as f32, dy as f32, dist);
                }
            }
            if cls & CLASS_HAZARD != 0 {
                s.counts[7] += 1.0;
                if dist < s.hazard.2 {
                    s.hazard = (dx as f32, dy as f32, dist);
                }
            }
            if cls & CLASS_POISON != 0 {
                s.counts[8] += 1.0;
                if dist < s.poison.2 {
                    s.poison = (dx as f32, dy as f32, dist);
                }
            }
            if cls & CLASS_PLANT != 0 {
                s.counts[9] += 1.0;
            }
        }
    }
    s
}


impl CreatureSystem {
    pub(crate) fn sense(
        agents: &[(u32, f32, f32, u32, u8, f32)],
        grid: &SpatialGrid,
        c: &Creature,
        world: &World,
        day_night: f32,
        temperature: f32,
        season: u8,
        scan: &LocalScan,
        shots: &[(f32, f32, u32)],
        own_turrets: u32,
        enemy_turrets: u32,
        species_population: u32,
        rival_population: u32,
        input: &mut [f32],
    ) {
        input.fill(0.0);
        let cx = c.x.round() as i32;
        let cy = c.y.round() as i32;

        // 8 directions × 6 channels = 48 (food, hazard, water, solid, metal, crystal)
        for (d, (dx, dy)) in DIRS8.iter().enumerate() {
            let mut food = 0.0;
            let mut hazard = 0.0;
            let mut water = 0.0;
            let mut solid = 0.0;
            let mut metal = 0.0;
            let mut crystal = 0.0;
            for step in 1..=8i32 {
                let sx = cx + dx * step;
                let sy = cy + dy * step;
                if sx < 0 || sy < 0 || sx >= world.w as i32 || sy >= world.h as i32 {
                    continue;
                }
                let k = world.kind[sy as usize * world.w + sx as usize];
                if k == Elem::Empty as u8 {
                    continue;
                }
                let e = Elem::from_u8(k);
                let w = 1.0 / step as f32;
                if e.nutrition() > 0.0 { food += w; }
                if e.toxicity() > 0.0 { hazard += w; }
                if k == Elem::Water as u8 { water += w; }
                if e.is_solid() { solid += w; }
                if matches!(e, Elem::Metal | Elem::Copper | Elem::Gold | Elem::Iron | Elem::Silver | Elem::Wire | Elem::Circuit) { metal += w; }
                if matches!(e, Elem::Crystal | Elem::Gem | Elem::Diamond | Elem::Mana) { crystal += w; }
            }
            let base = d * 6;
            input[base] = food.min(1.0);
            input[base + 1] = hazard.min(1.0);
            input[base + 2] = water.min(1.0);
            input[base + 3] = solid.min(1.0);
            input[base + 4] = metal.min(1.0);
            input[base + 5] = crystal.min(1.0);
        }

        // Creature proximity: one pass over the colony gives the nearest
        // neighbour, the nearest rival, the nearest ally and the local
        // friend/foe counts.
        let mut nearest_dist = f32::MAX;
        let mut nearest_dx = 0.0;
        let mut nearest_dy = 0.0;
        let mut others = 0.0;
        let mut same = 0.0;
        let mut enemy = 0.0;
        let (mut e_dx, mut e_dy, mut e_dist) = (0.0f32, 0.0f32, f32::MAX);
        let (mut a_dx, mut a_dy, mut a_dist) = (0.0f32, 0.0f32, f32::MAX);
        let mut e_is_queen = 0.0;
        let mut e_is_soldier = 0.0;
        let mut e_hp = 0.0;
        let mut a_is_queen = 0.0;
        let mut enemies_close = 0.0f32;
        let mut enemies_far = 0.0f32;
        grid.near(c.x, c.y, 20, |idx| {
            let a = &agents[idx as usize];
            let (id, ox, oy, ospecies, orole, ohp) =
                (a.0, a.1, a.2, a.3, a.4, a.5);
            if id == c.id {
                return;
            }
            let dx = ox - c.x;
            let dy = oy - c.y;
            let d2 = dx * dx + dy * dy;
            if d2 < 400.0 {
                others += 1.0;
                if ospecies == c.species {
                    same += 1.0;
                } else {
                    enemy += 1.0;
                }
            }
            if d2 < nearest_dist {
                nearest_dist = d2;
                nearest_dx = dx;
                nearest_dy = dy;
            }
            let d = d2.sqrt();
            if ospecies == c.species {
                if d < a_dist {
                    a_dist = d;
                    a_dx = dx;
                    a_dy = dy;
                    a_is_queen = if orole == Role::Queen as u8 { 1.0 } else { 0.0 };
                }
            } else {
                if d < e_dist {
                    e_dist = d;
                    e_dx = dx;
                    e_dy = dy;
                    e_hp = ohp;
                    e_is_queen = if orole == Role::Queen as u8 { 1.0 } else { 0.0 };
                    e_is_soldier = if orole == Role::Soldier as u8 { 1.0 } else { 0.0 };
                }
                if d < 10.0 {
                    enemies_close += 1.0;
                } else if d < 25.0 {
                    enemies_far += 1.0;
                }
            }
        });

        // Internal state (16) - slots 32-47
        input[32] = (c.energy / c.max_energy).clamp(0.0, 1.0);
        input[33] = (c.health / (if c.role == Role::Soldier { 150.0 } else { 100.0 })).clamp(0.0, 1.0);
        input[34] = (c.age as f32 / MAX_AGE as f32).clamp(0.0, 1.0);
        input[35] = (c.vx / 2.2).clamp(-1.0, 1.0);
        input[36] = (c.vy / 2.2).clamp(-1.0, 1.0);
        input[37] = (nearest_dx / 20.0f32).clamp(-1.0f32, 1.0f32);
        input[38] = (nearest_dy / 20.0f32).clamp(-1.0f32, 1.0f32);
        input[39] = (others / 10.0f32).clamp(0.0, 1.0);
        input[40] = c.hunger / 100.0;
        input[41] = c.thirst / 100.0;
        input[42] = c.stamina / c.max_stamina;
        input[43] = (c.mana / c.max_mana).clamp(0.0, 1.0);
        input[44] = (c.craft_cooldown as f32 / 50.0).clamp(0.0, 1.0);
        input[45] = (c.spell_cooldown as f32 / 60.0).clamp(0.0, 1.0);
        input[46] = if c.resting { 1.0 } else { 0.0 };
        input[47] = (c.carried / CARRY_CAP).clamp(0.0, 1.0);

        // Colony awareness (8) - slots 48-55
        let base_dx = c.base_x - c.x;
        let base_dy = c.base_y - c.y;
        let base_dist = (base_dx * base_dx + base_dy * base_dy).sqrt().max(1.0);
        input[48] = (base_dx / base_dist).clamp(-1.0, 1.0);
        input[49] = (base_dy / base_dist).clamp(-1.0, 1.0);
        input[50] = 1.0 - (base_dist / 100.0).min(1.0);
        // Same-species count nearby
        input[51] = (same / 10.0f32).clamp(0.0f32, 1.0f32);
        input[52] = (enemy / 10.0f32).clamp(0.0f32, 1.0f32);
        input[53] = match c.role { Role::Queen => 1.0, _ => 0.0 };
        input[54] = match c.role { Role::Soldier => 1.0, _ => 0.0 };
        input[55] = day_night;

        // Role one-hot - slots 56-62
        input[56] = if c.role == Role::Worker { 1.0 } else { 0.0 };
        input[57] = if c.role == Role::Priest { 1.0 } else { 0.0 };
        input[58] = if c.role == Role::Merchant { 1.0 } else { 0.0 };
        input[59] = if c.role == Role::Builder { 1.0 } else { 0.0 };
        input[60] = if c.role == Role::Explorer { 1.0 } else { 0.0 };
        input[61] = if c.role == Role::Shaman { 1.0 } else { 0.0 };
        input[62] = temperature / 40.0;

        // Tool & tech state (10) - slots 63-72
        input[63] = (c.tool as u8 as f32 / 14.0).clamp(0.0, 1.0);
        input[64] = (c.tool_durability as f32 / 2000.0).clamp(0.0, 1.0);
        input[65] = (c.tech_level as f32 / 4.0).clamp(0.0, 1.0);
        input[66] = (c.inventory.iter().sum::<u16>() as f32 / 2000.0).clamp(0.0, 1.0);
        input[67] = (c.craft_cooldown as f32 / 70.0).clamp(0.0, 1.0);
        input[68] = (c.inventory[0] as f32 / 50.0).clamp(0.0, 1.0); // stone
        input[69] = (c.inventory[1] as f32 / 50.0).clamp(0.0, 1.0); // metal
        input[70] = (c.inventory[2] as f32 / 30.0).clamp(0.0, 1.0); // copper
        input[71] = (c.inventory[3] as f32 / 20.0).clamp(0.0, 1.0); // gold
        input[72] = (c.inventory[4] as f32 / 20.0).clamp(0.0, 1.0); // crystal

        // Magic state (8) - slots 73-80
        input[73] = (c.mana / c.max_mana).clamp(0.0, 1.0);
        input[74] = (c.active_spell as u8 as f32 / 10.0).clamp(0.0, 1.0);
        input[75] = if c.known_spells > 0 { 1.0 } else { 0.0 };
        input[76] = (c.spell_cd[0] as f32 / 60.0).clamp(0.0, 1.0);
        input[77] = (c.spell_cd[1] as f32 / 60.0).clamp(0.0, 1.0);
        input[78] = (c.spell_cd[2] as f32 / 60.0).clamp(0.0, 1.0);
        input[79] = (c.spell_cd[3] as f32 / 60.0).clamp(0.0, 1.0);
        input[80] = (c.spell_cd[4] as f32 / 60.0).clamp(0.0, 1.0);

        // Mount/vehicle state (4) - slots 81-84
        input[81] = if c.mount_kind != MountKind::None { 1.0 } else { 0.0 };
        input[82] = c.mount_stamina / 100.0;
        input[83] = if c.vehicle.is_some() { 1.0 } else { 0.0 };
        input[84] = (c.reputation as f32 / 100.0).clamp(-1.0, 1.0);

        // Memory blackboard (8) - slots 85-92
        for i in 0..8 { input[85 + i] = c.memory[i]; }

        // Resource inventory detailed (16) - slots 93-108
        for i in 0..16 {
            input[93 + i] = (c.inventory[i] as f32 / 100.0).clamp(0.0, 1.0);
        }

        // Disease & social (8) - slots 109-116
        input[109] = match c.disease { Disease::None => 0.0, _ => 1.0 };
        input[110] = (c.disease_timer as f32 / 300.0).clamp(0.0, 1.0);
        input[111] = (c.loyalty as f32 / 100.0).clamp(-1.0, 1.0);
        input[112] = (c.devotion as f32 / 100.0).clamp(0.0, 1.0);
        input[113] = (c.mood as u8 as f32 / 9.0).clamp(0.0, 1.0);
        input[114] = (c.mood_timer as f32 / 180.0).clamp(0.0, 1.0);
        input[115] = (c.trade_partner.map_or(0, |p| p) as f32 / 1000.0).clamp(0.0, 1.0);
        input[116] = (c.squad_id as f32 / 10.0).clamp(0.0, 1.0);

        // Genetics (16) - slots 117-132
        for i in 0..16 { input[117 + i] = c.genes[i]; }

        // Offspring & kills (4) - slots 133-136
        input[133] = (c.offspring as f32 / 20.0).clamp(0.0, 1.0);
        input[134] = (c.kills as f32 / 20.0).clamp(0.0, 1.0);
        input[135] = (c.generation as f32 / 50.0).clamp(0.0, 1.0);
        input[136] = (c.mutations as f32 / 20.0).clamp(0.0, 1.0);

        // Season (4) - slots 137-140
        input[137] = if season == 0 { 1.0 } else { 0.0 };
        input[138] = if season == 1 { 1.0 } else { 0.0 };
        input[139] = if season == 2 { 1.0 } else { 0.0 };
        input[140] = if season == 3 { 1.0 } else { 0.0 };

        // ---- Extended senses (160-223) ----------------------------------
        // Enemy awareness (160-171)
        input[160] = (e_dx / 30.0).clamp(-1.0, 1.0);
        input[161] = (e_dy / 30.0).clamp(-1.0, 1.0);
        input[162] = (1.0 - (e_dist / 60.0).min(1.0)).max(0.0);
        input[163] = e_is_queen;
        input[164] = e_is_soldier;
        input[165] = (e_hp / 150.0).clamp(0.0, 1.0);
        input[166] = (enemies_close / 6.0).clamp(0.0, 1.0);
        input[167] = (enemies_far / 10.0).clamp(0.0, 1.0);
        input[168] = (a_dx / 30.0).clamp(-1.0, 1.0);
        input[169] = (a_dy / 30.0).clamp(-1.0, 1.0);
        input[170] = (1.0 - (a_dist / 60.0).min(1.0)).max(0.0);
        input[171] = a_is_queen;

        // Threats (172-181)
        let mut shot_dx = 0.0;
        let mut shot_dy = 0.0;
        let mut shots_near = 0.0f32;
        for &(sx, sy, s_species) in shots {
            if s_species == c.species {
                continue;
            }
            let dx = sx - c.x;
            let dy = sy - c.y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 20.0 {
                shots_near += 1.0;
                shot_dx += dx;
                shot_dy += dy;
            }
        }
        input[172] = (shots_near / 5.0).clamp(0.0, 1.0);
        input[173] = (shot_dx / 30.0).clamp(-1.0, 1.0);
        input[174] = (shot_dy / 30.0).clamp(-1.0, 1.0);
        input[175] = (scan.counts[7] / 6.0).clamp(0.0, 1.0);
        input[176] = (scan.hazard.0 / 16.0).clamp(-1.0, 1.0);
        input[177] = (scan.hazard.1 / 16.0).clamp(-1.0, 1.0);
        input[178] = (scan.counts[8] / 6.0).clamp(0.0, 1.0);
        input[179] = (scan.poison.0 / 16.0).clamp(-1.0, 1.0);
        input[180] = (scan.poison.1 / 16.0).clamp(-1.0, 1.0);
        let max_hp = if c.role == Role::Soldier { 150.0 } else { 100.0 };
        input[181] = if c.health < max_hp * 0.35 { 1.0 } else { 0.0 };

        // Foraging (182-193)
        input[182] = (scan.food.0 / 16.0).clamp(-1.0, 1.0);
        input[183] = (scan.food.1 / 16.0).clamp(-1.0, 1.0);
        input[184] = (1.0 - (scan.food.2 / 24.0).min(1.0)).max(0.0);
        input[185] = (scan.water.0 / 16.0).clamp(-1.0, 1.0);
        input[186] = (scan.water.1 / 16.0).clamp(-1.0, 1.0);
        input[187] = (1.0 - (scan.water.2 / 24.0).min(1.0)).max(0.0);
        input[188] = (scan.wood.0 / 16.0).clamp(-1.0, 1.0);
        input[189] = (scan.wood.1 / 16.0).clamp(-1.0, 1.0);
        input[190] = (1.0 - (scan.wood.2 / 24.0).min(1.0)).max(0.0);
        input[191] = (scan.ore.0 / 16.0).clamp(-1.0, 1.0);
        input[192] = (scan.ore.1 / 16.0).clamp(-1.0, 1.0);
        input[193] = (1.0 - (scan.ore.2 / 24.0).min(1.0)).max(0.0);

        // Resources & colony (194-203)
        input[194] = (scan.counts[0] / 20.0).clamp(0.0, 1.0);
        input[195] = (scan.counts[1] / 10.0).clamp(0.0, 1.0);
        input[196] = (scan.counts[2] / 10.0).clamp(0.0, 1.0);
        input[197] = (scan.counts[3] / 15.0).clamp(0.0, 1.0);
        input[198] = (scan.counts[4] / 10.0).clamp(0.0, 1.0);
        input[199] = (own_turrets as f32 / 6.0).clamp(0.0, 1.0);
        input[200] = (enemy_turrets as f32 / 6.0).clamp(0.0, 1.0);
        input[201] = (scan.counts[10] / 10.0).clamp(0.0, 1.0);
        input[202] = (base_dist / 120.0).min(1.0);
        input[203] = (base_dx / 60.0).clamp(-1.0, 1.0);

        // Environment (204-211)
        let surface_y = world.surface_y(cx);
        input[204] = ((cy - surface_y) as f32 / 60.0).clamp(0.0, 1.0);
        input[205] = if world.get(cx, cy - 3) == Elem::Empty { 1.0 } else { 0.0 };
        input[206] = (scan.counts[5] / 40.0).clamp(0.0, 1.0);
        input[207] = (scan.counts[6] / 40.0).clamp(0.0, 1.0);
        input[208] = (scan.counts[4] / 10.0).clamp(0.0, 1.0);
        input[209] = ((scan.counts[7] + scan.counts[8]) / 8.0).clamp(0.0, 1.0);
        input[210] = (scan.counts[3] / 15.0).clamp(0.0, 1.0);
        input[211] = (scan.counts[9] / 20.0).clamp(0.0, 1.0);

        // Personal readiness (212-221)
        input[212] = (c.fire_cooldown as f32 / 60.0).clamp(0.0, 1.0);
        input[213] = if c.fire_cooldown == 0 { 1.0 } else { 0.0 };
        input[214] = if c.tool == Tool::Bow { 1.0 } else { 0.0 };
        input[215] = if c.tool == Tool::Staff { 1.0 } else { 0.0 };
        input[216] = if matches!(c.tool, Tool::Pickaxe | Tool::Drill) { 1.0 } else { 0.0 };
        input[217] = if c.tool == Tool::Hammer { 1.0 } else { 0.0 };
        input[218] = if c.inventory[6] >= 8 && c.tech_level >= 1 { 1.0 } else { 0.0 };
        input[219] = (c.build_cooldown as f32 / 10.0).clamp(0.0, 1.0);
        input[220] = if c.hunger > 70.0 { 1.0 } else { 0.0 };
        input[221] = if c.thirst > 70.0 { 1.0 } else { 0.0 };

        // Colony strength (222-223)
        input[222] = (species_population as f32 / 40.0).clamp(0.0, 1.0);
        input[223] = (rival_population as f32 / 40.0).clamp(0.0, 1.0);
    }

}
