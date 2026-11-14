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
///
/// The hidden layers are wide on purpose. The senses are primitive now —
/// colour and light and smell, with no answers attached — so the brain has
/// to do the interpreting itself, and it needs the room.
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
        0 => [super::INPUTS + MEMORY, 56, 40, 24, super::OUTPUTS],
        1 => [super::INPUTS + MEMORY, 72, 52, 32, super::OUTPUTS],
        _ => [super::INPUTS + MEMORY, 88, 64, 40, super::OUTPUTS],
    }
}

/// How many past thoughts the brain keeps.
///
/// A creature that eats the food it was looking at has to connect the
/// reward to a decision it made a moment ago, not to the decision it
/// happens to be making now. These are the last few thoughts, kept so the
/// reward can be pushed back through all of them, the nearest one hardest.
const TRACE_LEN: usize = 4;

/// How much of a trace is left after one step of time.
///
/// The further back a decision was, the less it deserves the credit.
const TRACE_DECAY: f32 = 0.72;

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
    /// The input of each layer from the last thought, kept so the brain
    /// can learn from what worked.
    learn_in: Vec<Vec<f32>>,
    /// The output of each layer from the last thought.
    learn_out: Vec<Vec<f32>>,
    /// What the last layer computed before it squashed it.
    learn_pre: Vec<f32>,
    /// The last few thoughts, so a reward can be pushed back through them.
    traces: Vec<Trace>,
    /// The direct path from the raw senses to the desires.
    ///
    /// One weight per sensory channel. The path is *spatially* aligned:
    /// the smell in direction three can only push the desire for
    /// direction three. It says nothing about what any of it means.
    ///
    /// This is what makes the world learnable. A mind that has to carry
    /// "there is something sweet over there" through three hidden layers
    /// before it can act on it will never get there in a lifetime, but a
    /// mind with a direct path from the nose to the feet learns it in a
    /// few hundred steps - and then evolution is free to sharpen, weaken
    /// or reverse it.
    pub skip: Vec<f32>,
    /// The pull of the open sky.
    ///
    /// Every animal can feel whether it is under the sky or shut in the
    /// rock. It is not knowledge of the world: it is knowing which way is
    /// up. Without it a creature wanders into the deep, where nothing
    /// grows, and starves in a place it cannot smell a meal from.
    pub skip_up: f32,
    /// What the senses were reading on the last thought.
    ///
    /// Kept so that the inspector can show the brain what it was looking
    /// at when it decided.
    last_input: Vec<f32>,
}

/// How fast a hidden unit is changing right now.
///
/// The learning rule needs to know how much a unit is listening, so that a
/// unit which is switched off is not blamed for what happened. The
/// derivative is worked out from the unit's own output, which is all that
/// is kept.
fn activation_deriv(kind: u8, y: f32) -> f32 {
    match kind {
        activation::RELU => {
            if y > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        activation::LEAKY => {
            if y > 0.0 {
                1.0
            } else {
                0.1
            }
        }
        activation::TANH => {
            // The output is y = x / (1 + |x|), so the slope is 1/(1+|x|)^2,
            // which is (1 - |y|)^2.
            let s = 1.0 - y.abs();
            s * s
        }
        activation::SIGMOID => {
            let s = (y - 0.5).abs() * 2.0;
            let s = 1.0 - s;
            0.5 * s * s
        }
        activation::GELU => {
            if y > 0.0 {
                1.0
            } else {
                0.1
            }
        }
        activation::SINE => 0.6 * (1.0 - y * y).max(0.0).sqrt(),
        _ => 1.0,
    }
}

/// One remembered thought: what each layer saw, and what it said.
#[derive(Clone)]
struct Trace {
    in_layers: Vec<Vec<f32>>,
    out_layers: Vec<Vec<f32>>,
    pre: Vec<f32>,
}

/// A snapshot of a mind, for the inspector.
pub struct MindView {
    /// The shape of the network.
    pub sizes: Vec<usize>,
    /// Which tier the mind is.
    pub arch: u8,
    /// Which function its hidden layers fire with.
    pub activation: u8,
    /// The direct path from the raw senses to the desires.
    pub skip: Vec<f32>,
    /// The pull of the open sky.
    pub skip_up: f32,
    /// The weights of the last layer: what each decision is made of.
    pub out_weights: Vec<f32>,
    /// How many hidden units feed the last layer.
    pub hidden: usize,
    /// How many decisions there are.
    pub outputs: usize,
    /// What each decision was on the last thought.
    pub last_out: Vec<f32>,
    /// What each sensory channel was reading on the last thought.
    pub channels: Vec<f32>,
    /// How hard each hidden unit of the last hidden layer was firing.
    pub hidden_acts: Vec<f32>,
    /// How many units each hidden layer has.
    pub hidden_layers: Vec<usize>,
    /// The weights of every layer, for the inspector.
    pub layers: Vec<Vec<f32>>,
    /// How hard each unit of each hidden layer was firing.
    pub layer_acts: Vec<Vec<f32>>,
}

impl MindView {
    /// The weight between one hidden unit and one decision.
    pub fn weight(&self, hidden_unit: usize, output: usize) -> f32 {
        self.out_weights
            .get(hidden_unit * self.outputs + output)
            .copied()
            .unwrap_or(0.0)
    }

    /// How strongly one decision was felt on the last thought.
    pub fn drive(&self, output: usize) -> f32 {
        self.last_out.get(output).copied().unwrap_or(0.0)
    }

    /// What one sensory channel is reading.
    ///
    /// This is the creature's own last impression, kept so that the
    /// inspector can show what the brain was looking at when it decided.
    pub fn channel_value(&self, direction: usize, channel: usize) -> f32 {
        self.channels
            .get(direction * crate::creature::senses::DIR_CHANNELS + channel)
            .copied()
            .unwrap_or(0.0)
    }

    /// How hard one hidden unit was firing.
    pub fn hidden_fire(&self, unit: usize) -> f32 {
        self.hidden_acts.get(unit).copied().unwrap_or(0.0)
    }

    /// How hard one unit of one hidden layer was firing.
    pub fn layer_fire(&self, layer: usize, unit: usize) -> f32 {
        self.layer_acts
            .get(layer)
            .and_then(|l| l.get(unit))
            .copied()
            .unwrap_or(0.0)
    }

    /// The strongest firing of a layer, so the picture can be scaled to it.
    pub fn layer_peak(&self, layer: usize) -> f32 {
        self.layer_acts
            .get(layer)
            .map(|l| l.iter().fold(0.0f32, |a, b| a.max(b.abs())))
            .unwrap_or(0.0)
    }

    /// The weight between two units of one layer.
    pub fn layer_weight(&self, layer: usize, from: usize, to: usize) -> f32 {
        let in_n = self.sizes[layer];
        let out_n = self.sizes[layer + 1];
        self.layers
            .get(layer)
            .and_then(|l| l.get(from * out_n + to))
            .copied()
            .unwrap_or(0.0)
    }

    /// How much one whole input column matters to one hidden unit.
    ///
    /// The input layer is drawn as channels, so the line from a direction
    /// to a hidden unit is the sum of the weights of that direction's
    /// channels.
    pub fn column_weight(&self, layer: usize, direction: usize, unit: usize) -> f32 {
        let in_n = self.sizes[layer];
        let out_n = self.sizes[layer + 1];
        let dir_ch = crate::creature::senses::DIR_CHANNELS;
        let base = direction * dir_ch;
        let mut sum = 0.0;
        for c in 0..dir_ch {
            let idx = base + c;
            if idx >= in_n {
                break;
            }
            sum += self
                .layers
                .get(layer)
                .and_then(|l| l.get(idx * out_n + unit))
                .copied()
                .unwrap_or(0.0);
        }
        sum
    }

    /// How strongly one hidden unit pushes on one unit of the next layer.
    ///
    /// This is the weight times how hard the unit was firing: the pull
    /// the connection is actually applying right now. It is what makes
    /// the colour of a line mean something - a positive pull is the unit
    /// arguing for the one it feeds, a negative pull is the unit
    /// arguing against it.
    pub fn unit_pull(&self, layer: usize, from: usize, to: usize) -> f32 {
        let out_n = self.sizes[layer + 1];
        let w = self
            .layers
            .get(layer)
            .and_then(|l| l.get(from * out_n + to))
            .copied()
            .unwrap_or(0.0);
        let a = self
            .layer_acts
            .get(layer.saturating_sub(1))
            .and_then(|l| l.get(from))
            .copied()
            .unwrap_or(0.0);
        w * a
    }

    /// How strongly one input pushes on one hidden unit.
    ///
    /// The same idea, from the senses: the weight times what the channel
    /// was reading.
    pub fn input_pull(&self, layer: usize, input: usize, unit: usize) -> f32 {
        let out_n = self.sizes[layer + 1];
        let w = self
            .layers
            .get(layer)
            .and_then(|l| l.get(input * out_n + unit))
            .copied()
            .unwrap_or(0.0);
        let v = self.channels.get(input).copied().unwrap_or(0.0);
        w * v
    }

    /// How strongly one whole column of inputs pushes on one hidden unit.
    ///
    /// The input layer is drawn as columns - a direction, the body, the
    /// kin - so the line that is drawn is the sum of the pulls of every
    /// input in that column.
    pub fn column_pull(&self, layer: usize, first: usize, count: usize, unit: usize) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.input_pull(layer, first + i, unit);
        }
        sum
    }

    /// How hard one unit of one layer is working right now.
    ///
    /// For a hidden layer this is how hard the unit is firing; for the
    /// decision layer it is how hard the decision is being pushed.
    pub fn unit_drive(&self, layer: usize, unit: usize) -> f32 {
        if layer + 1 >= self.sizes.len() {
            return self.drive(unit);
        }
        self.layer_acts
            .get(layer)
            .and_then(|l| l.get(unit))
            .copied()
            .unwrap_or(0.0)
    }
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
        let layers = sizes.len() - 1;
        let mut net = Net {
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
            learn_in: vec![Vec::new(); layers],
            learn_out: vec![Vec::new(); layers],
            learn_pre: Vec::new(),
            traces: Vec::new(),
            skip: vec![0.0; crate::creature::senses::DIR_CHANNELS],
            skip_up: 0.0,
            last_input: Vec::new(),
        };
        net.seed_instincts();
        net
    }


    /// The bias every animal is born with.
    ///
    /// A creature that has to discover "walk towards the sweet smell"
    /// from nothing, and then discover that the sweet smell means food,
    /// and then discover that walking to food is worth doing, and then
    /// discover that eating it stops the hunger - that is four lessons
    /// multiplied together, and a random mind will not get them all
    /// before it starves. Evolution cannot sharpen a bias that no
    /// individual ever lived long enough to express.
    ///
    /// So the newborn mind is given the bias that every real animal is
    /// born with: a creature that smells something sweet is inclined to
    /// walk towards it, and a creature that smells something acrid is
    /// inclined to walk away from it.
    ///
    /// **This lives in the weights, not in the code.** The action layer
    /// knows nothing about food: it moves the creature where the desires
    /// point, and the desires come out of the network. Evolution is free
    /// to strengthen, weaken or reverse this bias - and it does, because
    /// a lineage that walked into the poison and died left no children.
    ///
    /// The bias goes on the **skip weights**: the direct path from the
    /// raw senses to the desires. That path is *spatially* aligned - the
    /// smell in direction three can only push the desire for direction
    /// three - but it says nothing about what any of it means. The
    /// network still has to work out that sweet means worth eating.
    fn seed_instincts(&mut self) {
        let sweet = crate::creature::senses::smell::SWEET;
        let acid = crate::creature::senses::smell::ACID;
        // The smell of something sweet pulls. This is the food cue, and it
        // is the sharpest one there is: almost nothing else in the world
        // smells sweet.
        self.skip[sweet] = 1.15;
        // The smell of something acrid pushes away.
        self.skip[acid] = -0.9;
        // A green thing is worth a look, but only a little: the world is
        // full of green things that are not food, and a creature that
        // chased every one of them would never eat.
        self.skip[crate::creature::senses::sight::GREEN] = 0.08;
        // The smell of your own kind pulls. An animal that can smell its
        // own kind walks towards it: that is how a herd stays together,
        // and how a creature that has wandered off finds its way back.
        self.skip[crate::creature::senses::smell::KIN] = 0.9;
        // The open sky pulls. Every animal can feel whether it is under
        // the sky or shut in the rock, and every animal that lives on the
        // ground stays near it. This is not knowledge of the world: it is
        // knowing which way is up.
        self.skip_up = 0.55;
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
            learn_in: self.learn_in.iter().map(|r| r.clone()).collect(),
            learn_out: self.learn_out.iter().map(|r| r.clone()).collect(),
            learn_pre: self.learn_pre.clone(),
            traces: self.traces.clone(),
            skip: self.skip.clone(),
            skip_up: self.skip_up,
            last_input: self.last_input.clone(),
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
        let mut first = true;
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
            // entirely - and plenty of the senses are zero most of the
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
                self.learn_pre.clear();
                self.learn_pre.extend_from_slice(&next);
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
            // ---- Remember what happened ----
            //
            // The brain keeps what each layer saw and what it said, so
            // that a good outcome can be pushed back through the whole
            // network afterwards.
            self.learn_in[layer].clear();
            self.learn_in[layer].extend_from_slice(act);
            self.learn_out[layer].clear();
            self.learn_out[layer].extend_from_slice(next);
            std::mem::swap(act, next);
            if first {
                first = false;
            }
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
        // ---- The direct path ----
        //
        // The raw senses reach the desires without going through the
        // hidden layers. This is what makes the world learnable.
        self.apply_skip(input, act);
        // Remember what the senses were saying, so that the inspector can
        // show what the brain was looking at when it decided.
        self.last_input.clear();
        self.last_input.extend_from_slice(input);
        // ---- Keep this thought for the credit assignment ----
        //
        // A creature that eats the food it was looking at has to connect
        // the reward to a decision it made a moment ago. The last few
        // thoughts are kept so that the reward can be pushed back through
        // all of them, the nearest one hardest.
        self.traces.push(Trace {
            in_layers: self.learn_in.clone(),
            out_layers: self.learn_out.clone(),
            pre: self.learn_pre.clone(),
        });
        while self.traces.len() > TRACE_LEN {
            self.traces.remove(0);
        }
    }

    /// Add the direct path from the raw senses to the desires.
    ///
    /// The path is spatially aligned: the channels of direction three can
    /// only push the desire for direction three. It says nothing about
    /// what any of those channels mean.
    fn apply_skip(&self, input: &[f32], out: &mut [f32]) {
        let dir_ch = crate::creature::senses::DIR_CHANNELS;
        // The pull of the open sky, added to the direction that is up.
        let sky = self.skip_up * input[crate::creature::senses::BODY_BASE + crate::creature::senses::BODY_OPEN_SKY];
        for d in 0..crate::creature::senses::DIRS {
            let base = d * dir_ch;
            let mut v = 0.0f32;
            for c in 0..dir_ch {
                v += self.skip[c] * input[base + c];
            }
            if d == crate::creature::senses::DIR_UP {
                v += sky;
            }
            let idx = super::outputs::DESIRE + d;
            if idx < out.len() {
                // The desire is squashed like everything else, so a strong
                // pull still reads as a strong pull and never runs away.
                let x = out[idx] + v;
                out[idx] = x / (1.0 + x.abs());
            }
        }
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
        self.learn_in = vec![Vec::new(); sizes.len() - 1];
        self.learn_out = vec![Vec::new(); sizes.len() - 1];
        self.learn_pre.clear();
        self.traces.clear();
    }

    // ------------------------------------------------------------------
    // ------------------------------------------------------------------
    // Learning inside a life
    // ------------------------------------------------------------------

    /// Learn from an outcome.
    ///
    /// The outcome is given as a list of *(output, how much)* pairs: how
    /// much stronger each of those decisions should have been.
    ///
    /// Only the **last layer** is moved here. The hidden layer is left
    /// alone, because the situation the creature was in has already been
    /// written into it by the desire path (see `credit_desire`), which does
    /// reach all the way down to the raw senses. Pushing the reward
    /// through every layer as well is a lot of arithmetic for a lesson
    /// the network has already had.
    pub fn learn(&mut self, targets: &[(usize, f32)], lr: f32) {
        if targets.is_empty() || self.traces.is_empty() {
            return;
        }
        // The traces are taken out of the mind for the length of the
        // lesson, so that the weights can be moved while the traces are
        // read. Copying them would be a great deal of allocation for
        // nothing.
        let traces = std::mem::take(&mut self.traces);
        // The newest thought gets the full weight; the one before it gets
        // a little less, because a decision made a while ago is less
        // responsible for what happened now.
        let n = traces.len().min(2);
        for ti in (0..n).rev() {
            let age = (n - 1 - ti) as f32;
            let strength = TRACE_DECAY.powf(age);
            for (out, amount) in targets.iter() {
                self.apply_target(&traces[ti], *out, *amount * strength, lr, false);
            }
        }
        self.traces = traces;
    }

    /// Push one target back through one remembered thought.
    ///
    /// The error at the output is what we want it to have been, and it is
    /// walked back through the layers: each input of a layer is blamed in
    /// proportion to the weight between it and the output that was wrong.
    fn apply_target(&mut self, trace: &Trace, out_idx: usize, amount: f32, lr: f32, deep: bool) {
        let layers = self.w.len();
        let out_n = self.sizes[layers];
        if out_idx >= out_n || amount == 0.0 {
            return;
        }
        // The error at the output. The last layer squashes, so the amount
        // it is asked to move is divided by the squash to get the change
        // in the raw signal that is really wanted.
        let mut delta = vec![0.0f32; out_n];
        let out_deriv = 1.0 / (1.0 + trace.pre.get(out_idx).unwrap_or(&0.0).abs()).powi(2);
        delta[out_idx] = amount * out_deriv;

        // The first layer is the expensive one: it holds every sense.
        // A shallow update skips it.
        let bottom = if deep { 0 } else { layers.saturating_sub(2) };
        for layer in (bottom..layers).rev() {
            let in_n = self.sizes[layer];
            let out_n = self.sizes[layer + 1];
            let x_in = &trace.in_layers[layer];
            let y_out = &trace.out_layers[layer];

            // ---- Move the weights ----
            //
            // The error at an output times the input that fed it is how
            // much that weight should change.
            {
                let ws = &mut self.w[layer];
                for j in 0..in_n {
                    let x = x_in.get(j).copied().unwrap_or(0.0);
                    if x == 0.0 {
                        continue;
                    }
                    for o in 0..out_n {
                        let d = delta[o];
                        if d == 0.0 {
                            continue;
                        }
                        let idx = j * out_n + o;
                        let v = ws[idx] + lr * d * x;
                        ws[idx] = v.clamp(-6.0, 6.0);
                    }
                }
            }

            // ---- Pass the error back ----
            //
            // Each input of this layer is blamed in proportion to the
            // weight between it and the output that was wrong.
            let mut in_delta = vec![0.0f32; in_n];
            {
                let ws = &self.w[layer];
                for o in 0..out_n {
                    let d = delta[o];
                    if d == 0.0 {
                        continue;
                    }
                    for j in 0..in_n {
                        in_delta[j] += d * ws[j * out_n + o];
                    }
                }
            }
            // A hidden unit that is not firing cannot be part of what
            // happened, so it is not blamed for it.
            for j in 0..in_n {
                let y = y_out.get(j).copied().unwrap_or(0.0);
                in_delta[j] *= activation_deriv(self.activation, y);
            }
            delta = in_delta;
        }
    }

    /// Push the credit for a reward into one direction of desire.
    ///
    /// This is the part that makes the world learnable, and it is proper
    /// back-propagation rather than a nudge.
    ///
    /// When a creature eats, the direction the food was in gets the
    /// credit. That means the network is told: "produce a stronger desire
    /// for this direction *in a situation like this one*". The error is
    /// pushed back through the hidden layers, so the hidden unit that
    /// fires on "something sweet-smelling in that direction" is
    /// strengthened, and the weights that connect it to the smell channel
    /// are strengthened too.
    ///
    /// Over a life, and then over the generations, the network works out
    /// that the direction with the sweet smell in it is the one worth
    /// walking towards. That is the whole learning, and nothing tells it.
    pub fn credit_desire(&mut self, dir: usize, amount: f32, lr: f32) {
        if self.traces.is_empty() || amount == 0.0 {
            return;
        }
        let out_idx = super::outputs::DESIRE + dir;
        if out_idx >= self.sizes[self.sizes.len() - 1] {
            return;
        }
        // The lesson is taken on the newest thought, all the way down to
        // the raw senses: this is the path that has to learn "the sweet
        // smell over there means walk over there".
        let trace = self.traces.last().unwrap().clone();
        self.apply_target(&trace, out_idx, amount, lr, true);
    }

    /// What the mind is thinking, for the inspector.
    ///
    /// This is a snapshot of the brain: the weights that matter, the
    /// activations of the last thought, and the shape of the thing. It is
    /// only built when somebody is looking at it.
    pub fn inspect(&self) -> MindView {
        MindView {
            sizes: self.sizes.clone(),
            arch: self.arch,
            activation: self.activation,
            skip: self.skip.clone(),
            skip_up: self.skip_up,
            /// The weights of the last layer: what each decision is made
            /// of.
            out_weights: self
                .w
                .last()
                .map(|l| l.clone())
                .unwrap_or_default(),
            /// The size of the last layer's input, so the reader can work
            /// out which hidden unit each weight belongs to.
            hidden: self.sizes[self.sizes.len() - 2],
            outputs: self.sizes[self.sizes.len() - 1],
            /// The activation of each output from the last thought.
            last_out: self
                .learn_out
                .last()
                .cloned()
                .unwrap_or_default(),
            /// What the senses were reading on the last thought.
            channels: self.last_input.clone(),
            /// How hard the last hidden layer was firing.
            hidden_acts: self
                .learn_out
                .get(self.learn_out.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default(),
            /// How many units each hidden layer has.
            hidden_layers: self.sizes[1..self.sizes.len() - 1].to_vec(),
            /// The weights of every layer, for the inspector.
            layers: self.w.clone(),
            /// How hard each unit of each hidden layer was firing.
            layer_acts: self.learn_out.clone(),
        }
    }


    /// Let the weights of a brain settle down a little.
    pub fn settle(&mut self, rate: f32) {
        for ws in &mut self.w {
            for v in ws.iter_mut() {
                *v *= 1.0 - rate;
            }
        }
    }
}
