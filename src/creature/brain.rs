use super::*;

/// The distance from a point to a line segment.
///
/// Used for swept projectile hits: a shot that passes through a creature
/// between two ticks still hits it.
pub(crate) fn segment_distance(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

// ---------------------------------------------------------------------------
// The brain: a small network with a memory
// ---------------------------------------------------------------------------

/// How many units the recurrent memory holds.
///
/// The memory is what the creature was thinking last time it thought. It is
/// fed back into the first layer, so a brain can act on what it saw a
/// while ago instead of only on what is in front of it right now: it can
/// keep going somewhere, remember a threat, or learn that a place is
/// worth coming back to.
pub const MEMORY: usize = 12;

/// How much of the old memory survives a thought.
///
/// Most of it carries over, so the memory is a slow trail rather than a
/// flicker.
const MEMORY_DECAY: f32 = 0.68;

/// The biggest a brain can get.
///
/// A brain is not fixed for ever: a species that works out how to think
/// better grows a bigger network, and a young animal can be born with a
/// slightly different shape from its mother. The three tiers are the
/// shapes a brain can have.
pub const MAX_ARCH: u8 = 2;

// ---------------------------------------------------------------------------
// Activation functions
// ---------------------------------------------------------------------------

/// The functions a hidden layer can use.
///
/// The choice is part of the animal's breeding: two species with the same
/// senses and the same needs still think differently, because one of them
/// fires on a hard threshold and the other one leaks. The functions are
/// cheap approximations, so a species that picks an expensive one does not
/// slow the whole world down.
pub mod activation {
    /// The threshold: nothing below zero gets through.
    pub const RELU: u8 = 0;
    /// The smooth squasher.
    pub const TANH: u8 = 1;
    /// The soft threshold: everything gets through, but the negatives are
    /// damped rather than cut off.
    pub const LEAKY: u8 = 2;
    /// The wave: useful for things that come in cycles.
    pub const SINE: u8 = 3;
    /// The saturating switch.
    pub const SIGMOID: u8 = 4;
    /// The smooth gate: it lets most of the signal through and only closes
    /// the door on the clearly negative.
    pub const GELU: u8 = 5;
    /// How many there are.
    pub const COUNT: u8 = 6;

    /// The name shown in the interface.
    pub fn name(a: u8) -> &'static str {
        match a {
            TANH => "tanh",
            LEAKY => "leaky",
            SINE => "sine",
            SIGMOID => "sigmoid",
            GELU => "smooth",
            _ => "threshold",
        }
    }

    /// Apply the function.
    ///
    /// The squashing functions use the cheap `x / (1 + |x|)` shape instead
    /// of the library's `tanh`, because these run for every neuron of every
    /// creature that thinks, and the difference in the curve does not
    /// matter to an animal.
    #[inline]
    pub fn apply(a: u8, x: f32) -> f32 {
        match a {
            TANH => x / (1.0 + x.abs()),
            LEAKY => if x > 0.0 { x } else { x * 0.1 },
            SINE => (x * 0.6).sin(),
            SIGMOID => {
                let s = x / (1.0 + x.abs());
                0.5 + 0.5 * s
            }
            GELU => {
                // A smooth gate: the cheap cousin of the smooth gate every
                // modern network uses.
                let s = x / (1.0 + x.abs());
                0.5 * x * (1.0 + s)
            }
            _ => x.max(0.0),
        }
    }
}

/// The shape of a creature's brain.
///
/// The first layer takes the senses and the memory together; the layers
/// after it are hidden layers of decreasing size, so the creature can work
/// out things that need several steps of reasoning; the last layer is the
/// decision.
pub fn brain_sizes() -> [usize; 5] {
    brain_sizes_for(0)
}

/// The shape of a brain at a given tier.
///
/// A bigger brain has more neurons in its hidden layers: it can hold more
/// of the world in mind at once. The first and last layers stay the same,
/// because the senses and the decisions do not change - only what happens
/// in between.
pub fn brain_sizes_for(arch: u8) -> [usize; 5] {
    match arch {
        0 => [super::INPUTS + MEMORY, 36, 24, 16, super::OUTPUTS],
        1 => [super::INPUTS + MEMORY, 48, 32, 20, super::OUTPUTS],
        _ => [super::INPUTS + MEMORY, 60, 40, 28, super::OUTPUTS],
    }
}

pub struct Net {
    sizes: Vec<usize>,
    /// The weights, one vector per layer.
    ///
    /// They are stored **input-major**: the weights of one input sit next
    /// to each other. That is the layout that lets the compiler work on
    /// several outputs at once when it adds up a layer, and it makes the
    /// forward pass about three times quicker than the other way round.
    w: Vec<Vec<f32>>,
    b: Vec<Vec<f32>>,
    /// The recurrent memory: what the creature was thinking last time.
    memory: Vec<f32>,
    /// How big the brain is. A species that works out how to think better
    /// raises this for everybody.
    pub arch: u8,
    /// Which function the hidden layers fire with.
    pub activation: u8,
    /// What fed into the last layer, kept so the brain can learn from what
    /// worked. It is the memory of the last thought in weight form.
    last_in: Vec<f32>,
    /// What each decision came out as, kept for the same reason.
    last_out: Vec<f32>,
}

impl Net {
    pub fn random(sizes: &[usize], rng: &mut Rng) -> Self {
        let mut w = Vec::new();
        let mut b = Vec::new();
        for layer in 0..sizes.len() - 1 {
            let fan_in = sizes[layer];
            let fan_out = sizes[layer + 1];
            let scale = (2.0 / (fan_in + fan_out) as f32).sqrt() * 2.2;
            // Input-major: the weights of one input sit together.
            let mut ws = Vec::with_capacity(fan_in * fan_out);
            for _ in 0..fan_in {
                for _ in 0..fan_out {
                    ws.push((rng.f32() * 2.0 - 1.0) * scale);
                }
            }
            let mut bs = Vec::with_capacity(fan_out);
            for _ in 0..fan_out {
                bs.push((rng.f32() * 2.0 - 1.0) * 0.1);
            }
            w.push(ws);
            b.push(bs);
        }
        Net {
            sizes: sizes.to_vec(),
            w,
            b,
            memory: vec![0.0; MEMORY],
            arch: 0,
            // Every species starts on a different foot: the function is
            // picked at random and then bred like anything else. Most
            // animals start with the plain threshold, because it is the one
            // a brain of this size can use best; the others are the odd
            // ones out, and they have to earn their keep.
            activation: if rng.chance(0.4) {
                activation::RELU
            } else {
                1 + rng.range(activation::COUNT as u32 - 1) as u8
            },
            last_in: Vec::new(),
            last_out: Vec::new(),
        }
    }

    pub fn clone_net(&self) -> Net {
        Net {
            sizes: self.sizes.clone(),
            w: self.w.iter().map(|r| r.clone()).collect(),
            b: self.b.iter().map(|r| r.clone()).collect(),
            // A young one starts with the state of mind its mother had.
            memory: self.memory.clone(),
            arch: self.arch,
            activation: self.activation,
            last_in: self.last_in.clone(),
            last_out: self.last_out.clone(),
        }
    }

    pub fn forward(&mut self, input: &[f32]) -> Vec<f32> {
        let mut act = Vec::with_capacity(input.len() + MEMORY);
        let mut next = Vec::with_capacity(self.sizes[self.sizes.len() - 1]);
        self.forward_into(input, &mut act, &mut next);
        act
    }

    /// Allocation-free forward pass: the caller reuses the scratch buffers.
    ///
    /// The memory is appended to the senses and updated from the last
    /// hidden layer, so the network is recurrent: what it decides now
    /// depends on what it was doing a moment ago.
    pub fn forward_into(&mut self, input: &[f32], act: &mut Vec<f32>, next: &mut Vec<f32>) {
        act.clear();
        act.extend_from_slice(input);
        act.extend_from_slice(&self.memory);
        let last = self.w.len() - 1;
        let act_kind = self.activation;
        for layer in 0..self.w.len() {
            let out_n = self.sizes[layer + 1];
            let in_n = self.sizes[layer];
            let ws = &self.w[layer];
            let bs = &self.b[layer];
            next.clear();
            next.resize(out_n, 0.0);
            // ---- The layer ----
            //
            // The weights are stored input-major, so the layer is worked
            // out by walking the inputs and adding each one's contribution
            // to every output. The compiler can do several outputs at once
            // this way, and an input that is exactly zero is skipped
            // altogether - and plenty of the senses are zero most of the
            // time.
            for o in 0..out_n {
                next[o] = bs[o];
            }
            for j in 0..in_n {
                let a = act[j];
                if a == 0.0 {
                    continue;
                }
                let row = &ws[j * out_n..(j + 1) * out_n];
                // The two are walked together, so there is no bounds check
                // in the loop and the compiler can work on several outputs
                // at once.
                for (n, w) in next.iter_mut().zip(row.iter()) {
                    *n += *w * a;
                }
            }
            // ---- The activation ----
            //
            // It is a second, separate pass, so the dot products above stay
            // a plain loop and the squashing stays a plain loop too.
            if layer == last {
                // The last layer always squashes into -1..1, so the
                // decisions are comparable whatever the hidden layers did.
                for v in next.iter_mut() {
                    *v /= 1.0 + v.abs();
                }
            } else {
                match act_kind {
                    activation::RELU => {
                        for v in next.iter_mut() {
                            *v = v.max(0.0);
                        }
                    }
                    activation::TANH => {
                        for v in next.iter_mut() {
                            *v /= 1.0 + v.abs();
                        }
                    }
                    activation::LEAKY => {
                        for v in next.iter_mut() {
                            *v = if *v > 0.0 { *v } else { *v * 0.1 };
                        }
                    }
                    activation::SINE => {
                        for v in next.iter_mut() {
                            *v = (*v * 0.6).sin();
                        }
                    }
                    activation::SIGMOID => {
                        for v in next.iter_mut() {
                            let s = *v / (1.0 + v.abs());
                            *v = 0.5 + 0.5 * s;
                        }
                    }
                    _ => {
                        for v in next.iter_mut() {
                            let s = *v / (1.0 + v.abs());
                            *v = 0.5 * *v * (1.0 + s);
                        }
                    }
                }
            }
            std::mem::swap(act, next);
        }
        // ---- The memory ----
        //
        // After the last swap `next` holds the last hidden layer, which is
        // what the memory is built from. The memory is bounded, so it can
        // never run away with the network.
        let hidden: &[f32] = if next.is_empty() { &[] } else { &next };
        for i in 0..MEMORY {
            let src = if hidden.is_empty() {
                0.0
            } else {
                hidden[i % hidden.len()]
            };
            let v = (src * 0.3).tanh();
            self.memory[i] = self.memory[i] * MEMORY_DECAY + v * (1.0 - MEMORY_DECAY);
        }
        // ---- What the brain learned from ----
        //
        // The inputs of the last layer and the values it produced are kept
        // so that a decision which worked can be strengthened later on.
        self.last_in.clear();
        self.last_in.extend_from_slice(hidden);
        self.last_out.clear();
        self.last_out.extend_from_slice(act);
    }

    /// The memory of the brain, for the tooltip.
    pub fn memory(&self) -> &[f32] {
        &self.memory
    }

    /// How big the brain is, and what shape it is in.
    pub fn shape(&self) -> (usize, usize, usize) {
        (
            self.sizes[1],
            self.sizes[2],
            self.sizes[3],
        )
    }

    /// How many weights the brain holds. Used to show the cost of a mind.
    pub fn weight_count(&self) -> usize {
        self.w.iter().map(|r| r.len()).sum()
    }

    pub fn mutate(&mut self, rng: &mut Rng, rate: f32, strength: f32) {
        for ws in &mut self.w {
            for v in ws.iter_mut() {
                if rng.chance(rate) {
                    *v += (rng.f32() * 2.0 - 1.0) * strength;
                }
            }
        }
        for bs in &mut self.b {
            for v in bs.iter_mut() {
                if rng.chance(rate) {
                    *v += (rng.f32() * 2.0 - 1.0) * strength;
                }
            }
        }
        // The function the brain fires with is bred too, but rarely: a
        // species that keeps changing its mind does not learn anything.
        if rng.chance(0.02) {
            self.activation = rng.range(activation::COUNT as u32) as u8;
        }
    }

    // ------------------------------------------------------------------
    // Growth
    // ------------------------------------------------------------------

    /// Grow the brain to a given tier, keeping what it already knows.
    ///
    /// The old weights are copied into the new, larger layers, so the
    /// animal does not start again from nothing when the colony works out
    /// how to think better. The new neurons get fresh random weights: they
    /// are the part of the brain that has to find out what it is for.
    pub fn grow_to(&mut self, arch: u8, rng: &mut Rng) -> bool {
        if arch <= self.arch {
            return false;
        }
        let target = brain_sizes_for(arch);
        self.resize_to(&target, rng);
        self.arch = arch;
        true
    }

    /// Add or take away a single neuron in one of the hidden layers.
    ///
    /// This is how the shape of a brain drifts down the generations: a
    /// child is born with a slightly different mind from its mother, and
    /// the shape that turns out to work is the one that gets bred.
    pub fn mutate_shape(&mut self, rng: &mut Rng) -> bool {
        // Only the hidden layers move: the senses in and the decisions out
        // are fixed, or the animal could not read the world at all.
        let layer = 1 + rng.range(2) as usize;
        let mut sizes = self.sizes.clone();
        let grow = rng.chance(0.5);
        let n = sizes[layer];
        if grow {
            sizes[layer] = n + 1;
        } else if n > 8 {
            sizes[layer] = n - 1;
        } else {
            return false;
        }
        self.resize_to(&sizes, rng);
        true
    }

    /// Rebuild the network with a new shape, copying the weights that
    /// still fit and making fresh ones for the rest.
    fn resize_to(&mut self, sizes: &[usize], rng: &mut Rng) {
        let old_sizes = self.sizes.clone();
        let mut w = Vec::with_capacity(sizes.len() - 1);
        let mut b = Vec::with_capacity(sizes.len() - 1);
        for layer in 0..sizes.len() - 1 {
            let in_n = sizes[layer];
            let out_n = sizes[layer + 1];
            let scale = (2.0 / (in_n + out_n) as f32).sqrt() * 2.2;
            let mut ws = vec![0.0f32; in_n * out_n];
            let mut bs = vec![0.0f32; out_n];
            let old_in = old_sizes.get(layer).copied().unwrap_or(0);
            let old_out = old_sizes.get(layer + 1).copied().unwrap_or(0);
            let had_old = layer < self.w.len();
            let old_ws: &[f32] = if had_old { &self.w[layer] } else { &[] };
            let old_bs: &[f32] = if had_old { &self.b[layer] } else { &[] };
            for o in 0..out_n {
                for j in 0..in_n {
                    // The old weights were stored output-major and the new
                    // ones are stored input-major, so the copy moves one
                    // weight at a time.
                    let v = if o < old_out && j < old_in && had_old {
                        old_ws[o * old_in + j]
                    } else {
                        (rng.f32() * 2.0 - 1.0) * scale
                    };
                    ws[j * out_n + o] = v;
                }
                bs[o] = if o < old_out && had_old {
                    old_bs[o]
                } else {
                    (rng.f32() * 2.0 - 1.0) * 0.1
                };
            }
            w.push(ws);
            b.push(bs);
        }
        self.sizes = sizes.to_vec();
        self.w = w;
        self.b = b;
        // What the mind remembers is about the shape it had, so it is
        // forgotten when the shape changes.
        self.last_in.clear();
        self.last_out.clear();
    }

    // ------------------------------------------------------------------
    // Learning inside a life
    // ------------------------------------------------------------------

    /// Strengthen the decision that worked.
    ///
    /// When an animal eats, builds or wins a fight, the decision that led
    /// to it is pushed a little further in that direction: the weights
    /// that fed it grow, so the same situation is more likely to end the
    /// same way. It is a slow pull, not a rule, and it only touches the
    /// last layer, so it costs almost nothing.
    pub fn reinforce(&mut self, out_idx: usize, amount: f32) {
        if self.w.is_empty() {
            return;
        }
        let last = self.w.len() - 1;
        let in_n = self.sizes[last];
        let out_n = self.sizes[last + 1];
        if out_idx >= out_n {
            return;
        }
        let o = match self.last_out.get(out_idx) {
            Some(v) => *v,
            None => return,
        };
        if o == 0.0 {
            return;
        }
        // The mind may have been reshaped since it last thought, in which
        // case what it remembers is shorter than the layer it feeds. Only
        // the part that still lines up is touched.
        let n = in_n.min(self.last_in.len());
        let ws = &mut self.w[last];
        for j in 0..n {
            let a = self.last_in[j];
            // The weights are input-major, so the weight of one output is
            // spread through the layer, one every out_n weights.
            let v = ws[j * out_n + out_idx] + amount * o * a;
            ws[j * out_n + out_idx] = v.clamp(-6.0, 6.0);
        }
    }

    /// Let the weights of a brain settle down a little.
    ///
    /// Without this a brain that is reinforced for a whole life would end
    /// up with a handful of enormous weights and nothing else.
    pub fn settle(&mut self, rate: f32) {
        for ws in &mut self.w {
            for v in ws.iter_mut() {
                *v *= 1.0 - rate;
            }
        }
    }
}

