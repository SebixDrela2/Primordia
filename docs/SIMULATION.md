# PRIMORDIA — Simulation Reference

Everything below describes the code as it actually runs. File paths and method
names are real; line numbers are omitted because they move, but the
`file::method` notation is stable.

---

## 1. What the game is

PRIMORDIA is a cellular-automaton ecosystem with a population of neural agents
living inside it.

Two simulations run side by side:

| Simulation | Granularity | Driven by |
|---|---|---|
| **The world** | one cell per grain of sand, drop of water, lump of lava | physics + chemistry, no intelligence |
| **The creatures** | one agent per organism | a small neural network, senses, needs, genes |

The creatures do not control the world. They live in it: they eat the plants,
mine the rock, fight each other, and reproduce. The world reacts to them
chemically. A creature that digs too deep eventually meets lava.

A **run** ends when one species is left (victory) or the population dies out
(extinction). The winner's genome seeds the next world, so a lineage carries
its traits forward across runs.

---

## 2. The world

### 2.1 The grid

The world is a `W × H` grid of cells — 1280 × 720 by default.

Each cell stores:

| Field | Type | Meaning |
|---|---|---|
| `kind` | `u8` | the element id (see `elements::Elem`) |
| `col` | `[u8; 3]` | the cell's colour, baked when the element is set |
| `heat` | `u8` | temperature tint, used for lava glow |
| `age` | `u32` | how long this cell has held its current element |

`World` is defined in `world/mod.rs`. The parallel arrays live in `world/cell.rs`
(`World::new`).

### 2.2 Coordinates and depth

**The single most important rule in the codebase:**

> Depth is always measured from `World::surface_y(x)`, never from an absolute
> row number.

`surface_y(x)` is the analytic terrain height for column `x`
(`world/terrain.rs`). It is around row 470 in a 720-row world. Anything placed
at an absolute height like `h/2` (row 360) or `h/3` (row 240) lands **in the
sky**. Several bugs in this project's history came from exactly that mistake.

`World::ground[x]` is the cached *actual* ground level: the topmost solid cell
that rests on more solid. It differs from `surface_y(x)` where terrain has been
eroded or built up, and it is what the renderer uses to decide sky versus
underground. It is refreshed by `World::update_ground` (`world/terrain.rs`),
called every 15 steps from `World::maintain`.

### 2.3 Elements

An element is a material with a behaviour, a density, a nutritional value and a
toxicity. `Elem` is a `#[repr(u8)]` enum in `elements/mod.rs`; all of its
per-element data lives in `elements/properties.rs`.

Element properties are read through **lookup tables**, never by matching on the
enum inside a per-cell loop (`AGENTS.md`). The tables are:

- `Elem::solid_table()` — is it solid
- `Elem::inert_table()` — can it ever change on its own
- `Elem::edible_table()` — can a creature eat it
- `Elem::nutrition_table()` — food value
- `Elem::toxicity_table()` — poison value
- `Elem::emissive_table()` — does it radiate light
- `Elem::emission_table()` — the colour it radiates
- `Elem::behavior_table()` — how it moves
- `Elem::lifetime_table()` — how long before it decays
- `Elem::reactive_table()` — does it take part in a reaction

#### Behaviours

| Behaviour | Rule | Examples |
|---|---|---|
| `Static` | never moves | Stone, Metal, Brick, Wall |
| `Fall` | falls straight down, then tries diagonals | Sand, Sulfur, Snow, Rust |
| `Flow` | falls, then spreads sideways | Water, Oil, Acid, Lava, Tar |
| `Rise` | floats upward, drifts off the top of the world | Smoke, Steam, Gas, Plasma |
| `Diffuse` | wanders in a random direction | Moss, Spore |
| `Burn` | ignites flammable neighbours, then rises | Fire |
| `Grow` | spreads along water, drawn to the surface | Plant, Fungus, Roots |
| `Energy` | drifts like a will-o'-the-wisp | Energy, Mana, Soul, Ember |
| `Conduct` | conducts electricity (currently static) | Wire, Circuit |
| `Spread` | spreads through organic matter | Virus, Bacteria |
| `Hatch` | hatches into a creature | Egg |
| `Decay` | rots into bone or dust | Corpse |

Density decides who displaces whom: a cell may move into a target if its own
density is **higher** and the target is movable (`Flow`/`Rise`/`Diffuse`).
That is why sand sinks in water but water does not sink in sand.

<!-- ELEMENT_TABLE -->

| # | Element | Category | Behaviour | Density | Nutrition | Toxicity | Lifetime | Solid | Edible | Emits light | Description |
|---|---------|----------|-----------|---------|-----------|----------|----------|-------|--------|------------|-------------|
| 0 | Sand | Powders | Fall | 16 | 0 | 0 | permanent | yes | - | - | Fine granular powder. Falls under gravity. Melts into glass near lava. |
| 1 | Sulfur | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Bright yellow powder. Flammable; reacts with lava to make gas. |
| 2 | Salt | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | White crystalline powder. Harmless seasoning. |
| 3 | Ash | Powders | Fall | 3 | 0 | 8 | permanent | yes | - | - | Grey powder left after burning. Can act as coal. |
| 4 | Dust | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Dry powder that drifts in the wind. |
| 5 | Gunpowder | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Explosive powder. Detonates when near fire or plasma. |
| 6 | Fertilizer | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Boosts plant growth when nearby. |
| 7 | Mud | Powders | Static | 3 | 5 | 0 | permanent | - | - | - | Wet dirt. Forms where water meets soil. |
| 8 | Water | Liquids | Flow | 10 | 2 | 0 | permanent | - | yes | - | Flowing liquid. Essential for life and plants. Extinguishes fire. |
| 9 | Oil | Liquids | Flow | 7 | 40 | 10 | permanent | - | - | - | Flammable dark liquid. Burns readily and can be gathered for energy. |
| 10 | Acid | Liquids | Flow | 12 | 0 | 100 | permanent | - | - | - | Corrosive green liquid. Dissolves most solids on contact. |
| 11 | Mercury | Liquids | Flow | 13 | 0 | 30 | permanent | - | - | - | Heavy silver liquid metal. Amalgamates with gold and copper. |
| 12 | Blood | Liquids | Flow | 11 | 30 | 0 | permanent | - | yes | - | Organic red liquid. Nutritious but can carry disease. |
| 13 | Poison | Liquids | Flow | 11 | 0 | 200 | permanent | - | - | - | Toxic green liquid. Harms anything that touches or drinks it. |
| 14 | Alcohol | Liquids | Flow | 8 | 15 | 0 | permanent | - | yes | - | Clear flammable liquid. Burns with a clean flame. |
| 15 | Honey | Liquids | Flow | 11 | 40 | 0 | permanent | - | yes | - | Thick golden sweet liquid. High energy food. |
| 16 | Tar | Liquids | Flow | 14 | 0 | 14 | permanent | - | - | - | Sticky black residue of burnt oil. Slow to burn, hard to wash off. |
| 17 | Steam | Gases | Rise | 1 | 1 | 0 | 260 steps | - | - | - | Hot water vapor. Rises and condenses back to water. |
| 18 | Smoke | Gases | Rise | 1 | 0.5 | 5 | 320 steps | - | - | - | Grey gas from fires. Rises and fades over time. |
| 19 | Gas | Gases | Rise | 1 | 20 | 0 | 400 steps | - | - | - | Generic flammable gas. Rises and can explode. |
| 20 | Plasma | Gases | Rise | 1 | 0 | 200 | 120 steps | - | - | yes | Superheated ionized gas. Very hot and toxic. |
| 21 | Chlorine | Gases | Rise | 2 | 1 | 150 | 300 steps | - | - | - | Toxic pale-green gas. Poisons water and organic matter. |
| 22 | Methane | Gases | Rise | 2 | 5 | 0 | 200 steps | - | - | - | Flammable gas from swamps. Explodes near fire. |
| 23 | Oxygen | Gases | Rise | 2 | 2 | 0 | 150 steps | - | - | - | Breathable gas. Makes fires burn hotter. |
| 24 | Stone | Solids | Static | 20 | 2 | 0 | permanent | yes | - | - | Hard common rock. Forms the crust of the world. |
| 25 | Metal | Solids | Static | 22 | 4 | 0 | permanent | yes | - | - | Generic sturdy metal. Can be melted and alloyed. |
| 26 | Copper | Solids | Static | 22 | 8 | 0 | permanent | yes | - | - | Reddish metal. Early-age conductor and tool material. |
| 27 | Tin | Solids | Static | 22 | 6 | 0 | permanent | yes | - | - | Soft silvery metal. Alloys with copper to make bronze. |
| 28 | Bronze | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Hard copper-tin alloy. Better tools and weapons. |
| 29 | Iron | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Strong dark metal. Foundation of tools and armor. |
| 30 | Steel | Solids | Static | 22 | 15 | 0 | permanent | yes | - | - | Refined iron-carbon alloy. Very strong and durable. |
| 31 | Gold | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Shiny yellow precious metal. Highly valued; soft and dense. |
| 32 | Silver | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Pale lustrous metal. Conducts energy well. |
| 33 | Glass | Solids | Static | 18 | 2 | 0 | permanent | yes | - | - | Transparent solid made from melted sand. |
| 34 | Crystal | Solids | Static | 18 | 6 | 0 | permanent | yes | - | yes | Geometric mineral formation. Stores magical energy. |
| 35 | Gem | Solids | Static | 18 | 20 | 0 | permanent | yes | - | yes | Polished precious crystal. Rare and valuable. |
| 36 | Diamond | Solids | Static | 18 | 50 | 0 | permanent | yes | - | yes | Hardest natural substance. Extremely valuable. |
| 37 | Obsidian | Solids | Static | 18 | 8 | 0 | permanent | yes | - | - | Volcanic glass. Forms where water meets lava. |
| 38 | Granite | Solids | Static | 22 | 1 | 0 | permanent | yes | - | - | Deep igneous rock. Slow to melt and harder than stone. |
| 39 | Rust | Powders | Fall | 3 | 3 | 0 | permanent | yes | - | - | Iron oxide. Feeds iron ore back into the ground. |
| 40 | Wood | Organic | Static | 8 | 20 | 0 | permanent | yes | - | - | Solid plant material. Burns and floats on water. |
| 41 | Plant | Organic | Grow | 9 | 30 | 0 | permanent | yes | yes | - | Living vegetation. Grows near water and spreads slowly. |
| 42 | Spore | Organic | Diffuse | 3 | 10 | 0 | permanent | - | yes | - | Reproductive dust from fungi and plants. Drifts on air. |
| 43 | Fungus | Organic | Grow | 9 | 15 | 0 | permanent | yes | yes | - | Mushroom-like organism. Thrives in damp shaded areas. |
| 44 | Moss | Organic | Diffuse | 9 | 8 | 0 | permanent | yes | yes | - | Low carpet-like plant. Grows on damp stone and wood. |
| 45 | Roots | Organic | Grow | 10 | 12 | 0 | permanent | yes | yes | - | Underground plant tendrils. Anchor vegetation. |
| 46 | Meat | Organic | Static | 11 | 50 | 0 | permanent | - | yes | - | Animal flesh. Nutritious food for creatures. |
| 47 | Bone | Organic | Static | 14 | 10 | 0 | permanent | yes | - | - | Hard skeletal remains. Slow to decay. |
| 48 | Leather | Organic | Static | 11 | 15 | 0 | permanent | yes | - | - | Treated hide. Durable crafting material. |
| 49 | Charcoal | Organic | Static | 12 | 5 | 0 | permanent | yes | yes | - | Burnt carbon. Burns slowly but burns hot. |
| 50 | Fire | Energy | Burn | 1 | 0.5 | 10 | 90 steps | - | - | yes | Hot flame. Spreads to flammable neighbors and rises. |
| 51 | Energy | Energy | Energy | 1 | 100 | 0 | 2600 steps | - | - | yes | Pure energy drifts above ground. High nutrition for creatures. |
| 52 | Ice | Energy | Static | 9 | 0 | 0 | permanent | yes | - | - | Frozen water. Melts near heat sources. |
| 53 | Lava | Energy | Flow | 15 | 0 | 50 | permanent | - | - | yes | Molten rock. Extremely hot; turns to stone when cooled by water. |
| 54 | Magma | Energy | Flow | 16 | 0 | 60 | permanent | - | - | yes | Superheated lava. Even hotter; forms obsidian with water. |
| 55 | Void | Void | Static | 100 | 0 | 0 | permanent | - | - | - | Absolute emptiness. Denser than anything; impassable. |
| 56 | Mana | Energy | Energy | 1 | 80 | 0 | 1000 steps | - | - | yes | Magical energy. Powers spells and activates runes. |
| 57 | Soul | Energy | Energy | 1 | 200 | 0 | 500 steps | - | - | yes | Rare spiritual energy. Very high nutrition. |
| 58 | Ember | Energy | Energy | 1 | 1 | 12 | 900 steps | - | - | yes | Glowing coal. Lights the dark and fades slowly. |
| 59 | Brick | Constructed | Static | 20 | 3 | 0 | permanent | yes | - | - | Fired clay block. Used in ancient ruins. |
| 60 | Concrete | Constructed | Static | 20 | 2 | 0 | permanent | yes | - | - | Strong artificial stone. |
| 61 | Wire | Constructed | Conduct | 22 | 5 | 0 | permanent | yes | - | - | Conductive filament for simple circuits. |
| 62 | Circuit | Constructed | Conduct | 24 | 15 | 0 | permanent | yes | - | yes | Complex electronics. Conducts and processes signals. |
| 63 | Tool | Constructed | Static | 24 | 25 | 0 | permanent | yes | - | - | A general-purpose tool left by ruins. |
| 64 | Machine | Constructed | Static | 24 | 40 | 0 | permanent | yes | - | - | Ancient machinery. Complex and conductive. |
| 65 | Wall | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | Solid barrier block. |
| 66 | Door | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | An entrance that can be opened or blocked. |
| 67 | Chest | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A container that may hold loot. |
| 68 | Furnace | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A structure for smelting ores. |
| 69 | Anvil | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A forging station for metalwork. |
| 70 | Forge | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A high-temperature working station. |
| 71 | Farm | Constructed | Cultivate | 24 | 0 | 0 | permanent | yes | - | - | A cultivated plot for growing food. |
| 72 | Temple | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A sacred structure for worship. |
| 73 | Tower | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A tall ancient observation tower. |
| 74 | Bridge | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A span crossing over gaps or water. |
| 75 | Road | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A paved path for travel. |
| 76 | Rune | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A magical sigil. Activates when touched by mana. |
| 77 | Ward | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A protective charm that repels disease. |
| 78 | Portal | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A gateway to another place. Rare and unstable. |
| 79 | Egg | Biological | Hatch | 15 | 60 | 0 | 2000 steps | yes | yes | - | Hatches into a new creature after time passes. |
| 80 | Corpse | Biological | Decay | 12 | 40 | 0 | 3000 steps | yes | yes | - | Remains of a dead creature. Decays into bone or dust. |
| 81 | Virus | Biological | Spread | 1 | 0.5 | 300 | 500 steps | - | - | - | Microscopic disease. Spreads rapidly through organic matter. |
| 82 | Bacteria | Biological | Spread | 1 | 0.5 | 100 | 400 steps | - | - | - | Microbe that decomposes meat and food. |
| 83 | Food | Biological | Static | 1 | 25 | 0 | permanent | - | yes | - | Prepared nourishment for creatures. |
| 84 | Snow | Powders | Fall | 3 | 1 | 0 | permanent | yes | - | - | Cold powder that falls in winter. Melts back into water. |
| 85 | Crop | Biological | Static | 9 | 45 | 0 | permanent | - | yes | - | A ripe crop, grown from a plant on a tilled farm. Better food than a wild plant. |
| 86 | Market | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | - | A trading post. Trade beside it is far more profitable. |
| 87 | Workshop | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | - | A craftsman's hall. Tools are made faster beside it. |
| 88 | Granary | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | - | A food store. The colony does not go hungry beside it. |
| 89 | Beacon | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | yes | A signal beacon. The colony sees much further from it. |
| 90 | Hearth | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | yes | A hearth. The cold does not bite beside it. |
| 91 | Totem | Settlement | Static | 22 | 0 | 0 | permanent | yes | - | yes | An animal totem: the symbol of a tribe, raised at its heart. |

---|---------|----------|-----------|---------|-----------|----------|----------|-------|--------|------------|-------------|
| 0 | Sand | Powders | Fall | 16 | 0 | 0 | permanent | yes | - | - | Fine granular powder. Falls under gravity. Melts into glass near lava. |
| 1 | Sulfur | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Bright yellow powder. Flammable; reacts with lava to make gas. |
| 2 | Salt | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | White crystalline powder. Harmless seasoning. |
| 3 | Ash | Powders | Fall | 3 | 0 | 8 | permanent | yes | - | - | Grey powder left after burning. Can act as coal. |
| 4 | Dust | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Dry powder that drifts in the wind. |
| 5 | Gunpowder | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Explosive powder. Detonates when near fire or plasma. |
| 6 | Fertilizer | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Boosts plant growth when nearby. |
| 7 | Mud | Powders | Static | 3 | 5 | 0 | permanent | - | - | - | Wet dirt. Forms where water meets soil. |
| 8 | Water | Liquids | Flow | 10 | 2 | 0 | permanent | - | yes | - | Flowing liquid. Essential for life and plants. Extinguishes fire. |
| 9 | Oil | Liquids | Flow | 7 | 40 | 10 | permanent | - | - | - | Flammable dark liquid. Burns readily and can be gathered for energy. |
| 10 | Acid | Liquids | Flow | 12 | 0 | 100 | permanent | - | - | - | Corrosive green liquid. Dissolves most solids on contact. |
| 11 | Mercury | Liquids | Flow | 13 | 0 | 30 | permanent | - | - | - | Heavy silver liquid metal. Amalgamates with gold and copper. |
| 12 | Blood | Liquids | Flow | 11 | 30 | 0 | permanent | - | yes | - | Organic red liquid. Nutritious but can carry disease. |
| 13 | Poison | Liquids | Flow | 11 | 0 | 200 | permanent | - | - | - | Toxic green liquid. Harms anything that touches or drinks it. |
| 14 | Alcohol | Liquids | Flow | 8 | 15 | 0 | permanent | - | yes | - | Clear flammable liquid. Burns with a clean flame. |
| 15 | Honey | Liquids | Flow | 11 | 40 | 0 | permanent | - | yes | - | Thick golden sweet liquid. High energy food. |
| 16 | Tar | Liquids | Flow | 14 | 0 | 14 | permanent | - | - | - | Sticky black residue of burnt oil. Slow to burn, hard to wash off. |
| 17 | Steam | Gases | Rise | 1 | 1 | 0 | 260 steps | - | - | - | Hot water vapor. Rises and condenses back to water. |
| 18 | Smoke | Gases | Rise | 1 | 0.5 | 5 | 320 steps | - | - | - | Grey gas from fires. Rises and fades over time. |
| 19 | Gas | Gases | Rise | 1 | 20 | 0 | 400 steps | - | - | - | Generic flammable gas. Rises and can explode. |
| 20 | Plasma | Gases | Rise | 1 | 0 | 200 | 120 steps | - | - | yes | Superheated ionized gas. Very hot and toxic. |
| 21 | Chlorine | Gases | Rise | 2 | 1 | 150 | 300 steps | - | - | - | Toxic pale-green gas. Poisons water and organic matter. |
| 22 | Methane | Gases | Rise | 2 | 5 | 0 | 200 steps | - | - | - | Flammable gas from swamps. Explodes near fire. |
| 23 | Oxygen | Gases | Rise | 2 | 2 | 0 | 150 steps | - | - | - | Breathable gas. Makes fires burn hotter. |
| 24 | Stone | Solids | Static | 20 | 2 | 0 | permanent | yes | - | - | Hard common rock. Forms the crust of the world. |
| 25 | Metal | Solids | Static | 22 | 4 | 0 | permanent | yes | - | - | Generic sturdy metal. Can be melted and alloyed. |
| 26 | Copper | Solids | Static | 22 | 8 | 0 | permanent | yes | - | - | Reddish metal. Early-age conductor and tool material. |
| 27 | Tin | Solids | Static | 22 | 6 | 0 | permanent | yes | - | - | Soft silvery metal. Alloys with copper to make bronze. |
| 28 | Bronze | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Hard copper-tin alloy. Better tools and weapons. |
| 29 | Iron | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Strong dark metal. Foundation of tools and armor. |
| 30 | Steel | Solids | Static | 22 | 15 | 0 | permanent | yes | - | - | Refined iron-carbon alloy. Very strong and durable. |
| 31 | Gold | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Shiny yellow precious metal. Highly valued; soft and dense. |
| 32 | Silver | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Pale lustrous metal. Conducts energy well. |
| 33 | Glass | Solids | Static | 18 | 2 | 0 | permanent | yes | - | - | Transparent solid made from melted sand. |
| 34 | Crystal | Solids | Static | 18 | 6 | 0 | permanent | yes | - | yes | Geometric mineral formation. Stores magical energy. |
| 35 | Gem | Solids | Static | 18 | 20 | 0 | permanent | yes | - | yes | Polished precious crystal. Rare and valuable. |
| 36 | Diamond | Solids | Static | 18 | 50 | 0 | permanent | yes | - | yes | Hardest natural substance. Extremely valuable. |
| 37 | Obsidian | Solids | Static | 18 | 8 | 0 | permanent | yes | - | - | Volcanic glass. Forms where water meets lava. |
| 38 | Granite | Solids | Static | 22 | 1 | 0 | permanent | yes | - | - | Deep igneous rock. Slow to melt and harder than stone. |
| 39 | Rust | Powders | Fall | 3 | 3 | 0 | permanent | yes | - | - | Iron oxide. Feeds iron ore back into the ground. |
| 40 | Wood | Organic | Static | 8 | 20 | 0 | permanent | yes | - | - | Solid plant material. Burns and floats on water. |
| 41 | Plant | Organic | Grow | 9 | 30 | 0 | permanent | yes | yes | - | Living vegetation. Grows near water and spreads slowly. |
| 42 | Spore | Organic | Diffuse | 3 | 10 | 0 | permanent | - | yes | - | Reproductive dust from fungi and plants. Drifts on air. |
| 43 | Fungus | Organic | Grow | 9 | 15 | 0 | permanent | yes | yes | - | Mushroom-like organism. Thrives in damp shaded areas. |
| 44 | Moss | Organic | Diffuse | 9 | 8 | 0 | permanent | yes | yes | - | Low carpet-like plant. Grows on damp stone and wood. |
| 45 | Roots | Organic | Grow | 10 | 12 | 0 | permanent | yes | yes | - | Underground plant tendrils. Anchor vegetation. |
| 46 | Meat | Organic | Static | 11 | 50 | 0 | permanent | - | yes | - | Animal flesh. Nutritious food for creatures. |
| 47 | Bone | Organic | Static | 14 | 10 | 0 | permanent | yes | - | - | Hard skeletal remains. Slow to decay. |
| 48 | Leather | Organic | Static | 11 | 15 | 0 | permanent | yes | - | - | Treated hide. Durable crafting material. |
| 49 | Charcoal | Organic | Static | 12 | 5 | 0 | permanent | yes | yes | - | Burnt carbon. Burns slowly but burns hot. |
| 50 | Fire | Energy | Burn | 1 | 0.5 | 10 | 90 steps | - | - | yes | Hot flame. Spreads to flammable neighbors and rises. |
| 51 | Energy | Energy | Energy | 1 | 100 | 0 | 2600 steps | - | - | yes | Pure energy drifts above ground. High nutrition for creatures. |
| 52 | Ice | Energy | Static | 9 | 0 | 0 | permanent | yes | - | - | Frozen water. Melts near heat sources. |
| 53 | Lava | Energy | Flow | 15 | 0 | 50 | permanent | - | - | yes | Molten rock. Extremely hot; turns to stone when cooled by water. |
| 54 | Magma | Energy | Flow | 16 | 0 | 60 | permanent | - | - | yes | Superheated lava. Even hotter; forms obsidian with water. |
| 55 | Void | Void | Static | 100 | 0 | 0 | permanent | - | - | - | Absolute emptiness. Denser than anything; impassable. |
| 56 | Mana | Energy | Energy | 1 | 80 | 0 | 1000 steps | - | - | yes | Magical energy. Powers spells and activates runes. |
| 57 | Soul | Energy | Energy | 1 | 200 | 0 | 500 steps | - | - | yes | Rare spiritual energy. Very high nutrition. |
| 58 | Ember | Energy | Energy | 1 | 1 | 12 | 900 steps | - | - | yes | Glowing coal. Lights the dark and fades slowly. |
| 59 | Brick | Constructed | Static | 20 | 3 | 0 | permanent | yes | - | - | Fired clay block. Used in ancient ruins. |
| 60 | Concrete | Constructed | Static | 20 | 2 | 0 | permanent | yes | - | - | Strong artificial stone. |
| 61 | Wire | Constructed | Conduct | 22 | 5 | 0 | permanent | yes | - | - | Conductive filament for simple circuits. |
| 62 | Circuit | Constructed | Conduct | 24 | 15 | 0 | permanent | yes | - | yes | Complex electronics. Conducts and processes signals. |
| 63 | Tool | Constructed | Static | 24 | 25 | 0 | permanent | yes | - | - | A general-purpose tool left by ruins. |
| 64 | Machine | Constructed | Static | 24 | 40 | 0 | permanent | yes | - | - | Ancient machinery. Complex and conductive. |
| 65 | Wall | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | Solid barrier block. |
| 66 | Door | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | An entrance that can be opened or blocked. |
| 67 | Chest | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A container that may hold loot. |
| 68 | Furnace | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A structure for smelting ores. |
| 69 | Anvil | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A forging station for metalwork. |
| 70 | Forge | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A high-temperature working station. |
| 71 | Farm | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A cultivated plot for growing food. |
| 72 | Temple | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A sacred structure for worship. |
| 73 | Tower | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A tall ancient observation tower. |
| 74 | Bridge | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A span crossing over gaps or water. |
| 75 | Road | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A paved path for travel. |
| 76 | Rune | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A magical sigil. Activates when touched by mana. |
| 77 | Ward | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A protective charm that repels disease. |
| 78 | Portal | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A gateway to another place. Rare and unstable. |
| 79 | Egg | Biological | Hatch | 15 | 60 | 0 | 2000 steps | yes | yes | - | Hatches into a new creature after time passes. |
| 80 | Corpse | Biological | Decay | 12 | 40 | 0 | 3000 steps | yes | yes | - | Remains of a dead creature. Decays into bone or dust. |
| 81 | Virus | Biological | Spread | 1 | 0.5 | 300 | 500 steps | - | - | - | Microscopic disease. Spreads rapidly through organic matter. |
| 82 | Bacteria | Biological | Spread | 1 | 0.5 | 100 | 400 steps | - | - | - | Microbe that decomposes meat and food. |
| 83 | Food | Biological | Static | 1 | 25 | 0 | permanent | - | yes | - | Prepared nourishment for creatures. |
| 84 | Snow | Powders | Fall | 3 | 1 | 0 | permanent | yes | - | - | Cold powder that falls in winter. Melts back into water. |

---|---------|----------|-----------|---------|-----------|----------|----------|-------|--------|------------|-------------|
| 0 | Sand | Powders | Fall | 16 | 0 | 0 | permanent | yes | - | - | Fine granular powder. Falls under gravity. Melts into glass near lava. |
| 1 | Sulfur | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Bright yellow powder. Flammable; reacts with lava to make gas. |
| 2 | Salt | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | White crystalline powder. Harmless seasoning. |
| 3 | Ash | Powders | Fall | 3 | 0 | 8 | permanent | yes | - | - | Grey powder left after burning. Can act as coal. |
| 4 | Dust | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Dry powder that drifts in the wind. |
| 5 | Gunpowder | Powders | Fall | 3 | 0 | 5 | permanent | yes | - | - | Explosive powder. Detonates when near fire or plasma. |
| 6 | Fertilizer | Powders | Fall | 3 | 0 | 0 | permanent | yes | - | - | Boosts plant growth when nearby. |
| 7 | Mud | Powders | Static | 3 | 5 | 0 | permanent | - | - | - | Wet dirt. Forms where water meets soil. |
| 8 | Water | Liquids | Flow | 10 | 2 | 0 | permanent | - | yes | - | Flowing liquid. Essential for life and plants. Extinguishes fire. |
| 9 | Oil | Liquids | Flow | 7 | 40 | 10 | permanent | - | - | - | Flammable dark liquid. Burns readily and can be gathered for energy. |
| 10 | Acid | Liquids | Flow | 12 | 0 | 100 | permanent | - | - | - | Corrosive green liquid. Dissolves most solids on contact. |
| 11 | Mercury | Liquids | Flow | 13 | 0 | 30 | permanent | - | - | - | Heavy silver liquid metal. Amalgamates with gold and copper. |
| 12 | Blood | Liquids | Flow | 11 | 30 | 0 | permanent | - | yes | - | Organic red liquid. Nutritious but can carry disease. |
| 13 | Poison | Liquids | Flow | 11 | 0 | 200 | permanent | - | - | - | Toxic green liquid. Harms anything that touches or drinks it. |
| 14 | Alcohol | Liquids | Flow | 8 | 15 | 0 | permanent | - | yes | - | Clear flammable liquid. Burns with a clean flame. |
| 15 | Honey | Liquids | Flow | 11 | 40 | 0 | permanent | - | yes | - | Thick golden sweet liquid. High energy food. |
| 16 | Tar | Liquids | Flow | 14 | 0 | 14 | permanent | - | - | - | Sticky black residue of burnt oil. Slow to burn, hard to wash off. |
| 17 | Steam | Gases | Rise | 1 | 1 | 0 | 260 steps | - | - | - | Hot water vapor. Rises and condenses back to water. |
| 18 | Smoke | Gases | Rise | 1 | 0.5 | 5 | 320 steps | - | - | - | Grey gas from fires. Rises and fades over time. |
| 19 | Gas | Gases | Rise | 1 | 20 | 0 | 400 steps | - | - | - | Generic flammable gas. Rises and can explode. |
| 20 | Plasma | Gases | Rise | 1 | 0 | 200 | 120 steps | - | - | yes | Superheated ionized gas. Very hot and toxic. |
| 21 | Chlorine | Gases | Rise | 2 | 1 | 150 | 300 steps | - | - | - | Toxic pale-green gas. Poisons water and organic matter. |
| 22 | Methane | Gases | Rise | 2 | 5 | 0 | 200 steps | - | - | - | Flammable gas from swamps. Explodes near fire. |
| 23 | Oxygen | Gases | Rise | 2 | 2 | 0 | 150 steps | - | - | - | Breathable gas. Makes fires burn hotter. |
| 24 | Stone | Solids | Static | 20 | 2 | 0 | permanent | yes | - | - | Hard common rock. Forms the crust of the world. |
| 25 | Metal | Solids | Static | 22 | 4 | 0 | permanent | yes | - | - | Generic sturdy metal. Can be melted and alloyed. |
| 26 | Copper | Solids | Static | 22 | 8 | 0 | permanent | yes | - | - | Reddish metal. Early-age conductor and tool material. |
| 27 | Tin | Solids | Static | 22 | 6 | 0 | permanent | yes | - | - | Soft silvery metal. Alloys with copper to make bronze. |
| 28 | Bronze | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Hard copper-tin alloy. Better tools and weapons. |
| 29 | Iron | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Strong dark metal. Foundation of tools and armor. |
| 30 | Steel | Solids | Static | 22 | 15 | 0 | permanent | yes | - | - | Refined iron-carbon alloy. Very strong and durable. |
| 31 | Gold | Solids | Static | 22 | 12 | 0 | permanent | yes | - | - | Shiny yellow precious metal. Highly valued; soft and dense. |
| 32 | Silver | Solids | Static | 22 | 10 | 0 | permanent | yes | - | - | Pale lustrous metal. Conducts energy well. |
| 33 | Glass | Solids | Static | 18 | 2 | 0 | permanent | yes | - | - | Transparent solid made from melted sand. |
| 34 | Crystal | Solids | Static | 18 | 6 | 0 | permanent | yes | - | yes | Geometric mineral formation. Stores magical energy. |
| 35 | Gem | Solids | Static | 18 | 20 | 0 | permanent | yes | - | yes | Polished precious crystal. Rare and valuable. |
| 36 | Diamond | Solids | Static | 18 | 50 | 0 | permanent | yes | - | yes | Hardest natural substance. Extremely valuable. |
| 37 | Obsidian | Solids | Static | 18 | 8 | 0 | permanent | yes | - | - | Volcanic glass. Forms where water meets lava. |
| 38 | Granite | Solids | Static | 22 | 1 | 0 | permanent | yes | - | - | Deep igneous rock. Slow to melt and harder than stone. |
| 39 | Rust | Powders | Fall | 3 | 3 | 0 | permanent | yes | - | - | Iron oxide. Feeds iron ore back into the ground. |
| 40 | Wood | Organic | Static | 8 | 20 | 0 | permanent | yes | - | - | Solid plant material. Burns and floats on water. |
| 41 | Plant | Organic | Grow | 9 | 30 | 0 | permanent | yes | yes | - | Living vegetation. Grows near water and spreads slowly. |
| 42 | Spore | Organic | Diffuse | 3 | 10 | 0 | permanent | - | yes | - | Reproductive dust from fungi and plants. Drifts on air. |
| 43 | Fungus | Organic | Grow | 9 | 15 | 0 | permanent | yes | yes | - | Mushroom-like organism. Thrives in damp shaded areas. |
| 44 | Moss | Organic | Diffuse | 9 | 8 | 0 | permanent | yes | yes | - | Low carpet-like plant. Grows on damp stone and wood. |
| 45 | Roots | Organic | Grow | 10 | 12 | 0 | permanent | yes | yes | - | Underground plant tendrils. Anchor vegetation. |
| 46 | Meat | Organic | Static | 11 | 50 | 0 | permanent | - | yes | - | Animal flesh. Nutritious food for creatures. |
| 47 | Bone | Organic | Static | 14 | 10 | 0 | permanent | yes | - | - | Hard skeletal remains. Slow to decay. |
| 48 | Leather | Organic | Static | 11 | 15 | 0 | permanent | yes | - | - | Treated hide. Durable crafting material. |
| 49 | Charcoal | Organic | Static | 12 | 5 | 0 | permanent | yes | yes | - | Burnt carbon. Burns slowly but burns hot. |
| 50 | Fire | Energy | Burn | 1 | 0.5 | 10 | 90 steps | - | - | yes | Hot flame. Spreads to flammable neighbors and rises. |
| 51 | Energy | Energy | Energy | 1 | 100 | 0 | 2600 steps | - | - | yes | Pure energy drifts above ground. High nutrition for creatures. |
| 52 | Ice | Energy | Static | 9 | 0 | 0 | permanent | yes | - | - | Frozen water. Melts near heat sources. |
| 53 | Lava | Energy | Flow | 15 | 0 | 50 | permanent | - | - | yes | Molten rock. Extremely hot; turns to stone when cooled by water. |
| 54 | Magma | Energy | Flow | 16 | 0 | 60 | permanent | - | - | yes | Superheated lava. Even hotter; forms obsidian with water. |
| 55 | Void | Void | Static | 100 | 0 | 0 | permanent | - | - | - | Absolute emptiness. Denser than anything; impassable. |
| 56 | Mana | Energy | Energy | 1 | 80 | 0 | 1000 steps | - | - | yes | Magical energy. Powers spells and activates runes. |
| 57 | Soul | Energy | Energy | 1 | 200 | 0 | 500 steps | - | - | yes | Rare spiritual energy. Very high nutrition. |
| 58 | Ember | Energy | Energy | 1 | 1 | 12 | 900 steps | - | - | yes | Glowing coal. Lights the dark and fades slowly. |
| 59 | Brick | Constructed | Static | 20 | 3 | 0 | permanent | yes | - | - | Fired clay block. Used in ancient ruins. |
| 60 | Concrete | Constructed | Static | 20 | 2 | 0 | permanent | yes | - | - | Strong artificial stone. |
| 61 | Wire | Constructed | Conduct | 22 | 5 | 0 | permanent | yes | - | - | Conductive filament for simple circuits. |
| 62 | Circuit | Constructed | Conduct | 24 | 15 | 0 | permanent | yes | - | yes | Complex electronics. Conducts and processes signals. |
| 63 | Tool | Constructed | Static | 24 | 25 | 0 | permanent | yes | - | - | A general-purpose tool left by ruins. |
| 64 | Machine | Constructed | Static | 24 | 40 | 0 | permanent | yes | - | - | Ancient machinery. Complex and conductive. |
| 65 | Wall | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | Solid barrier block. |
| 66 | Door | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | An entrance that can be opened or blocked. |
| 67 | Chest | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A container that may hold loot. |
| 68 | Furnace | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A structure for smelting ores. |
| 69 | Anvil | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A forging station for metalwork. |
| 70 | Forge | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A high-temperature working station. |
| 71 | Farm | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A cultivated plot for growing food. |
| 72 | Temple | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A sacred structure for worship. |
| 73 | Tower | Constructed | Static | 20 | 0 | 0 | permanent | yes | - | - | A tall ancient observation tower. |
| 74 | Bridge | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A span crossing over gaps or water. |
| 75 | Road | Constructed | Static | 24 | 0 | 0 | permanent | yes | - | - | A paved path for travel. |
| 76 | Rune | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A magical sigil. Activates when touched by mana. |
| 77 | Ward | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A protective charm that repels disease. |
| 78 | Portal | Magical | Static | 15 | 0 | 0 | permanent | yes | - | yes | A gateway to another place. Rare and unstable. |
| 79 | Egg | Biological | Hatch | 15 | 60 | 0 | 2000 steps | yes | yes | - | Hatches into a new creature after time passes. |
| 80 | Corpse | Biological | Decay | 12 | 40 | 0 | 3000 steps | yes | yes | - | Remains of a dead creature. Decays into bone or dust. |
| 81 | Virus | Biological | Spread | 1 | 0.5 | 300 | 500 steps | - | - | - | Microscopic disease. Spreads rapidly through organic matter. |
| 82 | Bacteria | Biological | Spread | 1 | 0.5 | 100 | 400 steps | - | - | - | Microbe that decomposes meat and food. |
| 83 | Food | Biological | Static | 1 | 25 | 0 | permanent | - | yes | - | Prepared nourishment for creatures. |
| 84 | Snow | Powders | Fall | 3 | 1 | 0 | permanent | yes | - | - | Cold powder that falls in winter. Melts back into water. |

---

## 3. World simulation

### 3.1 The step

`World::step` (`world/physics.rs`) runs one tick:

1. `self.mine_budget = 3` — a hard cap on how much can be excavated per step,
   so creatures cannot strip-mine the world.
2. The **queue** is moved into the **active list**. The queue holds every cell
   that is *not* inert — that is, every cell that could still change. The
   inert bulk of the world (solid rock) is never touched.
3. The active list is shuffled and walked. Each active cell runs
   `World::update_cell`, which applies reactions, decay and behaviour.
4. Cells that changed are queued for the next step (`World::mark_active`).
5. `World::maintain` runs the slow world upkeep.

The queue is moved at the *top* of the step rather than at the bottom of the
previous one. That matters: anything that changes the world **between** steps
— a creature building a farm, the world regrowing a plant — queues itself
too, and it has to survive until the next step. Clearing the queue at the end
of the step instead threw those cells away permanently, because the
per-cell `listed` flag stayed set and refused to queue them again.

`World::update_cell` (`world/physics.rs`) is the per-cell dispatcher:

1. If the element is reactive, try `World::try_reaction`. If it fired, stop.
2. If the element has a finite lifetime, age the cell and let it decay.
3. Otherwise run the behaviour: `step_fall`, `step_flow`, `step_rise`,
   `step_diffuse`, `step_burn`, `step_grow`, `step_energy`, `step_cultivate`.

### 3.2 Chemistry

`World::try_reaction` (`world/reactions.rs`) is a match over the element with
one arm per pair that interacts. The important ones:

| Trigger | Result |
|---|---|
| Water next to Lava | Water becomes Steam, Lava becomes Stone |
| Water next to Magma | Water becomes Steam, Magma becomes Obsidian |
| Lava next to Sand | Sand becomes Glass (slowly) |
| Lava next to Ice | Ice becomes Water |
| Lava next to Sulfur | Sulfur becomes Gas |
| Lava next to metal | metal becomes Magma |
| Acid next to a solid | the solid becomes Sand, the acid becomes Salt |
| Fire next to a flammable | the flammable ignites |
| Fire next to Gunpowder | explosion |
| Fire next to Methane | bigger fire |
| Fire next to Oxygen | Fire becomes Plasma |
| Fire next to Sulfur | Sulfur becomes Plasma |
| Methane next to Fire | explosion |
| Oxygen next to Fire | Fire becomes Plasma |
| Gunpowder next to Fire | explosion |
| Water next to Gunpowder | Gunpowder becomes Mud (damp) |
| Mercury next to Gold/Copper/Silver | becomes Metal (amalgam) |
| Chlorine next to Water or organic | becomes Poison |
| Virus next to organic | spreads |
| Bacteria next to meat/corpse | spreads |
| Mana next to Rune | Rune becomes Portal |
| Soul next to Ward | Ward becomes Energy |

`World::explosion` (`world/reactions.rs`) clears a radius, turning cells into
fire and smoke.

Explosion debris deliberately stays **dark**: the scorched cells become smoke
and charcoal rather than plasma, mana and energy. A bright debris cloud would
dump dozens of light sources into the light field at once and flash the whole
cave white for a frame. There is also no full-screen lightning flash on
storms — the storm lights the sky through the weather overlay and the light
field instead.

### 3.3 Behaviours in detail

All in `world/behaviors.rs`:

- `step_fall` — down first, then down-left, then down-right.
- `step_flow` — down, then diagonals, then up to three cells sideways.
- `step_rise` — up, then diagonals, then a sideways drift. A gas that reaches
  row 0 is deleted, so the sky does not fill with smoke.
- `step_diffuse` — a random step in any direction.
- `step_burn` — ignites flammable neighbours (Wood, Oil, Tar, Plant, Spore,
  Gas), then rises. **Fire at row 0 goes out**, because there is nothing above
  it to burn.
- `step_grow` — plants convert nearby water into more plants, grow upward
  where there is water, and die back when overcrowded.
- `step_cultivate` — a farm tends its plot (see §3.4).
- `step_energy` — drifts in a random direction, bouncing off walls.
- `decay` — Fire becomes Smoke, Steam becomes Water, everything else vanishes.

### 3.4 Farming

Farming is how a colony makes food grow back instead of only foraging what
is already there.

**The elements.** `Elem::Farm` is a tilled plot. `Elem::Crop` is a ripe crop:
solid, edible, and worth more food than the wild plant it grew from
(nutrition 45 against the plant's 30).

**The rule.** A farm (`world/behaviors.rs`, `World::step_cultivate`):

1. needs **water within `FARM_WATER_RADIUS`** (6 cells) — a plot with no
   water is dead ground and does nothing at all;
2. **sows** a seedling into a free neighbouring cell that has ground under
   it;
3. **ripens** the seedlings around it into crops.

The seedling and the crop are separate cells, so the plot keeps producing as
long as the creatures do not eat every seedling before it ripens.

**Building a farm.** A creature builds one when it has 4 wood and tech level
1 (`creature/actions.rs`, `act_build`). A farm is not placed anywhere: the
site is chosen by `World::find_farm_site` (`world/query.rs`), which requires

- an empty cell,
- standing on the surface (not buried, not floating),
- ground underneath,
- **water within `FARM_WATER_RADIUS`**,
- and a free neighbouring cell to sow into.

The same water radius is used when the plot is chosen and when it is worked,
so a farm that can be built is always a farm that can grow.

**Who builds them.** A hungry colony prefers a farm to a wall
(`c.hunger > 45` puts `Buildable::Farm` first in the choice). Each colony may
keep at most `MAX_FARMS` (4) plots — a plot that nobody tends is just a
field of weeds, and the wood is needed for other things.

**Why it matters.** The wild plant population is finite and the
`scarcity` pressure (§3.7) eats into it. A colony with farms keeps a
renewable food supply of its own, so it survives the pressure that kills a
colony which only forages.

### 3.5 World upkeep

`World::maintain` (`world/maintain.rs`) runs every step and handles the slow
changes:

| What | Cadence | Method |
|---|---|---|
| Ground cache refresh | every 15 steps | `update_ground` |
| Weather roll | every 500–1200 steps | inline |
| Rain and snow | every 15–60 steps | inline |
| Lightning (storm) | 2% per step | inline |
| Meteors | 0.5% per step | inline |
| Underground springs | every 200 steps | inline |
| Surface pond top-up | every 1000 steps | inline |
| Plant regrowth | every 15 steps | inline |
| Plant culling | every 20 steps | inline |
| Energy motes | every 25 steps | inline |
| Oil seeps, sulfur vents | every 50 steps | inline |
| Geological stone formation | every 100 steps | inline |
| Slow ore genesis | every 250 steps | inline |
| Crystal growth | every 200 steps | inline |
| Rising magma | every 600 steps | inline |
| Particle compaction | every 300 steps | `World::compact` |
| Pressure update | every step | `World::update_pressure` |
| Farm sowing and ripening | every step, 5% each | `World::step_cultivate` |

### 3.6 Environmental pressure

The world does not get harder by scaling one difficulty number. Instead it
carries **five separate pressures**, each of which rises for its own reason,
is measured against its own part of the world, and hurts in its own way. The
HUD prints all five so the player can see exactly which part of the world is
turning against them.

The pressures live in `world::Pressure` (`world/mod.rs`) and are stored on
`World::pressure`. `World::update_pressure` (`world/maintain.rs`) advances
them once per step.

| Pressure | What it means | What raises it | What it does |
|---|---|---|---|
| `scarcity` | The surface is being picked clean | Vegetation cover falls below 60% of what this world has recently been able to support | Fewer plant seedlings survive each regrowth batch |
| `depletion` | The world's metal is running out | Every ore genesis event | Fewer ore veins form, so mining is no longer sustainable |
| `harshness` | The climate is turning violent | Time spent in storms and meteor showers | Storms and meteors become more likely |
| `extremes` | The seasons swing wider | Each season that turns over | Wider daily temperature swing: fiercer summers, harder winters |
| `volcanic` | The deep rock is heating up | Steady rise over the run | Magma creeps up the shafts; the deep mines get more dangerous |

Notes on the design:

- **Scarcity is relative, not absolute.** The world keeps a `plant_capacity`
  high-water mark (the most plants it has recently supported, decaying
  slowly). Scarcity is measured as `plants / plant_capacity`, so a dry world
  is not condemned for having few plants — only for having fewer than it
  could grow. A colony that farms well can push the pressure back down.
- **Harshness is a feedback loop.** Violent weather raises harshness, and
  harshness makes violent weather more likely in the weather roll.
- **Depletion only ever rises.** Ore that has been mined does not come back,
  so the world's metal is a genuinely finite resource.
- The pressures are independent, so different runs develop different
  difficulty profiles: a run that grazes its surface out gets hit by
  scarcity, a run that mines deep gets hit by depletion and volcanic
  activity, and a long run always gets hit by extremes.

The HUD line reads:

```
pressure: scarcity 34%  depletion 5%  harshness 17%  extremes 0%  volcanic 9%
```

### 3.7 World generation

`World::seed` (`world/generation.rs`) builds a world in this order:

1. Bedrock (`Void`) at the very bottom.
2. A soil column per biome: sand in the desert, ice in the tundra, mud in the
   swamp and forest, stone below.
3. Surface decoration per biome: trees, dunes, swamp pools, ice, grass.
4. A deep granite layer.
5. **Caves** — `World::carve_caves` cuts winding, connected tunnels through
   the rock with a cellular automaton, keeping a solid floor under each one.
6. A lava chamber, always below the surface.
7. A surface lake and ponds, **carved into** the terrain so the water rests in
   a basin rather than floating on top of the ground.
8. Vegetation, wood, energy motes, copper, gold, crystal.
9. The element museum (`place_element_museum`) — one of every element, placed
   underground in a walled chamber so the whole palette exists at the start.
10. Ore veins, crystal caves, ancient ruins (`generate_ore_veins`,
    `generate_crystal_caves`, `generate_underground_structures`).
11. `bury_floating_resources` — anything left hovering above the terrain is
    dug back underground.
12. `update_ground` and `rebuild_active`.

---

## 4. Creatures

### 4.1 The agent

A `Creature` (`creature/mod.rs`) carries its needs, its skills, its social
state, its magic, its genes and its memory. The fields that matter:

| Group | Fields |
|---|---|
| Body | `x`, `y`, `vx`, `vy`, `size`, `health`, `energy`, `max_energy` |
| Identity | `id`, `species`, `role`, `name`, `team_color`, `generation` |
| Needs | `hunger`, `thirst`, `stamina`, `max_stamina`, `resting` |
| Mind | `net`, `memory`, `mood`, `mood_timer`, `belief`, `devotion` |
| Social | `loyalty`, `reputation`, `trade_partner`, `squad_id`, `squad_role` |
| Work | `tool`, `tool_durability`, `inventory`, `tech_level`, `carried` |
| Magic | `mana`, `max_mana`, `known_spells`, `active_spell`, `spell_cd` |
| Genetics | `genes` (32 floats), `mutations`, `traits` (bit flags) |

### 4.2 Roles

Defined in `creature/enums.rs`:

| Role | Job |
|---|---|
| `Worker` | gathers and hauls; the backbone of the colony |
| `Soldier` | fights; ranged with a bow or staff |
| `Queen` | the founder of a species; breeds and sets the base |
| `Priest` | keeps the faith; benefits from temples |
| `Merchant` | trades with other species |
| `Builder` | raises walls, farms and towers |
| `Explorer` | ranges far from the colony, looking for resources |
| `Shaman` | casts spells; the magic-user |

A child inherits its mother's role with some chance of variation
(`creature/colony.rs`, `CreatureSystem::spawn`).

#### Starting colonies

`CreatureSystem::populate` (`creature/colony.rs`) places the starting
species. **Every founder starts alone.** There used to be a rule that handed
each founder a guaranteed partner of the same species, placed right next to
it, so that young colonies could defend themselves. That partner was a free
ally: a species that spawned badly, landed in a bad spot or got attacked
early still survived because it had a second body. The rule has been
removed — a species that cannot hold its own ground dies out.

Note that the *intra-species* peace still holds: members of the same species
never attack each other, do not target each other with spells, and do not
flee from each other. What changed is that a species no longer starts with a
free ally.

### 4.3 The brain

`Net` (`creature/brain.rs`) is a tiny feed-forward network:
`224 inputs → 24 → 12 → 24 outputs`.

- `Net::forward_into` runs it without allocating. It uses eight independent
  accumulators so the float additions do not serialise on one dependency chain.
- `Net::mutate` perturbs weights and biases.
- `Net::clone_net` copies a genome for a child.

The 24 outputs are:

| Output | Meaning |
|---|---|
| 0, 1 | move right / down |
| 2 | eat |
| 3 | reproduce |
| 4 | attack |
| 5 | build |
| 6 | craft |
| 7 | cast a spell |
| 8 | mount / dismount |
| 9 | trade |
| 10 | rest |
| 11 | switch spell |
| 12 | dig / mine |
| 13 | gather water |
| 14 | shoot |
| 15 | flee |
| 16 | seek food or water |
| 17 | forage |
| 18 | rally to the base |
| 19 | defend the colony |
| 20 | hunt |
| 21 | prowl |
| 22 | hoard |
| 23 | cultivate |

### 4.4 Senses

`CreatureSystem::sense` (`creature/senses.rs`) fills the 224 inputs:

| Inputs | What they carry |
|---|---|
| 0–47 | eight directions × six channels (food, hazard, water, solid, metal, crystal) |
| 48–55 | colony awareness: base direction, base distance, friend count, enemy count, role flags, time of day |
| 56–62 | role one-hot, temperature |
| 63–72 | tool, durability, tech level, inventory, craft cooldown, stock of each resource |
| 73–80 | mana, active spell, known spells, spell cooldowns |
| 81–84 | mount, stamina, vehicle, reputation |
| 85–92 | the memory blackboard |
| 93–108 | detailed inventory |
| 109–116 | disease, mood, loyalty, devotion, trade partner, squad |
| 117–132 | the genes themselves |
| 133–136 | offspring, kills, generation, mutations |
| 137–140 | the season |
| 141–152 | enemy awareness: direction, distance, role, health, counts |
| 153–162 | incoming shots |
| 163–172 | threats and low-health warning |
| 173–184 | foraging directions for food, water, wood and ore |
| 185–194 | resource counts, turret counts, base direction and distance |
| 195–202 | depth, openness, solid/air counts, hazards, plants |
| 203–212 | personal readiness: cooldowns, tools, hunger and thirst warnings |
| 213–214 | colony strength and rival population |

`LocalScan` (`creature/senses.rs`) is a cheap 17 × 17 survey of the cells
around a creature: the nearest food, water, wood, ore, hazard and poison, plus
counts. It is shared between the senses and the steering so the world is only
scanned once per creature per step.

### 4.5 Actions

All in `creature/actions.rs`. Each one is a separate, readable rule:

| Function | What it does |
|---|---|
| `act_steer` | turns the outputs and the senses into a steering direction and a speed multiplier |
| `act_move` | moves the creature, shoving loose debris aside and escaping if buried |
| `act_eat` | finds the nearest edible cell and eats it, filling energy, hunger and thirst; also gathers whatever the cell was (wood, ore, crystal…) into the inventory |
| `act_mine` | mines a gatherable resource out of the rock, paying stamina and energy |
| `act_build` | places a structure: wall, door, chest, farm, temple, tower or bridge |
| `act_craft` | crafts a tool, bomb or salve from the inventory, using up materials |
| `act_combat` | decides to melee or to shoot, and spawns the projectile |

### 4.6 Energy costs

Actions are not free. The cost table lives in `creature::energy_cost`
(`creature/mod.rs`) and every action that costs energy subtracts it from the
actor. This is what makes foraging and the food web matter: a creature that
mines, builds and fights all day has to eat to pay for it.

| Action | Cost | Paid by |
|---|---|---|
| `MOVE` | 0.020 per cell travelled | `act_move` |
| `SHOVE` | 0.35 | `act_move` (shoving loose debris) |
| `ATTACK` | 6.0 | `resolve_attacks` (melee) |
| `SHOOT` | 4.0 | `act_combat` |
| `DIG` | 5.0 | `CreatureSystem::step` |
| `MINE` | 3.0 | `act_mine` |
| `BUILD` | 8.0 | `act_build` |
| `CRAFT` | 5.0 | `act_craft` |
| `BOMB` | 6.0 | `CreatureSystem::step` |
| `SALVE` | 4.0 | `CreatureSystem::step` |
| `TRADE` | 2.0 | `CreatureSystem::step` |
| `SPELL` | 10.0 | `CreatureSystem::step` |

Energy is also drained passively: a creature that is not resting loses
0.08 energy per step, and a resting one regains 0.02. The income side is
eating: a plant gives `nutrition × 4` energy, so a plant is worth roughly
120 energy. One plant therefore covers a good stretch of movement, mining and
fighting. A **crop** is worth `45 × 4 = 180` energy, so a colony with farms
eats better than one that only forages.

**A failed craft costs nothing.** The craft action is only charged when it
actually produces something. Charging for the attempt turned every toolless
creature into an energy sink: it tried to craft every few steps, paid 5
energy each time for work that never happened, and starved next to the
materials it could never afford. A failed attempt now costs nothing and
sets a short cooldown (`craft_cooldown = 25`) so the creature stops hammering
the same impossible recipe. The craft action also respects the cooldown, so
the network cannot ask for a craft on every single step.

### 4.7 Movement speed

Creature velocity is clamped to `MAX_SPEED` (`creature/mod.rs`), which is
**0.9 cells per step**. The old limit of 2.6 made a creature cross the
visible screen in well under a second, which read as zooming rather than
walking. A mounted creature (`MOUNT_SPEED_BONUS`, 1.5×) is the fastest a
creature can go.

### 4.8 Combat

In `creature/combat.rs`:

| Function | What it does |
|---|---|
| `resolve_turrets` | armed towers shoot at other species; towers of extinct species decay |
| `resolve_bombs` | bombs arc, and detonate when they hit something |
| `resolve_projectiles` | arrows, mana bolts and tower fire travel and hit |
| `resolve_attacks` | melee: soldiers prioritise enemy queens, squads focus fire |
| `resolve_deaths` | removes the dead and leaves remains behind |
| `resolve_births` | places newborns next to the colony |
| `apply_status` | applies spell damage, curses and healing |

### 4.9 Reproduction and genetics

Genes are 32 floats in `0..1`. `Creature::child_blueprint`
(`creature/colony.rs`) mutates them deterministically — the mutation is seeded
by the parent's id and age, so the same parent always produces the same
mutation.

`decode_traits` (`creature/enums.rs`) reads the genes and produces the trait
bit flags:

| Trait | Gene | Effect |
|---|---|---|
| `Ferocious` | gene 0 > 0.74 | +35% melee damage |
| `Industrious` | gene 1 > 0.74 | builds faster (shorter build cooldown) |
| `Hardy` | gene 2 > 0.74 | +40% health, hungers more slowly |
| `Fertile` | gene 3 > 0.74 | offspring cost 40% less energy, are born sooner |
| `KeenSenses` | gene 4 > 0.74 | sees and senses further |
| `Scavenger` | gene 5 > 0.74 | finds food further away |
| `Peaceful` | gene 6 < 0.18 | refuses to start fights, prefers trade |

A species converges when its members share traits; traits change as the genes
drift.

### 4.10 Diplomacy

Diplomacy is a sector of the simulation, not a scripted event. Each species
keeps a **relation** with every other species it has met, and every creature
negotiates those relations in person.

#### The relation

`creature/diplomacy.rs` holds one `Relation` per species pair:

| Field | Meaning |
|---|---|
| `state` | `War`, `Hostile`, `Neutral`, `Cordial`, `Allied`, `Vassal`, `Overlord` |
| `trust` | `-100` (hated) to `+100` (devoted) |
| `treaties` | the treaties currently in force |
| `wars` / `betrayals` | the history of the pair |
| `war_weariness` | rises while a war lasts; at 1.0 a peace is easy to sell |
| `tribute_paid` | how much tribute the overlord has collected |

#### The treaties

| Treaty | Effect |
|---|---|
| `Alliance` | mutual defence, shared vision (34 instead of 20 cells), faster healing |
| `TradePact` | trading yields 18 energy instead of 5 |
| `Ceasefire` | the first strike is suppressed; only a defensive blow breaks it |
| `NonAggression` | the species does not join its ally's wars |
| `Tributary` | the weaker species pays 40 energy to the overlord every 120 ticks |

#### How a negotiation works

1. A creature that wants to talk (`outputs::NEGOTIATE` high, or an envoy, or a
   merchant) looks for the nearest foreign creature through the spatial grid.
2. The two of them accumulate **rapport** at a rate that depends on the role
   (`negotiation_rate`): an envoy closes a deal in ~30 ticks, a worker needs
   ~150.
3. When the rapport is full, the negotiation is resolved
   (`resolve_negotiation`). The chance the other side accepts depends on the
   trust, the relative power of the two colonies, the reputation of the
   envoy, and whether the species has the `Peaceful` trait.
4. The outcome is signed into the relation: a treaty, a war, or nothing at all
   but a little more trust.

Envoys do not wait for strangers to walk past them: a creature that wants to
negotiate **travels to the nearest colony of another species** (see the
diplomatic steering in `creature/actions.rs`). An embassy is a real journey
across the map.

#### Where wars come from

A war is declared when the network's hostility output is high, or when the
trust between two species has fallen below `-45` and the creature is already
inclined that way. Every kill costs the two species a little trust
(`-2.5`), so a rivalry builds up out of the bodies it leaves behind.

A war overrides the `Peaceful` trait: once the banners are up, even a peaceful
species fights back.

#### The diplomatic inputs

The network reads 20 extra inputs (`224..244`) in `creature/senses.rs`: the
state of the relation with the nearest foreign species, the trust, the
distance, the number of alliances and wars the colony holds, the power
balance between the two colonies, and the creature's own diplomatic standing.

### 4.11 Life, personality and culture

A creature has three layers of inheritance, and they behave differently.

#### The genes: what the species is

The 32 genes decide the **traits**, decoded by `decode_traits`
(`creature/enums.rs`). A species can carry up to sixteen of them:

| Trait | Gene | Effect |
|---|---|---|
| `Ferocious` | gene 0 > 0.74 | +35% melee damage |
| `Industrious` | gene 1 > 0.74 | builds faster (shorter build cooldown) |
| `Hardy` | gene 2 > 0.74 | +40% health, hungers more slowly |
| `Fertile` | gene 3 > 0.74 | offspring cost 40% less energy, are born sooner |
| `KeenSenses` | gene 4 > 0.74 | sees and senses further |
| `Scavenger` | gene 5 > 0.74 | finds food further away |
| `Peaceful` | gene 6 < 0.18 | refuses to start fights, prefers trade |
| `Venomous` | gene 7 > 0.76 | melee attacks poison the wounded |
| `Amphibious` | gene 8 > 0.76 | swims without tiring |
| `Nocturnal` | gene 9 > 0.76 | sees much further at night |
| `Gregarious` | gene 10 > 0.76 | works faster among its own kind |
| `Hoarder` | gene 11 > 0.76 | carries 60% more |
| `Thermophilic` | gene 12 > 0.76 | shrugs off the heat, suffers in the cold |
| `Metallophilic` | gene 13 > 0.76 | reads metal and digs towards it |
| `HollowBoned` | gene 14 > 0.78 | moves 35% faster, has 25% less health |
| `Hivemind` | gene 15 > 0.80 | the colony shares one mind: it sees 60% further |

#### The heritage: what the species has become

`creature/heritage.rs` holds the second layer.

**The stage of life** decides what a creature can do:

| Stage | Age | Size | Speed | Health | Can fight | Can breed |
|---|---|---|---|---|---|---|
| `Juvenile` | < 18% | 55% | 72% | 55% | no | no |
| `Adult` | 18-78% | 100% | 100% | 100% | yes | yes |
| `Elder` | > 78% | 108% | 78% | 90% | yes | yes |

A juvenile is drawn small and wears a thin ring, an elder a faint golden
one, so the ages of a colony are visible at a glance.

**The affinity** is what a species has drifted into being adapted to. It is
read from gene 16 and inherited like any other trait:

| Affinity | Effect |
|---|---|
| `Ember` | immune to fire, but the cold bites twice as hard |
| `Tide` | swims freely, carries 15% more |
| `Crystal` | mana comes back twice as fast |
| `Iron` | mines 60% faster and reaches further for ore |
| `Timber` | builds 50% faster |
| `Stone` | takes 30% less damage, immune to fire and cold |

**The culture** is what the species has *learned to do*. Eight practices
are tracked: `Cultivation`, `Mining`, `Warfare`, `Commerce`, `Craft`,
`Ritual`, `Architecture` and `Hunting`. A creature picks one up by doing
it, the whole colony remembers it (`CreatureSystem::culture_pool`), and the
young inherit it — with a one-in-sixteen chance of innovating something
new. Each practice gives a real bonus: a mining colony digs for a third
less energy, a farming colony raises a farm even when it is not hungry, a
warring colony hits 25% harder.

#### The personality: what the individual is

`creature/personality.rs` holds the third layer: the temperament of one
animal, which is inherited from the parents and drifts a little each
generation. Five axes, each in `0..1`:

| Axis | What it changes |
|---|---|
| `aggression` | the distance at which it gives in, how hard it hits, how easily it starts a fight |
| `curiosity` | how far it will range to find what it needs |
| `sociability` | how strongly it is pulled back to the colony |
| `greed` | how much it hoards |
| `caution` | how quickly it runs, how far it ventures |

Two creatures with identical genes and an identical heritage still play
very differently, because one is bold and the other is wary.

#### Courtship

A creature that wants offspring looks for a mate instead of budding
(`creature/mating.rs`). The mate is chosen on **compatibility**: the
temperaments should be close enough to get along but different enough not
to be inbreeding, and the genes should be different enough to give the
child vigour. The child then inherits from *both* parents:

- the genes are blended and re-decoded, so a new trait can appear
- the temperament is the average of the two, with drift
- the size sits between the parents
- **hybrid vigour**: the further apart the parents are, the stronger the
  child. Two near-identical animals produce a weak one.

The elders teach the young: a juvenile that grows up near an elder
matures a little faster and picks up a practice it did not know.

### 4.12 Speciation: how one species becomes two

A species that has outgrown its patch of ground does not simply exist
forever. Part of the colony leaves, and if they get far enough they become
a species of their own (`creature/speciation.rs`).

| Rule | Value |
|---|---|
| A colony must have at least | 14 members to split |
| The old colony must keep | at least 8 members |
| The emigrants must get | 80 cells from the old colony |
| The emigrants must be gone | at least 300 ticks |
| A run may have at most | 6 splits |

The split is not cosmetic. The emigrants get a **new species id**, a **new
colour**, another 10% of genetic drift (which is where a new trait can
appear) and a **fresh diplomatic record** — they start with no relations
at all, and have to build them from nothing. One of them becomes the
queen of the new colony.

Without this a run can only ever lose species. With it, lineages split,
drift apart and then compete for the same ground.

### 4.13 Weather that matters

The weather is not only a picture on the screen. Each kind of weather
changes how the creatures live (`world/mod.rs`, `Weather`):

| Weather | Speed | Energy | Fear | Other |
|---|---|---|---|---|
| `Clear` | 100% | 100% | - | - |
| `Rain` | 90% | 130% | - | thirst builds a third slower |
| `Storm` | 72% | 210% | +8 | the colony shelters at its base |
| `Snow` | 84% | 170% | +2 | - |
| `Meteor` | 90% | 120% | +22 | the colony shelters at its base |

### 4.14 The buildings of a settlement

A colony can raise more than walls and turrets. Four buildings do real
work, and the ruins of older settlements are scattered around the world
for the first creatures to find:

| Building | What it does | Cost |
|---|---|---|
| `Market` | trade beside it is 60% more profitable | 10 wood, 4 metal, tech 2 |
| `Workshop` | tools are made 50% faster beside it | 8 wood, 6 metal, tech 2 |
| `Granary` | hunger builds much more slowly beside it | 12 wood, 4 stone, tech 1 |
| `Beacon` | the colony sees 50% further from it | 10 stone, 3 crystal, tech 3 |
| `Hearth` | the cold does not bite beside it | found in ruins |
| `Totem` | the symbol of a tribe | found in ruins |

A colony only builds a market and a workshop once it has the **culture**
for it: a society that has never traded does not think to build a
marketplace.

### 4.15 Plagues

A colony can be destroyed from the inside (`creature/plague.rs`). Diseases
are not labels: they change what a creature can do.

| Disease | Catches | Health/tick | Energy/tick | Speed | Special |
|---|---|---|---|---|---|
| `flu` | 1.0% | - | 0.10 | 70% | burns out on its own |
| `plague` | 1.4% | 0.30 | 0.22 | 80% | lethal |
| `poison` | - | 0.12 | 0.14 | 85% | comes from venom |
| `curse` | - | 0.08 | 0.12 | 90% | comes from a spell |
| `infection` | 0.4% | 0.10 | 0.10 | 80% | spreads through the flesh |
| `parasite` | 0.2% | - | 0.30 | 90% | drains the host |
| `madness` | 0.3% | - | 0.08 | 85% | drives the host to attack its own kind |

A disease spreads through a colony and, more slowly, between colonies
that trade with each other. Crowded colonies breed outbreaks on their
own: every few hundred ticks a colony that is not already suffering may
have one animal fall ill.

**Madness** is the cruel one: a mad creature attacks its own kind, so a
colony with madness in it tears itself apart.

### 4.16 Predation: the food web

A colony that has learned to **hunt** does not wait for its food to come
to it. A hungry hunter stalks the nearest animal of another species,
closes on it, and feeds on the kill (a hunter gets far more energy from
a kill than a fighter does, and the meal cuts its hunger by 45).

Hunters also pick their prey: a worker is easy meat and a soldier is a
fight, so a hunter goes for the worker. A species that has learned to
hunt is marked as a **PREDATOR** in the knowledge window.

### 4.17 The landmarks: the places worth living in

The world is not uniform. `world/landmarks.rs` scatters five to eight
special places across the surface, at least 180 cells apart.

| Landmark | What it is | What it does |
|---|---|---|
| `Crystal Grove` | spires of crystal | mana comes back twice as fast |
| `Hot Spring` | a warm pool | the cold does not bite there |
| `Bone Field` | old bones and corpses | prey comes within 48 cells instead of 34 |
| `Fertile Soil` | rich ground | hunger builds a third more slowly |
| `Old Ruin` | the remains of a settlement | crafting is 35% faster there |

A founding colony **settles beside a landmark** if there is one free, and
the first colony to arrive takes it. The second colony to want the same
crystal grove has to do without — or take it by force. The landmarks are
marked on the minimap, so the player can watch the map fill up and see
which species have claimed which piece of good ground.

---

## 5. Rendering

| File | Responsibility |
|---|---|
| `render/mod.rs` | the renderer: the sky, the terrain shading and the pixel loop |
| `render/fx.rs` | the per-element visual effect table (water waves, fire flicker, crystal sparkle…) |
| `render/light.rs` | the dynamic light field: what the emissive cells illuminate |

### 5.1 The light field

Emissive cells (lava, fire, crystals, mana…) radiate light. `LightField`
(`render/light.rs`) is a quarter-resolution grid of RGB light:

1. `LightField::add` seeds a light cell from an emissive world cell.
2. `LightField::blur` spreads it with a separable box blur. The window is
   clamped at the edges and divided by the number of samples actually taken,
   so the result does not depend on where the field starts.
3. `LightField::finalize` converts the raw light into the additive colour
   once per light cell.

The field is anchored to **world** coordinates, not to the camera window, so
panning and zooming never change how a cell is lit.

### 5.2 The pixel loop

`paint` (`render/mod.rs`) is a free function over plain slices, deliberately
not a method on the renderer: the compiler cannot prove that the pixel buffer
does not alias the renderer's own fields, so a method would reload them from
memory for every single pixel.

### 5.3 The UI

| File | Responsibility |
|---|---|
| `ui/hud.rs` | the heads-up display and the rolling ecology panel |
| `ui/knowledge.rs` | hover tooltips and the encyclopedia window |
| `ui/actors.rs` | the creatures, their projectiles, the weather and the colony campfires |
| `ui/effects.rs` | transient animations for hits, births, deaths and explosions |
| `ui/minimap.rs` | the minimap, the weather overlay and the vignette |
| `ui/victory.rs` | the victory and extinction screen |

---

## 6. Code legend

| File | Responsibility |
|---|---|
| `main.rs` | the game loop: input, camera, the fixed-step simulation, the UI |
| `rng.rs` | the deterministic random number generator |
| `elements/mod.rs` | the `Elem` enum and the lookup tables |
| `elements/properties.rs` | the per-element data: colour, behaviour, density, nutrition, toxicity, lifetime, description |
| `world/mod.rs` | the `World` struct, `Biome`, `Weather`, `Pressure`, `Census`, `Column` |
| `world/cell.rs` | cell access: `get`, `set_at`, `swap_cells`, movement rules |
| `world/terrain.rs` | the terrain shape, the biome layout, the cached ground level |
| `world/generation.rs` | world generation, cave carving, the lava chamber, the lakes |
| `world/elements_placement.rs` | the wide variety of elements scattered through the world |
| `world/elements_extreme.rs` | the rare and extreme element placements |
| `world/vaults.rs` | the rare vaults that hold the rarest materials |
| `world/structures.rs` | the generated ruins, towers, bridges and dungeons |
| `world/landmarks.rs` | the places worth living in |
| `world/physics.rs` | `World::step` and `World::update_cell` |
| `world/behaviors.rs` | the per-behaviour movement rules, and `World::step_cultivate` (farming) |
| `world/reactions.rs` | the chemistry and `World::explosion` |
| `world/maintain.rs` | the slow world upkeep: weather, regrowth, ore genesis, and `World::update_pressure` |
| `world/census.rs` | the population census of the world |
| `world/query.rs` | the queries the creatures use: find food, water, resources, and `World::find_farm_site` |
| `creature/mod.rs` | the `Creature`, `CreatureSystem`, the shared constants, the `energy_cost` table, `MAX_SPEED` and `MAX_FARMS` |
| `creature/enums.rs` | the roles, tools, diseases, beliefs, moods, spells, traits |
| `creature/brain.rs` | the neural network |
| `creature/names.rs` | the name and team-colour generator |
| `creature/senses.rs` | the spatial grid, the local scan and the sensory inputs |
| `creature/colony.rs` | spawning, the champion snapshot, the child blueprint |
| `creature/step.rs` | the per-creature tick: needs, senses, decisions |
| `creature/actions.rs` | the individual actions a creature can take: move, eat, mine, craft, fight |
| `creature/building.rs` | what a colony decides to build, and what it costs |
| `creature/combat.rs` | combat resolution: turrets, bombs, projectiles, melee, deaths |
| `creature/diplomacy.rs` | the relations between species: treaties, trust, wars, negotiations |
| `creature/personality.rs` | the temperament of an individual creature |
| `creature/heritage.rs` | the life stage, the elemental affinity and the culture of a species |
| `creature/mating.rs` | courtship, mate choice and the hybrid young |
| `creature/speciation.rs` | migration and the birth of a new species |
| `creature/plague.rs` | the diseases that run through a colony |
| `creature/stats.rs` | the population statistics |
| `render/mod.rs` | the renderer |
| `render/fx.rs` | the element effect table |
| `render/light.rs` | the dynamic light field |
| `ui/*.rs` | the user interface |
| `ui/diplomacy.rs` | the diplomacy panel |

### 6.1 Where the important rules live

A quick map from "what I want to change" to "the file and method to change".

| To change… | Look at |
|---|---|
| How an element behaves (falls, burns, dissolves) | `elements/properties.rs` (`properties()` table) and `world/behaviors.rs` |
| Which elements react with which | `world/reactions.rs` (`react`) |
| What an explosion destroys | `world/reactions.rs` (`World::explosion`) |
| How terrain is shaped | `world/terrain.rs` (`surface_y`, `biome_at`) |
| What the world looks like at the start | `world/generation.rs` (`World::seed`), `world/elements_placement.rs` |
| Weather and its cadence | `world/maintain.rs` (the weather block) |
| **How the world gets harder** | `world/maintain.rs` (`World::update_pressure`) and `world/mod.rs` (`Pressure`) |
| Plant regrowth, pond top-up, ore genesis | `world/maintain.rs` |
| How a creature decides what to do | `creature/brain.rs` (`Net::forward_into`) and `creature/step.rs` |
| What a creature can perceive | `creature/senses.rs` (`LocalScan`, the sensory input rows) |
| What a creature does when it acts | `creature/actions.rs` (`act_*`) |
| **What each action costs** | `creature/mod.rs` (`energy_cost`) and the `act_*` functions |
| **How fast a creature moves** | `creature/mod.rs` (`MAX_SPEED`) and `creature/step.rs` (the velocity clamp) |
| Who fights whom | `creature/combat.rs` (`resolve_attacks`, `resolve_projectiles`) |
| **Who is allied with whom** | `creature/diplomacy.rs` (`Diplomacy`, `sign_treaty`, `declare_war`) |
| **How a treaty is negotiated** | `creature/diplomacy.rs` (`resolve_negotiation`, `desired_action`, `advance_negotiation`) |
| **What a treaty does** | `creature/diplomacy.rs` (`tick_diplomacy`), `creature/combat.rs` (allies are never attacked), `creature/senses.rs` (shared vision) |
| **How a creature is born** | `creature/colony.rs` (`child_blueprint`), `creature/mating.rs` (`resolve_mating`) |
| **How a species specialises** | `creature/enums.rs` (`decode_traits`), `creature/heritage.rs` (`Affinity`, `Stage`) |
| **What a creature has learned** | `creature/heritage.rs` (`culture_bits`, `CreatureSystem::learn_culture`) |
| **How an individual behaves** | `creature/personality.rs` (`Personality`), `creature/actions.rs` (`act_steer`) |
| **How a disease spreads** | `creature/plague.rs` (`Plague`, `spread_diseases`) |
| **What a species hunts** | `creature/actions.rs` (`act_steer`, the hunt), `creature/combat.rs` (`resolve_attacks`) |
| **Where a colony settles** | `creature/colony.rs` (`find_landmark_spot`), `world/landmarks.rs` |
| **Where a new creature is born** | `creature/colony.rs` (`spawn`, `spawn_near_surface`, `populate`) |
| How traits are inherited | `creature/colony.rs` (`child_blueprint`), `creature/enums.rs` (`decode_traits`) |
| **How farming works** | `world/behaviors.rs` (`step_cultivate`) and `world/query.rs` (`find_farm_site`) |
| Where a farm may be built | `world/query.rs` (`World::find_farm_site`) |
| Which element is a crop | `elements/properties.rs` (`Elem::Crop`) |
| How light is computed | `render/light.rs` (`LightField`) |
| How a cell is drawn | `render/mod.rs` (`paint`), `render/fx.rs` (`fx_table`) |
| What the HUD shows | `ui/hud.rs` |

---

## 7. Checking the simulation

| Command | What it does |
|---|---|
| `cargo run --release --bin bench -- 6000` | runs a headless simulation and reports the element census |
| `cargo run --release --bin prof` | times the world step, the creature step and the render |
| `cargo run --release --bin spawncheck -- 3000` | asserts that no underground material ever appears above the terrain |
| `cargo run --release --bin skycheck` | reports what is sitting above the terrain |
| `cargo run --release --bin docgen` | regenerates the element table in this document |
| `cargo run --release --bin diplcheck -- 12000` | runs the diplomacy sector headless and asserts the invariants |
| `cargo run --release --bin lifecheck -- 9000` | runs the life and evolution sector headless and asserts the invariants |
