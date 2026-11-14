# PRIMORDIA — How the Player's Brain Works

*A complete technical reference for the creature AI, the neural network that
drives it, and the difference between what is **evolved**, what is
**learned**, and what is **hard-coded**.*

> This document describes the brain **after the great change**: the senses
> are primitive. A creature is told what things *look like*, *smell like*
> and *sound like* — and nothing else. It has to work out for itself that
> the green soft thing that smells sweet is worth eating, and that the
> green wet thing that smells acrid is not.

---

## Table of contents

0. [Quick reference](#0-quick-reference)
1. [The big picture](#1-the-big-picture)
2. [The brain: architecture](#2-the-brain-architecture)
3. [The inputs: all 161 of them](#3-the-inputs-all-161-of-them)
4. [The outputs: all 33 of them](#4-the-outputs-all-33-of-them)
5. [How a brain actually runs](#5-how-a-brain-actually-runs)
6. [The direct path: the skip weights](#6-the-direct-path-the-skip-weights)
7. [Evolution: the genetic layer](#7-evolution-the-genetic-layer)
8. [Heritage: the cultural layer](#8-heritage-the-cultural-layer)
9. [Personality: the individual layer](#9-personality-the-individual-layer)
10. [The tech tree: the knowledge layer](#10-the-tech-tree-the-knowledge-layer)
11. [What is random, what is evolved](#11-what-is-random-what-is-evolved)
12. [Everything that is hard-coded](#12-everything-that-is-hard-coded)
13. [Reference tables](#13-reference-tables)
14. [How to read the brain at runtime](#14-how-to-read-the-brain-at-runtime)

---

## 0. Quick reference

| | |
|---|---|
| **Inputs** | **181** = 136 perception (8 directions × 17 channels) + 30 body + 15 kin |
| **Outputs** | **33** = 25 behaviours + 8 desires |
| **First layer** | 181 senses + 12 recurrent memory units = **193** |
| **Hidden layers** | 3, sized `56/40/24`, `72/52/32` or `88/64/40` by tier |
| **Architecture** | fully connected, input-major weights, one activation function per brain, plus a **direct path** from the senses to the desires |
| **Activation functions** | 6 (threshold, tanh, leaky, sine, sigmoid, smooth) |
| **Think rate** | every other tick; the decision is cached in between |
| **Sight range** | 8 cells · **Smell range** | 30 cells, falling off gently · **Own kind** | 70 cells |
| **Genes** | 32 floats → 23 traits + affinity + immunity |
| **Practices** | 8 bits, inherited and multiplied by the colony |
| **Discoveries** | 32, in 10 tiers, earned by practice + generations + luck |
| **Personality** | 5 floats, inherited, blended from two parents |
| **Population cap** | 320 soft, 500 hard |

### The three inheritance layers

| Layer | What it is | How it changes | How it is used |
|-------|-----------|----------------|---------------|
| **Genes** | 32 floats | blended from both parents, ±0.04 | decoded into traits, affinity and immunity |
| **Heritage** | 8 practice bits + 32 discoveries | inherited, and *grown* by the colony doing the thing | multiplies how well the colony does things; unlocks tools and grows the brain |
| **Personality** | 5 floats | parents' average, ±0.09 | shifts the steering; decides mate choice |

---

## 1. The big picture

Every creature in PRIMORDIA is an agent with **one brain** and **three
inheritance layers**. The brain is a small recurrent neural network; it
does not decide everything, and a great deal of the world's behaviour is
written in plain Rust around it.

```
                    ┌──────────────────────────────────────────┐
                    │            THE BRAIN (a Net)             │
   161 senses ───► │  [161 + 12] ─► [56/72/88] ─► [40/52/64]  │ ───► 33 outputs
                    │        ─► [24/32/40] ─► [33 decisions]   │
                    └──────────────────────────────────────────┘
                              ▲              ▲
                reinforcement  │              │  inherited weights
                    (plasticity)│              │  + shape mutation
                              │              │
        ┌─────────────────────┴──┐      ┌────┴──────────────┐
        │   GENES (32 floats)    │      │  CULTURE (16 bit) │ ──► skip weights
        │   -> 23 traits        │      │  -> 8 practices   │     (the instincts)
        │   -> affinity         │      │  -> multipliers   │
        │   -> immunity         │      └───────────────────┘
        └────────────────────────┘
                     +
        ┌────────────────────────┐
        │ PERSONALITY (5 floats) │  -> the individual's own temperament
        └────────────────────────┘
```

**The key idea:** the brain is a *preference generator*, not a decision
maker. It outputs 33 numbers in the range −1..1. Each of those numbers is
then compared against a threshold in hand-written code that does the actual
work. So "attack" output of 0.4 does not mean "attack with 40% strength" —
it means "the network wants to attack, and the code decides whether 0.4 is
enough".

**The second key idea:** the senses do not contain answers. There is no
"there is food that way" input any more. The network gets colour, light,
warmth, texture, wetness, gloss, smell and sound, and the eight
**desires** are how it says what it wants to do about them.

---

## 2. The brain: architecture

Source: `src/creature/brain.rs`

### 2.1 Layer sizes

The network is **fully connected**, one hidden layer per "size", and the
first layer takes the senses and the recurrent memory together.

| Tier | First layer | Hidden 1 | Hidden 2 | Hidden 3 | Decisions |
|------|-------------|----------|----------|----------|-----------|
| **0** (everyone starts here) | 173 | 56 | 40 | 24 | 33 |
| **1** (after *Cognition*) | 173 | 72 | 52 | 32 | 33 |
| **2** (after *Grand Cognition*) | 173 | 88 | 64 | 40 | 33 |

The first layer is `INPUTS (161) + MEMORY (12) = 173`. The input and
output layers never change — the senses and the decisions are fixed — only
what happens in between grows.

### 2.2 The recurrent memory

The 12 extra units in the first layer are a **recurrent memory**. After
every forward pass the memory is rebuilt from the last hidden layer:

```rust
let v = (hidden[i] * 0.3).tanh();
memory[i] = memory[i] * 0.68 + v * 0.32;
```

So 68% of the old memory survives and 32% is new. This is what lets a
creature act on what it saw a while ago rather than only on what is in
front of it right now: it can keep going somewhere, remember a threat, or
learn that a place is worth coming back to.

### 2.3 Activation functions

Each brain picks one of **six** hidden-layer functions, inherited and bred
like any other trait:

| Code | Name | Formula | Character |
|------|------|---------|-----------|
| 0 | threshold | `max(0, x)` | the plain one; most species start here |
| 1 | tanh | `x / (1 + \|x\|)` | smooth squasher (cheap approximation) |
| 2 | leaky | `x > 0 ? x : 0.1x` | lets a little negative through |
| 3 | sine | `sin(0.6x)` | good for cyclic things |
| 4 | sigmoid | `0.5 + 0.5·x/(1+\|x\|)` | saturating switch, always positive |
| 5 | smooth | `0.5·x·(1 + x/(1+\|x\|))` | smooth gate |

**The last layer always squashes** into −1..1 with `x / (1 + |x|)`, so all
33 decisions are comparable whatever the hidden layers did.

At birth: **40% threshold, 60% uniform across the other five**. The
function mutates with a **2% chance per generational mutation**.

### 2.4 The forward pass

```rust
for each layer:
    1. compute the dot products   (one plain loop, vectorisable)
    2. apply the activation       (a second, separate loop)
    3. swap the buffers
```

The split into two passes matters: mixing them puts a branch in the middle
of the hot loop and costs about a third of the time.

**Performance note:** the weights are stored **input-major** (the weights
of one input sit next to each other), so the layer is computed by walking
the inputs and adding each one's contribution to every output. This is
about **three times faster** than the output-major layout. Inputs that are
exactly zero are skipped entirely — and plenty of the senses are zero most
of the time.

### 2.5 Weight layout and the shape of a mind

Weights are stored input-major: `w[j * out_n + o]` is the weight from
input `j` to output `o`.

A mind's **shape** (how many neurons in each hidden layer) is bred:
a child is born with a 12% chance of having one neuron more or less than
its mother in a random hidden layer. The weights that still fit are copied
across; the new neuron gets fresh random weights.

### 2.6 Growth

When a species works out **Cognition**, every living mind of that species
grows to tier 1. When it works out **Grand Cognition**, they grow to
tier 2. The old weights are carried over, so nothing already learnt is lost.

Growth also happens when knowledge is **shared through an alliance** — the
receiving species grows its minds too.

---

## 3. The inputs: all 181 of them

Source: `src/creature/senses.rs`. The vector is zeroed and filled every
time a creature thinks.

### 3.1 Block map

| Slots | Count | Block |
|-------|-------|-------|
| 0–135 | 136 | **Perception** — 8 directions × 17 channels |
| 136–165 | 30 | **Body** (proprioception) |
| 166–180 | 15 | **Kin** |

### 3.2 Perception (0–135) — 17 channels per direction

The eight directions run clockwise from south, so **up is the fifth**
(`DIR_UP = 4`). Each direction has the same fifteen channels:

#### Sight (9 channels)

| Offset | Channel | Meaning |
|--------|---------|---------|
| +0 | **near** | something is in this direction at all (1 = right in front) |
| +1 | **light** | how much it shines by itself |
| +2 | **red** | its colour, red |
| +3 | **green** | its colour, green |
| +4 | **blue** | its colour, blue |
| +5 | **warm** | how warm it is (−1 cold, 0 ordinary, 1 hot) |
| +6 | **soft** | how yielding it is (0 hard rock, 1 soft enough to eat) |
| +7 | **wet** | how wet it is |
| +8 | **sheen** | how glossy it is |
| +9 | **alive** | something alive is over there — a shape that moves |

The first thing in the direction is the thing it *sees*: its colour, its
warmth, its texture. Anything further along the ray only adds its glow
and its heat, behind the nearer one. The sight range is **8 cells**.

#### Smell (5 channels)

| Offset | Channel | Meaning |
|--------|---------|---------|
| +9 | **sweet** | ripe, worth eating |
| +10 | **rotten** | rot, bacteria, the deep's reek |
| +11 | **acid** | acrid, corrosive |
| +12 | **mineral** | stone, metal, salt |
| +13 | **smoke** | burning, ash, gunpowder |
| +14 | **own kind** | the smell of my own kind — it carries up to seventy cells |

The smell range is **30 cells**, and the falloff is *gentle*
(`1 / (1 + distance / 8)`), because a nose does not stop working when
something is far away — it only gets fainter. Beyond eight cells the ray
is sampled every second cell, so the nose does not have to read every cell
of a thirty-cell ray.

The smell of a direction is the strongest scent found along it, so plain
stone in between does not hide the sweet thing behind it.

#### Hearing (1 channel)

| Offset | Channel | Meaning |
|--------|---------|---------|
| +15 | **loud** | how loud something is over there |

Fire and lava roar (0.9), water and acid hiss (0.5), gunpowder bangs
(1.0). Sound carries as far as smell.

### 3.3 Body (136–165) — what the creature knows about itself

This is proprioception, not knowledge of the world: every animal knows how
hungry it is and which way it is going. It is the only place the network
is told a fact outright.

| Slot | Meaning | Range |
|------|---------|-------|
| 136 | energy | 0..1 of max |
| 137 | health | 0..1 of max |
| 138 | hunger | 0..1 |
| 139 | thirst | 0..1 |
| 140 | stamina | 0..1 |
| 141 | mana | 0..1 |
| 142 | resting | 0 or 1 |
| 143 | how much it is carrying | 0..1 of capacity |
| 144 | age | 0..1 of `MAX_AGE` |
| 145 | velocity x | −1..1 |
| 146 | velocity y | −1..1 |
| 147 | how fast it is going | 0..1 |
| 148 | which tool is held | 0–15, ÷15 |
| 149 | tool durability | ÷2000 |
| 150 | tech level | ÷4 |
| 151 | total inventory | ÷2000 |
| 152 | craft cooldown | ÷70 |
| 153 | spell cooldown | ÷60 |
| 154 | fire cooldown | ÷60 |
| 155 | build cooldown | ÷10 |
| 156 | has a disease | 0 or 1 |
| 157 | disease timer | ÷300 |
| 158 | mood | 0–9, ÷9 |
| 159 | how much daylight there is | 0..1 |
| 160 | temperature | ÷40, clamped −1..1 |
| 161 | how fresh the air is | 0..1 (oxygen within three cells) |
| 162 | rain on the skin | 0 or 1 |
| 163 | the wind | 0..1 (storm, sandstorm, rain) |
| 164 | **open sky above** | 0 or 1 |
| 165 | **how much light reaches here** | 0..1 — it knows it is dark, not that it is deep |

### 3.4 Kin (166–180) — what it can see of its own kind, and of others

| Slot | Meaning |
|------|---------|
| 166 | how close its own kind is (1 − distance/30) |
| 167 | how many of its own kind are near (÷10) |
| 168 | how close the nearest stranger is |
| 169 | how many strangers are near (÷10) |
| 170–172 | the colour of the nearest stranger |
| 173 | how big the nearest stranger is |
| 174–176 | the colour of its own species |
| 177–178 | which way the nearest of its own kind is |
| 179–180 | which way the nearest stranger is |

A creature recognises its own kind by how they look, and it can see the
colour of a stranger. It cannot see a treaty, and it is not told one:
whether the red ones are friends or enemies is something it has to work
out for itself.

The last four channels are the two directions: which way the nearest of
its own kind lies, and which way the nearest stranger lies. Without them
the network could see a colour and a bearing but could not join the two
up, and could never learn that *the red ones over there* are the ones to
run from.

---

## 4. The outputs: all 33 of them

Source: `src/creature/mod.rs` (`pub mod outputs`) and the consumers in
`src/creature/actions.rs`, `step.rs`, `building.rs`, `diplomacy/`.

Every output is in **−1..1** (the last layer squashes). The code below is
what actually happens.

### 4.1 The twenty original behaviours

| # | Name | Threshold | What it does |
|---|------|-----------|--------------|
| 0 | **move_x** | — | Blended into velocity: `vx = out[0]*0.8 + steer_x*0.8` |
| 1 | **move_y** | — | `vy = out[1]*0.8 + steer_y*0.8 + 0.2` (the +0.2 is gravity) |
| 2 | **eat** | > 0.05 | Bites the cell next to it |
| 3 | **reproduce** | > 0.05 | Wants a child (other gates apply) |
| 4 | **attack** | > threshold | Melee. The threshold depends on aggression, mood, truce and war |
| 5 | **build** | > 0.2 | Raises a building if materials allow |
| 6 | **craft** | > 0.25 | Crafts a tool, bomb or salve |
| 7 | **cast_spell** | > 0.5 | Casts (needs mana ≥ 10 and a known spell) |
| 8 | **mount** | > 0.45 | Mounts or dismounts a beast |
| 9 | **trade** | > 0.4 | Looks for a trading partner within 6 cells |
| 10 | **rest** | > 0.05 | Rests (only if stamina is below half) |
| 11 | **switch_spell** | > 0.5 | Cycles to the next known spell |
| 12 | **dig / mine** | > 0.35 (0.5 to dig) | Digs; also the primary "I want to mine" signal |
| 13 | **gather_water** | > 0.05 | Drinks the water next to it |
| 14 | **shoot** | > 0.3 | Looses an arrow (soldiers also shoot if out[4] > 0.6) |
| 15 | **flee** | > 0.35 | Steers away from the nearest threat at 1.4× speed |
| 16 | **seek** | > 0.05 | Wants whatever it needs most; also opens the mouth |
| 17 | **forage** | > 0.3 | Wants to go out and look for ore |
| 18 | **rally** | > 0.3 | Heads back to the colony base |
| 19 | **defend** | > 0.3 | Holds position near the base against a raider |

### 4.2 The five diplomatic and courtship outputs

| # | Name | Threshold | What it does |
|---|------|-----------|--------------|
| 20 | **NEGOTIATE** | > 0.3 (0.4 to walk there) | Opens talks with a foreign creature |
| 21 | **HOSTILE** | > 0.5 | Makes a hostile diplomatic move |
| 22 | **ALLY** | — | Offers an alliance or a pact |
| 23 | **TRIBUTE** | > 0.45 | Demands tribute |
| 24 | **SEEK_MATE** | > 0.35 | Courts a mate of its own kind |

Roles also bias these: an **Envoy** always wants talks, a **Merchant**
negotiates at out[9] > 0.25, and a **Priest** at out[7] > 0.4.

### 4.3 The eight desires

| # | Name | Meaning |
|---|------|---------|
| 25–32 | **desire per direction** | how much the creature wants to go in each of the eight directions |

The desires are the *new* outputs of the great change, and they are what
makes the whole thing work. With a single "move x" and "move y" the
network has to learn the hard mapping from "the thing I want is over
there" to "these are the coordinates to press". With one desire per
direction it only has to learn "this direction looks good" — and because
the percepts are laid out in the same eight directions, the mapping is
almost trivial for the hidden layer to find.

### 4.4 The output is a preference, not a command

This is the single most important thing to understand about the outputs.
Look at how **flee** (15) is used:

```rust
if out[15] > 0.35 {
    if let Some((dx, dy, d)) = flee_target {
        steer_x -= dx / d;      // steer AWAY from the threat
        speed_mult_extra = speed_mult_extra.max(1.4); // and run faster
    }
}
```

The network says *how much* it wants to flee; the code says *what fleeing
means*. The same pattern holds for all 33. There is no output that says
"attack the creature at (x, y)" — only "attack", and the code picks the
target. There is no output that says "the food is at (x, y)" either — only
a desire for a direction, and the code walks that way.

---

## 5. How a brain actually runs

### 5.1 The think cycle

```rust
every tick:
    if (tick is even) OR (this creature has never thought):
        1. scan_percept() — look, smell and listen in all eight directions
        2. write_percept / write_body / write_kin — fill the 161 inputs
        3. forward_into()       — run the network
        4. copy the 33 outputs into out_cache
    else:
        reuse out_cache  — yesterday's decision is still a good one
```

The brain runs on **every other tick**. This is the single biggest
saving in the simulation: halving the brain work halves the creature
step's cost, and because creatures move slowly, a decision from two ticks
ago is rarely wrong.

The percept scan is cached in the creature (`percept_cache`), so the world
is only scanned once per creature per think, and the same picture serves
the senses and the steering.

### 5.2 The plasticity loop

After a creature does something, the decision that worked is pushed back
through the network:

```rust
if ate_something {
    targets.push((2, learn * 1.5));                       // eating was right
    targets.push((DESIRE + desire_dir, learn * 2.5));     // and so was that way
}
if built_something { targets.push((5, learn * 1.0)); }
if killed          { targets.push((4, learn * 1.6));       // the strike landed
                     targets.push((DESIRE + kill_dir, learn * 2.0)); }  // and so was that way
if wants_attack    { targets.push((4, learn * 0.3)); }    // a swing at the air is worth little
if traded          { targets.push((9, learn * 0.8)); }
if cast_spell      { targets.push((7, learn * 0.8)); }
if hurt            { targets.push((15, learn * 1.2));      // run!
                     targets.push((2, -learn * 0.8));      // do not bite
                     targets.push((DESIRE + hurt_dir, -learn * 1.5)); }  // do not walk towards it
if c.hunger > 70   { targets.push((2, learn * 0.5)); }    // going hungry teaches it to want to eat
```

Two of these lessons are about **where** something was, not only about
what was done. The combat code remembers the direction a blow came from
and the direction the prey was in, and the credit goes to *that*
direction — so the network learns "the thing over there is the one that
hurts me, run the other way" and "the thing over there was worth
walking towards", instead of learning a direction that has nothing to do
with what happened.

The credit is **proper back-propagation**: the error at the output is
walked back through the hidden layers, each input blamed in proportion to
the weight between it and the output that was wrong, and the weights moved
in the direction that would have produced the wanted output. The last few
thoughts are kept (four of them, each worth 72% of the one before), so a
reward can be given to a decision the creature made a moment ago rather
than the one it happens to be making now.

The learning rate is:

| Condition | Rate |
|-----------|------|
| Normally | 0.012 |
| With *Grand Cognition* | 0.030 (2.5×) |
| Standing on a whispering stone | ×2.0 |

Every 97 ticks the weights are **settled** (multiplied by 0.996) so a life
of reinforcement cannot turn a few weights into everything.

### 5.3 The shaping: teaching without a lesson

Learning only from meals is learning from a very rare event: the creature
has to find food before it learns how. So the creature is also rewarded for
*wanting* the right thing.

A hungry creature that smells something sweet gets a credit for the
direction it smells it in. It does not know the smell is food — that is
the whole point — but the credit teaches it that *wanting that direction*
is a good idea, and over many meals the connection between "sweet" and
"food" is made.

The same is done for thirst: a thirsty creature is drawn to the wet, the
blue and the wet together.

Two rules keep this honest:

- only the **strongest** smell is credited, and
- the smell has to point **one way**: a smell that is the same in every
  direction is no direction at all, and crediting a direction on a tie
  teaches the animal to walk in circles.

### 5.4 The mutation

At birth, the child's brain is the mother's brain with:

| Change | Probability | Effect |
|--------|-------------|--------|
| Weight mutation | 8.5% per weight | ±0.15 |
| Activation change | 2% | a different firing function |
| Shape change | 12% | one neuron more or less in a hidden layer |
| Instinct mutation | every weight | ±0.04, clamped to ±1.5 |

The weight mutation rate is deliberately **scaled down** as the brain
grows, so the total amount of change per generation stays about the same:
a young animal is still its mother's breed with a little of its own.

---

## 6. The direct path: the skip weights

Source: `src/creature/brain.rs` (`seed_instincts`, `apply_skip`)

The raw senses reach the desires without going through the hidden layers.
There is **one skip weight per sensory channel** (15 of them, the same
for every direction) plus one for the pull of the open sky.

The path is *spatially* aligned: the smell in direction three can only
push the desire for direction three. It says nothing about what any of it
means.

```
desire[d] = squash( network_output[d] + Σ_c skip[c] · percept[d][c] )
```

### 6.1 The instincts every animal is born with

| Weight | Value | What it does |
|--------|-------|--------------|
| `skip[SWEET]` | +1.15 | the smell of something sweet pulls |
| `skip[ACID]` | −0.90 | the smell of something acrid pushes away |
| `skip[GREEN]` | +0.08 | a green thing is worth a look, but only a little |
| `skip[KIN]` | +0.40 | the smell of your own kind pulls — a herd stays together |
| `skip_up` | +0.55 | the open sky pulls, in the direction that is up |

This lives in the **weights, not in the code**. The action layer knows
nothing about food: it moves the creature where the desires point, and the
desires come out of the network. Evolution is free to strengthen, weaken
or reverse this bias — and it does, because a lineage that walked into the
poison and died left no children.

**This is what makes the world learnable.** A mind that has to carry
"there is something sweet over there" through three hidden layers before
it can act on it will never get there in a lifetime, but a mind with a
direct path from the nose to the feet learns it in a few hundred steps —
and then evolution is free to sharpen, weaken or reverse it.

---

## 7. Evolution: the genetic layer

Source: `src/creature/enums.rs`, `src/creature/heritage.rs`

### 7.1 The genome

32 floats, each 0..1, inherited from two parents and mutated:

```rust
for gi in g.iter_mut() {
    *gi = (*gi + (rng.f32() * 2.0 - 1.0) * 0.08).clamp(0.0, 1.0);
}
```

A **±0.04** wobble per generation.

### 7.2 The 23 traits

Traits are decoded from the genes with a threshold. A high gene means the
trait is expressed — except `Peaceful`, which is the mirror image (a *low*
gene means peacefulness).

| Gene | Trait | Threshold | Effect |
|------|-------|-----------|--------|
| 0 | Ferocious | > 0.74 | +35% melee damage, attacks more readily |
| 1 | Industrious | > 0.74 | builds and gathers faster |
| 2 | Hardy | > 0.74 | +40% health, hungers more slowly |
| 3 | Fertile | > 0.74 | cheaper, more frequent offspring |
| 4 | Keen Senses | > 0.74 | sees and senses further |
| 5 | Scavenger | > 0.74 | finds food and corpses further away |
| 6 | *Peaceful* | **< 0.18** | prefers trade over violence |
| 7 | Venomous | > 0.76 | melee attacks poison the wounded |
| 8 | Amphibious | > 0.76 | swims through water and damp |
| 9 | Nocturnal | > 0.76 | hunts at night, idles through the day |
| 10 | Gregarious | > 0.76 | works faster near its own kind |
| 11 | Hoarder | > 0.76 | carries 60% more |
| 12 | Thermophilic | > 0.76 | immune to cold, drawn to heat |
| 13 | Metallophilic | > 0.76 | senses metal, digs towards it |
| 14 | Hollow-Boned | > 0.78 | faster, but frail |
| 15 | Hivemind | > 0.80 | the colony shares one mind |
| 16 | Prolific | > 0.78 | many small cheap young, quick to mature |
| 17 | Prolonged | > 0.78 | few young, heavily invested in, long-lived |
| 18 | Pacific | > 0.80 | never fights over food with its own kind |
| 19 | Deep Digger | > 0.80 | digs deeper than the rest |
| 20 | Climber | > 0.80 | at home in the high places |
| 21 | Provisioner | > 0.80 | hoards food and shares it |
| 22 | Crepuscular | > 0.82 | works at dusk and dawn, rests at noon |

### 7.3 Affinity (gene 16)

The elemental adaptation a species drifts into, read from a single gene:

| Roll | Affinity | Effect |
|------|----------|--------|
| > 0.93 | Ember | immune to fire |
| > 0.88 | Tide | — |
| > 0.83 | Crystal | 2× mana regeneration |
| > 0.78 | Iron | 1.6× mining, 1.15× carry |
| > 0.73 | Timber | 1.5× building, 1.1× carry |
| > 0.70 | Stone | takes 70% of the damage |
| else | None | — |

### 7.4 Immunity (genes 23–26)

Four of the genes give resistance to four of the diseases:

| Gene | Disease | Threshold | Immunity |
|------|---------|----------|----------|
| 23 | Flu | > 0.74 | 2 |
| 24 | Plague | > 0.58 | 1 |
| 25 | Infection | > 0.58 | 1 |
| 26 | Madness | > 0.58 | 1 |

Genes **27–31** are currently unused — reserved for future traits.

---

## 8. Heritage: the cultural layer

Source: `src/creature/heritage.rs`

A **culture** is a 16-bit mask of practices. It is *not* genetic: it
accumulates when the colony does a thing, it is inherited by the young,
and it multiplies how well the colony does it.

| Bit | Practice | Learned when | Multiplier |
|-----|----------|--------------|------------|
| 0 | Cultivation | the colony harvests crops | 1.5× |
| 1 | Mining | out[12] > 0.35 | 1.4× |
| 2 | Warfare | the creature wants to attack | 1.25× |
| 3 | Commerce | the creature trades | 1.35× |
| 4 | Craft | the creature holds a tool | 1.3× |
| 5 | Ritual | the creature casts a spell | 1.25× |
| 6 | Architecture | the creature builds | 1.35× |
| 7 | Hunting | the creature eats | 1.3× |

**Discovery** (as opposed to practice) is a *species-level* piece of
knowledge: 32 discoveries in ten tiers, gated by practice, generations and
luck. See §9.

---

## 9. Personality: the individual layer

Source: `src/creature/personality.rs`

Five axes, each 0..1, that make two creatures with identical genes behave
differently. A child inherits its parents' average and drifts ±0.09.

| Axis | What it moves |
|------|--------------|
| aggression | how eagerly it starts a fight and how hard it hits |
| curiosity | how far it ranges to find what it needs |
| sociability | how strongly it is pulled back towards the colony |
| greed | how much it hoards and competes |
| caution | how much it avoids risk |

Personality is also used for **mate choice**. A pair's compatibility is
computed from both the temperament distance and the gene distance:

```rust
pd_fit = 1.0 - (pd - 0.35).abs() / 0.5;   // ideal temperament distance 0.35
gd_fit = 1.0 - (gd - 0.12).abs() / 0.35;   // ideal gene distance 0.12
compatibility = (pd_fit * 0.5 + gd_fit * 0.5).clamp(0.0, 1.0);
```

So a creature prefers a partner who is neither too alike (inbreeding)
nor too different (that would be a different species) — and the same
rules decide which pairs are allowed to breed at all.

---

## 10. The tech tree: the knowledge layer

Source: `src/creature/tech.rs`, `src/creature/technology.rs`

Ten tiers, each holding **tools** (things a creature carries), **crafts**
(things a colony makes) and **discoveries** (things a species knows).

### 10.1 How a discovery is earned

Every 300 ticks, for every species with at least 3 members:

1. The tier below must be **complete**.
2. The required **practices** must be in the culture bitmask.
3. The species must have lived through enough **generations**.
4. A **22%** chance (34% if the colony sits beside the drowned temple).

A species gets **one** discovery per check, and the discoveries are
tried in tree order.

### 10.2 Alliances share knowledge

Every 240 ticks, allied species exchange everything they know. This is
what makes an alliance worth having — and the receiving species grows its
brains too.

### 10.3 The discoveries that change the brain

| Tier | Discovery | Requirement | Effect |
|------|-----------|-------------|--------|
| 1 | **Cognition** | Craft | every mind grows to tier 1 (72/52/32) |
| 3 | **Grand Cognition** | Craft + Mining | every mind grows to tier 2 (88/64/40), and learning inside a life is 2.5× faster |

### 10.4 The full tree

**Tier 1 — THE FIRST HANDS:** Pickaxe, Spear, Husbandry, Pottery, **Cognition**
**Tier 2 — THE CRAFT:** Sword, Hammer, Smithing, Ranged Craft
**Tier 3 — THE WORKSHOP:** Bow, Shield, Architecture, Masonry, **Grand Cognition**
**Tier 4 — THE TRADE:** Cart, Backpack, Commerce, Cartography
**Tier 5 — THE MEDICINE:** Salve, Medicine, Herbalism, Surgery
**Tier 6 — THE MINE:** Drill, Scanner, Deep Mining, Geodesy, Smelting
**Tier 7 — THE STARS:** Staff, Amulet, Astronomy, Sanctuary, Navigation
**Tier 8 — THE LIGHT:** Sunfire, Sanctification, Wardcraft, Antitoxins, Purge
**Tier 9 — THE WAR:** Bomb, Siegecraft, Tactics, Ironwill, Sunfirecraft, Radiance
**Tier 10 — THE DEEP:** Boat, Glider, Metallurgy, Alchemy, Deepdiving, Dominion

---

## 11. What is random, what is evolved

### 11.1 Pure randomness (no inheritance at all)

| Thing | Where | Chance / range |
|-------|-------|---------------|
| Founder's colour | `colony.rs` | uniform 90..256 per channel |
| Founder's name | `names.rs` | generated from syllables |
| Founder's genes | `colony.rs` | uniform 0..1 |
| Founder's personality | `personality.rs` | uniform 0..1 per axis |
| The initial brain | `brain.rs` | uniform random weights, random activation |
| The initial role | `colony.rs` | 12% soldiers, the rest workers |
| Mood drift | `step.rs` | re-rolled every 60–180 ticks |
| Colony campfires, squads, trade pairings | various | random picks |
| The ZORG's carried disease | `zorg.rs` | uniform over 5 |
| Discovery rolls | `technology.rs` | 22% or 34% |
| Speech/attack variations | `combat.rs` | random spread |

### 11.2 Inherited but **not** neural

| Thing | Mechanism | Mutation |
|-------|-----------|----------|
| Body colour | blended from the parent, ±18 per channel | inherited |
| Size | parent's size ± 0.15 | inherited |
| Genes (traits, affinity, immunity) | blended from both parents, ±0.04 | inherited |
| Personality | parents' average ± 0.09 | inherited |
| Culture (practices) | parent's bitmask OR the species pool | inherited, and *grows* when the colony practises |
| Affinity | decoded from gene 16 | inherited via the gene |
| Role | mostly inherited from the parent, with variation | inherited |

### 11.3 Neural network (evolved by weight mutation)

| Thing | Mechanism |
|-------|-----------|
| **Every behaviour preference** | the 25 behaviour outputs |
| **Every direction preference** | the 8 desires |
| **The instincts** | the skip weights (15 channels + the sky), mutated ±0.04 per generation |
| Activation function | bred, 2% mutation |
| Network shape | bred, 12% chance per birth |
| Network size (tier) | grows when the species discovers Cognition |
| Learned associations | plastic reinforcement inside a life |

### 11.4 Speciation — a new species, a new brain

A colony that has outgrown its ground can **split**: it sends some of its
members away, and if they travel far enough they become a species of their
own, with a new id, a new colour and a fresh diplomatic record.

The emigrants keep their brains, so a new species starts from a copy of
its parent's mind — and then drifts.

### 11.5 Hard-coded (never evolves)

Everything in §12.

---

## 12. Everything that is hard-coded

A large part of the simulation is *not* the brain at all. This is the
complete list of the important ones.

### 12.1 Steering — always on, regardless of the network

The steering vector is computed from hard rules and then **blended** with
the network's desires:

```rust
for d in 0..8 {
    let v = out[DESIRE + d];
    steer_x += DIRS8[d].0 * v;
    steer_y += DIRS8[d].1 * v;
}
c.vx = (out[0] * 0.8 + steer_x * 0.8).clamp(-speed_cap, speed_cap);
c.vy = (out[1] * 0.8 + steer_y * 0.8 + 0.2).clamp(-speed_cap, speed_cap);
```

The steering rules that always apply:

| Rule | Effect |
|------|--------|
| Threat within range | steer away, ×1.4 speed |
| Colony pull | juveniles are pulled home; sociable adults at `(sociability − 0.45) × 0.8` |
| Weather sheltering | storms, meteors and heatwaves pull the colony home |
| Hunting | a hungry hunter stalks the nearest other species at ×1.3 |
| Courtship | a creature that wants a mate walks towards the nearest of its own kind |
| Migration | the emigrants push away from the old colony |
| Diplomacy | a creature that wants to talk walks towards the foreign colony |

**What is *not* in this list any more:** the direction to the nearest food,
the direction of the nearest ore, the direction of the nearest poison.
Those used to be looked up in the world and handed to the creature. Now
they come from the creature's own desires.

### 12.2 The local actions

The actions are **local** too. Before, a creature could reach anything
editable within six cells without knowing where it was, which meant the
network never had to perceive food at all: the hard-coded action did the
finding. Now:

- **Eat**: the creature bites the cell next to it (or the cell it is
  standing on). The mouth knows what food is — that is a mouth, not a
  brain — but the creature has to have walked there first.
- **Drink**: the creature drinks the water next to it.
- **Dig**: the creature breaks the wall next to it.

### 12.3 Survival rules

| Rule | Numbers |
|------|---------|
| Movement cap | `MAX_SPEED` 0.9 cells/step, ×1.5 mounted |
| Daylight speed | speed is multiplied by the daylight factor |
| Energy drain | 0.08/tick × weather factor × (1.4 − 0.4·fresh_air) × (0.55 + 0.45·hardship) |
| Breath cost | 0.05/tick × (1 − fresh_air) |
| Stamina regen | 0.2/tick × (0.7 + 0.6·fresh_air) |
| Hunger | +0.06/tick × hardy × stores |
| Thirst | +0.045/tick × hardy ÷ weather thirst factor |
| Health regen | towards `max_health()`, which traits and affinity modify |
| Death | energy ≤ 0, health ≤ 0, or age over the lifespan (+0–1400 per id) |

### 12.4 Action gating

Every network output is gated by hard conditions before it does anything:

- **Eat** needs food within reach and `hunger > 0`.
- **Attack** needs a target, no truce, and `stage.can_fight()`.
- **Build** needs materials, a free site and a cooldown.
- **Craft** needs the recipe's materials and a cooldown.
- **Cast** needs `mana ≥ 10`, a known spell and a cooldown.
- **Shoot** needs a bow and ammunition.
- **Reproduce** needs a mate, energy, a cooldown and population headroom.
- **Trade** needs a partner of another species within 6 cells.
- **Mine** needs a `world.mine_budget` — the world limits how much can be
  dug out per tick, so creatures cannot strip-mine it.

### 12.5 The difficulty ramp

The world starts forgiving and gets harder. The ramp is the run's own
progress plus what the world has become, and it is a **smoothstep** over
twenty thousand ticks:

```rust
let t = (frame as f32 / 20_000.0).min(1.0);
let ease = t * t * (3.0 - 2.0 * t);
self.difficulty = (ease * 0.65 + strain * 0.35).clamp(0.0, 1.0);
```

The curve barely moves at first, so that a colony of poor brains has the
time to work out how to eat, and then it climbs hard in the middle of
the run — which is exactly when the field has narrowed to a handful of
species and the survivors have to fight for what is left.

Hunger, thirst, the cold, the cost of breeding and the size of the ZORG
swarm all read it. A young world in good condition is gentle; a world
that has been picked clean and battered is not.

### 12.6 The late world is a lean world

The vegetation slows down as the run wears on:

```rust
let lean = 1.0 - 0.75 * self.difficulty;
```

A surface that can only ever feed one colony is a surface the last two
colonies have to fight over. This, more than anything else, is what ends
a run: the field narrows to two, the ground cannot carry both, and one
of them wins.

### 12.7 Idling is punished

A colony that sits in one place does not harvest and does not explore,
and the world takes it at its word:

- **The crowd costs.** A creature with its neighbours pressed all around
  it burns more energy: the drain is multiplied by `1 + 0.08 × (the
  number within four cells, up to sixteen)`. A heap of fifty animals
  round the fire burns nearly twice what the same animals would burn
  spread out over the ground.
- **The fire has to be fed.** A colony's fire burns wood every minute of
  the world's time, and the wood has to be *carried home* by somebody. The
  fuel is trees, not grass: a mouthful of green fills the belly and is no
  firewood at all. A colony with no wood goes cold, and the cold comes
  in.
- **The fire keeps the cold out.** A lit fire at the base means the
  winter does not bite the colony. That is the whole of the reward for
  gathering: a warm nest is worth going out for.

### 12.8 The world

Everything about the world is hard-coded: the cellular automaton, the
reactions between elements, the weather table, the seasons, the volcano,
the earthquakes, the landmarks and their bonuses, the biome table, the
tech tree gates, the diplomacy rules, the plague table and the ZORG
ecology.

The vegetation is one of these. The surface regrows: every fifteen frames
seedlings are put on the ground beside the surface, and a plant that
already grows there puts out a seedling beside itself. The world is kept
from turning into one green mat by a hard cap that thins the vegetation
when it gets too thick.

### 12.9 Diplomatic negotiation

The outcome of a negotiation is computed by hard rules in
`diplomacy/negotiation.rs` using the network's outputs 20–23 as *inputs*
to the rules, not as decisions.

---

## 13. Reference tables

### 13.1 The fitness function

Fitness is recomputed every tick from hard counters — it is what a
successful lineage looks like, and it decides which genome seeds the next
run:

```rust
fitness = (age / lifespan) * 0.15
        + (energy / max_energy) * 0.12
        + offspring * 0.18
        + kills * 0.12
        + (carried / carry_cap) * 0.08
        + tech_level * 0.10
        + tool * 0.04
        + (mana / max_mana) * 0.06
        + (stamina / max_stamina) * 0.05
        - (hunger / 100) * 0.05
        - (thirst / 100) * 0.05
```

### 13.2 The energy costs of every action

| Action | Cost |
|--------|------|
| Move | 0.020 |
| Shove through debris | 0.35 |
| Chew (ZORG only) | 0.06 |
| Melee attack | 6.0 |
| Shoot | 4.0 |
| Dig | 5.0 |
| Mine | 3.0 |
| Build | 8.0 |
| Craft | 5.0 |
| Bomb | 6.0 |
| Salve | 4.0 |
| Trade | 2.0 |
| Cast a spell | 10.0 |

### 13.3 The roles

| Role | How it is chosen | What it does |
|------|------------------|--------------|
| **Queen** | the first of a species; 5% of a queen's children, 5% of a worker's, 3% of a soldier's, 5% of an envoy's | holds the ground; the colony's centre |
| **Soldier** | 12% of founders; 10% of a queen's children; 1% of a worker's; 2% of a soldier's | forms squads; shoots if out[4] > 0.6 |
| **Worker** | the default; 78% of a queen's children, 55% of an envoy's | gathers, farms, mines |
| **Envoy** | 7% of a queen's children; 2% of a worker's; 2% of a soldier's; 20% of an envoy's | always wants talks; carries treaties |
| **Priest** | 15% of an envoy's children | casts spells; learns them freely |
| **Shaman** | 5% of a priest's children | learns spells, carries crystal |
| **Merchant** | 10% of an envoy's children | trades; negotiates at out[9] > 0.25 |
| **Builder** | given to a founder | starts with 20 extra stone and 5 metal |
| **Explorer** | given to a founder | ranges further |

**Roles are inherited from the parent**, with the exceptions above. In a
**succession crisis** a colony raises queens far more readily — 35% of a
queen's children, 30% of a worker's, 22% of a soldier's — because without
a queen there is nobody to organise the colony, and this is what ends the
crisis.

The ZORGs have their own rules: their first is the queen of the cave, and
their children are 63% soldiers, 25% workers and 12% queens.

### 13.4 The landmark bonuses

| Landmark | Gives | Effect |
|----------|-------|--------|
| Crystal Grove | mana | 2× mana regeneration |
| Hot Spring | warmth | the cold does not bite |
| Bone Field | hunting | easier hunting |
| Fertile Soil | food | food is plentiful |
| Old Ruin | craft | faster crafting |
| Fungal Warren | plague | sickness breeds here |
| Salt Flat | purity | the ZORG filth cannot take hold |
| Sunken Temple | lore | discoveries come 34% instead of 22% |
| Amber Field | stone | buildings rise faster |
| Whispering Stone | mind | plasticity ×2 |
| Iron Womb | metal | mining yields double metal |
| Stormspire | energy | +mana, +stamina |
| Undergrove | air | the air is fresh everywhere in the grove |

### 13.5 The weather

| Weather | Speed | Energy | Fear | Vision | Temperature | Vision factor |
|---------|-------|--------|------|--------|-------------|---------------|
| Clear | 1.00 | 1.0 | 0 | 1.00 | 0 | 1.00 |
| Rain | 0.90 | 1.3 | 0 | 1.00 | 0 | 1.00 |
| Storm | 0.72 | 2.1 | 8 | 0.80 | −4 | 0.80 |
| Snow | 0.84 | 1.7 | 2 | 0.85 | −6 | 0.85 |
| Meteor | 0.90 | 1.2 | 22 | 0.90 | 0 | 0.90 |
| Fog | 0.88 | 1.1 | 3 | 1.00 | −2 | 0.45 |
| Heatwave | 0.95 | 1.6 | 5 | 1.00 | +11 | 1.00 |
| Sandstorm | 0.78 | 1.5 | 6 | 1.00 | +4 | 0.50 |

### 13.6 The depth zones

| Zone | Depth below the surface | Character |
|------|------------------------|-----------|
| Shallows | 0–58 | ordinary rock, the odd seam of ore |
| Crystal Depths | 58–104 | crystal, gem, diamond |
| Sulphur Works | 104–150 | sulfur, acid, gas |
| Bone Galleries | 150–215 | bone, old ruins |
| Magma Layer | 215+ | lava, magma, the volcano's roots |

The bands are measured **from the terrain**, never from an absolute row:
the deep has to be the deep wherever the ground happens to be.

### 13.7 Performance

Measured with `cargo run --release --bin prof`:

| Stage | Cost |
|-------|------|
| World step | ~0.57 ms |
| Creature step | ~3.29 ms |
| Render region (what is on screen) | ~1.67 ms |
| Render whole world | ~12.6 ms |

The creature step is dominated by the brain, and by the perception scan
that feeds it: eight rays of thirty cells for the nose, every other tick.
The input-major weight layout and the two-pass layer computation are what
keep it affordable.

---

## 14. How to read the brain at runtime

The HUD shows the leading lineage's mind on the line below its traits:

```text
lineage 42 evolved: Keen Senses, Industrious   mind: tier 2 (88-64-40 neurons, tanh firing)
```

The tier is the brain's size, the three numbers are the hidden layer
sizes, and the word is the activation function.

The two headless checks that guard this machinery are:

- `cargo run --release --bin braincheck -- 12000` — that the senses are
  primitive, that a mind learns to want what it smells, that the colony
  feeds itself, and that the colony does not go extinct.
- `cargo run --release --bin aircheck -- 3000` — that the undergroves are
  deep, that the open air beats the deep, that sealed fires go out, and
  that stale air costs something.

---

*Document generated from the PRIMORDIA source tree. Every number in this
document was read from the code, not estimated.*
