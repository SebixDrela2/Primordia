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
// Tiny feed-forward network
// ---------------------------------------------------------------------------

pub struct Net {
    sizes: Vec<usize>,
    w: Vec<Vec<f32>>,
    b: Vec<Vec<f32>>,
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
        }
    }

    pub fn clone_net(&self) -> Net {
        Net {
            sizes: self.sizes.clone(),
            w: self.w.iter().map(|r| r.clone()).collect(),
            b: self.b.iter().map(|r| r.clone()).collect(),
        }
    }

    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut act = Vec::with_capacity(input.len());
        let mut next = Vec::with_capacity(self.sizes[self.sizes.len() - 1]);
        self.forward_into(input, &mut act, &mut next);
        act
    }

    /// Allocation-free forward pass: the caller reuses the scratch buffers.
    pub fn forward_into(&self, input: &[f32], act: &mut Vec<f32>, next: &mut Vec<f32>) {
        act.clear();
        act.extend_from_slice(input);
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
