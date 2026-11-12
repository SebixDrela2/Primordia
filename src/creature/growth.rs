//! Growing up, and what the young learn from the old.
//!
//! A creature passes through three stages of life: a juvenile that cannot
//! fight or breed, a full adult, and finally an elder that is slower but
//! wiser. A young animal that grows up near an old one learns from it: it
//! matures a little faster and picks up a practice it did not know. That is
//! what gives old age a purpose in a colony.

use super::*;

/// Move the creature through the stages of its life and let the elders
/// teach it.
pub(crate) fn grow_and_learn(
    c: &mut Creature,
    grid: &SpatialGrid,
    agents: &[(u32, f32, f32, u32, u8, f32)],
    lives: &[(Stage, u16)],
) {
    let lifespan = c.lifespan();
    let new_stage = Stage::of(c.age, lifespan, c.maturity_fraction());
    if new_stage != c.stage {
        // Growing up costs energy, and the body changes with it.
        if new_stage == Stage::Adult {
            c.health = (c.health + 30.0).min(c.max_health());
            c.energy = (c.energy - 80.0).max(0.0);
        }
        c.stage = new_stage;
    }

    if c.stage != Stage::Juvenile {
        return;
    }

    // Find an elder of the same kind nearby.
    let mut elder: Option<u16> = None;
    let cx = c.x;
    let cy = c.y;
    grid.near(cx, cy, 24, |idx| {
        let i = idx as usize;
        if i >= lives.len() || !lives[i].0.can_teach() {
            return;
        }
        let a = &agents[i];
        if a.0 == c.id || a.3 != c.species {
            return;
        }
        let d = ((a.1 - cx).powi(2) + (a.2 - cy).powi(2)).sqrt();
        if d < 24.0 {
            elder = Some(lives[i].1);
        }
    });

    let Some(culture) = elder else {
        return;
    };
    // The lesson speeds the young up.
    c.age += 1;
    // And it teaches one practice the young did not know yet.
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
        if culture & bit != 0 && c.culture & bit == 0 {
            c.culture |= bit;
            break;
        }
    }
}
