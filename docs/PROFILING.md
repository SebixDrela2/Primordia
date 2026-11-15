# Profiling PRIMORDIA

How to find out which functions actually cost time, **without putting an
attribute on every single method**.

The short version:

| # | Tool | Attributes needed? | Answers |
|---|------|--------------------|---------|
| 1 | **`cargo flamegraph`** | **none** | "Where is the CPU going?" — a picture of the hot call stacks |
| 2 | **`tracing` / `hotpath`** | a few, at *phase* boundaries | "Which function is slowest?" — a sortable table, dotnet-trace style |
| 3 | `samply` | none | Same as #1, interactive (Firefox Profiler UI) |
| 4 | `puffin` | optional, runtime toggle | Real-time overlay while you play |

**Pick #1 first.** It needs zero code changes, it works on the binary you
already ship, and it will tell you *which module* is slow. Only reach for
#2 once #1 has told you *where* to look and you want a per-function
breakdown inside that one module.

Everything below has been **run and verified on this machine** (Windows,
MSVC, Administrator PowerShell).

---

## The one command

Run this from an **Administrator** PowerShell:

```powershell
cd D:\AI\ParticleSim\native; $env:PRIMORDIA_SNAPSHOT="$env:TEMP\opencode\fg.png"; $env:PRIMORDIA_SNAPSHOT_AFTER="30"; cargo flamegraph --profile profiling --bin primordia --deterministic --notes "primordia: real game loop, 30s" -o "$env:TEMP\opencode\flamegraph.svg"
```

Then turn the SVG into a table:

```powershell
python docs\flamegraph_summary.py "$env:TEMP\opencode\flamegraph.svg"
```

### Why each piece

- **Administrator** — on Windows the backend is *blondie*, which suspends
  threads to sample them. In a normal shell it fails with
  `Error: could not find dtrace and could not profile using blondie: NotAnAdmin`.
- **`--profile profiling`** — the release profile has no debug symbols, so
  the graph would be a wall of raw addresses. `Cargo.toml` carries a
  `profiling` profile for exactly this:

  ```toml
  [profile.profiling]
  inherits = "release"
  debug = 2
  ```

  `debug = 2` (full) rather than `line-tables-only`, because release uses
  `lto = "thin"` + `codegen-units = 1` and inlines hard — the inline
  frames only show up with full DWARF.
- **`PRIMORDIA_SNAPSHOT_AFTER=30`** — makes the game exit on its own after
  30 s. **Do not press Ctrl-C**: on Windows a manual stop discards the
  recording and writes no SVG.
- **`--deterministic`** — same colour per function across runs, so two
  graphs can be diffed.

### Simulation only (no renderer)

```powershell
cargo flamegraph --profile profiling --bin primordia -- bench 6000 -o "$env:TEMP\opencode\flamegraph-sim.svg"
```

---

## Reading the result

`cargo flamegraph` writes an SVG with one frame per unique stack: the
box width is the time, the title carries the sample count and percentage.
`docs/flamegraph_summary.py` reads that back and rebuilds the call tree,
then prints:

* **SELF time** — samples inside the function itself.
* **INCLUSIVE time** — the function plus everything it called.
* **the hottest path** from the root.
* **the hottest child of each hot function** — i.e. where inside the
  function the time goes.

It needs nothing but the SVG and Python.

### Comparing two profiles

```powershell
python docs\flamegraph_summary.py --compare before.svg after.svg
```

This prints the biggest movers in self time and the totals per area,
before and after. Keep the SVG of the run you want to compare against:

```powershell
Copy-Item "$env:TEMP\opencode\flamegraph.svg" "$env:TEMP\opencode\flamegraph-before.svg"
```

**Two profiles of two different worlds cannot be compared by their
totals.** A random seed gives a different colony every time: a heavier
colony spends more of the frame on the simulation, runs fewer frames
per second, and so does less rendering per second. The totals then
move for a reason that has nothing to do with the code.

What *can* be compared is the share a function takes of **its own
subsystem**, because that work is the same every frame:

| function | of | before | after |
|---|---|---|---|
| `take::next` (the no-op `take` adapters) | the brain | 14.3 % | **0.0 %** |
| `cmp::lt` (the `i < end` of a range) | the brain | 29.1 % | **7.2 %** |
| `unchecked_add` | the brain | 15.7 % | **0.3 %** |
| `clamp_u8` | the pixel loop | 24.5 % | **11.2 %** |
| `finish_cell` | the pixel loop | 19.9 % | **15.9 %** |
| `sky_cloud` | the sky | 43.5 % | **30.2 %** |
| `smoothstep` | the sky | 11.0 % | **7.8 %** |

A share can rise even when the function got cheaper — `sample_cloud`
went from 31.8 % to 48.0 % of the sky's time because the cloud's
*amount* moved out of the per-pixel path, so the sampling is left with
a bigger share of what is left. That is the price of a cheaper
neighbour, not a cost.

For the end-to-end numbers use `primordia prof`, which has a fixed
seed and measures the three hot paths on their own:

```
pop 224 | world 0.590 ms | creature 1.940 ms | render region 1.536 ms | render world 11.478 ms
```

**So: always profile with `PRIMORDIA_SEED` set.** Two runs with the
same seed are of the same world, and then the totals mean something.

---

## Two things to know about reading a flame graph

**1. The self times are solid; the fine nesting is not.**
The profiler reads the stack of every sample, and Rust inlines so
heavily (`lto = "thin"`, `codegen-units = 1`) that a frame can be the
*inlined* copy of a function rather than a real call. The tool then
draws the inline chain as if it were a call chain, so a hot function can
appear nested under something that never calls it. The **self time** of
each frame is still exact — it is the time spent in that code — so rank
by self time, and treat the tree as a hint rather than a fact.

Concretely: in the first profile, `World::mark_active` was shown as
"10 % of the frame, nearly all of it `Vec::index`". Its own self time
was 0.33 %. The rest was the world step's own indexing, attributed to a
frame that was really the inlined body of `World::step`.

**2. The picture is a wall clock.**
`Clock.time` is the real time since the program started, so the sky, the
sun, the aurora and the weather's shimmer all move with the wall clock.
Two runs of the same binary therefore never paint the same picture, and
two PNGs of the same scene cannot be compared. To compare a renderer
change, render both versions **in the same process** on the same world —
that is what a unit test is for.

---

## Reproducible runs

`PRIMORDIA_SEED` fixes the seed of the world:

```powershell
$env:PRIMORDIA_SEED = "12345678901234"
```

With it, the same world is generated every time, which makes a picture
comparable and a profile repeatable. Without it the seed comes from the
clock.

---

## What we already had

`primordia prof` (`src/checks/prof.rs`) is the coarse split:

```
pop 224 | world 0.600 ms | creature 2.380 ms | render region 1.946 ms | render world 15.620 ms
```

That ranks the three hot paths but cannot say *which function* inside
them is responsible. The flame graph closes that gap.

---

## Measured: where the frame actually goes

From a 30 s run of the **real game** (not the bench harness), 239 835
samples on the main thread. The three hot paths are the whole frame:

| Hot path | Share of the main thread |
|----------|---------------------------|
| `CreatureSystem::step` | **47.4 %** |
| `Renderer::render_region` | **30.3 %** |
| `World::step` | **14.1 %** |

### Self time, by area

Self time is the honest measure (see the caveat above).

| Area | Self time |
|------|-----------|
| `core` / `std` (bounds checks, comparisons, iterators) | 23.4 % |
| render `paint` + `bloom` | 18.5 % |
| brain (the neural net) | 12.3 % |
| creature (everything else) | 11.7 % |
| native / GPU driver | 10.1 % |
| world | 8.1 % |
| render (everything else) | 7.0 % |
| `alloc` | 4.6 % |
| macroquad | 1.5 % |
| rng | 1.4 % |

### The hottest single functions, by self time

| Function | Self |
|----------|------|
| `brain::Net::apply_target` | 6.1 % |
| `brain::Net::forward_into` | 5.5 % |
| `render::paint::paint` (the pixel loop) | 5.2 % |
| `core::cmp::lt` (the `i < end` of a range) | 3.5 % |
| `render::paint::sky_cloud` | 2.5 % |
| `Vec::index` (bounds checks) | 2.5 % |
| `f32::clamp` | 2.4 % |
| `render::clamp_u8` | 2.4 % |
| `render::bloom::blur_box` | 2.3 % |
| `senses::scan_ray` | 2.3 % |

Inside them:

* **`Net::apply_target`** — 4.2 % of the frame was the `Range` iterator
  and another 1.6 % the `take`/`enumerate` adapter, all of it wrapping a
  loop that only needed an index.
* **`Net::forward_into`** — the trace of a thought was built by cloning
  every layer's activations, and the clone was thrown away again.
* **`paint`** — `clamp_u8` did a float clamp on every channel of every
  pixel, and the cloud was worked out per pixel when the grid is 30 x 13.
* **`scan_ray`** — the ray walked 42 cells to sample 25 of them, and
  tested each channel of the scent separately.

---

## What was done about it, and what it bought

Five changes, all of them behaviour-preserving. Measured with
`primordia prof`, old binary and new binary back to back on the same
machine:

| | before | after | |
|---|---|---|---|
| `World::step` | 0.590 ms | 0.589 ms | unchanged |
| `CreatureSystem::step` | 2.078 ms | **1.940 ms** | **-6.6 %** |
| `Renderer::render_region` | 1.575 ms | **1.536 ms** | **-2.5 %** |
| simulation + region | 4.653 ms | **4.476 ms** | **-3.8 %** |

Three runs of each, back to back on the same machine. An earlier
measurement of the day gave -7.0 % and -3.6 %; the numbers move about
half a point with the machine.

What each one was:

1. **`World::mark_active`** — the "is this cell already on the list"
   flag was a `Vec<bool>`, which is a bitfield: every read was a shift
   and a mask. It is a byte now, and the element byte is passed in
   rather than read a second time.
2. **`Net::apply_target`** — the back-propagation walked its deltas
   through `enumerate().take(n)` where `n` was already the length of
   the vector, and it allocated two vectors on every call and another
   one for every layer. It now uses indexed loops over slices, so the
   bounds checks go, and the two delta buffers are kept on the mind and
   reused.
3. **`Net::forward_into` / `credit_desire`** — every thought built a
   fresh trace by cloning the activations of every layer, and every
   lesson cloned the trace again. The traces are now a ring of four
   that are written into over and over, and a lesson reads the trace
   where it lies instead of copying it.
4. **`render::paint`** — `clamp_u8` clamped in floating point before a
   cast that saturates anyway; the cloud's colour and coverage are the
   same for every pixel of the picture and are worked out once; the
   amount of cloud is worked out when the cloud changes rather than on
   every pixel; the position of the row in the cloud grid is worked out
   once per row.
5. **`senses` and the spatial grid** — the ray now jumps over the cells
   it was going to skip instead of testing each one; a creature that is
   out of reach is turned away before the square root is worked out;
   the buckets of the grid are taken a row at a time.

### How it was checked that nothing changed

* The renderer was proved **pixel-identical**: the old pixel loop was
  kept as a second painter and both were run in the same process on the
  same world, over two world sizes, two windows, three times of day and
  all eight weathers — 96 pictures, byte for byte the same.
* Every headless check that has a fixed seed prints **byte-identical
  output** before and after: `braincheck`, `lifecheck`, `spawncheck`,
  `zorgcheck`, `aircheck`, `diplcheck`, `revivalcheck`, `crowdcheck`
  and `foodcheck`.
* `cargo clippy --all-targets --all-features` is clean, and no `allow`
  was added anywhere.

## The other tools

### 2. `tracing` + `PrintTreeLayer` — the dotnet-trace table

The hierarchical tree where every span shows total and self time.

**The important part: do NOT put an attribute on every method.**
`#[instrument]` **cannot** be applied to a module or a crate — it is an
open upstream request ([tokio-rs/tracing#1638]). It only works on an
individual `fn`, so "instrument everything" means hundreds of attributes.
Do not do that.

Instead, instrument **the phase boundaries** — which is where the useful
information is anyway. This codebase was refactored into exactly those
phases, so the surface is small and the payoff is high:

| File | Attributes | Functions |
|------|-----------|-----------|
| `src/creature/step.rs` | 8 | `step`, `advance_clock`, `assign_squads`, `build_grid`, `run_creatures`, `close_tick`, `tend_the_fires` |
| `src/creature/step_mind.rs` | 12 | `think`, `talks`, `steer`, `forage`, `fight`, `mount_and_trade`, `learn`, `dig`, `breed`, `weapons`, `spells_and_plague`, `finish` |
| `src/world/physics.rs` | 9 | `step`, `step_fall`, `step_flow`, `step_rise`, `step_diffuse`, `step_burn`, `step_grow`, `step_cultivate`, `step_energy`, `decay` |
| `src/render/mod.rs` | 8 | `render_region`, `scan_lights`, `copy_light_rows`, `build_cloud_row`, `build_aurora`, `build_cloud_shadows`, `build_vignette`, `build_sky`, `build_sun_shafts` |
| `src/world/maintain.rs` | 15 | the maintenance dispatcher |

**~50 attributes, and not one on a leaf helper.** You get the full
breakdown of all three hot paths.

`skip_all` is not optional: `World` and `CreatureSystem` do not implement
`Debug`, and the default `#[instrument]` records every argument with
`fmt::Debug`.

Wiring it up without touching the shipping build:

```toml
[dependencies]
tracing = { version = "0.1", optional = true }
tracing-subscriber = { version = "0.3", optional = true }

[features]
profile = ["dep:tracing", "dep:tracing-subscriber"]
```

```rust
// src/main.rs
#[cfg(feature = "profile")]
fn init_profiler() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::tree::PrintTreeLayer::new())
        .init();
}
```

**What you must NOT instrument**

* `src/render/paint.rs` `paint_pixel`, `paint_cell`, `paint_sky`,
  `apply_fx` — these run **once per pixel**. A span there costs more than
  the work it measures. `paint` itself is once per region and is fine.
* The per-neuron level of the brain's forward pass. The weights loop is
  vectorised; a span per neuron would destroy it.

With no subscriber installed `tracing` spans are nearly free, but the
attribute still expands to span creation, so keep it off the per-pixel
path as above.

### 3. `samply` — the interactive one

Zero attributes, renders in the **Firefox Profiler UI**, with a
Bottom-Up / Call Tree view: a sortable table of function → self time →
call count. That is the "which function takes longest" answer with no
instrumentation at all.

```powershell
samply record --main-thread-only --save-only --output "$env:TEMP\opencode\game.json" -- cargo run --profile profiling --bin primordia
samply load "$env:TEMP\opencode\game.json"
```

Caveats on Windows: it records through ETW/xperf and **wants
Administrator** — we have seen it fail with
`samply requires Administrator privileges for profiling on Windows.`
And with `--save-only` the frames come out as raw addresses unless you
add `--unstable-presymbolicate` (which writes a `.syms.json` sidecar).
The flame graph route above avoids all of that.

### 4. `puffin` — for watching it move while you play

* Real-time overlay, so you can see the frame times live.
* `puffin::profile_function!()` expands to nothing unless the `puffin`
  feature is on, so it can sit in the code without costing anything in a
  normal build — but it still wants a macro on each function, which is
  the thing we are avoiding.
* Good for the renderer: watch the pixel loop's cost move as you change
  the light field or the bloom.

Use it when you are *iterating* on performance and want the number to
move. Use the flame graph when you want to know *why*.

---

## Decision table

| Question | Tool |
|----------|------|
| Which module is hot? | `cargo flamegraph` |
| Which function is slowest, exactly? | `docs/flamegraph_summary.py` on the SVG, or `samply` |
| Why did the frame time change? | Two flame graphs, diffed (`--deterministic`) |
| Where does the renderer spend its time? | Look under `paint` in the graph |
| Is the simulation or the renderer the bottleneck? | `primordia prof` (already in the repo) |
| Watch it live while playing? | `puffin` |

---

## House rules

1. **Flame graph before instrumenting.** The sampling tools need no code
   changes and will usually point at the answer by themselves.
2. **Instrument phases, never leaves.** ~50 attributes on the phase
   boundaries of the three hot paths beat 500 attributes on everything.
3. **Never instrument the per-pixel path.** `src/render/paint.rs` is
   per-pixel; a span there costs more than the measurement.
4. **Keep the profiler optional.** `tracing` behind a Cargo feature; the
   release build must not carry it.
5. **Use the `profiling` cargo profile**, not `release`, so the shipping
   binary keeps its size and the graph has symbols.
6. **Profile a workload long enough to sample.** Seconds, not
   milliseconds: `bench 6000`, or the game with
   `PRIMORDIA_SNAPSHOT_AFTER=30`.
7. **Do not stop the profiler with Ctrl-C on Windows** — the recording is
   discarded. Let the program exit on its own.
8. **Profile the real game, not the bench harness.** `bench` runs the
   simulation without the renderer, so it answers a different question.
