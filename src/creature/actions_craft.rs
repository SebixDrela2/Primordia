//! The recipes a creature may work out, in the order it tries them.
//! Each group is tried in turn and the first one that comes true ends
//! the search, so a creature always makes the same thing it made
//! before.

use super::{Creature, Tool};

/// The sanctified brand of fire.
pub(super) fn sanctified_fire(c: &mut Creature, knows_sanctify: bool) -> bool {
    if knows_sanctify && c.tech_level >= 1 && c.inventory[4] >= 1 && c.inventory[6] >= 1 {
        c.inventory[4] = c.inventory[4].saturating_sub(1);
        c.inventory[6] = c.inventory[6].saturating_sub(1);
        c.tool = Tool::Sunfire;
        c.tool_durability = 900;
        c.craft_cooldown = 50;
        return true;
    }
    false
}

/// The first tools: a pick, a blade, a hammer.
pub(super) fn first_tools(c: &mut Creature) -> bool {
    if c.tool == Tool::None && c.inventory[0] >= 5 && c.inventory[6] >= 2 {
        c.inventory[0] = c.inventory[0].saturating_sub(5);
        c.inventory[6] = c.inventory[6].saturating_sub(2);
        c.tool = Tool::Pickaxe;
        c.tool_durability = 1000;
        c.craft_cooldown = 30;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[1] >= 3 && c.inventory[6] >= 2 {
        c.inventory[1] = c.inventory[1].saturating_sub(3);
        c.inventory[6] = c.inventory[6].saturating_sub(2);
        c.tool = Tool::Sword;
        c.tool_durability = 800;
        c.craft_cooldown = 30;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[0] >= 5 && c.inventory[1] >= 3 {
        c.inventory[0] = c.inventory[0].saturating_sub(5);
        c.inventory[1] = c.inventory[1].saturating_sub(3);
        c.tool = Tool::Hammer;
        c.tool_durability = 1500;
        c.craft_cooldown = 30;
        return true;
    }
    false
}

/// The deeper tools and the bow.
pub(super) fn deeper_tools(c: &mut Creature, knows_ranged: bool) -> bool {
    if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[2] >= 3 && c.inventory[1] >= 2 {
        c.inventory[2] = c.inventory[2].saturating_sub(3);
        c.inventory[1] = c.inventory[1].saturating_sub(2);
        c.tool = Tool::Drill;
        c.tool_durability = 600;
        c.craft_cooldown = 40;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[3] >= 2 && c.inventory[4] >= 1 {
        c.inventory[3] = c.inventory[3].saturating_sub(2);
        c.inventory[4] = c.inventory[4].saturating_sub(1);
        c.tool = Tool::Scanner;
        c.tool_durability = 2000;
        c.craft_cooldown = 50;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 1 && (knows_ranged || c.inventory[6] >= 3) {
        c.inventory[6] = c.inventory[6].saturating_sub(if knows_ranged { 3 } else { 5 });
        c.tool = Tool::Bow;
        c.tool_durability = if knows_ranged { 800 } else { 500 };
        c.craft_cooldown = 30;
        return true;
    }
    false
}

/// The weapons that are carried, and the staff of the shrine.
pub(super) fn carried_weapons(c: &mut Creature) -> bool {
    if c.tool == Tool::None && c.inventory[6] >= 2 && c.inventory[0] >= 1 {
        c.inventory[6] = c.inventory[6].saturating_sub(2);
        c.inventory[0] = c.inventory[0].saturating_sub(1);
        c.tool = Tool::Spear;
        c.tool_durability = 400;
        c.craft_cooldown = 20;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 1 && c.inventory[6] >= 3 && c.inventory[1] >= 2 {
        c.inventory[6] = c.inventory[6].saturating_sub(3);
        c.inventory[1] = c.inventory[1].saturating_sub(2);
        c.tool = Tool::Shield;
        c.tool_durability = 700;
        c.craft_cooldown = 30;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 1 && c.inventory[4] >= 1 {
        c.inventory[6] = c.inventory[6].saturating_sub(1);
        c.inventory[4] = c.inventory[4].saturating_sub(1);
        c.tool = Tool::Staff;
        c.tool_durability = 1200;
        c.craft_cooldown = 40;
        return true;
    }
    false
}

/// The things a colony carries about with it.
pub(super) fn pack_and_carriage(c: &mut Creature) -> bool {
    if c.tool == Tool::None && c.tech_level >= 4 && c.inventory[3] >= 1 && c.inventory[5] >= 1 {
        c.inventory[3] = c.inventory[3].saturating_sub(1);
        c.inventory[5] = c.inventory[5].saturating_sub(1);
        c.tool = Tool::Amulet;
        c.tool_durability = 1500;
        c.craft_cooldown = 50;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 2 {
        c.inventory[6] = c.inventory[6].saturating_sub(4);
        c.tool = Tool::Backpack;
        c.tool_durability = 800;
        c.craft_cooldown = 40;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 5 && c.inventory[1] >= 3 {
        c.inventory[6] = c.inventory[6].saturating_sub(5);
        c.inventory[1] = c.inventory[1].saturating_sub(3);
        c.tool = Tool::Cart;
        c.tool_durability = 1000;
        c.craft_cooldown = 60;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 2 && c.inventory[6] >= 6 {
        c.inventory[6] = c.inventory[6].saturating_sub(6);
        c.tool = Tool::Boat;
        c.tool_durability = 900;
        c.craft_cooldown = 60;
        return true;
    }

    if c.tool == Tool::None && c.tech_level >= 3 && c.inventory[6] >= 4 {
        c.inventory[6] = c.inventory[6].saturating_sub(4);
        c.tool = Tool::Glider;
        c.tool_durability = 700;
        c.craft_cooldown = 70;
        return true;
    }
    false
}

/// The things that are used up rather than carried: the bomb and the
/// salve.
pub(super) fn consumables(c: &mut Creature, knows_siegecraft: bool, knows_medicine: bool) -> bool {
    if c.bombs < 3 && knows_siegecraft && c.inventory[9] >= 3 && c.inventory[6] >= 2 {
        c.inventory[9] = c.inventory[9].saturating_sub(3);
        c.inventory[6] = c.inventory[6].saturating_sub(2);
        c.bombs += 1;
        c.craft_cooldown = 40;
        return true;
    }

    if c.salves < 3 && c.inventory[9] == 0 && knows_medicine && (c.inventory[6] >= 2 && c.inventory[4] >= 1) {
        c.inventory[6] = c.inventory[6].saturating_sub(2);
        if !knows_medicine {
            c.inventory[4] = c.inventory[4].saturating_sub(1);
        }
        c.salves += 1;
        c.craft_cooldown = 40;
        return true;
    }
    false
}
