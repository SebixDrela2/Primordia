use crate::creature;
use crate::creature::{CreatureSystem, trait_list, culture_list, Affinity};
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
    /// Whether the diplomacy panel is on screen (toggled with G).
    pub     diplomacy_open: bool,
    pub     discovered_elements: Vec<u8>,
    pub     discovered_creatures: Vec<u32>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            hover: HoverTarget::None,
            knowledge_open: false,
            diplomacy_open: false,
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
                creature::Role::Envoy => "Envoy",
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
                creature::Tool::Sunfire => "sunfire".to_string(),
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
            lines.push(format!(
                "the {}  gen {}  role {}",
                crate::creature::species_name_for(c.species),
                c.generation,
                role_str
            ));
            lines.push(format!("energy {:.0}/{:.0}  health {:.0}  fitness {:.2}",
                c.energy, c.max_energy, c.health, c.fitness));
            lines.push(format!("age {}  size {:.1}  tech {}", c.age, c.size, c.tech_level));
            lines.push(format!("tool {}  carried {:.0}", tool_str, c.carried));
            lines.push(format!("kills {}  offspring {}", c.kills, c.offspring));
            // The life stage and the temperament.
            lines.push(format!(
                "stage: {} ({})",
                c.stage.name(),
                c.stage.blurb()
            ));
            lines.push(format!("temperament: {}", c.personality.describe()));
            lines.push(format!(
                "adapted to: {} ({})",
                c.affinity.name(),
                c.affinity.blurb()
            ));
            // The life history of the species.
            let strategy = if c.traits & creature::trait_bits::PROLIFIC != 0 {
                "prolific"
            } else if c.traits & creature::trait_bits::PROLONGED != 0 {
                "prolonged"
            } else {
                "balanced"
            };
            lines.push(format!(
                "life history: {}  ({} young, {:.0} ticks, lives {:.0})",
                strategy,
                c.litter_size(),
                c.lifespan() as f32,
                c.brood_cost()
            ));
            // The bond, if it has one.
            if let Some(partner) = c.bond {
                let alive = creatures.list.iter().any(|o| o.id == partner && o.health > 0.0);
                lines.push(format!(
                    "bonded to {} ({})",
                    partner,
                    if alive { "alive" } else { "lost" }
                ));
            }
            // What the species has learned to do. One practice per line: a
            // colony that knows everything would otherwise make a line
            // wider than the screen.
            let culture: Vec<String> = culture_list(c.culture)
                .iter()
                .map(|cu| format!("{} ({})", cu.name(), cu.blurb()))
                .collect();
            if !culture.is_empty() {
                lines.push(format!("colony knows {}:", culture.len()));
                for cu in culture {
                    lines.push(format!("  {}", cu));
                }
            }
            if c.disease != creature::Disease::None {
                let plague = creature::Plague::from(c.disease);
                lines.push(format!(
                    "disease: {} ({})",
                    plague.name(),
                    plague.blurb()
                ));
            }
            // The diplomatic standing of this creature and its species.
            if c.role == creature::Role::Envoy || c.pacts_signed > 0 {
                lines.push(format!(
                    "envoy: {} pacts signed  rep {}",
                    c.pacts_signed, c.reputation
                ));
            }
            let relations: Vec<String> = creatures
                .diplomacy
                .relations
                .iter()
                .filter(|r| r.a == c.species || r.b == c.species)
                .map(|r| {
                    let other = if r.a == c.species { r.b } else { r.a };
                    let treaties: Vec<&str> = r
                        .treaties
                        .iter()
                        .map(|t| t.name())
                        .collect();
                    let extra = if treaties.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", treaties.join(", "))
                    };
                    format!(
                        "vs the {}: {} trust {:.0}{}",
                        crate::creature::species_name_for(other),
                        r.state.name(),
                        r.trust,
                        extra
                    )
                })
                .collect();
            for line in relations.iter().take(4) {
                lines.push(line.clone());
            }
            // Who this species is afraid of, and who fears it.
            let fears: Vec<String> = creatures
                .predation
                .entries()
                .iter()
                .filter(|(p, _, f)| *p == c.species && *f > 1.0)
                .map(|(_, q, f)| {
                    format!(
                        "afraid of the {} (fear {:.0})",
                        crate::creature::species_name_for(*q),
                        f
                    )
                })
                .collect();
            for line in fears.iter().take(3) {
                lines.push(line.clone());
            }
            // What the treaties actually mean, spelled out for the player.
            for line in crate::ui::diplomacy::treaty_blurbs(creatures, c.species) {
                lines.push(format!("  {}", line));
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
    let margin = 4.0;
    // The box never grows wider than the screen. A long line is cut short
    // rather than the box running off the edge, which would leave the
    // clamp below with nothing to clamp to.
    let box_w = (max_w + pad * 2.0).min(screen_width() - margin * 2.0).max(64.0);
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
    // The bounds must never cross over, or the clamp panics on a window
    // that is smaller than the box.
    let max_bx = (screen_width() - box_w - margin).max(margin);
    let max_by = (screen_height() - box_h - margin).max(margin);
    bx = bx.clamp(margin, max_bx);
    by = by.clamp(margin, max_by);

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
pub fn draw_knowledge_window(world: &World, creatures: &CreatureSystem, ui: &UiState) {
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
        ("HUNT", "charge the closest rival"),
        ("PROWL", "roam away from the colony"),
        ("HOARD", "carry the haul back home"),
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

    // ---- The landmarks and who has claimed them ----
    if !world.landmarks.is_empty() {
        yy += 8.0;
        draw_text(
            "LANDMARKS",
            x + 16.0,
            yy,
            16.0,
            Color::new(0.85, 0.9, 0.95, 0.9),
        );
        yy += 22.0;
        for l in world.landmarks.iter() {
            // Who lives here?
            let mut occupants: Vec<(u32, u32)> = Vec::new();
            for c in &creatures.list {
                if c.role == creature::Role::Queen
                    && ((c.base_x - l.x).powi(2) + (c.base_y - l.y).powi(2)).sqrt()
                        <= l.kind.radius() * 1.6
                {
                    let n = creatures
                        .list
                        .iter()
                        .filter(|o| o.species == c.species)
                        .count() as u32;
                    occupants.push((c.species, n));
                }
            }
            occupants.sort_by(|a, b| b.1.cmp(&a.1));
            let holders: Vec<String> = occupants
                .iter()
                .take(3)
                .map(|(sp, n)| {
                    format!("{} ({})", creature::species_name_for(*sp), n)
                })
                .collect();
            draw_text(
                &format!("{}: {}", l.kind.name(), l.kind.blurb()),
                x + 24.0,
                yy + 8.0,
                12.0,
                Color::new(0.75, 0.85, 0.95, 0.9),
            );
            if !holders.is_empty() {
                draw_text(
                    &format!("held by {}", holders.join(", ")),
                    x + 40.0,
                    yy + 22.0,
                    11.0,
                    Color::new(0.6, 0.7, 0.8, 0.8),
                );
            }
            yy += 36.0;
        }
        yy += 6.0;
    }

    {
        // (species, members, traits, colour, affinity, culture)
        let mut counts: Vec<(u32, u32, u32, [u8; 3], Affinity, u16)> = Vec::new();
        for c in &creatures.list {
            match counts.iter_mut().find(|(id, _, _, _, _, _)| *id == c.species) {
                Some(e) => e.1 += 1,
                None => counts.push((
                    c.species,
                    1,
                    c.traits,
                    c.team_color,
                    c.affinity,
                    c.culture,
                )),
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        if !counts.is_empty() {
            draw_text(
                "LIVING SPECIES",
                x + 16.0,
                yy,
                16.0,
                Color::new(0.85, 0.9, 0.95, 0.9),
            );
            yy += 22.0;
            for (sp, n, traits, color, affinity, culture) in counts.iter().take(12) {
                draw_rectangle(
                    x + 24.0,
                    yy - 2.0,
                    10.0,
                    10.0,
                    Color::new(
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                        0.9,
                    ),
                );
                draw_text(
                    &format!("the {}", crate::creature::species_name_for(*sp)),
                    x + 40.0,
                    yy + 8.0,
                    13.0,
                    Color::new(0.9, 0.95, 1.0, 0.95),
                );
                draw_text(
                    &format!(
                        "{} members  {}",
                        n,
                        trait_list(*traits)
                            .iter()
                            .map(|t| t.name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    x + 130.0,
                    yy + 8.0,
                    11.0,
                    Color::new(0.65, 0.75, 0.7, 0.85),
                );
                // What the species has become adapted to, and what it has
                // learned to do.
                let practices: Vec<&str> = culture_list(*culture)
                    .iter()
                    .map(|cu| cu.name())
                    .collect();
                // A colony that has learned to hunt is a predator, and the
                // player should be able to see that at a glance.
                let hunting = culture_list(*culture)
                    .iter()
                    .any(|cu| *cu == creature::Culture::Hunting);
                // What the species has worked out for itself.
                let found: Vec<&str> = creature::discovery_list(
                    creatures.species_discoveries(*sp),
                )
                .iter()
                .map(|d| d.name())
                .collect();
                let extra = format!(
                    "adapted to: {}   practices: {}{}",
                    affinity.name(),
                    if practices.is_empty() {
                        "none yet".to_string()
                    } else {
                        practices.join(", ")
                    },
                    if hunting { "   [PREDATOR]" } else { "" }
                );
                draw_text(
                    &extra,
                    x + 40.0,
                    yy + 24.0,
                    11.0,
                    Color::new(0.6, 0.66, 0.78, 0.8),
                );
                if !found.is_empty() {
                    draw_text(
                        &format!("knows: {}", found.join(", ")),
                        x + 40.0,
                        yy + 38.0,
                        11.0,
                        Color::new(0.95, 0.82, 0.45, 0.85),
                    );
                    yy += 14.0;
                }
                yy += 40.0;
            }
            yy += 10.0;
        }
    }

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
                creature::Role::Envoy => "Envoy",
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
