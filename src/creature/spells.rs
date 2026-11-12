//! Spells: what a shaman or a priest can do.
//!
//! The spells are not scripted events: a creature decides to cast one, and
//! the effect follows from the spell. A fireball scorches everything of
//! another species that stands too close, a curse spreads a disease, a
//! teleport puts the caster somewhere else entirely.
//!
//! The effects are resolved after the main loop, so a spell that kills
//! something is resolved in the same way as any other death.

use super::*;

/// Resolve the spell a creature has just cast.
///
/// The targets are collected and applied after the loop, so the creature
/// that casts the spell is not affected by its own fire.
pub(crate) fn cast_spell_effects(
    rng: &mut Rng,
    index: usize,
    c: &mut Creature,
    world: &mut World,
    grid: &SpatialGrid,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    spell_targets: &mut Vec<(usize, usize, f32)>,
    disease_targets: &mut Vec<usize>,
) {
    c.energy = (c.energy - energy_cost::SPELL).max(0.0);
    c.spell_cooldown = 30;
    c.mana = (c.mana - 10.0).max(0.0);
    c.spell_cd[c.active_spell as usize] = 60;
    match c.active_spell {
        Spell::Fireball => {
            let (sx, sy, ssp) = (c.x, c.y, c.species);
            grid.near(sx, sy, 10, |ji| {
                let j = ji as usize;
                let other = &agents[j];
                if other.3 == ssp {
                    return;
                }
                let d = ((other.1 - sx).powi(2) + (other.2 - sy).powi(2)).sqrt();
                if d < 10.0 {
                    spell_targets.push((index, j, 35.0));
                }
            });
        }
        Spell::Heal => {
            c.health = (c.health + 40.0).min(100.0);
            c.energy = (c.energy + 30.0).min(c.max_energy);
        }
        Spell::Teleport => {
            c.x = rng.range_i(10, world.w as i32 - 10) as f32;
            c.y = rng.range_i(10, world.h as i32 - 10) as f32;
        }
        Spell::Wall => {
            // A wall of stone, raised in the first free direction.
            let gx = c.x.round() as i32;
            let gy = c.y.round() as i32;
            for (dx, dy) in DIRS4 {
                if world.get(gx + dx, gy + dy) == Elem::Empty {
                    world.set_xy(gx + dx, gy + dy, Elem::Wall);
                    break;
                }
            }
        }
        Spell::Light => {
            c.fitness += 1.0;
        }
        Spell::Summon => {
            // A minion is raised by the birth machinery, not here.
        }
        Spell::Curse => {
            let (sx, sy, ssp) = (c.x, c.y, c.species);
            grid.near(sx, sy, 8, |ji| {
                let j = ji as usize;
                let other = &agents[j];
                if other.3 == ssp {
                    return;
                }
                let d = ((other.1 - sx).powi(2) + (other.2 - sy).powi(2)).sqrt();
                if d < 8.0 {
                    disease_targets.push(j);
                }
            });
        }
        Spell::Bless => {
            c.health = (c.health + 20.0).min(100.0);
            c.energy = (c.energy + 20.0).min(c.max_energy);
        }
        Spell::Lightning => {
            let (sx, sy, ssp) = (c.x, c.y, c.species);
            grid.near(sx, sy, 6, |ji| {
                let j = ji as usize;
                let other = &agents[j];
                if other.3 == ssp {
                    return;
                }
                let d = ((other.1 - sx).powi(2) + (other.2 - sy).powi(2)).sqrt();
                if d < 6.0 {
                    spell_targets.push((index, j, 50.0));
                }
            });
        }
        Spell::None => {}
    }
}

/// Should this creature switch to the next spell it knows?
///
/// The creature decides with its network, so the choice of spell is
/// behaviour, not a script.
pub(crate) fn switch_spell(c: &mut Creature) {
    let next = (c.active_spell as u8 + 1) % 10;
    c.active_spell = match next {
        1 => Spell::Fireball,
        2 => Spell::Heal,
        3 => Spell::Teleport,
        4 => Spell::Wall,
        5 => Spell::Light,
        6 => Spell::Summon,
        7 => Spell::Curse,
        8 => Spell::Bless,
        9 => Spell::Lightning,
        _ => Spell::None,
    };
}
