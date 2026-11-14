# PRIMORDIA - working rules

## Documentation
- `docs/SIMULATION.md` - the full simulation reference.
- `docs/BRAIN.md` - the complete reference for the creature brain: all
  181 inputs, all 33 outputs, the architecture, and exactly what is
  random, what is evolved and what is hard-coded. Read this before
  changing anything in `src/creature/brain.rs` or `src/creature/senses.rs`.

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
- `cargo run --release --bin zorgcheck -- [seed]` checks the ZORG cave:
  that it exists, that it is deep in the world, that it has a floor and a
  larder, and that every founding ZORG is born inside it.
- `cargo run --release --bin braincheck -- 12000` checks the minds: that
  the senses are primitive, that a mind learns to want what it smells,
  that the colony feeds itself, and that the colony does not go extinct.
  It steps the **world** as well as the creatures: a check that only moved
  the animals would measure a world that never changes.
- `cargo run --release --bin aircheck -- 3000` checks the air: that the
  undergroves are deep and have air in them, that the open air is fresher
  than the deep, that fire in a sealed cave goes out, and that stale air
  costs a creature something.
- `cargo run --release --bin prof` prints the per-step cost of the world,
  the creatures and the renderer. The renderer is measured on the region
  that is actually on screen, because that is the number that has to fit
  inside a frame.
- `PRIMORDIA_SNAPSHOT=shot.png PRIMORDIA_SNAPSHOT_AFTER=60 primordia`
  makes the game write a picture of itself and exit. Add
  `PRIMORDIA_PANEL=diplomacy` to open the diplomacy panel in it, and
  `PRIMORDIA_ZOOM=3.0` to start closer to the ground.
  `PRIMORDIA_CAM=x,y` opens the run somewhere else, and
  `PRIMORDIA_CAM=zorg` opens it on the ZORG cave in the deep.

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
  - `src/elements/`  mod.rs (enum + lookup tables), properties.rs, percept.rs
  - `src/world/`   cell, terrain, generation, element placement, vaults,
                   structures, landmarks, physics, behaviors, reactions,
                   maintain, census, query, zorg_cave
  - `src/creature/` brain, enums, names, senses, colony, step, actions,
                   building, bonds, combat, diplomacy, personality,
                   heritage, mating, speciation, plague, predation,
                   succession, spells, technology, tech, stats, zorg,
                   sanctify
  - `src/ui/`      effects, hud, knowledge, minimap, victory, actors,
                   atmosphere, diplomacy, tech
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
- The pixel loop slices the world arrays **row by row** and indexes them
  with the column, which is already known to be in range. That lets the
  compiler drop the bounds check on every pixel.
- The weather's veil over the sky is folded into the per-column sky when
  the columns are worked out, not applied to every sky pixel.
- The per-element effects are a fixed-size table (`[FxEntry; 256]`), so
  the pixel loop can index it with the element byte itself.
- The weights of a brain are stored **input-major**: the weights of one
  input sit next to each other. The forward pass then walks the inputs
  and adds each one's contribution to every output, which the compiler
  can vectorise. The other way round is about three times slower.
- The layers are worked out in two passes: the dot products first, then
  the activation. Mixing them puts a branch in the middle of the hot
  loop and costs a third of the time.
- The **bloom** is seeded from the emissive cells while the region is
  scanned for its light sources, blurred at a quarter of the picture's
  size and laid over the picture in the pixel loop itself. A separate
  pass over the whole picture would cost twice as much.

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
- Allies share what they know: a discovery one species works out is passed
  to its partner. At war the other side comes and breaks your buildings, so
  a raid wears a camp down over time rather than deleting it.
- A creature has three layers of inheritance: **genes** (the species
  traits, decoded by `decode_traits`), **heritage** (the life stage, the
  elemental affinity and the culture the species has learned) and
  **personality** (the individual's own temperament). A child of two
  parents blends all three.
- The **brain** is a small network with a memory. The senses and the
  memory feed the first layer together, and the memory is rebuilt from
  the last hidden layer every time the creature thinks, so a brain can act
  on what it saw a while ago and not only on what is in front of it right
  now. The layers are `[senses + memory, 56, 40, 24, decisions]`, and the
  mutation rate is scaled down to match the extra weights, so a young
  animal is still its mother's breed with a little of its own.
- A brain is **not** a fixed shape. It has three tiers, and a species
  that works out **Cognition** grows every mind it has to the next tier
  (bigger hidden layers, the old weights carried over, so nothing that
  was learnt is lost). **Grand Cognition** grows them again.
- A brain also has a **function**: the hidden layers can fire with a
  threshold, a smooth squasher, a leaky gate, a sine, a switch or a
  smooth gate. The function is chosen at random, inherited, and breeds
  like anything else, so two species with the same senses think
  differently.
- A brain **learns inside a life**. When an animal eats, builds, fights
  or trades, the decision that worked is strengthened a little by
  back-propagation. The pull is slow, and the weights are settled now and
  then so a life of reinforcement cannot turn a few weights into everything.
- The **shape** of a mind is bred as well: a young animal is born now and
  then with one neuron more or less than its mother had.
- The **senses are primitive**. A creature is told what things look
  like (colour, light, warmth, texture, wetness, gloss), what they smell
  like (sweet, rotten, acid, mineral, smoke) and how loud they are - and
  nothing else. There is no "there is food that way" input any more: the
  creature has to work out for itself that the green soft sweet-smelling
  thing is worth eating. The eight **desires** (one per direction) are
  how it says what it wants to do about what it perceives, and the
  actions are **local**: it bites the cell next to it, not the best thing
  within six cells.
- The creature senses **other creatures** the same way: a sight channel
  for "something alive is over there" and a smell channel for "my own kind
  is over there", which carries seventy cells. The kin block also says
  which way the nearest of its own kind and the nearest stranger are, so
  the network can join a colour up with a bearing and learn that the red
  ones over there are the ones to run from.
- The **lessons are about where, not only what**. A kill reinforces the
  attack and the direction the prey was in; being hurt reinforces
  fleeing and penalises the direction the blow came from. The combat code
  remembers both directions, so the credit goes to the thing that
  actually happened.
- The **instincts are weights, not code**. A newborn mind is born with a
  direct path from the raw senses to the desires: the sweet smell pulls,
  the acrid smell pushes away, and the open sky pulls upward. Evolution
  is free to strengthen, weaken or reverse that bias, and it does.
- The **surface regrows**. Vegetation is seeded on the ground beside the
  surface and a plant puts out seedlings of its own, so a colony that
  grazes its neighbourhood bare finds the green coming back - and a
  creature that has learnt to walk towards a sweet smell is walking
  towards something that is there again. A hard cap keeps the world from
  turning into one green mat, and the regrowth slows as the run wears on:
  the late world is a lean world, and a surface that can only feed one
  colony is what ends a run.
- **Idling is punished**. A crowd burns energy, so a heap of animals on
  one spot costs more than the same animals spread over the ground. A
  colony's fire has to be fed with **wood** - and wood is trees, not
  grass - and a colony with no firewood goes cold, and the cold comes
  in. A colony that does nothing but sit by its own fire goes hungry.
- **A heap breeds sickness.** Ten creatures packed onto the same spot is
  how a plague starts: the risk is `0.00008 x (the heap size - 9)` per
  creature per tick. The victim catches the **plague**, which catches
  easily and kills, and the ground under the heap turns to **virus**,
  which spreads through the organic matter around it. This is the oldest
  way for a colony to die, and it is why the herds are worth spreading
  out over.
- **No magnet**. The pull of the hunt only reaches as far as a hunt does
  (fourteen cells) and it is a gentle one: a hunter that could smell
  prey from across the valley would bring the whole colony with it, and
  the colony would pile onto one animal in open ground.
- The **tech tree** (`src/creature/tech.rs`) has **ten tiers** and holds
  both kinds of thing: the **tools** a creature carries (the pickaxe, the
  bow, the sunfire) and the **discoveries** a species knows (how to work
  steel, how to read the stars). Every entry has a real effect, and the
  tiers build on each other: nothing in a tier can be worked out or made
  before the tier below it is finished. A colony can only make what it
  knows how to make - the sword comes with smithing, the bow with ranged
  craft, the sunfire with sanctification.
- The tech tree is on the screen: **T** opens the whole tree (it scrolls
  with the wheel), and a small panel on the side shows the tier the world
  has reached, how much of the tree is known, and the latest and the
  greatest discovery.
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
- The world has **thirteen** kinds of landmark, and each one does something
  different to the colony that settles beside it: mana, warmth, hunting,
  food, craft, and - the newer ones - **sickness** (a fungal warren
  breeds disease), **purity** (a salt flat burns the filth of the ZORGs
  out of the air), **lore** (a drowned temple makes the colony work
  things out faster), **stone** (an amber field raises the buildings
  faster), **mind** (a whispering stone makes the animals learn faster
  inside their own lives), **metal** (an iron womb gives its metal up
  twice as freely), **charge** (a stormspire fills the air with
  lightning, and the colony is quick and strong there) and **air** (an
  undergrove is the only place in the deep with fresh air in it). The
  kinds are drawn at random for every world, so no two runs have the
  same map.
- The world has **air**. The plants of the surface breathe out oxygen in
  the daylight, and the oxygen floats up and hangs about, so the open air
  is fresh. The deep is a stale, breathless place, and a creature in stale
  air tires faster and recovers its wind more slowly. **Fire needs air
  too**: a flame in the open burns for a long time, and a flame in a
  sealed cave eats its own oxygen and goes out. The **undergroves** are
  the exception: they are chambers of the deep where pale fungus has
  been breathing without any sun, and they are the only fresh air under
  the ground. They are worth digging for and worth fighting over.
- The world has **eight** climates: tundra, ashland (the ground the
  volcano has burned over), forest, bloomwood (a wood in bloom),
  grassland, swamp, mire (a fungal mire, wet ground and spore air) and
  desert. The climate sets the tint, the
  temperature and what the ground is made of.
- The weather has **eight** kinds as well. A fog is the one that matters
  most: it cuts the sight of every creature almost in half, so the
  colonies stop noticing what is coming until it is on top of them. A
  heatwave beats the temperature up and the energy down. A sandstorm
  carries the desert across the world: it closes the view in, drinks the
  shallow water and leaves dust in the air.
- The world fights back. There is **one volcano** in the world: it sleeps,
  grumbles and erupts, throwing lava, fire and ash over the map and melting
  the ground around it. Creatures steer away from the crater and an
  eruption burns anything within thirty cells of it.
- The ground moves: **earthquakes** shake dust out of cave roofs and bring
  unsupported rock down, so the shape of the caves changes under the
  colonies.
- The seasons change the world: spring grows plants fast, summer dries the
  shallow water, autumn is lean and winter freezes the surface.
- Every colony holds the ground around its home, and the claim grows with
  the buildings it has. A soldier of another species that walks into the
  claim is the first thing the colony throws itself at, and the owner of
  the ground remembers the insult. The border is drawn on the map as a
  dashed ring in the colour of the species that holds it.
- A war that goes on long enough ends in a tired truce: the weariness of a
  war climbs with its length, and when neither side can afford it any more
  they lay down their arms and sign a ceasefire.
- The prey remembers the hunter. When a species loses animals to another
  species, the survivors keep a fear of that hunter; it belongs to the
  species, not to the individual. A frightened animal runs from the hunter
  much earlier, and a species that has been hunted hard enough learns to
  hunt itself.
- A colony is organised around its queen. When she dies the colony falls
  into a crisis: it holds no ground, it builds slowly and its animals lose
  heart. The crisis ends when a new queen takes the throne, and a colony
  in crisis raises queens far more readily than a colony that has one.
- The **ZORGs** are the deep's own ecology, not a player team. They have a
  cave of their own in the deepest rock of the world (`src/world/zorg_cave.rs`),
  and every founding ZORG is born inside it. They eat rock and poison, they
  never thirst, they never ally, and they can never win: if they are the
  last thing standing the run is lost.
- The ZORGs are a **dirty** faction. Every one of them carries a sickness
  from the deep and passes it to anything that stands close, they leave
  virus, bacteria and poison in the cells they walk through, and the
  chlorine they leave behind rises up through the rock towards the
  surface colonies. The deep is never empty: while its cave is there the
  ZORGs breed back up out of it, so a player can thin them out but never
  empty the deep for good.
- The ZORGs therefore **cannot be out-fought**, and a run cannot end
  against them by killing them all. The answer is **sanctification**
  (`src/creature/sanctify.rs`), a discovery that only a player species
  can work out (the ZORGs never learn anything):
  - the **Sunfire** is a brand of sanctified fire: it does 2.6x damage
    against a ZORG and 0.45x against anything else, so it is a weapon
    made for the deep and a poor weapon against a rival. It costs the
    creature whatever tool it was carrying.
  - the **Ward** is a pillar of the same fire. It burns the ZORGs that
    come near it and scours the corruption out of the ground. A ward
    raised more than `WARD_DEPTH` below the surface seals the cave: the
    deep stops breeding, and the run can be won. The victory condition
    is one player species left **and** the deep closed (or every ZORG
    dead). The HUD shows how many ZORGs are left and whether the cave is
    sealed.

