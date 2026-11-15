use super::{Tool, Discovery};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TechKind {

    Tool(Tool),

    Discovery(Discovery),

    Craft(Consumable),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Consumable {
    Bomb,
    Salve,
}

#[derive(Clone, Copy)]
pub struct TechEntry {
    pub kind: TechKind,

    pub tier: u8,
    pub name: &'static str,

    pub effect: &'static str,

    pub cost: &'static str,
}

impl TechEntry {

    #[must_use]
    pub fn is_discovery(&self) -> bool {
        matches!(self.kind, TechKind::Discovery(_))
    }
}

#[must_use]
pub fn tier_name(tier: u8) -> &'static str {
    match tier {
        1 => "THE FIRST HANDS",
        2 => "THE CRAFT",
        3 => "THE WORKSHOP",
        4 => "THE TRADE",
        5 => "THE MEDICINE",
        6 => "THE MINE",
        7 => "THE STARS",
        8 => "THE LIGHT",
        9 => "THE WAR",
        _ => "THE DEEP",
    }
}

#[must_use]
pub fn tier_blurb(tier: u8) -> &'static str {
    match tier {
        1 => "stone in the hand, and something to hunt with",
        2 => "the colony works its own metal",
        3 => "a place to work, and a wall to stand behind",
        4 => "more than the colony can use alone",
        5 => "the sick are mended",
        6 => "the rock gives up its secrets",
        7 => "the sky is read, and the old rites understood",
        8 => "the light of the surface, made into a weapon",
        9 => "the colony learns to fight as one",
        _ => "the deep itself",
    }
}

#[must_use]
pub fn tech_tree() -> Vec<TechEntry> {
    vec![

        TechEntry {
            kind: TechKind::Tool(Tool::Pickaxe),
            tier: 1,
            name: "Pickaxe",
            effect: "breaks the rock and the ore",
            cost: "5 stone + 2 wood",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Spear),
            tier: 1,
            name: "Spear",
            effect: "a reach and a point",
            cost: "2 wood + 1 stone",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Husbandry),
            tier: 1,
            name: "Husbandry",
            effect: "the colony eats better",
            cost: "farming",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Pottery),
            tier: 1,
            name: "Pottery",
            effect: "hunger grows 15% slower",
            cost: "crafting",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Sword),
            tier: 2,
            name: "Sword",
            effect: "+35 melee damage",
            cost: "3 metal + 2 wood",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Hammer),
            tier: 2,
            name: "Hammer",
            effect: "raises buildings faster",
            cost: "5 stone + 3 metal",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Smithing),
            tier: 2,
            name: "Smithing",
            effect: "+30% melee damage",
            cost: "crafting and mining",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Ranged),
            tier: 2,
            name: "Ranged Craft",
            effect: "a bow from wood alone, and it flies 25% further",
            cost: "crafting and warfare",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Bow),
            tier: 3,
            name: "Bow",
            effect: "a shot from a distance",
            cost: "3 wood",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Shield),
            tier: 3,
            name: "Shield",
            effect: "15% less damage taken",
            cost: "3 wood + 2 metal",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Architecture),
            tier: 3,
            name: "Architecture",
            effect: "buildings cost 30% less",
            cost: "building",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Masonry),
            tier: 3,
            name: "Masonry",
            effect: "buildings stand 40% longer",
            cost: "crafting and building",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Cart),
            tier: 4,
            name: "Cart",
            effect: "carries twice as much",
            cost: "5 wood + 3 metal",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Backpack),
            tier: 4,
            name: "Backpack",
            effect: "carries half again as much",
            cost: "4 leather + 2 wood",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Commerce),
            tier: 4,
            name: "Commerce",
            effect: "trade brings in 50% more",
            cost: "trade and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Cartography),
            tier: 4,
            name: "Cartography",
            effect: "+20% sight",
            cost: "crafting and warfare",
        },

        TechEntry {
            kind: TechKind::Craft(Consumable::Salve),
            tier: 5,
            name: "Salve",
            effect: "heals a wounded colonymate",
            cost: "2 plant + 1 crystal",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Medicine),
            tier: 5,
            name: "Medicine",
            effect: "salves heal 40% more",
            cost: "crafting and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Herbalism),
            tier: 5,
            name: "Herbalism",
            effect: "food is worth 15% more",
            cost: "farming and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Surgery),
            tier: 5,
            name: "Surgery",
            effect: "wounds mend 40% faster",
            cost: "ritual and crafting",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Cognition),
            tier: 1,
            name: "Cognition",
            effect: "every mind grows: bigger brains, more to think with",
            cost: "crafting",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Drill),
            tier: 6,
            name: "Drill",
            effect: "digs twice as fast",
            cost: "3 copper + 2 metal",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Scanner),
            tier: 6,
            name: "Scanner",
            effect: "sees twice as far",
            cost: "2 gold + 1 crystal",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::DeepMining),
            tier: 6,
            name: "Deep Mining",
            effect: "the colony digs 50% deeper",
            cost: "mining and crafting",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Geodesy),
            tier: 6,
            name: "Geodesy",
            effect: "the colony finds food 30% further off",
            cost: "astronomy and sanctuary",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Smelting),
            tier: 6,
            name: "Smelting",
            effect: "+25% melee damage",
            cost: "crafting and mining",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Staff),
            tier: 7,
            name: "Staff",
            effect: "casts the old spells",
            cost: "1 wood + 1 crystal",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Amulet),
            tier: 7,
            name: "Amulet",
            effect: "+40% mana",
            cost: "1 gold + 1 gem",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Astronomy),
            tier: 7,
            name: "Astronomy",
            effect: "+40% sight",
            cost: "ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Sanctuary),
            tier: 7,
            name: "Sanctuary",
            effect: "mana comes 50% faster",
            cost: "ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Navigation),
            tier: 7,
            name: "Navigation",
            effect: "the colony moves 15% faster",
            cost: "warfare and crafting",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Sunfire),
            tier: 8,
            name: "Sunfire",
            effect: "scours the ZORGs; a poor weapon against ordinary flesh",
            cost: "1 crystal + 1 gold + 1 wood",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Sanctification),
            tier: 8,
            name: "Sanctification",
            effect: "the light of the surface, made into a weapon",
            cost: "crafting and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Wardcraft),
            tier: 8,
            name: "Wardcraft",
            effect: "wards burn 50% wider",
            cost: "building and sanctuary",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Antitoxins),
            tier: 8,
            name: "Antitoxins",
            effect: "poison does half the harm",
            cost: "crafting and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Purge),
            tier: 8,
            name: "Purge",
            effect: "the sickness of the deep is resisted",
            cost: "ritual and crafting",
        },

        TechEntry {
            kind: TechKind::Craft(Consumable::Bomb),
            tier: 9,
            name: "Bomb",
            effect: "thrown at a rival",
            cost: "3 gunpowder + 2 wood",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Siegecraft),
            tier: 9,
            name: "Siegecraft",
            effect: "shots fly 25% further and hit 25% harder",
            cost: "ranged craft and warfare",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Tactics),
            tier: 9,
            name: "Tactics",
            effect: "15% less damage taken",
            cost: "warfare and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Ironwill),
            tier: 9,
            name: "Ironwill",
            effect: "sickness does half the harm",
            cost: "warfare and farming",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Sunfirecraft),
            tier: 9,
            name: "Sunfirecraft",
            effect: "the sunfire burns 30% harder",
            cost: "sanctification and crafting",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Radiance),
            tier: 9,
            name: "Radiance",
            effect: "+40% mana",
            cost: "sanctuary and astronomy",
        },

        TechEntry {
            kind: TechKind::Tool(Tool::Boat),
            tier: 10,
            name: "Boat",
            effect: "crosses the water",
            cost: "6 wood",
        },
        TechEntry {
            kind: TechKind::Tool(Tool::Glider),
            tier: 10,
            name: "Glider",
            effect: "crosses the sky",
            cost: "4 wood + 2 cloth",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Metallurgy),
            tier: 10,
            name: "Metallurgy",
            effect: "+20% health",
            cost: "crafting and mining",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Alchemy),
            tier: 10,
            name: "Alchemy",
            effect: "wounds mend 30% faster, salves 20% more",
            cost: "crafting and ritual",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Deepdiving),
            tier: 10,
            name: "Deepdiving",
            effect: "the deep does no harm",
            cost: "navigation and sanctuary",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::Dominion),
            tier: 10,
            name: "Dominion",
            effect: "the deep is closed: it breeds far more slowly",
            cost: "sanctification and building",
        },
        TechEntry {
            kind: TechKind::Discovery(Discovery::GrandCognition),
            tier: 3,
            name: "Grand Cognition",
            effect: "the minds grow again, and learn from what works",
            cost: "cognition and mining",
        },
    ]
}

#[must_use]
pub fn tool_gate(tool: Tool) -> Option<Discovery> {
    Some(match tool {
        Tool::Pickaxe | Tool::Spear | Tool::None => return None,
        Tool::Sword | Tool::Hammer => Discovery::Smithing,
        Tool::Bow => Discovery::Ranged,
        Tool::Shield | Tool::Backpack => Discovery::Masonry,
        Tool::Cart => Discovery::Architecture,
        Tool::Drill => Discovery::Metallurgy,
        Tool::Scanner => Discovery::Cartography,
        Tool::Staff => Discovery::Sanctuary,
        Tool::Amulet => Discovery::Radiance,
        Tool::Sunfire => Discovery::Sanctification,
        Tool::Boat | Tool::Glider => Discovery::Navigation,
    })
}

#[must_use]
pub fn craft_gate(craft: Consumable) -> Option<Discovery> {
    match craft {
        Consumable::Bomb => Some(Discovery::Siegecraft),
        Consumable::Salve => Some(Discovery::Medicine),
    }
}
