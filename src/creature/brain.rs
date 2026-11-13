use super::*;

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

/// The shape of a creature's brain.
///
/// The first layer takes the senses and the memory together; the layers
/// after it are hidden layers of decreasing size, so the creature can work
/// out things that need several steps of reasoning; the last layer is the
/// decision.
pub fn brain_sizes() -> [usize; 5] {
    [super::INPUTS + MEMORY, 36, 24, 16, super::OUTPUTS]
}

pub struct Net {
    sizes: Vec<usize>,
    w: Vec<Vec<f32>>,
    b: Vec<Vec<f32>>,
    /// The recurrent memory: what the creature was thinking last time.
    memory: Vec<f32>,
}

impl Net {
    pub fn random(sizes: &[usize], rng: &mut Rng) -> Self {
        let mut w = Vec::new();
        let mut b = Vec::new();
        for layer in 0..sizes.len() - 1 {
            let fan_in = sizes[layer];
            let fan_out = sizes[layer + 1];
            let scale = (2.0 / (fan_in + fan_out) as f32).sqrt() * 2.2;
            let mut ws = Vec::with_capacity(fan_in * fan_out);
            for _ in 0..fan_in * fan_out {
                ws.push((rng.f32() * 2.0 - 1.0) * scale);
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
        }
    }

    pub fn clone_net(&self) -> Net {
        Net {
            sizes: self.sizes.clone(),
            w: self.w.iter().map(|r| r.clone()).collect(),
            b: self.b.iter().map(|r| r.clone()).collect(),
            // A young one starts with the state of mind its mother had.
            memory: self.memory.clone(),
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
        for layer in 0..self.w.len() {
            let out_n = self.sizes[layer + 1];
            let in_n = self.sizes[layer];
            let ws = &self.w[layer];
            let bs = &self.b[layer];
            next.clear();
            next.resize(out_n, 0.0);
            for o in 0..out_n {
                let row = &ws[o * in_n..(o + 1) * in_n];
                // Eight independent accumulators: float adds cannot be
                // re-associated automatically, so we break the dependency
                // chain by hand. This roughly halves the cost of the network.
                let mut a0 = 0.0f32;
                let mut a1 = 0.0f32;
                let mut a2 = 0.0f32;
                let mut a3 = 0.0f32;
                let mut a4 = 0.0f32;
                let mut a5 = 0.0f32;
                let mut a6 = 0.0f32;
                let mut a7 = 0.0f32;
                let mut i = 0usize;
                let mut chunks = row.chunks_exact(8);
                for ch in &mut chunks {
                    a0 += ch[0] * act[i];
                    a1 += ch[1] * act[i + 1];
                    a2 += ch[2] * act[i + 2];
                    a3 += ch[3] * act[i + 3];
                    a4 += ch[4] * act[i + 4];
                    a5 += ch[5] * act[i + 5];
                    a6 += ch[6] * act[i + 6];
                    a7 += ch[7] * act[i + 7];
                    i += 8;
                }
                let mut sum = bs[o] + ((a0 + a1) + (a2 + a3)) + ((a4 + a5) + (a6 + a7));
                for j in i..in_n {
                    sum += act[j] * row[j];
                }
                next[o] = if layer == last { sum.tanh() } else { sum.max(0.0) };
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
    }

    /// The memory of the brain, for the tooltip.
    pub fn memory(&self) -> &[f32] {
        &self.memory
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
    }
}

// Name generator
