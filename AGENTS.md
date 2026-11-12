# PRIMORDIA - working rules

## Build
- Always compile with the native CPU target: `-C target-cpu=native`.
  This is already configured in `.cargo/config.toml`, so a plain
  `cargo build --release` picks it up.
  For a portable build use
  `cargo build --release --config 'build.rustflags=[]'`.
- Release builds are the ones to benchmark (`cargo build --release`).
- Headless checks: `cargo run --release --bin bench -- 6000`.
- `cargo run --release --bin spawncheck -- 3000` asserts that no
  underground-only material ever appears above the terrain.
- `cargo run --release --bin diplcheck -- 12000` runs the diplomacy
  sector headless and asserts its invariants (no allied-and-at-war pair,
  trust in range, no relation with an extinct species).
- `cargo run --release --bin lifecheck -- 9000` runs the life and
  evolution sector headless and asserts its invariants (the stages are
  reached, the temperament stays in range, a species specialises and
  learns, and young are born to two parents).
- `cargo run --release --bin prof` prints the per-step cost of the world,
  the creatures and (with `PRIMORDIA_RENDER_TIMING=1`) the renderer.
- `PRIMORDIA_SNAPSHOT=shot.png PRIMORDIA_SNAPSHOT_AFTER=60 primordia`
  makes the game write a picture of itself and exit. Add
  `PRIMORDIA_PANEL=diplomacy` to open the diplomacy panel in it.

## Code size rules
- **No file may exceed ~1000 lines.** A file may run slightly over that
  (1100 is fine) when a split would hurt readability, but 2000+ lines is
  never acceptable and 20k-40k line files are a hard no.
- Split by responsibility into folders, one folder per top-level module:
  `src/creature/`, `src/world/`, `src/ui/`.
- Each module folder has a `mod.rs` that declares the submodules and
  re-exports the public API, so `use creature::Foo` keeps working.
- Put the shared data types in `mod.rs` of their folder and the behaviour
  in the sibling files; private fields stay reachable because every
  submodule is a child of the module that defines the struct.
- Current layout:
  - `src/elements/`  mod.rs (enum + lookup tables), properties.rs
  - `src/world/`   cell, terrain, generation, element placement, vaults,
                   structures, landmarks, physics, behaviors, reactions,
                   maintain, census, query
  - `src/creature/` brain, enums, names, senses, colony, step, actions,
                   building, bonds, combat, diplomacy, personality,
                   heritage, mating, speciation, plague, technology, stats
  - `src/ui/`      effects, hud, knowledge, minimap, victory, actors,
                   diplomacy
  - `src/render/`  mod.rs (pixel loop), fx.rs (element effects),
                   light.rs (dynamic light field)

## Performance
- The hot paths are `World::step`, `CreatureSystem::step` and
  `render::Renderer::render_region`. Keep allocations out of them (reuse
  buffers).
- Element properties are read through lookup tables
  (`Elem::solid_table()`, `Elem::inert_table()`, ...), never by matching on
  the enum in a per-cell loop. The renderer follows the same rule through
  `render::fx::fx_table()`.
- The renderer's pixel loop lives in a free function that takes plain
  slices. Keeping it out of a `&mut self` method matters: the compiler
  cannot prove the pixel buffer does not alias the renderer's own fields,
  so every pixel would reload them from memory.
- Neighbour queries go through the spatial grid, never a full scan.

## Simulation rules
- Never compare raw element bytes against numeric ranges; use the decoded
  `Elem` enum (`Elem::from_u8`).
- The world must stay stable: creatures may not strip-mine it, terrain must
  not be permanently destroyed, and resources belong underground.
- **Depth is always measured from the terrain, never from an absolute
  row.** `self.surface_y(x)` is the reference. Absolute heights (`h/2`,
  `h/3`) sit *above* the surface, so they put materials in the sky.
- Lakes and pools are *carved into* the ground: the basin is dug out and
  the water fills it. Placing water above the ground leaves it floating.
- A run ends when a single species is left (victory) or the population dies
  out (extinction). The winning genome seeds the next revival.
- Diplomacy lives in `src/creature/diplomacy.rs`. Every species pair that has
  met has one `Relation` with a state, a trust and a set of treaties. Allies
  are never attacked, a war overrides the `Peaceful` trait, and a treaty
  breaks the alliance it was signed with.
- A creature has three layers of inheritance: **genes** (the species
  traits, decoded by `decode_traits`), **heritage** (the life stage, the
  elemental affinity and the culture the species has learned) and
  **personality** (the individual's own temperament). A child of two
  parents blends all three.
- The life history of a species is its r/K strategy: `Prolific` raises
  three small cheap young and dies young, `Prolonged` raises one
  well-invested-in young and lives long. It changes the litter size, the
  cost of a young, its value, the lifespan and the age at which a
  creature grows up.
- A species can split: a colony that has outgrown its ground sends some
  of its members away, and if they travel far enough they become a
  species of their own with a new id, a new colour and a fresh
  diplomatic record. The run may have at most six splits.
- A colony can be destroyed from the inside: diseases spread through it,
  slow the sick, drain them and — in the case of madness — turn them
  against their own kind.
- A species accumulates **discoveries** as well as practices: a discovery
  is a piece of knowledge that needs both the right practice and enough
  generations behind it. The knowledge belongs to the species and is lost
  with it.
- The world is not uniform: crystal groves, hot springs, bone fields,
  fertile soil and old ruins are worth settling beside, and a colony that
  can have one takes it. Two colonies that want the same grove fight over
  it.
- The world fights back. There is **one volcano** in the world: it sleeps,
  grumbles and erupts, throwing lava, fire and ash over the map and melting
  the ground around it. Creatures steer away from the crater and an
  eruption burns anything within thirty cells of it.
- The ground moves: **earthquakes** shake dust out of cave roofs and bring
  unsupported rock down, so the shape of the caves changes under the
  colonies.

