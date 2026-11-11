#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Elem {
    Empty = 0,
    // Powders
    Sand,
    Sulfur,
    Salt,
    Ash,
    Dust,
    Gunpowder,
    Fertilizer,
            Mud,
    // Liquids
    Water,
    Oil,
    Acid,
    Mercury,
    Blood,
    Poison,
    Alcohol,
    Honey,
    // Gases
    Steam,
    Smoke,
    Gas,
    Plasma,
    Chlorine,
    Methane,
    Oxygen,
    // Solids
    Stone,
    Metal,
    Copper,
    Tin,
    Bronze,
    Iron,
    Steel,
    Gold,
    Silver,
    Glass,
    Crystal,
    Gem,
    Diamond,
    Obsidian,
    // Organic
    Wood,
    Plant,
    Spore,
    Fungus,
    Moss,
    Roots,
    Meat,
    Bone,
    Leather,
    // Energy/Exotic
    Fire,
    Energy,
    Ice,
    Lava,
    Magma,
    Void,
    Mana,
    Soul,
    // Constructed
    Brick,
    Concrete,
    Wire,
    Circuit,
    Tool,
    Machine,
    Wall,
    Door,
    Chest,
    Furnace,
    Anvil,
    Forge,
    Farm,
    Temple,
    Tower,
    Bridge,
    Road,
    // Magical
    Rune,
    Ward,
    Portal,
    // Biological
    Egg,
    Corpse,
    Virus,
    Bacteria,
            Food,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    Static,
    Fall,
    Flow,
    Rise,
    Diffuse,
    Burn,
    Grow,
    Energy,
    Conduct, // for electricity/wires
    Spread,  // for viruses/bacteria
    Hatch,   // for eggs
    Decay,   // for corpses
}

impl Elem {
    /// Convert a raw byte back into an element. `Elem` is `#[repr(u8)]`, so the
    /// discriminant is authoritative. This must stay in lock-step with the enum
    /// declaration (it previously hand-duplicated a shifted table and silently
    /// mapped every stored cell to the wrong element).
    pub fn from_u8(v: u8) -> Elem {
        if v <= Elem::Food as u8 {
            // SAFETY: `Elem` is `#[repr(u8)]` and `v` is a declared discriminant.
            unsafe { std::mem::transmute::<u8, Elem>(v) }
        } else {
            Elem::Empty
        }
    }

    /// The largest valid element discriminant (inclusive).
    pub const MAX: u8 = Elem::Food as u8;

    pub fn color(self) -> [u8; 3] {
        match self {
            Elem::Empty => [10, 14, 23],
            // Powders
            Elem::Sand => [232, 197, 138],
            Elem::Sulfur => [255, 220, 60],
            Elem::Salt => [220, 220, 230],
            Elem::Ash => [80, 80, 85],
            Elem::Dust => [140, 130, 120],
            Elem::Gunpowder => [40, 40, 45],
            Elem::Fertilizer => [100, 140, 80],
            Elem::Mud => [120, 100, 80],
            // Liquids
            Elem::Water => [56, 132, 255],
            Elem::Oil => [58, 46, 34],
            Elem::Acid => [154, 255, 82],
            Elem::Mercury => [190, 195, 210],
            Elem::Blood => [180, 30, 30],
            Elem::Poison => [100, 255, 100],
            Elem::Alcohol => [200, 200, 220],
            Elem::Honey => [255, 200, 60],
            // Gases
            Elem::Steam => [196, 214, 226],
            Elem::Smoke => [96, 104, 116],
            Elem::Gas => [130, 200, 150],
            Elem::Plasma => [255, 80, 240],
            Elem::Chlorine => [180, 255, 180],
            Elem::Methane => [100, 100, 120],
            Elem::Oxygen => [150, 200, 255],
            // Solids
            Elem::Stone => [116, 120, 128],
            Elem::Metal => [172, 182, 196],
            Elem::Copper => [184, 115, 51],
            Elem::Tin => [180, 180, 190],
            Elem::Bronze => [205, 127, 50],
            Elem::Iron => [120, 120, 130],
            Elem::Steel => [160, 160, 170],
            Elem::Gold => [255, 215, 60],
            Elem::Silver => [192, 192, 192],
            Elem::Glass => [186, 226, 232],
            Elem::Crystal => [160, 200, 255],
            Elem::Gem => [100, 255, 180],
            Elem::Diamond => [220, 240, 255],
            Elem::Obsidian => [30, 20, 40],
            // Organic
            Elem::Wood => [124, 86, 48],
            Elem::Plant => [52, 186, 92],
            Elem::Spore => [148, 92, 214],
            Elem::Fungus => [180, 100, 160],
            Elem::Moss => [60, 140, 70],
            Elem::Roots => [100, 70, 40],
            Elem::Meat => [200, 80, 80],
            Elem::Bone => [230, 225, 210],
            Elem::Leather => [160, 120, 80],
            // Energy/Exotic
            Elem::Fire => [255, 107, 32],
            Elem::Energy => [255, 232, 64],
            Elem::Ice => [168, 222, 255],
            Elem::Lava => [255, 92, 20],
            Elem::Magma => [200, 60, 10],
            Elem::Void => [0, 0, 0],
            Elem::Mana => [180, 100, 255],
            Elem::Soul => [255, 255, 200],
            // Constructed
            Elem::Brick => [178, 90, 60],
            Elem::Concrete => [140, 140, 145],
            Elem::Wire => [120, 120, 140],
            Elem::Circuit => [80, 180, 255],
            Elem::Tool => [200, 180, 100],
            Elem::Machine => [100, 120, 160],
            Elem::Wall => [90, 90, 100],
            Elem::Door => [130, 100, 70],
            Elem::Chest => [140, 110, 60],
            Elem::Furnace => [120, 60, 40],
            Elem::Anvil => [100, 100, 110],
            Elem::Forge => [150, 80, 40],
            Elem::Farm => [80, 150, 60],
            Elem::Temple => [200, 180, 140],
            Elem::Tower => [110, 110, 120],
            Elem::Bridge => [130, 120, 110],
            Elem::Road => [80, 80, 85],
            // Magical
            Elem::Rune => [255, 150, 255],
            Elem::Ward => [150, 255, 255],
            Elem::Portal => [255, 200, 255],
            // Biological
            Elem::Egg => [240, 230, 180],
            Elem::Corpse => [80, 70, 60],
            Elem::Virus => [255, 80, 80],
            Elem::Bacteria => [200, 180, 100],
            Elem::Food => [200, 180, 120],
        }
    }

    /// Elements that have a branch in `World::try_reaction`. Kept in sync
    /// with the reaction match in `world.rs`.
    pub fn is_reactive(self) -> bool {
        matches!(
            self,
            Elem::Water
                | Elem::Lava
                | Elem::Magma
                | Elem::Acid
                | Elem::Ice
                | Elem::Plant
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Fire
                | Elem::Oil
                | Elem::Sulfur
                | Elem::Mercury
                | Elem::Plasma
                | Elem::Chlorine
                | Elem::Methane
                | Elem::Oxygen
                | Elem::Gunpowder
                | Elem::Fertilizer
                | Elem::Virus
                | Elem::Bacteria
                | Elem::Egg
                | Elem::Corpse
                | Elem::Mana
                | Elem::Soul
                | Elem::Rune
                | Elem::Ward
        )
    }

    /// True when a cell can never change on its own: no reaction, no
    /// behaviour and no lifetime. The world step skips these entirely, which
    /// is what keeps the simulation fast (most of the world is stone).
    pub fn is_inert(self) -> bool {
        !self.is_reactive()
            && self.behavior() == Behavior::Static
            && self.lifetime() == u32::MAX
    }

    /// Lookup table: is this element byte inert? Built once at startup.
    pub fn inert_table() -> [bool; 80] {
        let mut t = [false; 80];
        let mut i = 0;
        while i < 80 {
            t[i] = Elem::from_u8(i as u8).is_inert();
            i += 1;
        }
        t
    }

    /// Lookup table: is this element byte solid? Built once at startup.
    pub fn solid_table() -> [bool; 80] {
        let mut t = [false; 80];
        let mut i = 0;
        while i < 80 {
            t[i] = Elem::from_u8(i as u8).is_solid();
            i += 1;
        }
        t
    }

    pub fn behavior(self) -> Behavior {
        match self {
            Elem::Sand | Elem::Sulfur | Elem::Salt | Elem::Ash | Elem::Dust | Elem::Gunpowder | Elem::Fertilizer => Behavior::Fall,
            Elem::Water | Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Blood | Elem::Poison | Elem::Alcohol | Elem::Honey | Elem::Lava | Elem::Magma => Behavior::Flow,
            Elem::Fire => Behavior::Burn,
            Elem::Smoke | Elem::Steam | Elem::Gas | Elem::Plasma | Elem::Chlorine | Elem::Methane | Elem::Oxygen => Behavior::Rise,
            Elem::Spore | Elem::Moss => Behavior::Diffuse,
            Elem::Energy | Elem::Mana | Elem::Soul => Behavior::Energy,
            Elem::Plant | Elem::Fungus | Elem::Roots => Behavior::Grow,
            Elem::Wire | Elem::Circuit => Behavior::Conduct,
            Elem::Virus | Elem::Bacteria => Behavior::Spread,
            Elem::Egg => Behavior::Hatch,
            Elem::Corpse => Behavior::Decay,
            _ => Behavior::Static,
        }
    }

    pub fn density(self) -> i16 {
        match self {
            Elem::Empty => 0,
            Elem::Gas | Elem::Smoke | Elem::Steam | Elem::Plasma | Elem::Energy | Elem::Mana | Elem::Soul | Elem::Fire => 1,
            Elem::Chlorine | Elem::Methane | Elem::Oxygen => 2,
            Elem::Sulfur | Elem::Salt | Elem::Ash | Elem::Dust | Elem::Spore | Elem::Gunpowder | Elem::Fertilizer | Elem::Mud => 3,
            Elem::Oil => 7,
            Elem::Alcohol => 8,
            Elem::Water => 10,
            Elem::Poison | Elem::Honey => 11,
            Elem::Acid => 12,
            Elem::Blood => 11,
            Elem::Sand => 16,
            Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Ice => 9,
            Elem::Wood => 8,
            Elem::Roots => 10,
            Elem::Meat | Elem::Leather => 11,
            Elem::Bone => 14,
            Elem::Mercury => 13,
            Elem::Lava => 15,
            Elem::Magma => 16,
            Elem::Stone | Elem::Brick | Elem::Concrete | Elem::Wall | Elem::Furnace | Elem::Anvil | Elem::Forge | Elem::Tower => 20,
            Elem::Metal | Elem::Iron | Elem::Steel => 22,
            Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Wire | Elem::Gold | Elem::Silver => 22,
            Elem::Glass | Elem::Crystal | Elem::Gem | Elem::Diamond | Elem::Obsidian => 18,
            Elem::Circuit | Elem::Tool | Elem::Machine | Elem::Door | Elem::Chest | Elem::Forge | Elem::Farm | Elem::Temple | Elem::Bridge | Elem::Road => 24,
            Elem::Rune | Elem::Ward | Elem::Portal | Elem::Egg => 15,
            Elem::Corpse => 12,
            Elem::Virus | Elem::Bacteria | Elem::Food => 1,
            Elem::Void => 100,
        }
    }

    pub fn nutrition(self) -> f32 {
        match self {
            Elem::Water => 2.0,
            Elem::Wood => 20.0,
            Elem::Oil => 40.0,
            Elem::Plant => 30.0,
            Elem::Spore => 10.0,
            Elem::Fungus => 15.0,
            Elem::Moss => 8.0,
            Elem::Roots => 12.0,
            Elem::Energy => 100.0,
            Elem::Gas => 20.0,
            Elem::Stone => 2.0,
            Elem::Metal => 4.0,
            Elem::Copper => 8.0,
            Elem::Tin => 6.0,
            Elem::Bronze => 10.0,
            Elem::Iron => 12.0,
            Elem::Steel => 15.0,
            Elem::Gold => 12.0,
            Elem::Silver => 10.0,
            Elem::Glass => 2.0,
            Elem::Crystal => 6.0,
            Elem::Gem => 20.0,
            Elem::Diamond => 50.0,
            Elem::Obsidian => 8.0,
            Elem::Brick => 3.0,
            Elem::Concrete => 2.0,
            Elem::Wire => 5.0,
            Elem::Circuit => 15.0,
            Elem::Tool => 25.0,
            Elem::Machine => 40.0,
            Elem::Meat => 50.0,
            Elem::Bone => 10.0,
            Elem::Leather => 15.0,
            Elem::Blood => 30.0,
            Elem::Honey => 40.0,
            Elem::Alcohol => 15.0,
            Elem::Mana => 80.0,
            Elem::Soul => 200.0,
            Elem::Egg => 60.0,
            Elem::Corpse => 40.0,
            Elem::Mud => 5.0,
            Elem::Food => 25.0,
            // Gases/energy: small nutrition
            Elem::Chlorine => 1.0,
            Elem::Methane => 5.0,
            Elem::Oxygen => 2.0,
            Elem::Steam => 1.0,
            Elem::Smoke => 0.5,
            Elem::Fire => 0.5,
            Elem::Virus => 0.5,
            Elem::Bacteria => 0.5,
            // Constructed/decorative: not food
            Elem::Brick => 0.0,
            Elem::Concrete => 0.0,
            Elem::Chest => 0.0,
            Elem::Forge => 0.0,
            Elem::Farm => 0.0,
            Elem::Temple => 0.0,
            Elem::Machine => 0.0,
            Elem::Tower => 0.0,
            Elem::Bridge => 0.0,
            Elem::Road => 0.0,
            Elem::Wall => 0.0,
            Elem::Door => 0.0,
            Elem::Wire => 0.0,
            Elem::Circuit => 0.0,
            Elem::Tool => 0.0,
            _ => 0.0,
        }
    }

    pub fn toxicity(self) -> f32 {
        match self {
            Elem::Fire => 10.0,
            Elem::Smoke => 5.0,
            Elem::Lava => 50.0,
            Elem::Magma => 60.0,
            Elem::Oil => 10.0,
            Elem::Acid => 100.0,
            Elem::Mercury => 30.0,
            Elem::Plasma => 200.0,
            Elem::Sulfur => 5.0,
            Elem::Ash => 8.0,
            Elem::Poison => 200.0,
            Elem::Chlorine => 150.0,
            Elem::Virus => 300.0,
            Elem::Bacteria => 100.0,
            Elem::Gunpowder => 5.0,
            _ => 0.0,
        }
    }

    pub fn lifetime(self) -> u32 {
        match self {
            Elem::Fire => 90,
            Elem::Smoke => 320,
            Elem::Steam => 260,
            Elem::Gas => 400,
            Elem::Energy => 2600,
            Elem::Plasma => 120,
            Elem::Chlorine => 300,
            Elem::Methane => 200,
            Elem::Oxygen => 150,
            Elem::Mana => 1000,
            Elem::Soul => 500,
            Elem::Virus => 500,
            Elem::Bacteria => 400,
            Elem::Egg => 2000,
            Elem::Corpse => 3000,
            _ => u32::MAX,
        }
    }

    /// True for things a creature can actually eat/drink. Structural rock,
    /// ores and refined materials are *not* food even though they may have a
    /// small `nutrition` value used elsewhere. This stops creatures from
    /// eating the terrain bare.
    pub fn is_edible(self) -> bool {
        matches!(
            self,
            Elem::Water
                | Elem::Plant
                | Elem::Spore
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Meat
                | Elem::Food
                | Elem::Egg
                | Elem::Corpse
                | Elem::Honey
                | Elem::Blood
                | Elem::Alcohol
        )
    }

    pub fn is_passable(self) -> bool {
        matches!(
            self,
            Elem::Empty
                | Elem::Water
                | Elem::Smoke
                | Elem::Steam
                | Elem::Gas
                | Elem::Energy
                | Elem::Fire
                | Elem::Plasma
                | Elem::Chlorine
                | Elem::Methane
                | Elem::Oxygen
                | Elem::Mana
                | Elem::Soul
                | Elem::Alcohol
                | Elem::Honey
                | Elem::Blood
                | Elem::Poison
        )
    }

    pub fn is_solid(self) -> bool {
        matches!(
            self,
            Elem::Sand
                | Elem::Sulfur
                | Elem::Salt
                | Elem::Ash
                | Elem::Dust
                | Elem::Gunpowder
                | Elem::Fertilizer
                | Elem::Wood
                | Elem::Stone
                | Elem::Ice
                | Elem::Metal
                | Elem::Copper
                | Elem::Tin
                | Elem::Bronze
                | Elem::Iron
                | Elem::Steel
                | Elem::Gold
                | Elem::Silver
                | Elem::Glass
                | Elem::Crystal
                | Elem::Gem
                | Elem::Diamond
                | Elem::Obsidian
                | Elem::Brick
                | Elem::Concrete
                | Elem::Wire
                | Elem::Circuit
                | Elem::Tool
                | Elem::Machine
                | Elem::Wall
                | Elem::Door
                | Elem::Chest
                | Elem::Furnace
                | Elem::Anvil
                | Elem::Forge
                | Elem::Farm
                | Elem::Temple
                | Elem::Tower
                | Elem::Bridge
                | Elem::Road
                | Elem::Plant
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Bone
                | Elem::Leather
                | Elem::Rune
                | Elem::Ward
                | Elem::Portal
                | Elem::Egg
                | Elem::Corpse
        )
    }

    pub fn flammable(self) -> bool {
        matches!(
            self,
            Elem::Wood | Elem::Plant | Elem::Oil | Elem::Sulfur | Elem::Fungus | Elem::Spore | Elem::Moss | Elem::Roots | Elem::Gunpowder | Elem::Alcohol | Elem::Honey | Elem::Meat | Elem::Leather | Elem::Fertilizer
        )
    }

    pub fn conducts(self) -> bool {
        matches!(self, Elem::Metal | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Iron | Elem::Steel | Elem::Gold | Elem::Silver | Elem::Wire | Elem::Circuit | Elem::Machine | Elem::Tool)
    }

    pub fn melt_temp(self) -> u16 {
        match self {
            Elem::Ice => 0,
            Elem::Sulfur => 115,
            Elem::Salt => 800,
            Elem::Glass => 1400,
            Elem::Tin => 232,
            Elem::Copper => 1085,
            Elem::Gold => 1064,
            Elem::Silver => 962,
            Elem::Iron => 1538,
            Elem::Steel => 1370,
            Elem::Bronze => 950,
            Elem::Metal => 1500,
            Elem::Stone => 1600,
            Elem::Obsidian => 1200,
            _ => u16::MAX,
        }
    }

    pub fn is_metal(self) -> bool {
        matches!(self, Elem::Metal | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Iron | Elem::Steel | Elem::Gold | Elem::Silver)
    }

    pub fn is_gemstone(self) -> bool {
        matches!(self, Elem::Crystal | Elem::Gem | Elem::Diamond)
    }

    pub fn is_organic(self) -> bool {
        matches!(self, Elem::Wood | Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss | Elem::Roots | Elem::Meat | Elem::Bone | Elem::Leather | Elem::Egg | Elem::Corpse | Elem::Blood | Elem::Honey)
    }

    pub fn is_constructed(self) -> bool {
        matches!(self, Elem::Brick | Elem::Concrete | Elem::Wire | Elem::Circuit | Elem::Tool | Elem::Machine | Elem::Wall | Elem::Door | Elem::Chest | Elem::Furnace | Elem::Anvil | Elem::Forge | Elem::Farm | Elem::Temple | Elem::Tower | Elem::Bridge | Elem::Road)
    }

    pub fn is_magical(self) -> bool {
        matches!(self, Elem::Mana | Elem::Soul | Elem::Rune | Elem::Ward | Elem::Portal)
    }

    pub fn is_hazardous(self) -> bool {
        matches!(self, Elem::Fire | Elem::Lava | Elem::Magma | Elem::Acid | Elem::Poison | Elem::Chlorine | Elem::Plasma | Elem::Virus | Elem::Bacteria | Elem::Gunpowder)
    }

    /// Human-readable name of the element.
    pub fn name(self) -> &'static str {
        match self {
            Elem::Empty => "Empty",
            // Powders
            Elem::Sand => "Sand",
            Elem::Sulfur => "Sulfur",
            Elem::Salt => "Salt",
            Elem::Ash => "Ash",
            Elem::Dust => "Dust",
            Elem::Gunpowder => "Gunpowder",
            Elem::Fertilizer => "Fertilizer",
            Elem::Mud => "Mud",
            // Liquids
            Elem::Water => "Water",
            Elem::Oil => "Oil",
            Elem::Acid => "Acid",
            Elem::Mercury => "Mercury",
            Elem::Blood => "Blood",
            Elem::Poison => "Poison",
            Elem::Alcohol => "Alcohol",
            Elem::Honey => "Honey",
            // Gases
            Elem::Steam => "Steam",
            Elem::Smoke => "Smoke",
            Elem::Gas => "Gas",
            Elem::Plasma => "Plasma",
            Elem::Chlorine => "Chlorine",
            Elem::Methane => "Methane",
            Elem::Oxygen => "Oxygen",
            // Solids
            Elem::Stone => "Stone",
            Elem::Metal => "Metal",
            Elem::Copper => "Copper",
            Elem::Tin => "Tin",
            Elem::Bronze => "Bronze",
            Elem::Iron => "Iron",
            Elem::Steel => "Steel",
            Elem::Gold => "Gold",
            Elem::Silver => "Silver",
            Elem::Glass => "Glass",
            Elem::Crystal => "Crystal",
            Elem::Gem => "Gem",
            Elem::Diamond => "Diamond",
            Elem::Obsidian => "Obsidian",
            // Organic
            Elem::Wood => "Wood",
            Elem::Plant => "Plant",
            Elem::Spore => "Spore",
            Elem::Fungus => "Fungus",
            Elem::Moss => "Moss",
            Elem::Roots => "Roots",
            Elem::Meat => "Meat",
            Elem::Bone => "Bone",
            Elem::Leather => "Leather",
            // Energy/Exotic
            Elem::Fire => "Fire",
            Elem::Energy => "Energy",
            Elem::Ice => "Ice",
            Elem::Lava => "Lava",
            Elem::Magma => "Magma",
            Elem::Void => "Void",
            Elem::Mana => "Mana",
            Elem::Soul => "Soul",
            // Constructed
            Elem::Brick => "Brick",
            Elem::Concrete => "Concrete",
            Elem::Wire => "Wire",
            Elem::Circuit => "Circuit",
            Elem::Tool => "Tool",
            Elem::Machine => "Machine",
            Elem::Wall => "Wall",
            Elem::Door => "Door",
            Elem::Chest => "Chest",
            Elem::Furnace => "Furnace",
            Elem::Anvil => "Anvil",
            Elem::Forge => "Forge",
            Elem::Farm => "Farm",
            Elem::Temple => "Temple",
            Elem::Tower => "Tower",
            Elem::Bridge => "Bridge",
            Elem::Road => "Road",
            // Magical
            Elem::Rune => "Rune",
            Elem::Ward => "Ward",
            Elem::Portal => "Portal",
            // Biological
            Elem::Egg => "Egg",
            Elem::Corpse => "Corpse",
            Elem::Virus => "Virus",
            Elem::Bacteria => "Bacteria",
            Elem::Food => "Food",
        }
    }

    /// Short encyclopedia description shown in the knowledge window and hover tooltip.
    pub fn description(self) -> &'static str {
        match self {
            Elem::Empty => "Nothing here. Creatures can move freely.",
            // Powders
            Elem::Sand => "Fine granular powder. Falls under gravity. Melts into glass near lava.",
            Elem::Sulfur => "Bright yellow powder. Flammable; reacts with lava to make gas.",
            Elem::Salt => "White crystalline powder. Harmless seasoning.",
            Elem::Ash => "Grey powder left after burning. Can act as coal.",
            Elem::Dust => "Dry powder that drifts in the wind.",
            Elem::Gunpowder => "Explosive powder. Detonates when near fire or plasma.",
            Elem::Fertilizer => "Boosts plant growth when nearby.",
            Elem::Mud => "Wet dirt. Forms where water meets soil.",
            // Liquids
            Elem::Water => "Flowing liquid. Essential for life and plants. Extinguishes fire.",
            Elem::Oil => "Flammable dark liquid. Burns readily and can be gathered for energy.",
            Elem::Acid => "Corrosive green liquid. Dissolves most solids on contact.",
            Elem::Mercury => "Heavy silver liquid metal. Amalgamates with gold and copper.",
            Elem::Blood => "Organic red liquid. Nutritious but can carry disease.",
            Elem::Poison => "Toxic green liquid. Harms anything that touches or drinks it.",
            Elem::Alcohol => "Clear flammable liquid. Burns with a clean flame.",
            Elem::Honey => "Thick golden sweet liquid. High energy food.",
            // Gases
            Elem::Steam => "Hot water vapor. Rises and condenses back to water.",
            Elem::Smoke => "Grey gas from fires. Rises and fades over time.",
            Elem::Gas => "Generic flammable gas. Rises and can explode.",
            Elem::Plasma => "Superheated ionized gas. Very hot and toxic.",
            Elem::Chlorine => "Toxic pale-green gas. Poisons water and organic matter.",
            Elem::Methane => "Flammable gas from swamps. Explodes near fire.",
            Elem::Oxygen => "Breathable gas. Makes fires burn hotter.",
            // Solids
            Elem::Stone => "Hard common rock. Forms the crust of the world.",
            Elem::Metal => "Generic sturdy metal. Can be melted and alloyed.",
            Elem::Copper => "Reddish metal. Early-age conductor and tool material.",
            Elem::Tin => "Soft silvery metal. Alloys with copper to make bronze.",
            Elem::Bronze => "Hard copper-tin alloy. Better tools and weapons.",
            Elem::Iron => "Strong dark metal. Foundation of tools and armor.",
            Elem::Steel => "Refined iron-carbon alloy. Very strong and durable.",
            Elem::Gold => "Shiny yellow precious metal. Highly valued; soft and dense.",
            Elem::Silver => "Pale lustrous metal. Conducts energy well.",
            Elem::Glass => "Transparent solid made from melted sand.",
            Elem::Crystal => "Geometric mineral formation. Stores magical energy.",
            Elem::Gem => "Polished precious crystal. Rare and valuable.",
            Elem::Diamond => "Hardest natural substance. Extremely valuable.",
            Elem::Obsidian => "Volcanic glass. Forms where water meets lava.",
            // Organic
            Elem::Wood => "Solid plant material. Burns and floats on water.",
            Elem::Plant => "Living vegetation. Grows near water and spreads slowly.",
            Elem::Spore => "Reproductive dust from fungi and plants. Drifts on air.",
            Elem::Fungus => "Mushroom-like organism. Thrives in damp shaded areas.",
            Elem::Moss => "Low carpet-like plant. Grows on damp stone and wood.",
            Elem::Roots => "Underground plant tendrils. Anchor vegetation.",
            Elem::Meat => "Animal flesh. Nutritious food for creatures.",
            Elem::Bone => "Hard skeletal remains. Slow to decay.",
            Elem::Leather => "Treated hide. Durable crafting material.",
            // Energy/Exotic
            Elem::Fire => "Hot flame. Spreads to flammable neighbors and rises.",
            Elem::Energy => "Pure energy drifts above ground. High nutrition for creatures.",
            Elem::Ice => "Frozen water. Melts near heat sources.",
            Elem::Lava => "Molten rock. Extremely hot; turns to stone when cooled by water.",
            Elem::Magma => "Superheated lava. Even hotter; forms obsidian with water.",
            Elem::Void => "Absolute emptiness. Denser than anything; impassable.",
            Elem::Mana => "Magical energy. Powers spells and activates runes.",
            Elem::Soul => "Rare spiritual energy. Very high nutrition.",
            // Constructed
            Elem::Brick => "Fired clay block. Used in ancient ruins.",
            Elem::Concrete => "Strong artificial stone.",
            Elem::Wire => "Conductive filament for simple circuits.",
            Elem::Circuit => "Complex electronics. Conducts and processes signals.",
            Elem::Tool => "A general-purpose tool left by ruins.",
            Elem::Machine => "Ancient machinery. Complex and conductive.",
            Elem::Wall => "Solid barrier block.",
            Elem::Door => "An entrance that can be opened or blocked.",
            Elem::Chest => "A container that may hold loot.",
            Elem::Furnace => "A structure for smelting ores.",
            Elem::Anvil => "A forging station for metalwork.",
            Elem::Forge => "A high-temperature working station.",
            Elem::Farm => "A cultivated plot for growing food.",
            Elem::Temple => "A sacred structure for worship.",
            Elem::Tower => "A tall ancient observation tower.",
            Elem::Bridge => "A span crossing over gaps or water.",
            Elem::Road => "A paved path for travel.",
            // Magical
            Elem::Rune => "A magical sigil. Activates when touched by mana.",
            Elem::Ward => "A protective charm that repels disease.",
            Elem::Portal => "A gateway to another place. Rare and unstable.",
            // Biological
            Elem::Egg => "Hatches into a new creature after time passes.",
            Elem::Corpse => "Remains of a dead creature. Decays into bone or dust.",
            Elem::Virus => "Microscopic disease. Spreads rapidly through organic matter.",
            Elem::Bacteria => "Microbe that decomposes meat and food.",
            Elem::Food => "Prepared nourishment for creatures.",
        }
    }

    /// Category for grouping in the knowledge window.
    pub fn category(self) -> &'static str {
        match self {
            Elem::Empty => "Void",
            Elem::Sand | Elem::Sulfur | Elem::Salt | Elem::Ash | Elem::Dust | Elem::Gunpowder | Elem::Fertilizer | Elem::Mud => "Powders",
            Elem::Water | Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Blood | Elem::Poison | Elem::Alcohol | Elem::Honey => "Liquids",
            Elem::Steam | Elem::Smoke | Elem::Gas | Elem::Plasma | Elem::Chlorine | Elem::Methane | Elem::Oxygen => "Gases",
            Elem::Stone | Elem::Metal | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Iron | Elem::Steel | Elem::Gold | Elem::Silver | Elem::Glass | Elem::Crystal | Elem::Gem | Elem::Diamond | Elem::Obsidian => "Solids",
            Elem::Wood | Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss | Elem::Roots | Elem::Meat | Elem::Bone | Elem::Leather => "Organic",
            Elem::Fire | Elem::Energy | Elem::Ice | Elem::Lava | Elem::Magma | Elem::Mana | Elem::Soul => "Energy",
            Elem::Void => "Void",
            Elem::Brick | Elem::Concrete | Elem::Wire | Elem::Circuit | Elem::Tool | Elem::Machine | Elem::Wall | Elem::Door | Elem::Chest | Elem::Furnace | Elem::Anvil | Elem::Forge | Elem::Farm | Elem::Temple | Elem::Tower | Elem::Bridge | Elem::Road => "Constructed",
            Elem::Rune | Elem::Ward | Elem::Portal => "Magical",
            Elem::Egg | Elem::Corpse | Elem::Virus | Elem::Bacteria | Elem::Food => "Biological",
        }
    }
}