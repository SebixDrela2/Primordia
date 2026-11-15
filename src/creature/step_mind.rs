use super::{Creature, Role, Elem, Faction, Culture, outputs, SimEvent, Bomb, discovery_bits, trait_bits, Disease, Discovery, Tool, MountKind, energy_cost, DIRS4, REPRO_SOFT_CAP, ZORG_POP_CAP, SteerContext, SpellContext, scan_percept, write_percept, write_body, write_kin, act_steer, act_move, act_eat, act_mine, act_build, act_craft, act_combat, switch_spell, cast_spell_effects, zorg_filth, find_diplomatic_partner, desired_action, strongest_direction, Treaty};

use super::step_creature::{Lessons, StepCtx, StepOut, Turn};

/// What the creature has been doing, as the colony counts it.
fn record_activity(c: &Creature, out: &mut StepOut<'_>) {
    let mut bits = 0u32;
    if out.tick.net_act[2] > 0.05 {
        bits |= crate::creature::technology::activity::EAT;
    }
    if out.tick.net_act[6] > 0.25 {
        bits |= crate::creature::technology::activity::CRAFT;
    }
    if out.tick.net_act[5] > 0.2 {
        bits |= crate::creature::technology::activity::BUILD;
    }
    if out.tick.net_act[9] > 0.4 {
        bits |= crate::creature::technology::activity::TRADE;
    }
    if out.tick.net_act[12] > 0.35 {
        bits |= crate::creature::technology::activity::MINE;
    }
    if out.tick.net_act[7] > 0.5 {
        bits |= crate::creature::technology::activity::CAST;
    }
    if out.tick.net_act[4] > 0.3 {
        bits |= crate::creature::technology::activity::ATTACK;
    }
    if bits != 0 {
        match out.activity.iter_mut().find(|(sp, _)| *sp == c.species) {
            Some(e) => e.1 |= bits,
            None => out.activity.push((c.species, bits)),
        }
    }
}

/// The direction the creature most wants to go.
fn choose_direction(c: &mut Creature) {
    let mut best_dir = 0usize;
    let mut best_desire = f32::MIN;
    for d in 0..outputs::DESIRE_COUNT {
        let v = c.out_cache[outputs::DESIRE + d];
        if v > best_desire {
            best_desire = v;
            best_dir = d;
        }
    }
    c.desire_dir = best_dir;
}

/// The mind: what the creature sees, thinks and decides.
pub(super) fn think(
    c: &mut Creature,
    ctx: &StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
) -> [f32; super::OUTPUTS] {
    let think = ctx.tick.is_multiple_of(2) || !c.brain_ready;
    if think {
        c.percept_cache = scan_percept(
            &*ctx.world,
            c.x.round() as i32,
            c.y.round() as i32,
            c.species,
            ctx.agents,
            ctx.grid,
        );

        write_percept(&c.percept_cache, &mut out.tick.input_buf);
        write_body(
            c,
            &*ctx.world,
            ctx.day_night,
            ctx.temperature,
            &mut out.tick.input_buf,
        );

        out.tick.input_buf
            [crate::creature::senses::BODY_BASE + crate::creature::senses::BODY_CROWDED] =
            (turn.crowd as f32 / 16.0).clamp(0.0, 1.0);
        write_kin(c, ctx.agents, ctx.grid, &mut out.tick.input_buf);
        c.net
            .forward_into(&out.tick.input_buf, &mut out.tick.net_act, &mut out.tick.net_next);
        c.out_cache.copy_from_slice(&out.tick.net_act);
        c.brain_ready = true;

        record_activity(c, out);
        choose_direction(c);
    }

    let mut decisions = [0.0f32; super::OUTPUTS];
    decisions.copy_from_slice(&c.out_cache);
    decisions
}

/// The talks: what the creature wants to say to a stranger.
pub(super) fn talks(
    c: &mut Creature,
    ctx: &StepCtx<'_>,
    out: &mut StepOut<'_>,
    index: usize,
    decisions: &[f32],
) {
    let wants_talks = decisions[outputs::NEGOTIATE] > 0.3;

    let wants_talks = wants_talks && c.faction != Faction::Zorg;
    out.tick.diplo_willing[index] = if wants_talks { 1.0 } else { 0.0 };
    if wants_talks && c.diplo_cooldown == 0 {
        if let Some(partner_idx) = find_diplomatic_partner(c, ctx.grid, ctx.agents) {
            let partner_species = ctx.agents[partner_idx].3;
            let action =
                desired_action(&*ctx.diplomacy, decisions, c.species, partner_species);
            out.tick.proposals.push((index, partner_idx, action));
        }
    }
}

/// The step itself: where the creature goes and how fast.
pub(super) fn steer(
    c: &mut Creature,
    ctx: &mut StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    decisions: &[f32],
) {
    let place = turn.place;
    let ds = turn.ds;
    let volcano = ctx.world.volcano().map(|(vx, vy)| (vx as f32, vy as f32));
    let (steer_x, steer_y, speed_mult_extra) = act_steer(
        c,
        decisions,
        &SteerContext {
            agents: ctx.agents,
            grid: ctx.grid,
            foreign_bases: ctx.centroids,
            ds: &ds,
            fear: ctx.fear,
            weather: ctx.world.weather,
            place: &place,
            volcano,
        },
    );

    if ctx.world.volcano_state() == crate::world::Volcano::Erupting {
        if let Some((vx, vy)) = volcano {
            let dx = c.x - vx;
            let dy = c.y - vy;
            let d2 = dx * dx + dy * dy;
            if d2 < 900.0 {
                c.health -= 3.0;
            }
        }
    }

    let speed_cap = c.speed_cap() * ctx.world.weather.speed_factor() * c.disease_speed_factor();

    let speed_cap = if c.knows_tech(Discovery::Navigation) {
        speed_cap * 1.15
    } else {
        speed_cap
    };

    let climbing = if c.traits & trait_bits::CLIMBER != 0
        && ctx.world.get(c.x.round() as i32, c.y.round() as i32) == Elem::Empty
    {
        1.25
    } else {
        1.0
    };
    let speed_cap = speed_cap * climbing * turn.daylight;

    c.vx = (steer_x * 1.0 + decisions[0] * 0.25).clamp(-speed_cap, speed_cap);
    c.vy = (steer_y * 1.0 + decisions[1] * 0.25 + 0.2).clamp(-speed_cap, speed_cap);

    act_move(&mut *ctx.world, c, speed_mult_extra);

    {
        let depth = (c.y.round() as i32 - ctx.world.surface_y(c.x.round() as i32)).max(0) as u32;
        if depth > 0 {
            match out.deepest.iter_mut().find(|(sp, _)| *sp == c.species) {
                Some(e) => e.1 = e.1.max(depth),
                None => out.deepest.push((c.species, depth)),
            }
        }
    }

    if c.faction == Faction::Zorg && out.rng.chance(0.05) {
        let fx = c.last_x.round() as i32;
        let fy = c.last_y.round() as i32;
        if ctx.world.get(fx, fy) == Elem::Empty {
            let filth = zorg_filth(out.rng.range(8));
            ctx.world.set_xy(fx, fy, filth);
        }
    }
}

/// The work: eating, mining, building and making.
pub(super) fn forage(
    c: &mut Creature,
    ctx: &mut StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    index: usize,
    decisions: &[f32],
) {
    if act_eat(&mut *ctx.world, c, decisions, &*ctx.diplomacy) {
        turn.flags |= Turn::ATE;
    }
    act_mine(&mut *ctx.world, c, decisions, turn.company);

    let discoveries = ctx.discoveries.get(c.species as usize).copied().unwrap_or(0);
    let (built, built_count) = act_build(
        &mut *ctx.world,
        c,
        decisions,
        &mut *out.structures,
        &mut *out.events,
        turn.company,
        discoveries & discovery_bits::SANCTIFY != 0,
    );
    if built && turn.crisis > 0.0 {
        c.build_cooldown = c.build_cooldown.max(60);
    }
    if built {
        turn.flags |= Turn::BUILT;
    }
    out.tick.structures_total += built_count;

    act_craft(
        c,
        decisions,
        &mut *out.events,
        turn.has(Turn::NEAR_WORKSHOP),
        turn.place.has(crate::world::Landmark::OldRuin),
        discoveries,
    );

    if turn.has(Turn::BUILT) {
        out.tick.learned.push((index, Culture::Architecture));
    }
    if c.tool != Tool::None {
        out.tick.learned.push((index, Culture::Craft));
    }
    if decisions[12] > 0.35 {
        out.tick.learned.push((index, Culture::Mining));
    }
}

/// The fight: the teeth, the bow and the spells.
pub(super) fn fight(
    c: &mut Creature,
    ctx: &StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    index: usize,
    decisions: &[f32],
) {
    let shots_before = out.tick.shots.len();
    let attacked = act_combat(
        c,
        decisions,
        ctx.grid,
        ctx.agents,
        &mut out.tick.shots,
        &mut *out.events,
        &*ctx.diplomacy,
    );
    if attacked {
        turn.flags |= Turn::ATTACKED;
    }
    out.tick.shots_fired += (out.tick.shots.len() - shots_before) as u32;

    if turn.has(Turn::ATTACKED) {
        out.tick.learned.push((index, Culture::Warfare));
    }
    if turn.has(Turn::ATE) {
        if c.last_meal == Elem::Corpse {
            out.tick.learned.push((index, Culture::Hunting));
        }
        if c.inventory[10] > 0 {
            out.tick.learned.push((index, Culture::Cultivation));
        }
    }

    if decisions[7] > 0.5 && c.spell_cooldown == 0 && c.known_spells > 0 && c.mana >= 10.0 {
        if decisions[11] > 0.5 {
            switch_spell(c);
        }
        turn.flags |= Turn::CAST;
        out.tick.learned.push((index, Culture::Ritual));
    }
}

/// The beast it rides and the stranger it trades with.
pub(super) fn mount_and_trade(
    c: &mut Creature,
    ctx: &mut StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    index: usize,
    decisions: &[f32],
) {
    let exploratory_mount = c.mount_kind == MountKind::None
        && c.energy > 100.0
        && (c.age * 11 + c.id * 7) % 60 < 1;
    if decisions[8] > 0.45 || exploratory_mount {
        if c.mount_kind == MountKind::None {
            if c.energy > 100.0 {
                c.mount_kind = MountKind::Beast;
                c.mount_stamina = 100.0;
                c.energy -= 30.0;
                out.tick.mounts_total += 1;
            }
        } else {
            c.mount_kind = MountKind::None;
            c.mount_stamina = 100.0;
        }
    }

    let trade_wanted = decisions[9] > 0.4;
    if trade_wanted && c.craft_cooldown == 0 {
        let (cx, cy, cspecies) = (c.x, c.y, c.species);
        let mut partner: Option<(f32, f32, u32)> = None;
        ctx.grid.near(cx, cy, 6, |ji| {
            if partner.is_some() {
                return;
            }
            let other = &ctx.agents[ji as usize];
            if other.3 == cspecies {
                return;
            }
            let d = ((other.1 - cx).powi(2) + (other.2 - cy).powi(2)).sqrt();
            if d < 6.0 {
                partner = Some((other.1, other.2, other.3));
            }
        });
        if let Some((ox, oy, ospecies)) = partner {
            turn.flags |= Turn::TRADED;

            c.craft_cooldown = 25;
            out.events.push(SimEvent::Trade {
                x: (cx + ox) * 0.5,
                y: (cy + oy) * 0.5,
                color: c.team_color,
            });
            c.reputation = (c.reputation as i32 + 1).clamp(-100, 100) as i16;

            let pact = ctx.diplomacy.has_treaty(cspecies, ospecies, Treaty::TradePact);

            let gain = if pact { 18.0 } else { 5.0 } * c.culture_factor(Culture::Commerce);

            let gain = if turn.has(Turn::NEAR_MARKET) {
                gain * 1.6
            } else {
                gain
            };

            let gain = if c.knows_tech(Discovery::Commerce) {
                gain * 1.5
            } else {
                gain
            };
            c.energy = (c.energy + gain).min(c.max_energy);
            c.energy = (c.energy - energy_cost::TRADE).max(0.0);

            ctx.diplomacy.adjust_trust(cspecies, ospecies, 0.8);
            out.tick.learned.push((index, Culture::Commerce));
        }
    }

    let spent = c.stamina < c.max_stamina * 0.2 || c.energy < c.max_energy * 0.15;
    c.resting = (decisions[10] > 0.05 && c.stamina < c.max_stamina * 0.5) || spent;
}

/// What the creature has learned from what just happened.
fn lesson_targets(c: &Creature, turn: &Turn, learn: f32) -> (Lessons, usize) {
    let mut targets: Lessons = [(0, 0.0); 6];
    let mut n = 0usize;
    let mut push = |out: usize, amount: f32| {
        if n < targets.len() {
            targets[n] = (out, amount);
            n += 1;
        }
    };
    if turn.has(Turn::ATE) {
        push(2, learn * 1.5);

        push(outputs::DESIRE + c.desire_dir, learn * 2.5);
    }
    if turn.has(Turn::BUILT) {
        push(5, learn * 1.0);
    }

    if turn.has(Turn::KILLED) {
        push(4, learn * 1.6);
        if c.kill_dir >= 0 {
            push(outputs::DESIRE + c.kill_dir as usize, learn * 2.0);
        }
    } else if turn.has(Turn::ATTACKED) {
        push(4, learn * 0.3);
    }
    if turn.has(Turn::TRADED) {
        push(9, learn * 0.8);
    }
    if turn.has(Turn::CAST) {
        push(7, learn * 0.8);
    }

    if turn.has(Turn::HURT) {
        push(15, learn * 1.2);
        push(2, -learn * 0.8);
        if c.hurt_dir >= 0 {
            push(outputs::DESIRE + c.hurt_dir as usize, -learn * 1.5);
        }
    }

    if turn.crowd >= 8 {
        let penalty = if c.hunger > 40.0 && !turn.has(Turn::ATE) {
            learn * 1.4
        } else {
            learn * 0.6
        };
        push(outputs::DESIRE + c.desire_dir, -penalty);
    }

    if c.hunger > 70.0 {
        push(2, learn * 0.5);
    }
    (targets, n)
}

/// The lesson: the mind is strengthened by what just happened.
pub(super) fn learn(c: &mut Creature, ctx: &StepCtx<'_>, turn: &Turn) {
    let disc = turn.discoveries;

    let mut learn = if disc & discovery_bits::GRANDCOGNITION != 0 {
        0.030
    } else {
        0.012
    };

    if turn.place.has(crate::world::Landmark::WhisperingStone) {
        learn *= 2.0;
    }

    if ctx.tick.is_multiple_of(4) {
        let (targets, n) = lesson_targets(c, turn, learn);
        if !targets.is_empty() && n > 0 {
            c.net.learn(&targets[..n], learn);
        }
    }

    if c.hunger > 25.0 && ctx.tick.is_multiple_of(4) {
        let drive = ((c.hunger - 25.0) / 75.0).min(1.0);
        let (best, best_strength) = strongest_direction(&c.percept_cache, |_, smell| {
            smell[crate::creature::senses::smell::SWEET]
        });

        if best_strength > 0.08 {
            c.net.credit_desire(best, learn * 2.0 * drive, learn);
        }
    }

    if c.thirst > 30.0 && ctx.tick.is_multiple_of(8) {
        let drive = ((c.thirst - 30.0) / 70.0).min(1.0);
        let (best, best_strength) = strongest_direction(&c.percept_cache, |sight, _| {
            sight[crate::creature::senses::sight::WET] * sight[crate::creature::senses::sight::BLUE]
        });
        if best_strength > 0.05 {
            c.net.credit_desire(best, learn * 1.6 * drive, learn);
        }
    }

    if ctx.tick.is_multiple_of(97) {
        c.net.settle(0.004);
    }
}

/// The pick: the creature that digs the ground it stands on.
pub(super) fn dig(c: &mut Creature, ctx: &mut StepCtx<'_>, decisions: &[f32]) {
    if decisions[12] > 0.5 && c.tool_durability > 0 && ctx.world.mine_budget > 0 {
        let gx = c.x.round() as i32;
        let gy = c.y.round() as i32;
        for (dx, dy) in DIRS4 {
            let tx = gx + dx;
            let ty = gy + dy;
            let e = ctx.world.get(tx, ty);
            if (e == Elem::Wall || e == Elem::Stone)
                && (c.tool == Tool::Pickaxe || c.tool == Tool::Drill)
            {
                ctx.world.mine_budget -= 1;
                ctx.world.set_xy(tx, ty, Elem::Empty);
                c.tool_durability = c.tool_durability.saturating_sub(5);
                c.energy = (c.energy - energy_cost::DIG).max(0.0);
                c.inventory[0] = (c.inventory[0].saturating_add(2)).min(255);
                break;
            }
        }
    }
}

/// The young: whether the creature will have a child, and what it
/// costs.
pub(super) fn breed(
    c: &mut Creature,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    index: usize,
    pop_now: usize,
    zorg_pop: &mut u32,
    decisions: &[f32],
) {
    let seeking_mate = decisions[outputs::SEEK_MATE] > 0.35
        && c.stage.can_breed()
        && c.energy > 260.0
        && c.cooldown == 0
        && c.hunger < 70.0
        && pop_now < REPRO_SOFT_CAP;
    out.tick.mate_willing[index] = seeking_mate;
    let nn_wants_reproduce = decisions[3] > 0.05 && !seeking_mate;
    let exploratory_reproduce = c.energy > 380.0
        && c.cooldown == 0
        && c.hunger < 60.0
        && c.thirst < 70.0
        && (c.age * 13 + c.id * 29) % 120 < 2;

    let hardship = turn.hardship;
    let need = |base: f32| base * (0.6 + 0.9 * hardship);
    let well_fed = 60.0 + 35.0 * (1.0 - hardship);
    let can_reproduce = match c.role {
        Role::Queen => c.energy > need(300.0),
        Role::Worker => c.energy > need(430.0) && c.hunger < well_fed && (c.age * 17 + c.id * 23) % 220 < 2,
        Role::Soldier => c.energy > need(460.0) && c.hunger < well_fed && (c.age * 19 + c.id * 29) % 220 < 2,
        Role::Priest => c.energy > need(440.0) && c.hunger < well_fed && (c.age * 13 + c.id * 17) % 220 < 2,
        Role::Merchant => c.energy > need(460.0) && c.hunger < well_fed && (c.age * 19 + c.id * 23) % 220 < 2,
        Role::Builder => c.energy > need(430.0) && c.hunger < well_fed && (c.age * 11 + c.id * 13) % 220 < 2,
        Role::Explorer => c.energy > need(410.0) && c.hunger < well_fed && (c.age * 7 + c.id * 11) % 220 < 2,
        Role::Shaman => c.energy > need(440.0) && c.hunger < well_fed && (c.age * 17 + c.id * 19) % 220 < 2,
        Role::Envoy => c.energy > need(420.0) && c.hunger < well_fed && (c.age * 11 + c.id * 29) % 220 < 2,
    };

    let litter = c.litter_size();

    let zorg_cap = (30.0 + hardship * (ZORG_POP_CAP - 30) as f32) as u32;
    let zorg_room = *zorg_pop + litter <= zorg_cap;
    if can_reproduce
        && (nn_wants_reproduce || exploratory_reproduce)
        && c.energy > 340.0
        && c.cooldown == 0
        && pop_now < REPRO_SOFT_CAP
        && (c.faction != Faction::Zorg || zorg_room)
        && c.stage.can_breed()
    {
        let fertile = if c.traits & trait_bits::FERTILE != 0 { 0.6 } else { 1.0 };
        let cost = c.brood_cost() * fertile;
        if c.energy > cost {
            if c.faction == Faction::Zorg {
                *zorg_pop += litter;
            }
            c.energy -= cost;
            c.hunger = (c.hunger + 15.0).clamp(0.0, 100.0);
            c.cooldown = if c.traits & trait_bits::FERTILE != 0 { 50 } else { 70 };
            c.offspring += litter;
            turn.flags |= Turn::WANTS_CHILD;
            turn.child_count = litter;
            turn.child_value = c.brood_value();
        }
    }
}

/// The bomb and the salve.
pub(super) fn weapons(c: &mut Creature, ctx: &StepCtx<'_>, out: &mut StepOut<'_>) {
    if c.bombs > 0 && c.craft_cooldown == 0 {
        let mut target: Option<(f32, f32, f32)> = None;
        ctx.grid.near(c.x, c.y, 18, |idx| {
            let a = &ctx.agents[idx as usize];
            if a.0 == c.id || a.3 == c.species {
                return;
            }
            let dx = a.1 - c.x;
            let dy = a.2 - c.y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 16.0 && target.is_none_or(|t| d < t.2) {
                target = Some((dx, dy, d));
            }
        });
        if let Some((dx, dy, d)) = target {
            let ux = dx / d.max(0.1);
            let uy = dy / d.max(0.1);
            out.tick.bombs.push(Bomb {
                x: c.x,
                y: c.y,
                vx: ux * 2.0,
                vy: uy * 2.0,
                life: 40,
                owner: c.species,
                color: c.team_color,
            });
            c.bombs -= 1;
            c.craft_cooldown = 25;
            c.energy = (c.energy - energy_cost::BOMB).max(0.0);
            out.events.push(SimEvent::Shoot {
                x: c.x,
                y: c.y,
                color: c.team_color,
            });
        }
    }

    if c.salves > 0 && c.craft_cooldown == 0 {
        for &(aid, ax, ay, aspecies, _arole, ahp) in ctx.agents {
            if aid == c.id || aspecies != c.species || ahp >= 95.0 {
                continue;
            }
            let dx = ax - c.x;
            let dy = ay - c.y;
            if dx * dx + dy * dy < 64.0 {
                out.tick.heal_targets.push((aid, ax, ay));
                c.salves -= 1;
                c.craft_cooldown = 30;
                c.energy = (c.energy - energy_cost::SALVE).max(0.0);
                out.events.push(SimEvent::Heal {
                    x: ax,
                    y: ay,
                    color: c.team_color,
                });
                break;
            }
        }
    }
}

/// The spell and the sickness it carries.
pub(super) fn spells_and_plague(
    c: &mut Creature,
    ctx: &mut StepCtx<'_>,
    out: &mut StepOut<'_>,
    turn: &mut Turn,
    index: usize,
) {
    if turn.has(Turn::CAST) {
        out.tick.spells_cast_total += 1;

        cast_spell_effects(
            &mut *out.rng,
            index,
            c,
            &mut *ctx.world,
            &mut SpellContext {
                grid: ctx.grid,
                agents: ctx.agents,
                spell_targets: &mut out.tick.spell_targets,
                disease_targets: &mut out.tick.disease_targets,
            },
        );
    }

    if c.disease != Disease::None && out.rng.chance(0.02) {
        let (dx0, dy0, dsp) = (c.x, c.y, c.species);
        ctx.grid.near(dx0, dy0, 4, |ji| {
            let j = ji as usize;
            let other = &ctx.agents[j];
            if other.3 == dsp {
                return;
            }
            let d = ((other.1 - dx0).powi(2) + (other.2 - dy0).powi(2)).sqrt();
            if d < 4.0 && out.rng.chance(0.15) {
                out.tick.disease_targets.push(j);
            }
        });
    }

    if c.disease != Disease::None {
        let _ = c.apply_disease();
    }
}

/// The end of the turn: how fit the creature is, and whether it lives.
pub(super) fn finish(c: &mut Creature, out: &mut StepOut<'_>, turn: &mut Turn) {
    c.fitness = (c.age as f32 / turn.lifespan as f32) * 0.15
        + (c.energy / c.max_energy) * 0.12
        + c.offspring as f32 * 0.18
        + c.kills as f32 * 0.12
        + (c.carried / c.carry_cap()) * 0.08
        + (c.tech_level as f32) * 0.1
        + (c.tool as u8 as f32) * 0.04
        + (c.mana / c.max_mana) * 0.06
        + (c.stamina / c.max_stamina) * 0.05
        - (c.hunger / 100.0) * 0.05
        - (c.thirst / 100.0) * 0.05;

    if c.energy <= 0.0
        || c.health <= 0.0
        || c.age > turn.lifespan + c.id.wrapping_mul(2_654_435_761) % 1400
    {
        turn.flags |= Turn::DIED;
    }

    c.last_x = c.x;
    c.last_y = c.y;

    c.health_seen = c.health;
    c.kills_seen = c.kills;

    if turn.has(Turn::WANTS_CHILD) {
        for _ in 0..turn.child_count.max(1) {
            let mut child = c.child_blueprint();
            child.energy = (child.energy * turn.child_value).max(60.0);
            child.max_energy = (child.max_energy * turn.child_value).max(200.0);
            child.size = (child.size * (0.85 + turn.child_value * 0.15)).clamp(0.8, 2.4);
            out.tick.births.push((c.base_x, c.base_y, child));
        }
    }
}
