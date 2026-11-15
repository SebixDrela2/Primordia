use crate::creature;
use crate::creature::{Creature, CreatureSystem, trait_list, culture_list, Affinity};
use crate::elements::Elem;
use crate::world::World;
use macroquad::prelude::*;

pub enum HoverTarget {
    None,
    Element { x: i32, y: i32, elem: Elem },
    Creature { id: u32 },
}

/// The panels the player can open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Panel {
    /// The window of what has been seen.
    Knowledge,
    /// The whole tech tree.
    Tech,
    /// The diplomatic record.
    Diplomacy,
    /// The minds of the cleverest.
    Minds,
}

impl Panel {
    /// The bit this panel takes in the set of open panels.
    #[must_use]
    pub fn bit(self) -> u8 {
        1u8 << (self as u8)
    }
}

/// The panels that are open. They are independent of each other - the
/// knowledge window and the tech tree can be open at the same time - so
/// they are held as a set of flags rather than as one state.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenPanels {
    bits: u8,
}

impl OpenPanels {
    /// Is this panel open?
    #[must_use]
    pub fn has(self, panel: Panel) -> bool {
        self.bits & panel.bit() != 0
    }

    /// Open this panel.
    pub fn open(&mut self, panel: Panel) {
        self.bits |= panel.bit();
    }

    /// Close this panel.
    pub fn close(&mut self, panel: Panel) {
        self.bits &= !panel.bit();
    }

    /// Open this panel if it is closed, and close it if it is open.
    pub fn toggle(&mut self, panel: Panel) {
        self.bits ^= panel.bit();
    }
}

/// What the player has found and which panels they have open.
pub struct UiState {
    pub     hover: HoverTarget,

    pub     panels: OpenPanels,

    pub     tech_scroll: f32,

    pub     minds_index: f32,
    pub     discovered_elements: Vec<u8>,
    pub     discovered_creatures: Vec<u32>,
}

impl UiState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            hover: HoverTarget::None,
            panels: OpenPanels::default(),
            tech_scroll: 0.0,
            minds_index: 0.0,
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

pub fn draw_tooltip(_world: &World, creatures: &CreatureSystem, ui: &UiState, mx: f32, my: f32) {
    match &ui.hover {
        HoverTarget::None => (),
        HoverTarget::Element { x, y, elem } => {
            let lines = element_tooltip_lines(*elem, *x, *y);
            draw_tooltip_box(&lines, mx, my);
        }
        HoverTarget::Creature { id } => {
            let Some(c) = creatures.list.iter().find(|c| c.id == *id) else {
                return;
            };
            let lines = creature_tooltip_lines(creatures, c);
            draw_tooltip_box(&lines, mx, my);
        }
    }
}

/// The lines of the tooltip for an element of the world.
fn element_tooltip_lines(elem: Elem, x: i32, y: i32) -> Vec<String> {
    let name = elem.name();
    let desc = elem.description();
    let category = elem.category();
    let density = elem.density();
    let tox = elem.toxicity();
    let nut = elem.nutrition();

    let mut lines: Vec<String> = Vec::new();
    lines.push(name.to_string());
    lines.push(format!("[{}]  pos=({},{})", category, x, y));
    lines.push(format!(
        "density={}  toxicity={:.0}  nutrition={:.1}",
        density, tox, nut
    ));
    if elem.is_hazardous() {
        lines.push("HAZARDOUS".to_string());
    }
    if elem.is_solid() {
        lines.push("solid".to_string());
    }
    if elem.flammable() {
        lines.push("flammable".to_string());
    }
    lines.push(desc.to_string());
    lines
}

/// The lines of the tooltip for one creature.
fn creature_tooltip_lines(creatures: &CreatureSystem, c: &Creature) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    push_creature_identity(&mut lines, c);
    push_creature_life_history(&mut lines, creatures, c);
    push_creature_culture(&mut lines, c);
    push_creature_envoy(&mut lines, c);
    push_creature_relations(&mut lines, creatures, c);
    push_creature_fears(&mut lines, creatures, c);
    push_creature_treaties(&mut lines, creatures, c);
    lines
}

/// Who it is and what state it is in.
fn push_creature_identity(lines: &mut Vec<String>, c: &Creature) {
    let role_str = role_name(c.role);
    let tool_str = tool_label(c.tool);

    let mut end = 10;
    for i in 0..10 {
        if c.name[i] == 0 {
            end = i;
            break;
        }
    }
    let name_str = core::str::from_utf8(&c.name[..end]).unwrap_or("");

    lines.push(format!("{}  (id {})", name_str, c.id));
    lines.push(format!(
        "the {}  gen {}  role {}",
        crate::creature::species_name_for(c.species),
        c.generation,
        role_str
    ));
    lines.push(format!(
        "energy {:.0}/{:.0}  health {:.0}  fitness {:.2}",
        c.energy, c.max_energy, c.health, c.fitness
    ));
    lines.push(format!(
        "age {}  size {:.1}  tech {}",
        c.age, c.size, c.tech_level
    ));
    lines.push(format!("tool {}  carried {:.0}", tool_str, c.carried));
    lines.push(format!("kills {}  offspring {}", c.kills, c.offspring));

    lines.push(format!("stage: {} ({})", c.stage.name(), c.stage.blurb()));
    lines.push(format!("temperament: {}", c.personality.describe()));
    lines.push(format!(
        "adapted to: {} ({})",
        c.affinity.name(),
        c.affinity.blurb()
    ));
}

/// How it breeds and who it is bound to.
fn push_creature_life_history(lines: &mut Vec<String>, creatures: &CreatureSystem, c: &Creature) {
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

    if let Some(partner) = c.bond {
        let alive = creatures.list.iter().any(|o| o.id == partner && o.health > 0.0);
        lines.push(format!(
            "bonded to {} ({})",
            partner,
            if alive { "alive" } else { "lost" }
        ));
    }
}

/// The practices of the colony, what it knows and what ails it.
fn push_creature_culture(lines: &mut Vec<String>, c: &Creature) {
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

    let tech = crate::ui::tech::tech_of(c);
    if tech != "nothing yet" {
        lines.push(format!("tech: {}", tech));
    }
    if c.disease != creature::Disease::None {
        let plague = creature::Plague::from(c.disease);
        lines.push(format!("disease: {} ({})", plague.name(), plague.blurb()));
    }
}

/// The pacts it has had a hand in.
fn push_creature_envoy(lines: &mut Vec<String>, c: &Creature) {
    if c.role == creature::Role::Envoy || c.pacts_signed > 0 {
        lines.push(format!(
            "envoy: {} pacts signed  rep {}",
            c.pacts_signed, c.reputation
        ));
    }
}

/// How its own species stands against the others.
fn push_creature_relations(lines: &mut Vec<String>, creatures: &CreatureSystem, c: &Creature) {
    let relations: Vec<String> = creatures
        .diplomacy
        .relations
        .iter()
        .filter(|r| r.a == c.species || r.b == c.species)
        .map(|r| {
            let other = if r.a == c.species { r.b } else { r.a };
            let treaties: Vec<&str> = r.treaties.iter().map(|t| t.name()).collect();
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
}

/// The hunters its species still runs from.
fn push_creature_fears(lines: &mut Vec<String>, creatures: &CreatureSystem, c: &Creature) {
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
}

/// The treaties its species has signed.
fn push_creature_treaties(lines: &mut Vec<String>, creatures: &CreatureSystem, c: &Creature) {
    for line in crate::ui::diplomacy::treaty_blurbs(creatures, c.species) {
        lines.push(format!("  {}", line));
    }
}

/// The name of a role, as the player reads it.
fn role_name(role: creature::Role) -> &'static str {
    match role {
        creature::Role::Worker => "Worker",
        creature::Role::Soldier => "Soldier",
        creature::Role::Queen => "Queen",
        creature::Role::Priest => "Priest",
        creature::Role::Merchant => "Merchant",
        creature::Role::Builder => "Builder",
        creature::Role::Explorer => "Explorer",
        creature::Role::Shaman => "Shaman",
        creature::Role::Envoy => "Envoy",
    }
}

/// The name of a tool, as the player reads it.
fn tool_label(tool: creature::Tool) -> String {
    match tool {
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
    }
}

pub fn draw_tooltip_box(lines: &[String], mx: f32, my: f32) {
    if lines.is_empty() {
        return;
    }

    let mut max_w: f32 = 0.0;
    let mut total_h = 0.0;
    for line in lines {
        let dims = measure_text(line, None, 14, 1.0);
        max_w = max_w.max(dims.width);
        total_h += 16.0;
    }
    total_h += 8.0;

    let pad = 6.0;
    let margin = 4.0;

    let box_w = (max_w + pad * 2.0).min(screen_width() - margin * 2.0).max(64.0);
    let box_h = total_h + pad * 2.0;

    let mut bx = mx + 14.0;
    let mut by = my + 14.0;
    if bx + box_w > screen_width() {
        bx = mx - box_w - 14.0;
    }
    if by + box_h > screen_height() {
        by = my - box_h - 14.0;
    }

    let max_box_x = (screen_width() - box_w - margin).max(margin);
    let max_box_y = (screen_height() - box_h - margin).max(margin);
    bx = bx.clamp(margin, max_box_x);
    by = by.clamp(margin, max_box_y);

    draw_rectangle(bx, by, box_w, box_h, Color::new(0.05, 0.05, 0.08, 0.85));
    draw_rectangle_lines(bx, by, box_w, box_h, 1.5, Color::new(0.3, 0.5, 0.7, 0.9));

    let title_color = Color::new(0.7, 0.85, 1.0, 1.0);
    draw_text(&lines[0], bx + pad, by + pad + 12.0, 14.0, title_color);

    let mut yy = by + pad + 12.0 + 16.0;
    for line in &lines[1..] {
        draw_text(line, bx + pad, yy, 12.0, Color::new(0.85, 0.9, 0.95, 0.85));
        yy += 15.0;
    }
}

pub fn draw_knowledge_window(world: &World, creatures: &CreatureSystem, ui: &UiState) {
    let panel_w = 520.0;
    let panel_h = screen_height() * 0.85;
    let panel_x = screen_width() - panel_w - 20.0;
    let panel_y = 40.0;

    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.09, 0.93));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, Color::new(0.3, 0.5, 0.7, 0.9));

    draw_knowledge_header(panel_x, panel_y);
    let mut yy = draw_behaviours(panel_x, panel_y) + 14.0;
    yy = draw_landmarks(world, creatures, panel_x, yy);
    yy = draw_living_species(creatures, panel_x, yy);
    yy = draw_discovered_elements(ui, panel_x, yy);
    yy = draw_discovered_creatures(creatures, ui, panel_x, yy);

    if ui.discovered_elements.is_empty() && ui.discovered_creatures.is_empty() {
        draw_text(
            "Hover over anything in the world to record it here.",
            panel_x + 24.0,
            yy,
            14.0,
            Color::new(0.6, 0.65, 0.7, 0.7),
        );
    }
}

/// The title of the window and the hint that closes it.
fn draw_knowledge_header(panel_x: f32, panel_y: f32) {
    draw_text(
        "KNOWLEDGE",
        panel_x + 16.0,
        panel_y + 28.0,
        20.0,
        Color::new(0.7, 0.85, 1.0, 1.0),
    );
    draw_rectangle_lines(
        panel_x + 16.0,
        panel_y + 34.0,
        100.0,
        2.0,
        1.0,
        Color::new(0.3, 0.5, 0.7, 0.6),
    );
    draw_text(
        "press K to close",
        panel_x + 16.0,
        panel_y + 52.0,
        14.0,
        Color::new(0.5, 0.55, 0.6, 0.7),
    );
}

/// The list of what a creature can be told to do.
fn draw_behaviours(panel_x: f32, panel_y: f32) -> f32 {
    draw_text(
        "BEHAVIOURS",
        panel_x + 16.0,
        panel_y + 76.0,
        13.0,
        Color::new(0.4, 0.75, 0.9, 0.85),
    );
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
    let mut ay = panel_y + 96.0;
    for (name, desc) in &abilities {
        draw_text(
            name,
            panel_x + 24.0,
            ay,
            12.0,
            Color::new(0.55, 0.9, 1.0, 0.9),
        );
        draw_text(
            desc,
            panel_x + 90.0,
            ay,
            12.0,
            Color::new(0.62, 0.68, 0.75, 0.8),
        );
        ay += 17.0;
    }
    ay
}

/// The landmarks of the world and who holds them.
fn draw_landmarks(world: &World, creatures: &CreatureSystem, panel_x: f32, mut yy: f32) -> f32 {
    if world.landmarks.is_empty() {
        return yy;
    }

    yy += 8.0;
    draw_text(
        "LANDMARKS",
        panel_x + 16.0,
        yy,
        16.0,
        Color::new(0.85, 0.9, 0.95, 0.9),
    );
    yy += 22.0;
    for landmark in &world.landmarks {
        let holders = landmark_holders(creatures, landmark);
        draw_text(
            format!("{}: {}", landmark.kind.name(), landmark.kind.blurb()),
            panel_x + 24.0,
            yy + 8.0,
            12.0,
            Color::new(0.75, 0.85, 0.95, 0.9),
        );
        if !holders.is_empty() {
            draw_text(
                format!("held by {}", holders.join(", ")),
                panel_x + 40.0,
                yy + 22.0,
                11.0,
                Color::new(0.6, 0.7, 0.8, 0.8),
            );
        }
        yy += 36.0;
    }
    yy += 6.0;
    yy
}

/// The colonies that sit on a landmark, the biggest ones first.
fn landmark_holders(
    creatures: &CreatureSystem,
    landmark: &crate::world::LandmarkSite,
) -> Vec<String> {
    let mut occupants: Vec<(u32, u32)> = Vec::new();
    for unit in &creatures.list {
        if unit.role == creature::Role::Queen
            && ((unit.base_x - landmark.x).powi(2) + (unit.base_y - landmark.y).powi(2)).sqrt()
                <= landmark.kind.radius() * 1.6
        {
            let members = creatures
                .list
                .iter()
                .filter(|o| o.species == unit.species)
                .count() as u32;
            occupants.push((unit.species, members));
        }
    }
    occupants.sort_by_key(|b| std::cmp::Reverse(b.1));
    occupants
        .iter()
        .take(3)
        .map(|(sp, members)| format!("{} ({})", creature::species_name_for(*sp), members))
        .collect()
}

/// The species that are still alive and what each of them knows.
fn draw_living_species(creatures: &CreatureSystem, panel_x: f32, mut yy: f32) -> f32 {
    let mut counts: Vec<(u32, u32, u32, [u8; 3], Affinity, u16)> = Vec::new();
    for unit in &creatures.list {
        match counts.iter_mut().find(|(id, _, _, _, _, _)| *id == unit.species) {
            Some(entry) => entry.1 += 1,
            None => counts.push((
                unit.species,
                1,
                unit.traits,
                unit.team_color,
                unit.affinity,
                unit.culture,
            )),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if counts.is_empty() {
        return yy;
    }

    draw_text(
        "LIVING SPECIES",
        panel_x + 16.0,
        yy,
        16.0,
        Color::new(0.85, 0.9, 0.95, 0.9),
    );
    yy += 22.0;
    for row in counts.iter().take(12) {
        yy = draw_species_row(creatures, panel_x, yy, row);
    }
    yy += 10.0;
    yy
}

/// One species: its colour, its numbers, its practices and its discoveries.
fn draw_species_row(
    creatures: &CreatureSystem,
    panel_x: f32,
    yy: f32,
    row: &(u32, u32, u32, [u8; 3], Affinity, u16),
) -> f32 {
    let (sp, members, traits, color, affinity, culture) = row;

    draw_rectangle(
        panel_x + 24.0,
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
        format!("the {}", crate::creature::species_name_for(*sp)),
        panel_x + 40.0,
        yy + 8.0,
        13.0,
        Color::new(0.9, 0.95, 1.0, 0.95),
    );
    draw_text(
        format!(
            "{} members  {}",
            members,
            trait_list(*traits)
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        panel_x + 130.0,
        yy + 8.0,
        11.0,
        Color::new(0.65, 0.75, 0.7, 0.85),
    );

    draw_species_details(creatures, panel_x, yy, *sp, *affinity, *culture)
}

/// What a species has adapted to, what it does and what it knows.
fn draw_species_details(
    creatures: &CreatureSystem,
    panel_x: f32,
    mut yy: f32,
    sp: u32,
    affinity: Affinity,
    culture: u16,
) -> f32 {
    let practices: Vec<&str> = culture_list(culture).iter().map(|cu| cu.name()).collect();
    let hunting = culture_list(culture).contains(&creature::Culture::Hunting);
    let found: Vec<&str> = creature::discovery_list(creatures.species_discoveries(sp))
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
        panel_x + 40.0,
        yy + 24.0,
        11.0,
        Color::new(0.6, 0.66, 0.78, 0.8),
    );
    if !found.is_empty() {
        draw_text(
            format!("knows: {}", found.join(", ")),
            panel_x + 40.0,
            yy + 38.0,
            11.0,
            Color::new(0.95, 0.82, 0.45, 0.85),
        );
        yy += 14.0;
    }
    yy += 40.0;
    yy
}

/// The elements the player has found, grouped by what they are.
fn draw_discovered_elements(ui: &UiState, panel_x: f32, mut yy: f32) -> f32 {
    if ui.discovered_elements.is_empty() {
        return yy;
    }

    draw_text(
        "ELEMENTS DISCOVERED",
        panel_x + 16.0,
        yy,
        16.0,
        Color::new(0.85, 0.9, 0.95, 0.9),
    );
    yy += 22.0;

    let mut cats: Vec<(&str, Vec<u8>)> = Vec::new();
    for &id in &ui.discovered_elements {
        let elem = Elem::from_u8(id);
        let cat = elem.category();
        if let Some(pos) = cats.iter_mut().find(|(name, _)| *name == cat) {
            pos.1.push(id);
        } else {
            cats.push((cat, vec![id]));
        }
    }

    for (cat, ids) in cats {
        draw_text(
            cat,
            panel_x + 24.0,
            yy,
            13.0,
            Color::new(0.6, 0.7, 0.8, 0.8),
        );
        yy += 18.0;
        for &id in &ids {
            yy = draw_element_entry(panel_x, yy, id);
        }
        yy += 8.0;
    }
    yy += 16.0;
    yy
}

/// One discovered element: its colour, its name and a line about it.
fn draw_element_entry(panel_x: f32, yy: f32, id: u8) -> f32 {
    let elem = Elem::from_u8(id);
    let name = elem.name();
    let desc = elem.description();
    let color = elem.color();

    draw_rectangle(
        panel_x + 24.0,
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
        name,
        panel_x + 40.0,
        yy + 8.0,
        13.0,
        Color::new(0.9, 0.95, 1.0, 0.95),
    );

    let short = if desc.len() > 60 {
        format!("{}...", &desc[..60])
    } else {
        desc.to_string()
    };
    draw_text(
        &short,
        panel_x + 40.0,
        yy + 20.0,
        11.0,
        Color::new(0.65, 0.7, 0.75, 0.8),
    );
    yy + 34.0
}

/// The creatures the player has laid eyes on.
fn draw_discovered_creatures(
    creatures: &CreatureSystem,
    ui: &UiState,
    panel_x: f32,
    mut yy: f32,
) -> f32 {
    if ui.discovered_creatures.is_empty() {
        return yy;
    }

    draw_text(
        "CREATURES OBSERVED",
        panel_x + 16.0,
        yy,
        16.0,
        Color::new(0.85, 0.9, 0.95, 0.9),
    );
    yy += 22.0;

    for &id in &ui.discovered_creatures {
        let found = creatures.list.iter().find(|unit| unit.id == id);
        let Some(unit) = found else { continue };

        let role_str = role_name(unit.role);

        let mut end = 10;
        for idx in 0..10 {
            if unit.name[idx] == 0 {
                end = idx;
                break;
            }
        }
        let name_str = core::str::from_utf8(&unit.name[..end]).unwrap_or("");

        let swatch_color = unit.team_color;
        draw_rectangle(
            panel_x + 24.0,
            yy - 2.0,
            10.0,
            10.0,
            Color::new(
                swatch_color[0] as f32 / 255.0,
                swatch_color[1] as f32 / 255.0,
                swatch_color[2] as f32 / 255.0,
                0.9,
            ),
        );
        draw_text(
            name_str,
            panel_x + 40.0,
            yy + 8.0,
            13.0,
            Color::new(0.9, 0.95, 1.0, 0.95),
        );
        draw_text(format!("species {}  gen {}  role {}  energy {:.0}", unit.species, unit.generation, role_str, unit.energy),
            panel_x + 40.0, yy + 20.0, 11.0, Color::new(0.65, 0.7, 0.75, 0.8));
        yy += 34.0;
    }
    yy
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
