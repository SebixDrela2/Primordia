# PRIMORDIA - working rules

## Build
- Always compile with the native CPU target: `-C target-cpu=native`.
  This is already configured in `.cargo/config.toml`, so a plain
  `cargo build --release` picks it up.
  For a portable build use
  `cargo build --release --config 'build.rustflags=[]'`.
- Release builds are the ones to benchmark (`cargo build --release`).
- Headless checks: `cargo run --release --bin bench -- 6000`.

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
  - `src/world/`   cell, terrain, generation, element placement, vaults,
                   structures, physics, behaviors, reactions, maintain,
                   census, query
  - `src/creature/` brain, enums, names, senses, colony, step, actions,
                   combat, stats
  - `src/ui/`      effects, hud, knowledge, minimap, victory, actors

## Performance
- The hot paths are `World::step`, `CreatureSystem::step` and
  `render::render_region`. Keep allocations out of them (reuse buffers).
- Element properties are read through lookup tables
  (`Elem::solid_table()`, `Elem::inert_table()`, ...), never by matching on
  the enum in a per-cell loop.
- Neighbour queries go through the spatial grid, never a full scan.

## Simulation rules
- Never compare raw element bytes against numeric ranges; use the decoded
  `Elem` enum (`Elem::from_u8`).
- The world must stay stable: creatures may not strip-mine it, terrain must
  not be permanently destroyed, and resources belong underground.
- A run ends when a single species is left (victory) or the population dies
  out (extinction). The winning genome seeds the next revival.
