use crate::creature::{
    self, Consumable, CreatureSystem, Discovery, TechEntry, TechKind, Tool, TECH_TIERS,
};
use macroquad::prelude::*;

#[must_use]
pub fn tech_tree_under_pointer(mx: f32, my: f32) -> bool {
    let w = 660.0;
    let h = (screen_height() - 40.0).min(820.0);
    let x = ((screen_width() - w) * 0.5 - 90.0).max(10.0);
    let y = (screen_height() - h) * 0.5;
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

pub fn draw_tech_tree(creatures: &CreatureSystem, ui: &mut crate::ui::knowledge::UiState) {
    let w = 660.0;
    let h = (screen_height() - 40.0).min(820.0);

    let x = ((screen_width() - w) * 0.5 - 90.0).max(10.0);
    let y = (screen_height() - h) * 0.5;

    draw_rectangle(x, y, w, h, Color::new(0.05, 0.06, 0.09, 0.985));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.35, 0.75, 0.85, 0.9));

    let tier = creatures.tech_tier();
    draw_tree_header(x, y, w, tier);

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

    let content_h = tree_content_h(&tree, row_h);
    let view_h = y + h - top - 12.0;
    let max_scroll = (content_h - view_h).max(0.0);
    ui.tech_scroll = ui.tech_scroll.clamp(0.0, max_scroll);

    let view_top = top - 4.0;
    let view_bottom = y + h - 10.0;

    let layout = TreeLayout {
        x,
        row_h,
        tier,
        view_top,
        view_bottom,
    };
    draw_tier_rows(creatures, &tree, &layout, top - ui.tech_scroll);

    if max_scroll > 0.0 {
        draw_scrollbar(x + w - 8.0, top, view_h, content_h, max_scroll, ui.tech_scroll);
    }
}

/// The title of the tree and the line that says where the world stands.
fn draw_tree_header(x: f32, y: f32, w: f32, tier: u8) {
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

    draw_text(
        format!("the world stands at tier {} of {}", tier, TECH_TIERS),
        x + w - 280.0,
        y + 36.0,
        16.0,
        Color::new(0.85, 0.8, 0.45, 0.95),
    );
}

/// How tall the whole tree is, so the scrollbar can be sized.
fn tree_content_h(tree: &[TechEntry], row_h: f32) -> f32 {
    let mut content_h = 0.0f32;
    for t in 1..=TECH_TIERS {
        content_h += row_h + 3.0;
        content_h += tree.iter().filter(|e| e.tier == t).count() as f32 * row_h;
        content_h += 8.0;
    }
    content_h
}

/// The tiers and their entries, scrolled to where the mouse left them.
fn draw_tier_rows(
    creatures: &CreatureSystem,
    tree: &[TechEntry],
    layout: &TreeLayout,
    mut yy: f32,
) {
    for t in 1..=TECH_TIERS {
        if yy > layout.view_bottom {
            break;
        }

        let reached = layout.tier >= t;
        let head_col = if reached {
            Color::new(0.55, 0.9, 0.75, 0.95)
        } else {
            Color::new(0.45, 0.5, 0.58, 0.8)
        };
        if yy >= layout.view_top {
            draw_text(
                format!(
                    "{:>2}. {:<14} {}",
                    t,
                    creature::tier_name(t),
                    creature::tier_blurb(t)
                ),
                layout.x + 20.0,
                yy,
                17.0,
                head_col,
            );
        }
        yy += layout.row_h + 3.0;

        for e in tree.iter().filter(|e| e.tier == t) {
            if yy > layout.view_bottom {
                break;
            }
            draw_tree_entry(creatures, e, layout.x, yy, layout.view_top);
            yy += layout.row_h;
        }
        yy += 8.0;
    }
}

/// Where the rows of the tree sit and how far the world has come.
struct TreeLayout {
    x: f32,
    row_h: f32,
    tier: u8,
    view_top: f32,
    view_bottom: f32,
}

/// One line of the tree: the entry and what it does.
fn draw_tree_entry(
    creatures: &CreatureSystem,
    e: &TechEntry,
    x: f32,
    yy: f32,
    view_top: f32,
) {
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
            format!("   [{}] {:<14} {}", mark, e.name, kind),
            x + 32.0,
            yy,
            15.0,
            col,
        );
        draw_text(
            format!("{}  ({})", e.effect, e.cost),
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
}

/// The thin track on the right that says how far down the tree we are.
fn draw_scrollbar(
    track_x: f32,
    top: f32,
    view_h: f32,
    content_h: f32,
    max_scroll: f32,
    scroll: f32,
) {
    draw_rectangle(track_x, top, 4.0, view_h, Color::new(0.12, 0.15, 0.2, 0.8));
    let grip = (view_h * view_h / content_h).max(20.0);
    let grip_y = top + (scroll / max_scroll) * (view_h - grip);
    draw_rectangle(track_x, grip_y, 4.0, grip, Color::new(0.4, 0.7, 0.8, 0.9));
}

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

pub fn draw_tech_panel(creatures: &CreatureSystem) {
    let panel_w = 314.0;
    let panel_h = 152.0;
    let panel_x = screen_width() - panel_w - 18.0;
    let panel_y = 254.0;

    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.04, 0.05, 0.08, 0.72));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, Color::new(0.3, 0.5, 0.7, 0.55));

    let tier = creatures.tech_tier();
    let known = creatures.discoveries_found();

    draw_tech_panel_header(panel_x, panel_y, tier, &known);
    draw_tech_tier_bar(panel_x, panel_y, panel_w, tier);

    let mut yy = panel_y + 76.0;
    yy = draw_tech_latest(creatures, panel_x, yy);
    draw_tech_greatest(creatures, panel_x, yy);
}

/// The name of the panel and how much of the tree has been worked out.
fn draw_tech_panel_header(
    panel_x: f32,
    panel_y: f32,
    tier: u8,
    known: &[(Discovery, u32)],
) {
    draw_text(
        "TECH",
        panel_x + 12.0,
        panel_y + 20.0,
        14.0,
        Color::new(0.55, 0.8, 0.95, 0.9),
    );
    draw_text(
        format!("tier {} of {}", tier, TECH_TIERS),
        panel_x + 62.0,
        panel_y + 20.0,
        13.0,
        Color::new(0.85, 0.8, 0.45, 0.95),
    );

    let tier_entries = creature::discovery_tier(tier.max(1));
    let tier_done = tier_entries
        .iter()
        .filter(|d| known.iter().any(|(k, _)| k == *d))
        .count();
    let total = Discovery::all().len();
    draw_text(
        format!(
            "{} of {} known   ({}/{} of tier {})",
            known.len(),
            total,
            tier_done,
            tier_entries.len(),
            tier.max(1)
        ),
        panel_x + 12.0,
        panel_y + 40.0,
        12.0,
        Color::new(0.7, 0.78, 0.85, 0.9),
    );
}

/// The row of notches that shows how far the world has come up the tree.
fn draw_tech_tier_bar(panel_x: f32, panel_y: f32, panel_w: f32, tier: u8) {
    let bar_x = panel_x + 12.0;
    let bar_y = panel_y + 52.0;
    let bar_w = panel_w - 24.0;
    let notch = bar_w / TECH_TIERS as f32;
    for t in 1..=TECH_TIERS {
        let filled = t <= tier;
        let shade = if filled {
            Color::new(0.45, 0.85, 0.7, 0.9)
        } else {
            Color::new(0.22, 0.26, 0.32, 0.9)
        };
        draw_rectangle(
            bar_x + (t as f32 - 1.0) * notch + 1.0,
            bar_y,
            notch - 2.0,
            6.0,
            shade,
        );
    }
}

/// The last thing that was worked out, and who worked it out.
fn draw_tech_latest(creatures: &CreatureSystem, panel_x: f32, yy: f32) -> f32 {
    if let Some((discovery, sp)) = creatures.latest_discovery() {
        draw_text(
            format!("LATEST  {}", discovery.name()),
            panel_x + 12.0,
            yy,
            12.0,
            Color::new(0.6, 0.95, 0.85, 0.95),
        );
        draw_text(
            format!("  {}", discovery.effect()),
            panel_x + 12.0,
            yy + 14.0,
            11.0,
            Color::new(0.6, 0.68, 0.75, 0.8),
        );
        draw_text(
            format!("  the {}", creature::species_name_for(sp)),
            panel_x + 190.0,
            yy,
            11.0,
            Color::new(0.5, 0.55, 0.62, 0.8),
        );
        yy + 30.0
    } else {
        draw_text(
            "LATEST  nothing has been worked out yet",
            panel_x + 12.0,
            yy,
            12.0,
            Color::new(0.5, 0.55, 0.62, 0.8),
        );
        yy + 24.0
    }
}

/// The greatest thing that has been worked out.
fn draw_tech_greatest(creatures: &CreatureSystem, panel_x: f32, yy: f32) {
    if let Some(discovery) = creatures.greatest_discovery() {
        draw_text(
            format!("GREATEST  {}", discovery.name()),
            panel_x + 12.0,
            yy,
            12.0,
            Color::new(0.95, 0.8, 0.4, 0.95),
        );
        draw_text(
            format!("  {}", discovery.effect()),
            panel_x + 12.0,
            yy + 14.0,
            11.0,
            Color::new(0.6, 0.68, 0.75, 0.8),
        );
    }
}

#[must_use]
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

#[must_use]
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

#[must_use]
pub fn craft_name(c: Consumable) -> &'static str {
    match c {
        Consumable::Bomb => "bomb",
        Consumable::Salve => "salve",
    }
}
