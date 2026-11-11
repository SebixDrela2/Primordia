use crate::creature;
use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::World;
use macroquad::prelude::*;


pub enum HoverTarget {
    None,
    Element { x: i32, y: i32, elem: Elem },
    Creature { id: u32 },
}

pub struct UiState {
    pub     hover: HoverTarget,
    pub     knowledge_open: bool,
    pub     discovered_elements: Vec<u8>,
    pub     discovered_creatures: Vec<u32>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            hover: HoverTarget::None,
            knowledge_open: false,
            discovered_elements: Vec::new(),
            discovered_creatures: Vec::new(),
        }
    }

    pub fn discover_element(&mut self, e: Elem) {
        let id = e as u8;
        if !self.discovered_elements.contains(&id) {
            self.discovered_elements.push(id);
        }
    }

    pub fn discover_creature(&mut self, id: u32) {
        if !self.discovered_creatures.contains(&id) {
            self.discovered_creatures.push(id);
        }
    }
}

/// Draw a tooltip at the mouse position showing what's under the cursor.
pub fn draw_tooltip(_world: &World, creatures: &CreatureSystem, ui: &UiState, mx: f32, my: f32) {
    match &ui.hover {
        HoverTarget::None => return,
        HoverTarget::Element { x, y, elem } => {
            let e = *elem;
            let name = e.name();
            let desc = e.description();
            let category = e.category();
            let density = e.density();
            let tox = e.toxicity();
            let nut = e.nutrition();

            let mut lines: Vec<String> = Vec::new();
            lines.push(format!("{}", name));
            lines.push(format!("[{}]  pos=({},{})", category, x, y));
            lines.push(format!("density={}  toxicity={:.0}  nutrition={:.1}", density, tox, nut));
            if e.is_hazardous() {
                lines.push("HAZARDOUS".to_string());
            }
            if e.is_solid() {
                lines.push("solid".to_string());
            }
            if e.flammable() {
                lines.push("flammable".to_string());
            }
            lines.push(desc.to_string());

            draw_tooltip_box(lines, mx, my);
        }
        HoverTarget::Creature { id } => {
            let c = creatures.list.iter().find(|c| c.id == *id);
            let Some(c) = c else { return };

            let role_str = match c.role {
                creature::Role::Worker => "Worker",
                creature::Role::Soldier => "Soldier",
                creature::Role::Queen => "Queen",
                creature::Role::Priest => "Priest",
                creature::Role::Merchant => "Merchant",
                creature::Role::Builder => "Builder",
                creature::Role::Explorer => "Explorer",
                creature::Role::Shaman => "Shaman",
            };

            let tool_str = match c.tool {
                creature::Tool::None => "none".to_string(),
                creature::Tool::Pickaxe => "pickaxe".to_string(),
                creature::Tool::Sword => "sword".to_string(),
                creature::Tool::Hammer => "hammer".to_string(),
                creature::Tool::Drill => "drill".to_string(),
                creature::Tool::Scanner => "scanner".to_string(),
                creature::Tool::Bow => "bow".to_string(),
                creature::Tool::Spear => "spear".to_string(),
                creature::Tool::Shield => "shield".to_string(),
                creature::Tool::Staff => "staff".to_string(),
                creature::Tool::Amulet => "amulet".to_string(),
                creature::Tool::Backpack => "backpack".to_string(),
                creature::Tool::Cart => "cart".to_string(),
                creature::Tool::Boat => "boat".to_string(),
                creature::Tool::Glider => "glider".to_string(),
            };

            let mut end = 10;
            for i in 0..10 {
                if c.name[i] == 0 {
                    end = i;
                    break;
                }
            }
            let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");

            let mut lines: Vec<String> = Vec::new();
            lines.push(format!("{}  (id {})", name_str, c.id));
            lines.push(format!("species {}  gen {}  role {}", c.species, c.generation, role_str));
            lines.push(format!("energy {:.0}/{:.0}  health {:.0}  fitness {:.2}",
                c.energy, c.max_energy, c.health, c.fitness));
            lines.push(format!("age {}  size {:.1}  tech {}", c.age, c.size, c.tech_level));
            lines.push(format!("tool {}  carried {:.0}", tool_str, c.carried));
            lines.push(format!("kills {}  offspring {}", c.kills, c.offspring));
            if c.disease != creature::Disease::None {
                lines.push(format!("disease: {:?}", c.disease));
            }

            draw_tooltip_box(lines, mx, my);
        }
    }
}

pub fn draw_tooltip_box(lines: Vec<String>, mx: f32, my: f32) {
    if lines.is_empty() {
        return;
    }

    // Measure the widest line
    let mut max_w: f32 = 0.0;
    let mut total_h = 0.0;
    for line in &lines {
        let dims = measure_text(line, None, 14, 1.0);
        max_w = max_w.max(dims.width);
        total_h += 16.0;
    }
    total_h += 8.0;

    let pad = 6.0;
    let box_w = max_w + pad * 2.0;
    let box_h = total_h + pad * 2.0;

    // Position below-right of cursor, clamped to screen
    let mut bx = mx + 14.0;
    let mut by = my + 14.0;
    if bx + box_w > screen_width() {
        bx = mx - box_w - 14.0;
    }
    if by + box_h > screen_height() {
        by = my - box_h - 14.0;
    }
    bx = bx.clamp(4.0, screen_width() - box_w - 4.0);
    by = by.clamp(4.0, screen_height() - box_h - 4.0);

    // Background
    draw_rectangle(bx, by, box_w, box_h, Color::new(0.05, 0.05, 0.08, 0.85));
    draw_rectangle_lines(bx, by, box_w, box_h, 1.5, Color::new(0.3, 0.5, 0.7, 0.9));

    // Title (first line)
    let title_color = Color::new(0.7, 0.85, 1.0, 1.0);
    draw_text(&lines[0], bx + pad, by + pad + 12.0, 14.0, title_color);

    // Remaining lines
    let mut yy = by + pad + 12.0 + 16.0;
    for line in &lines[1..] {
        draw_text(line, bx + pad, yy, 12.0, Color::new(0.85, 0.9, 0.95, 0.85));
        yy += 15.0;
    }
}

/// Draw the knowledge window: an encyclopedia of all discovered elements and creatures.
pub fn draw_knowledge_window(_world: &World, creatures: &CreatureSystem, ui: &UiState) {
    let w = 520.0;
    let h = screen_height() * 0.85;
    let x = screen_width() - w - 20.0;
    let y = 40.0;

    // Background
    draw_rectangle(x, y, w, h, Color::new(0.06, 0.06, 0.09, 0.93));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.3, 0.5, 0.7, 0.9));

    draw_text("KNOWLEDGE", x + 16.0, y + 28.0, 20.0, Color::new(0.7, 0.85, 1.0, 1.0));
    draw_rectangle_lines(x + 16.0, y + 34.0, 100.0, 2.0, 1.0, Color::new(0.3, 0.5, 0.7, 0.6));
    draw_text("press K to close", x + 16.0, y + 52.0, 14.0, Color::new(0.5, 0.55, 0.6, 0.7));

    // What a creature can decide to do.
    draw_text("BEHAVIOURS", x + 16.0, y + 76.0, 13.0, Color::new(0.4, 0.75, 0.9, 0.85));
    let abilities = [
        ("SHOOT", "fire a bow or staff at a rival"),
        ("FLEE", "run from a stronger enemy"),
        ("SEEK", "head for the food or water it needs"),
        ("FORAGE", "collect wood and ore to build with"),
        ("RALLY", "return to the colony base"),
        ("DEFEND", "guard the colony against intruders"),
        ("BOMB", "craft and throw explosives at rivals"),
        ("SALVE", "craft and use healing salves on allies"),
    ];
    let mut ay = y + 96.0;
    for (name, desc) in abilities.iter() {
        draw_text(name, x + 24.0, ay, 12.0, Color::new(0.55, 0.9, 1.0, 0.9));
        draw_text(desc, x + 90.0, ay, 12.0, Color::new(0.62, 0.68, 0.75, 0.8));
        ay += 17.0;
    }

    let mut yy = ay + 14.0;

    // ---- Elements section ----
    if !ui.discovered_elements.is_empty() {
        draw_text("ELEMENTS DISCOVERED", x + 16.0, yy, 16.0, Color::new(0.85, 0.9, 0.95, 0.9));
        yy += 22.0;

        // Group by category
        let mut cats: Vec<(&str, Vec<u8>)> = Vec::new();
        for &id in &ui.discovered_elements {
            let e = Elem::from_u8(id);
            let cat = e.category();
            if let Some(pos) = cats.iter_mut().find(|(c, _)| *c == cat) {
                pos.1.push(id);
            } else {
                cats.push((cat, vec![id]));
            }
        }

        for (cat, ids) in cats {
            draw_text(cat, x + 24.0, yy, 13.0, Color::new(0.6, 0.7, 0.8, 0.8));
            yy += 18.0;
            for &id in &ids {
                let e = Elem::from_u8(id);
                let name = e.name();
                let desc = e.description();
                let color = e.color();
                // color swatch
                draw_rectangle(x + 24.0, yy - 2.0, 10.0, 10.0, Color::new(
                    color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0, 0.9,
                ));
                draw_text(name, x + 40.0, yy + 8.0, 13.0, Color::new(0.9, 0.95, 1.0, 0.95));
                // short desc
                let short = if desc.len() > 60 { format!("{}...", &desc[..60]) } else { desc.to_string() };
                draw_text(&short, x + 40.0, yy + 20.0, 11.0, Color::new(0.65, 0.7, 0.75, 0.8));
                yy += 34.0;
            }
            yy += 8.0;
        }
        yy += 16.0;
    }

    // ---- Creatures section ----
    if !ui.discovered_creatures.is_empty() {
        draw_text("CREATURES OBSERVED", x + 16.0, yy, 16.0, Color::new(0.85, 0.9, 0.95, 0.9));
        yy += 22.0;

        for &id in &ui.discovered_creatures {
            let c = creatures.list.iter().find(|c| c.id == id);
            let Some(c) = c else { continue };

            let role_str = match c.role {
                creature::Role::Worker => "Worker",
                creature::Role::Soldier => "Soldier",
                creature::Role::Queen => "Queen",
                creature::Role::Priest => "Priest",
                creature::Role::Merchant => "Merchant",
                creature::Role::Builder => "Builder",
                creature::Role::Explorer => "Explorer",
                creature::Role::Shaman => "Shaman",
            };

            let mut end = 10;
            for i in 0..10 {
                if c.name[i] == 0 {
                    end = i;
                    break;
                }
            }
            let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");

            let swatch_color = c.team_color;
            draw_rectangle(x + 24.0, yy - 2.0, 10.0, 10.0, Color::new(
                swatch_color[0] as f32 / 255.0, swatch_color[1] as f32 / 255.0, swatch_color[2] as f32 / 255.0, 0.9,
            ));
            draw_text(name_str, x + 40.0, yy + 8.0, 13.0, Color::new(0.9, 0.95, 1.0, 0.95));
            draw_text(&format!("species {}  gen {}  role {}  energy {:.0}", c.species, c.generation, role_str, c.energy),
                x + 40.0, yy + 20.0, 11.0, Color::new(0.65, 0.7, 0.75, 0.8));
            yy += 34.0;
        }
    }

    if ui.discovered_elements.is_empty() && ui.discovered_creatures.is_empty() {
        draw_text("Hover over anything in the world to record it here.", x + 24.0, yy, 14.0, Color::new(0.6, 0.65, 0.7, 0.7));
    }
}
