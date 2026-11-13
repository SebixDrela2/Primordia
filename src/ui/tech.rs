//! The tech tree on the screen.
//!
//! Two windows: the **tree** (T), which shows every tier of the tree and
//! what the world has worked out so far, and the **panel**, a small sign
//! on the side of the screen that says where the world has got to.

use crate::creature::{
    self, Consumable, CreatureSystem, Discovery, TechEntry, TechKind, Tool, TECH_TIERS,
};
use macroquad::prelude::*;

/// Is the pointer over the tech tree window?
///
/// The window scrolls under the pointer, and the wheel over it scrolls the
/// tree rather than zooming the world.
pub fn tech_tree_under_pointer(mx: f32, my: f32) -> bool {
    let w = 660.0;
    let h = (screen_height() - 40.0).min(820.0);
    let x = ((screen_width() - w) * 0.5 - 90.0).max(10.0);
    let y = (screen_height() - h) * 0.5;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

/// Draw the tech tree window: every tier, every entry, and what it does.
///
/// The window is taller than the tree, so it scrolls: the wheel over the
/// window moves it up and down.
pub fn draw_tech_tree(creatures: &CreatureSystem, ui: &mut crate::ui::knowledge::UiState) {
    let w = 660.0;
    let h = (screen_height() - 40.0).min(820.0);
    // The window sits a little to the left of the middle, so it never
    // collides with the tech panel on the right.
    let x = ((screen_width() - w) * 0.5 - 90.0).max(10.0);
    let y = (screen_height() - h) * 0.5;

    draw_rectangle(x, y, w, h, Color::new(0.05, 0.06, 0.09, 0.985));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.35, 0.75, 0.85, 0.9));

    let tier = creatures.tech_tier();
    draw_text(
        "THE TECH TREE",
        x + 20.0,
        y + 34.0,
        26.0,
        Color::new(0.7, 0.9, 1.0, 1.0),
    );
    draw_rectangle_lines(x + 20.0, y + 44.0, 170.0, 2.0, 1.0, Color::new(0.3, 0.5, 0.7, 0.6));
    draw_text(
        "press T to close  -  scroll with the wheel",
        x + 20.0,
        y + 66.0,
        15.0,
        Color::new(0.5, 0.55, 0.6, 0.7),
    );
    // How far the world has got.
    draw_text(
        &format!("the world stands at tier {} of {}", tier, TECH_TIERS),
        x + w - 280.0,
        y + 36.0,
        16.0,
        Color::new(0.85, 0.8, 0.45, 0.95),
    );

    // ---- Scrolling ----
    //
    // The wheel over the window moves the tree. The scroll is clamped to
    // the height of the tree, so it never runs off the end.
    let mouse = mouse_position();
    let over = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    if over {
        let (_, wheel) = mouse_wheel();
        if wheel != 0.0 {
            ui.tech_scroll -= wheel * 30.0;
        }
    }

    let tree = creature::tech_tree();
    let row_h = 16.0;
    let top = y + 88.0;
    // How tall the whole tree is, so the scroll can be clamped.
    let mut content_h = 0.0f32;
    for t in 1..=TECH_TIERS {
        content_h += row_h + 3.0;
        content_h += tree.iter().filter(|e| e.tier == t).count() as f32 * row_h;
        content_h += 8.0;
    }
    let view_h = y + h - top - 12.0;
    let max_scroll = (content_h - view_h).max(0.0);
    ui.tech_scroll = ui.tech_scroll.clamp(0.0, max_scroll);

    // The tree is clipped to the window by hand: rows outside it are
    // simply not drawn.
    let view_top = top - 4.0;
    let view_bottom = y + h - 10.0;

    let mut yy = top - ui.tech_scroll;
    for t in 1..=TECH_TIERS {
        if yy > view_bottom {
            break;
        }
        // The tier heading.
        let reached = tier >= t;
        let head_col = if reached {
            Color::new(0.55, 0.9, 0.75, 0.95)
        } else {
            Color::new(0.45, 0.5, 0.58, 0.8)
        };
        if yy >= view_top {
            draw_text(
                &format!(
                    "{:>2}. {:<14} {}",
                    t,
                    creature::tier_name(t),
                    creature::tier_blurb(t)
                ),
                x + 20.0,
                yy,
                17.0,
                head_col,
            );
        }
        yy += row_h + 3.0;

        // The entries of the tier.
        for e in tree.iter().filter(|e| e.tier == t) {
            if yy > view_bottom {
                break;
            }
            let known = entry_known(creatures, e);
            let mark = if known { "x" } else { "-" };
            let col = if known {
                Color::new(0.9, 0.94, 0.98, 0.95)
            } else {
                Color::new(0.45, 0.5, 0.56, 0.75)
            };
            let kind = match e.kind {
                TechKind::Tool(_) => "tool",
                TechKind::Craft(_) => "craft",
                TechKind::Discovery(_) => "known",
            };
            if yy >= view_top {
                draw_text(
                    &format!("   [{}] {:<14} {}", mark, e.name, kind),
                    x + 32.0,
                    yy,
                    15.0,
                    col,
                );
                draw_text(
                    &format!("{}  ({})", e.effect, e.cost),
                    x + 268.0,
                    yy,
                    14.0,
                    if known {
                        Color::new(0.6, 0.72, 0.8, 0.85)
                    } else {
                        Color::new(0.38, 0.42, 0.48, 0.7)
                    },
                );
            }
            yy += row_h;
        }
        yy += 8.0;
    }

    // A scroll bar, so it is clear there is more of the tree below.
    if max_scroll > 0.0 {
        let track_x = x + w - 8.0;
        draw_rectangle(track_x, top, 4.0, view_h, Color::new(0.12, 0.15, 0.2, 0.8));
        let grip = (view_h * view_h / content_h).max(20.0);
        let grip_y = top + (ui.tech_scroll / max_scroll) * (view_h - grip);
        draw_rectangle(track_x, grip_y, 4.0, grip, Color::new(0.4, 0.7, 0.8, 0.9));
    }
}

/// Has the world got this entry of the tree?
///
/// A discovery is known when a species has worked it out. A tool is
/// available when the colony knows the craft that goes with it.
fn entry_known(creatures: &CreatureSystem, e: &TechEntry) -> bool {
    match e.kind {
        TechKind::Discovery(d) => creatures
            .discoveries
            .iter()
            .any(|(_, bits)| bits & d.bit() != 0),
        TechKind::Tool(t) => match creature::tool_gate(t) {
            None => true,
            Some(d) => creatures
                .discoveries
                .iter()
                .any(|(_, bits)| bits & d.bit() != 0),
        },
        TechKind::Craft(c) => match creature::craft_gate(c) {
            None => true,
            Some(d) => creatures
                .discoveries
                .iter()
                .any(|(_, bits)| bits & d.bit() != 0),
        },
    }
}

/// The side panel: where the world has got to.
pub fn draw_tech_panel(creatures: &CreatureSystem) {
    let w = 314.0;
    let h = 152.0;
    let x = screen_width() - w - 18.0;
    let y = 254.0;

    draw_rectangle(x, y, w, h, Color::new(0.04, 0.05, 0.08, 0.72));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.3, 0.5, 0.7, 0.55));

    let tier = creatures.tech_tier();
    let known = creatures.discoveries_found();

    draw_text(
        "TECH",
        x + 12.0,
        y + 20.0,
        14.0,
        Color::new(0.55, 0.8, 0.95, 0.9),
    );
    draw_text(
        &format!("tier {} of {}", tier, TECH_TIERS),
        x + 62.0,
        y + 20.0,
        13.0,
        Color::new(0.85, 0.8, 0.45, 0.95),
    );

    // The fraction: how much of the tier the world has finished, and how
    // much of the tree in all.
    let tier_entries = creature::discovery_tier(tier.max(1));
    let tier_done = tier_entries
        .iter()
        .filter(|d| known.iter().any(|(k, _)| k == *d))
        .count();
    let total = Discovery::all().len();
    draw_text(
        &format!(
            "{} of {} known   ({}/{} of tier {})",
            known.len(),
            total,
            tier_done,
            tier_entries.len(),
            tier.max(1)
        ),
        x + 12.0,
        y + 40.0,
        12.0,
        Color::new(0.7, 0.78, 0.85, 0.9),
    );

    // A bar of the whole tree, one notch per tier.
    let bar_x = x + 12.0;
    let bar_y = y + 52.0;
    let bar_w = w - 24.0;
    let notch = bar_w / TECH_TIERS as f32;
    for t in 1..=TECH_TIERS {
        let filled = t <= tier;
        let c = if filled {
            Color::new(0.45, 0.85, 0.7, 0.9)
        } else {
            Color::new(0.22, 0.26, 0.32, 0.9)
        };
        draw_rectangle(
            bar_x + (t as f32 - 1.0) * notch + 1.0,
            bar_y,
            notch - 2.0,
            6.0,
            c,
        );
    }

    // The latest thing that was worked out.
    let mut yy = y + 76.0;
    if let Some((d, sp)) = creatures.latest_discovery() {
        draw_text(
            &format!("LATEST  {}", d.name()),
            x + 12.0,
            yy,
            12.0,
            Color::new(0.6, 0.95, 0.85, 0.95),
        );
        draw_text(
            &format!("  {}", d.effect()),
            x + 12.0,
            yy + 14.0,
            11.0,
            Color::new(0.6, 0.68, 0.75, 0.8),
        );
        draw_text(
            &format!("  the {}", creature::species_name_for(sp)),
            x + 190.0,
            yy,
            11.0,
            Color::new(0.5, 0.55, 0.62, 0.8),
        );
        yy += 30.0;
    } else {
        draw_text(
            "LATEST  nothing has been worked out yet",
            x + 12.0,
            yy,
            12.0,
            Color::new(0.5, 0.55, 0.62, 0.8),
        );
        yy += 24.0;
    }

    // And the greatest thing the world knows.
    if let Some(d) = creatures.greatest_discovery() {
        draw_text(
            &format!("GREATEST  {}", d.name()),
            x + 12.0,
            yy,
            12.0,
            Color::new(0.95, 0.8, 0.4, 0.95),
        );
        draw_text(
            &format!("  {}", d.effect()),
            x + 12.0,
            yy + 14.0,
            11.0,
            Color::new(0.6, 0.68, 0.75, 0.8),
        );
    }
}

/// The names of the tech a creature knows, for the tooltip.
pub fn tech_of(c: &crate::creature::Creature) -> String {
    let mut out = String::new();
    for d in Discovery::all() {
        if c.tech & d.bit() != 0 {
            if !out.is_empty() {
                out.push_str(", ");
            }
            out.push_str(d.name());
        }
    }
    if out.is_empty() {
        out.push_str("nothing yet");
    }
    out
}

/// The tools a creature is carrying, named.
pub fn tool_name(t: Tool) -> &'static str {
    match t {
        Tool::None => "nothing",
        Tool::Pickaxe => "pickaxe",
        Tool::Sword => "sword",
        Tool::Hammer => "hammer",
        Tool::Drill => "drill",
        Tool::Scanner => "scanner",
        Tool::Bow => "bow",
        Tool::Spear => "spear",
        Tool::Shield => "shield",
        Tool::Staff => "staff",
        Tool::Amulet => "amulet",
        Tool::Backpack => "backpack",
        Tool::Cart => "cart",
        Tool::Boat => "boat",
        Tool::Glider => "glider",
        Tool::Sunfire => "sunfire",
    }
}

/// The names of the things a colony makes.
pub fn craft_name(c: Consumable) -> &'static str {
    match c {
        Consumable::Bomb => "bomb",
        Consumable::Salve => "salve",
    }
}
