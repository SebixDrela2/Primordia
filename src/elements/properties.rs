use super::{Elem, Behavior};

impl Elem {
    #[must_use]
    pub fn color(self) -> [u8; 3] {
        match self {
            Elem::Empty => [10, 14, 23],

            Elem::Sand => [232, 197, 138],
            Elem::Sulfur => [255, 220, 60],
            Elem::Salt => [220, 220, 230],
            Elem::Ash | Elem::Road => [80, 80, 85],
            Elem::Dust => [140, 130, 120],
            Elem::Gunpowder => [40, 40, 45],
            Elem::Fertilizer => [100, 140, 80],
            Elem::Mud => [120, 100, 80],
            Elem::Snow => [238, 244, 252],
            Elem::Crop => [226, 196, 86],

            Elem::Water => [56, 132, 255],
            Elem::Oil => [58, 46, 34],
            Elem::Acid => [154, 255, 82],
            Elem::Mercury => [190, 195, 210],
            Elem::Blood => [180, 30, 30],
            Elem::Poison => [100, 255, 100],
            Elem::Alcohol => [200, 200, 220],
            Elem::Honey => [255, 200, 60],
            Elem::Tar => [28, 22, 20],

            Elem::Steam => [196, 214, 226],
            Elem::Smoke => [96, 104, 116],
            Elem::Gas => [130, 200, 150],
            Elem::Plasma => [255, 80, 240],
            Elem::Chlorine => [180, 255, 180],
            Elem::Methane => [100, 100, 120],
            Elem::Oxygen => [150, 200, 255],

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
            Elem::Granite => [92, 86, 96],
            Elem::Rust => [126, 74, 38],

            Elem::Wood => [124, 86, 48],
            Elem::Plant => [52, 186, 92],
            Elem::Spore => [148, 92, 214],
            Elem::Fungus => [180, 100, 160],
            Elem::Moss => [60, 140, 70],
            Elem::Roots => [100, 70, 40],
            Elem::Meat => [200, 80, 80],
            Elem::Bone => [230, 225, 210],
            Elem::Leather => [160, 120, 80],
            Elem::Charcoal => [38, 36, 38],

            Elem::Fire => [255, 107, 32],
            Elem::Energy => [255, 232, 64],
            Elem::Ice => [168, 222, 255],
            Elem::Lava => [255, 92, 20],
            Elem::Magma => [200, 60, 10],
            Elem::Void => [0, 0, 0],
            Elem::Mana => [180, 100, 255],
            Elem::Soul => [255, 255, 200],
            Elem::Ember => [220, 90, 30],

            Elem::Brick => [178, 90, 60],
            Elem::Concrete => [140, 140, 145],
            Elem::Wire => [120, 120, 140],
            Elem::Circuit => [80, 180, 255],
            Elem::Tool | Elem::Bacteria => [200, 180, 100],
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

            Elem::Rune => [255, 150, 255],
            Elem::Ward => [150, 255, 255],
            Elem::Portal => [255, 200, 255],

            Elem::Egg => [240, 230, 180],
            Elem::Corpse => [80, 70, 60],
            Elem::Virus => [255, 80, 80],
            Elem::Food => [200, 180, 120],

            Elem::Market => [190, 150, 90],
            Elem::Workshop => [150, 130, 110],
            Elem::Granary => [160, 140, 80],
            Elem::Beacon => [255, 170, 90],
            Elem::Hearth => [230, 120, 60],
            Elem::Totem => [180, 120, 200],
        }
    }

    #[must_use]
    pub fn emission(self) -> [f32; 3] {
        match self {
            Elem::Fire => [1.00, 0.42, 0.10],
            Elem::Lava => [1.00, 0.34, 0.06],
            Elem::Magma => [1.00, 0.26, 0.04],
            Elem::Ember => [0.80, 0.30, 0.08],
            Elem::Plasma => [0.95, 0.30, 0.95],
            Elem::Mana => [0.55, 0.28, 0.95],
            Elem::Soul => [0.90, 0.88, 0.60],
            Elem::Energy => [0.80, 0.72, 0.22],
            Elem::Crystal => [0.26, 0.42, 0.70],
            Elem::Gem => [0.20, 0.70, 0.45],
            Elem::Diamond => [0.30, 0.38, 0.55],
            Elem::Rune => [0.62, 0.30, 0.62],
            Elem::Ward => [0.30, 0.70, 0.70],
            Elem::Portal => [0.70, 0.45, 0.70],
            Elem::Circuit => [0.20, 0.48, 0.70],
            Elem::Beacon => [1.00, 0.62, 0.22],
            Elem::Hearth => [1.00, 0.45, 0.12],
            Elem::Totem => [0.50, 0.26, 0.72],

            Elem::Poison => [0.11, 0.19, 0.07],
            Elem::Acid => [0.15, 0.21, 0.05],
            Elem::Virus => [0.24, 0.05, 0.08],
            Elem::Bacteria => [0.16, 0.13, 0.03],
            Elem::Chlorine => [0.08, 0.16, 0.10],
            _ => [0.0, 0.0, 0.0],
        }
    }

    #[must_use]
    pub fn is_reactive(self) -> bool {
        matches!(
            self,
            Elem::Water
                | Elem::Lava
                | Elem::Magma
                | Elem::Acid
                | Elem::Ice
                | Elem::Snow
                | Elem::Plant
                | Elem::Crop
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Fire
                | Elem::Oil
                | Elem::Tar
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
                | Elem::Iron
                | Elem::Wood
                | Elem::Ember
                | Elem::Market
                | Elem::Workshop
                | Elem::Granary
                | Elem::Totem
        )
    }

    #[must_use]
    pub fn is_inert(self) -> bool {
        !self.is_reactive()
            && self.behavior() == Behavior::Static
            && self.lifetime() == u32::MAX
    }

    #[must_use]
    pub fn behavior(self) -> Behavior {
        match self {
            Elem::Sand | Elem::Sulfur | Elem::Salt | Elem::Ash | Elem::Dust | Elem::Gunpowder | Elem::Fertilizer | Elem::Snow | Elem::Rust => Behavior::Fall,
            Elem::Water | Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Blood | Elem::Poison | Elem::Alcohol | Elem::Honey | Elem::Lava | Elem::Magma | Elem::Tar => Behavior::Flow,
            Elem::Fire => Behavior::Burn,
            Elem::Smoke | Elem::Steam | Elem::Gas | Elem::Plasma | Elem::Chlorine | Elem::Methane | Elem::Oxygen => Behavior::Rise,
            Elem::Spore | Elem::Moss => Behavior::Diffuse,
            Elem::Energy | Elem::Mana | Elem::Soul | Elem::Ember => Behavior::Energy,
            Elem::Plant | Elem::Fungus | Elem::Roots => Behavior::Grow,
            Elem::Farm => Behavior::Cultivate,
            Elem::Wire | Elem::Circuit => Behavior::Conduct,
            Elem::Virus | Elem::Bacteria => Behavior::Spread,
            Elem::Egg => Behavior::Hatch,
            Elem::Corpse => Behavior::Decay,
            _ => Behavior::Static,
        }
    }

    #[must_use]
    pub fn density(self) -> i16 {
        match self {
            Elem::Empty => 0,
            Elem::Gas | Elem::Smoke | Elem::Steam | Elem::Plasma | Elem::Energy | Elem::Mana | Elem::Soul | Elem::Fire | Elem::Ember | Elem::Virus | Elem::Bacteria | Elem::Food => 1,
            Elem::Chlorine | Elem::Methane | Elem::Oxygen => 2,
            Elem::Sulfur | Elem::Salt | Elem::Ash | Elem::Dust | Elem::Spore | Elem::Gunpowder | Elem::Fertilizer | Elem::Mud | Elem::Snow | Elem::Rust => 3,
            Elem::Oil => 7,
            Elem::Alcohol | Elem::Wood => 8,
            Elem::Water | Elem::Roots => 10,
            Elem::Poison | Elem::Honey | Elem::Blood | Elem::Meat | Elem::Leather => 11,
            Elem::Acid | Elem::Charcoal | Elem::Corpse => 12,
            Elem::Tar | Elem::Bone => 14,
            Elem::Sand | Elem::Magma => 16,
            Elem::Plant | Elem::Fungus | Elem::Moss | Elem::Ice | Elem::Crop => 9,
            Elem::Mercury => 13,
            Elem::Lava | Elem::Rune | Elem::Ward | Elem::Portal | Elem::Egg => 15,
            Elem::Stone | Elem::Brick | Elem::Concrete | Elem::Wall | Elem::Furnace | Elem::Anvil | Elem::Forge | Elem::Tower => 20,
            Elem::Granite | Elem::Metal | Elem::Iron | Elem::Steel | Elem::Copper | Elem::Tin | Elem::Bronze | Elem::Wire | Elem::Gold | Elem::Silver | Elem::Market | Elem::Workshop | Elem::Granary | Elem::Beacon | Elem::Hearth | Elem::Totem => 22,
            Elem::Glass | Elem::Crystal | Elem::Gem | Elem::Diamond | Elem::Obsidian => 18,
            Elem::Circuit | Elem::Tool | Elem::Machine | Elem::Door | Elem::Chest | Elem::Farm | Elem::Temple | Elem::Bridge | Elem::Road => 24,
            Elem::Void => 100,
        }
    }

    #[must_use]
    pub fn nutrition(self) -> f32 {
        match self {
            Elem::Water | Elem::Stone | Elem::Glass | Elem::Concrete | Elem::Oxygen => 2.0,
            Elem::Wood | Elem::Gas | Elem::Gem => 20.0,
            Elem::Oil | Elem::Machine | Elem::Honey | Elem::Corpse => 40.0,
            Elem::Plant => 30.0,
            Elem::Spore | Elem::Bronze | Elem::Silver | Elem::Bone => 10.0,
            Elem::Fungus | Elem::Steel | Elem::Circuit | Elem::Leather | Elem::Alcohol => 15.0,
            Elem::Moss | Elem::Copper | Elem::Obsidian => 8.0,
            Elem::Roots | Elem::Iron | Elem::Gold => 12.0,

            Elem::Metal => 4.0,
            Elem::Tin | Elem::Crystal => 6.0,
            Elem::Diamond | Elem::Meat => 50.0,
            Elem::Granite | Elem::Snow | Elem::Chlorine | Elem::Steam | Elem::Ember => 1.0,
            Elem::Rust | Elem::Brick => 3.0,
            Elem::Wire | Elem::Mud | Elem::Charcoal | Elem::Methane => 5.0,
            Elem::Tool | Elem::Food => 25.0,
            Elem::Mana => 80.0,
            Elem::Soul => 200.0,
            Elem::Egg => 60.0,

            Elem::Crop => 55.0,
            Elem::Smoke | Elem::Fire | Elem::Virus | Elem::Bacteria => 0.5,

                                                _ => 0.0,
        }
    }

    #[must_use]
    pub fn toxicity(self) -> f32 {
        match self {
            Elem::Fire | Elem::Oil => 10.0,
            Elem::Smoke | Elem::Sulfur | Elem::Gunpowder => 5.0,
            Elem::Lava => 50.0,
            Elem::Magma => 60.0,
            Elem::Ember => 12.0,
            Elem::Tar => 14.0,
            Elem::Acid | Elem::Bacteria => 100.0,
            Elem::Mercury => 30.0,
            Elem::Plasma | Elem::Poison => 200.0,
            Elem::Ash => 8.0,
            Elem::Chlorine => 150.0,
            Elem::Virus => 300.0,
            _ => 0.0,
        }
    }

    #[must_use]
    pub fn lifetime(self) -> u32 {
        match self {
            Elem::Fire => 90,
            Elem::Smoke => 320,
            Elem::Steam => 260,
            Elem::Gas | Elem::Oxygen | Elem::Bacteria => 400,
            Elem::Energy => 2600,
            Elem::Plasma => 120,
            Elem::Chlorine => 300,
            Elem::Methane => 200,

            Elem::Mana => 1000,
            Elem::Soul | Elem::Virus => 500,
            Elem::Ember => 900,
            Elem::Egg => 2000,
            Elem::Corpse => 3000,
            _ => u32::MAX,
        }
    }

    #[must_use]
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
                | Elem::Crop
                | Elem::Egg
                | Elem::Corpse
                | Elem::Honey
                | Elem::Alcohol
                | Elem::Charcoal
        )
    }

    #[must_use]
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
                | Elem::Ember
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

                | Elem::Corpse
                | Elem::Bone
                | Elem::Meat
                | Elem::Leather
        )
    }

    #[must_use]
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
                | Elem::Snow
                | Elem::Rust
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
                | Elem::Granite
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
                | Elem::Market
                | Elem::Workshop
                | Elem::Granary
                | Elem::Beacon
                | Elem::Hearth
                | Elem::Totem
                | Elem::Plant
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Bone
                | Elem::Leather
                | Elem::Charcoal
                | Elem::Rune
                | Elem::Ward
                | Elem::Portal
                | Elem::Egg
        )
    }

    #[must_use]
    pub fn flammable(self) -> bool {
        matches!(
            self,
            Elem::Wood
                | Elem::Plant
                | Elem::Oil
                | Elem::Tar
                | Elem::Sulfur
                | Elem::Fungus
                | Elem::Spore
                | Elem::Moss
                | Elem::Roots
                | Elem::Gunpowder
                | Elem::Alcohol
                | Elem::Honey
                | Elem::Meat
                | Elem::Leather
                | Elem::Fertilizer
                | Elem::Charcoal
        )
    }

    #[must_use]
    pub fn conducts(self) -> bool {
        matches!(
            self,
            Elem::Metal
                | Elem::Copper
                | Elem::Tin
                | Elem::Bronze
                | Elem::Iron
                | Elem::Steel
                | Elem::Gold
                | Elem::Silver
                | Elem::Wire
                | Elem::Circuit
                | Elem::Machine
                | Elem::Tool
        )
    }

    #[must_use]
    pub fn melt_temp(self) -> u16 {
        match self {
            Elem::Ice => 0,
            Elem::Snow => 1,
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
            Elem::Granite => 1650,
            Elem::Obsidian => 1200,
            Elem::Tar => 180,
            Elem::Charcoal => 900,
            _ => u16::MAX,
        }
    }

    #[must_use]
    pub fn is_metal(self) -> bool {
        matches!(
            self,
            Elem::Metal
                | Elem::Copper
                | Elem::Tin
                | Elem::Bronze
                | Elem::Iron
                | Elem::Steel
                | Elem::Gold
                | Elem::Silver
                | Elem::Rust
        )
    }

    #[must_use]
    pub fn is_gemstone(self) -> bool {
        matches!(self, Elem::Crystal | Elem::Gem | Elem::Diamond)
    }

    #[must_use]
    pub fn is_organic(self) -> bool {
        matches!(
            self,
            Elem::Wood
                | Elem::Plant
                | Elem::Spore
                | Elem::Fungus
                | Elem::Moss
                | Elem::Roots
                | Elem::Meat
                | Elem::Bone
                | Elem::Leather
                | Elem::Egg
                | Elem::Corpse
                | Elem::Blood
                | Elem::Honey
                | Elem::Charcoal
        )
    }

    #[must_use]
    pub fn is_constructed(self) -> bool {
        matches!(
            self,
            Elem::Brick
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
        )
    }

    #[must_use]
    pub fn is_magical(self) -> bool {
        matches!(
            self,
            Elem::Mana | Elem::Soul | Elem::Rune | Elem::Ward | Elem::Portal
        )
    }

    #[must_use]
    pub fn is_hazardous(self) -> bool {
        matches!(
            self,
            Elem::Fire
                | Elem::Lava
                | Elem::Magma
                | Elem::Ember
                | Elem::Acid
                | Elem::Poison
                | Elem::Chlorine
                | Elem::Plasma
                | Elem::Virus
                | Elem::Bacteria
                | Elem::Gunpowder
                | Elem::Tar
        )
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Elem::Empty => "Empty",

            Elem::Sand => "Sand",
            Elem::Sulfur => "Sulfur",
            Elem::Salt => "Salt",
            Elem::Ash => "Ash",
            Elem::Dust => "Dust",
            Elem::Gunpowder => "Gunpowder",
            Elem::Fertilizer => "Fertilizer",
            Elem::Mud => "Mud",
            Elem::Snow => "Snow",
            Elem::Crop => "Crop",

            Elem::Water => "Water",
            Elem::Oil => "Oil",
            Elem::Acid => "Acid",
            Elem::Mercury => "Mercury",
            Elem::Blood => "Blood",
            Elem::Poison => "Poison",
            Elem::Alcohol => "Alcohol",
            Elem::Honey => "Honey",
            Elem::Tar => "Tar",

            Elem::Steam => "Steam",
            Elem::Smoke => "Smoke",
            Elem::Gas => "Gas",
            Elem::Plasma => "Plasma",
            Elem::Chlorine => "Chlorine",
            Elem::Methane => "Methane",
            Elem::Oxygen => "Oxygen",

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
            Elem::Granite => "Granite",
            Elem::Rust => "Rust",

            Elem::Wood => "Wood",
            Elem::Plant => "Plant",
            Elem::Spore => "Spore",
            Elem::Fungus => "Fungus",
            Elem::Moss => "Moss",
            Elem::Roots => "Roots",
            Elem::Meat => "Meat",
            Elem::Bone => "Bone",
            Elem::Leather => "Leather",
            Elem::Charcoal => "Charcoal",

            Elem::Fire => "Fire",
            Elem::Energy => "Energy",
            Elem::Ice => "Ice",
            Elem::Lava => "Lava",
            Elem::Magma => "Magma",
            Elem::Void => "Void",
            Elem::Mana => "Mana",
            Elem::Soul => "Soul",
            Elem::Ember => "Ember",

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

            Elem::Rune => "Rune",
            Elem::Ward => "Ward",
            Elem::Portal => "Portal",

            Elem::Egg => "Egg",
            Elem::Corpse => "Corpse",
            Elem::Virus => "Virus",
            Elem::Bacteria => "Bacteria",
            Elem::Food => "Food",
            Elem::Market => "Market",
            Elem::Workshop => "Workshop",
            Elem::Granary => "Granary",
            Elem::Beacon => "Beacon",
            Elem::Hearth => "Hearth",
            Elem::Totem => "Totem",
        }
    }

    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Elem::Empty => "Nothing here. Creatures can move freely.",

            Elem::Sand => "Fine granular powder. Falls under gravity. Melts into glass near lava.",
            Elem::Sulfur => "Bright yellow powder. Flammable; reacts with lava to make gas.",
            Elem::Salt => "White crystalline powder. Harmless seasoning.",
            Elem::Ash => "Grey powder left after burning. Can act as coal.",
            Elem::Dust => "Dry powder that drifts in the wind.",
            Elem::Gunpowder => "Explosive powder. Detonates when near fire or plasma.",
            Elem::Fertilizer => "Boosts plant growth when nearby.",
            Elem::Mud => "Wet dirt. Forms where water meets soil.",
            Elem::Snow => "Cold powder that falls in winter. Melts back into water.",
            Elem::Crop => "A ripe crop, grown from a plant on a tilled farm. Better food than a wild plant.",

            Elem::Water => "Flowing liquid. Essential for life and plants. Extinguishes fire.",
            Elem::Oil => "Flammable dark liquid. Burns readily and can be gathered for energy.",
            Elem::Acid => "Corrosive green liquid. Dissolves most solids on contact.",
            Elem::Mercury => "Heavy silver liquid metal. Amalgamates with gold and copper.",
            Elem::Blood => "Organic red liquid. Spilled blood is not food; it is only a sign that something died here.",
            Elem::Poison => "Toxic green liquid. Harms anything that touches or drinks it.",
            Elem::Alcohol => "Clear flammable liquid. Burns with a clean flame.",
            Elem::Honey => "Thick golden sweet liquid. High energy food.",
            Elem::Tar => "Sticky black residue of burnt oil. Slow to burn, hard to wash off.",

            Elem::Steam => "Hot water vapor. Rises and condenses back to water.",
            Elem::Smoke => "Grey gas from fires. Rises and fades over time.",
            Elem::Gas => "Generic flammable gas. Rises and can explode.",
            Elem::Plasma => "Superheated ionized gas. Very hot and toxic.",
            Elem::Chlorine => "Toxic pale-green gas. Poisons water and organic matter.",
            Elem::Methane => "Flammable gas from swamps. Explodes near fire.",
            Elem::Oxygen => "Breathable gas. Makes fires burn hotter.",

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
            Elem::Granite => "Deep igneous rock. Slow to melt and harder than stone.",
            Elem::Rust => "Iron oxide. Feeds iron ore back into the ground.",

            Elem::Wood => "Solid plant material. Burns and floats on water.",
            Elem::Plant => "Living vegetation. Grows near water and spreads slowly.",
            Elem::Spore => "Reproductive dust from fungi and plants. Drifts on air.",
            Elem::Fungus => "Mushroom-like organism. Thrives in damp shaded areas.",
            Elem::Moss => "Low carpet-like plant. Grows on damp stone and wood.",
            Elem::Roots => "Underground plant tendrils. Anchor vegetation.",
            Elem::Meat => "Animal flesh. Nutritious, and a prize worth taking off a rival.",
            Elem::Bone => "Hard skeletal remains. Slow to decay.",
            Elem::Leather => "Treated hide. Durable crafting material.",
            Elem::Charcoal => "Burnt carbon. Burns slowly but burns hot.",

            Elem::Fire => "Hot flame. Spreads to flammable neighbors and rises.",
            Elem::Energy => "Pure energy drifts above ground. It lights the air, and nothing in this world can eat it.",
            Elem::Ice => "Frozen water. Melts near heat sources.",
            Elem::Lava => "Molten rock. Extremely hot; turns to stone when cooled by water.",
            Elem::Magma => "Superheated lava. Even hotter; forms obsidian with water.",
            Elem::Void => "Absolute emptiness. Denser than anything; impassable.",
            Elem::Mana => "Magical energy. Powers spells and activates runes.",
            Elem::Soul => "Rare spiritual energy. Very high nutrition.",
            Elem::Ember => "Glowing coal. Lights the dark and fades slowly.",

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

            Elem::Rune => "A magical sigil. Activates when touched by mana.",
            Elem::Ward => "A protective charm that repels disease.",
            Elem::Portal => "A gateway to another place. Rare and unstable.",

            Elem::Egg => "Hatches into a new creature after time passes.",
            Elem::Corpse => "Remains of a dead creature. It remembers whose body it was: a rival's is a meal, your own is not. Decays into bone or dust.",
            Elem::Virus => "Microscopic disease. Spreads rapidly through organic matter.",
            Elem::Bacteria => "Microbe that decomposes meat and food.",
            Elem::Food => "Prepared nourishment for creatures.",
            Elem::Market => "A trading post. Trade beside it is far more profitable.",
            Elem::Workshop => "A craftsman's hall. Tools are made faster beside it.",
            Elem::Granary => "A food store. The colony does not go hungry beside it.",
            Elem::Beacon => "A signal beacon. The colony sees much further from it.",
            Elem::Hearth => "A hearth. The cold does not bite beside it.",
            Elem::Totem => "An animal totem: the symbol of a tribe, raised at its heart.",
        }
    }

    #[must_use]
    pub fn category(self) -> &'static str {
        match self {
            Elem::Empty | Elem::Void => "Void",
            Elem::Sand
            | Elem::Sulfur
            | Elem::Salt
            | Elem::Ash
            | Elem::Dust
            | Elem::Gunpowder
            | Elem::Fertilizer
            | Elem::Mud
            | Elem::Snow
            | Elem::Rust => "Powders",
            Elem::Water
            | Elem::Oil
            | Elem::Acid
            | Elem::Mercury
            | Elem::Blood
            | Elem::Poison
            | Elem::Alcohol
            | Elem::Honey
            | Elem::Tar => "Liquids",
            Elem::Steam
            | Elem::Smoke
            | Elem::Gas
            | Elem::Plasma
            | Elem::Chlorine
            | Elem::Methane
            | Elem::Oxygen => "Gases",
            Elem::Stone
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
            | Elem::Granite => "Solids",
            Elem::Wood
            | Elem::Plant
            | Elem::Spore
            | Elem::Fungus
            | Elem::Moss
            | Elem::Roots
            | Elem::Meat
            | Elem::Bone
            | Elem::Leather
            | Elem::Charcoal => "Organic",
            Elem::Fire
            | Elem::Energy
            | Elem::Ice
            | Elem::Lava
            | Elem::Magma
            | Elem::Mana
            | Elem::Soul
            | Elem::Ember => "Energy",
            Elem::Brick
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
            | Elem::Road => "Constructed",
            Elem::Rune | Elem::Ward | Elem::Portal => "Magical",
            Elem::Egg | Elem::Corpse | Elem::Virus | Elem::Bacteria | Elem::Food | Elem::Crop => "Biological",
            Elem::Market
            | Elem::Workshop
            | Elem::Granary
            | Elem::Beacon
            | Elem::Hearth
            | Elem::Totem => "Settlement",
        }
    }
}
