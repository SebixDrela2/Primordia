use super::Rng;

#[must_use]
pub fn segment_distance(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
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

pub const MEMORY: usize = 12;

const MEMORY_DECAY: f32 = 0.68;

pub const MAX_ARCH: u8 = 2;

pub mod activation {

    pub const RELU: u8 = 0;

    pub const TANH: u8 = 1;

    pub const LEAKY: u8 = 2;

    pub const SINE: u8 = 3;

    pub const SIGMOID: u8 = 4;

    pub const GELU: u8 = 5;

    pub const COUNT: u8 = 6;

    #[must_use]
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

    #[inline]
    #[must_use]
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

                let s = x / (1.0 + x.abs());
                0.5 * x * (1.0 + s)
            }
            _ => x.max(0.0),
        }
    }
}

#[must_use]
pub fn brain_sizes() -> [usize; 5] {
    brain_sizes_for(0)
}

#[must_use]
pub fn brain_sizes_for(arch: u8) -> [usize; 5] {
    match arch {
        0 => [super::INPUTS + MEMORY, 56, 40, 24, super::OUTPUTS],
        1 => [super::INPUTS + MEMORY, 72, 52, 32, super::OUTPUTS],
        _ => [super::INPUTS + MEMORY, 88, 64, 40, super::OUTPUTS],
    }
}

const TRACE_LEN: usize = 4;

const TRACE_DECAY: f32 = 0.72;

pub struct Net {
    sizes: Vec<usize>,

    w: Vec<Vec<f32>>,
    b: Vec<Vec<f32>>,

    memory: Vec<f32>,

    pub arch: u8,

    pub activation: u8,

    learn_in: Vec<Vec<f32>>,

    learn_out: Vec<Vec<f32>>,

    learn_pre: Vec<f32>,

    traces: Vec<Trace>,

    pub skip: Vec<f32>,

    pub skip_up: f32,

    last_input: Vec<f32>,
}

fn activation_deriv(kind: u8, y: f32) -> f32 {
    match kind {
        activation::RELU => {
            if y > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        activation::LEAKY | activation::GELU => {
        if y > 0.0 {
        1.0
        } else {
        0.1
        }
        }
        activation::TANH => {

            let s = 1.0 - y.abs();
            s * s
        }
        activation::SIGMOID => {
            let s = (y - 0.5).abs() * 2.0;
            let s = 1.0 - s;
            0.5 * s * s
        }
        activation::SINE => 0.6 * (1.0 - y * y).max(0.0).sqrt(),
        _ => 1.0,
    }
}

#[derive(Clone)]
struct Trace {
    in_layers: Vec<Vec<f32>>,
    out_layers: Vec<Vec<f32>>,
    pre: Vec<f32>,
}

pub struct MindView {

    pub sizes: Vec<usize>,

    pub arch: u8,

    pub activation: u8,

    pub skip: Vec<f32>,

    pub skip_up: f32,

    pub out_weights: Vec<f32>,

    pub hidden: usize,

    pub outputs: usize,

    pub last_out: Vec<f32>,

    pub channels: Vec<f32>,

    pub hidden_acts: Vec<f32>,

    pub hidden_layers: Vec<usize>,

    pub layers: Vec<Vec<f32>>,

    pub layer_acts: Vec<Vec<f32>>,
}

impl MindView {

    #[must_use]
    pub fn weight(&self, hidden_unit: usize, output: usize) -> f32 {
        self.out_weights
            .get(hidden_unit * self.outputs + output)
            .copied()
            .unwrap_or(0.0)
    }

    #[must_use]
    pub fn drive(&self, output: usize) -> f32 {
        self.last_out.get(output).copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn channel_value(&self, direction: usize, channel: usize) -> f32 {
        self.channels
            .get(direction * crate::creature::senses::DIR_CHANNELS + channel)
            .copied()
            .unwrap_or(0.0)
    }

    #[must_use]
    pub fn hidden_fire(&self, unit: usize) -> f32 {
        self.hidden_acts.get(unit).copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn layer_fire(&self, layer: usize, unit: usize) -> f32 {
        self.layer_acts
            .get(layer)
            .and_then(|l| l.get(unit))
            .copied()
            .unwrap_or(0.0)
    }

    #[must_use]
    pub fn layer_peak(&self, layer: usize) -> f32 {
        self.layer_acts
            .get(layer)
            .map_or(0.0, |l| l.iter().fold(0.0f32, |a, b| a.max(b.abs())))
    }

    #[must_use]
    pub fn layer_weight(&self, layer: usize, from: usize, to: usize) -> f32 {
        let _ = self.sizes[layer];
        let out_n = self.sizes[layer + 1];
        self.layers
            .get(layer)
            .and_then(|l| l.get(from * out_n + to))
            .copied()
            .unwrap_or(0.0)
    }

    #[must_use]
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

    #[must_use]
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

    #[must_use]
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

    #[must_use]
    pub fn column_pull(&self, layer: usize, first: usize, count: usize, unit: usize) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.input_pull(layer, first + i, unit);
        }
        sum
    }

    #[must_use]
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
    #[must_use]
    pub fn random(sizes: &[usize], rng: &mut Rng) -> Self {
        let mut w = Vec::new();
        let mut b = Vec::new();
        for layer in 0..sizes.len() - 1 {
            let fan_in = sizes[layer];
            let fan_out = sizes[layer + 1];
            let scale = (2.0 / (fan_in + fan_out) as f32).sqrt() * 2.2;

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

    fn seed_instincts(&mut self) {
        let sweet = crate::creature::senses::smell::SWEET;
        let acid = crate::creature::senses::smell::ACID;

        self.skip[sweet] = 1.15;

        self.skip[acid] = -0.9;

        self.skip[crate::creature::senses::sight::GREEN] = 0.08;

        self.skip[crate::creature::senses::smell::KIN] = 0.15;

        self.skip_up = 0.55;
    }

    #[must_use]
    pub fn clone_net(&self) -> Net {
        Net {
            sizes: self.sizes.clone(),
            w: self.w.clone(),
            b: self.b.clone(),

            memory: self.memory.clone(),
            arch: self.arch,
            activation: self.activation,
            learn_in: self.learn_in.clone(),
            learn_out: self.learn_out.clone(),
            learn_pre: self.learn_pre.clone(),
            traces: self.traces.clone(),
            skip: self.skip.clone(),
            skip_up: self.skip_up,
            last_input: self.last_input.clone(),
        }
    }

    #[must_use]
    pub fn forward(&mut self, input: &[f32]) -> Vec<f32> {
        let mut act = Vec::with_capacity(input.len() + MEMORY);
        let mut next = Vec::with_capacity(self.sizes[self.sizes.len() - 1]);
        self.forward_into(input, &mut act, &mut next);
        act
    }

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

            next[..out_n].copy_from_slice(&bs[..out_n]);
            for j in 0..in_n {
                let a = act[j];
                if a == 0.0 {
                    continue;
                }
                let row = &ws[j * out_n..(j + 1) * out_n];

                for (n, w) in next.iter_mut().zip(row.iter()) {
                    *n += *w * a;
                }
            }

            if layer == last {

                self.learn_pre.clear();
                self.learn_pre.extend_from_slice(next);
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

            self.learn_in[layer].clear();
            self.learn_in[layer].extend_from_slice(act);
            self.learn_out[layer].clear();
            self.learn_out[layer].extend_from_slice(next);
            std::mem::swap(act, next);
            if first {
                first = false;
            }
        }

        let hidden: &[f32] = if next.is_empty() { &[] } else { next };
        for i in 0..MEMORY {
            let src = if hidden.is_empty() {
                0.0
            } else {
                hidden[i % hidden.len()]
            };
            let v = (src * 0.3).tanh();
            self.memory[i] = self.memory[i] * MEMORY_DECAY + v * (1.0 - MEMORY_DECAY);
        }

        self.apply_skip(input, act);

        self.last_input.clear();
        self.last_input.extend_from_slice(input);

        self.traces.push(Trace {
            in_layers: self.learn_in.clone(),
            out_layers: self.learn_out.clone(),
            pre: self.learn_pre.clone(),
        });
        while self.traces.len() > TRACE_LEN {
            self.traces.remove(0);
        }
    }

    fn apply_skip(&self, input: &[f32], out: &mut [f32]) {
        let dir_ch = crate::creature::senses::DIR_CHANNELS;

        let light = input[crate::creature::senses::BODY_BASE
            + crate::creature::senses::BODY_LIGHT];
        let air = input[crate::creature::senses::BODY_BASE
            + crate::creature::senses::BODY_AIR];
        let roof = input[crate::creature::senses::BODY_BASE
            + crate::creature::senses::BODY_OPEN_SKY];
        let enclosed = (1.0 - light) * (1.0 - air) * (1.0 - roof);
        let sky = self.skip_up * enclosed;
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

                let x = out[idx] + v;
                out[idx] = x / (1.0 + x.abs());
            }
        }
    }

    #[must_use]
    pub fn memory(&self) -> &[f32] {
        &self.memory
    }

    #[must_use]
    pub fn shape(&self) -> (usize, usize, usize) {
        (
            self.sizes[1],
            self.sizes[2],
            self.sizes[3],
        )
    }

    #[must_use]
    pub fn weight_count(&self) -> usize {
        self.w.iter().map(std::vec::Vec::len).sum()
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

        if rng.chance(0.02) {
            self.activation = rng.range(activation::COUNT as u32) as u8;
        }
    }

    #[must_use]
    pub fn grow_to(&mut self, arch: u8, rng: &mut Rng) -> bool {
        if arch <= self.arch {
            return false;
        }
        let target = brain_sizes_for(arch);
        self.resize_to(&target, rng);
        self.arch = arch;
        true
    }

    #[must_use]
    pub fn mutate_shape(&mut self, rng: &mut Rng) -> bool {

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

        self.learn_in = vec![Vec::new(); sizes.len() - 1];
        self.learn_out = vec![Vec::new(); sizes.len() - 1];
        self.learn_pre.clear();
        self.traces.clear();
    }

    pub fn learn(&mut self, targets: &[(usize, f32)], lr: f32) {
        if targets.is_empty() || self.traces.is_empty() {
            return;
        }

        let traces = std::mem::take(&mut self.traces);

        let n = traces.len().min(2);
        for ti in (0..n).rev() {
            let age = (n - 1 - ti) as f32;
            let strength = TRACE_DECAY.powf(age);
            for (out, amount) in targets {
                self.apply_target(&traces[ti], *out, *amount * strength, lr, false);
            }
        }
        self.traces = traces;
    }

    fn apply_target(&mut self, trace: &Trace, out_idx: usize, amount: f32, lr: f32, deep: bool) {
        let layers = self.w.len();
        let out_n = self.sizes[layers];
        if out_idx >= out_n || amount == 0.0 {
            return;
        }

        let mut delta = vec![0.0f32; out_n];
        let out_deriv = 1.0 / (1.0 + trace.pre.get(out_idx).unwrap_or(&0.0).abs()).powi(2);
        delta[out_idx] = amount * out_deriv;

        let bottom = if deep { 0 } else { layers.saturating_sub(2) };
        for layer in (bottom..layers).rev() {
            let in_n = self.sizes[layer];
            let out_n = self.sizes[layer + 1];
            let x_in = &trace.in_layers[layer];
            let y_out = &trace.out_layers[layer];

            {
                let ws = &mut self.w[layer];
                for j in 0..in_n {
                    let x = x_in.get(j).copied().unwrap_or(0.0);
                    if x == 0.0 {
                        continue;
                    }
                    for (o, &d) in delta.iter().enumerate().take(out_n) {
                        if d == 0.0 {
                            continue;
                        }
                        let idx = j * out_n + o;
                        let v = ws[idx] + lr * d * x;
                        ws[idx] = v.clamp(-6.0, 6.0);
                    }
                }
            }

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

            for (j, d) in in_delta.iter_mut().enumerate().take(in_n) {
                let y = y_out.get(j).copied().unwrap_or(0.0);
                *d *= activation_deriv(self.activation, y);
            }
            delta = in_delta;
        }
    }

    pub fn credit_desire(&mut self, dir: usize, amount: f32, lr: f32) {
        if self.traces.is_empty() || amount == 0.0 {
            return;
        }
        let out_idx = super::outputs::DESIRE + dir;
        if out_idx >= self.sizes[self.sizes.len() - 1] {
            return;
        }

        let trace = self.traces.last().unwrap().clone();
        self.apply_target(&trace, out_idx, amount, lr, true);
    }

    #[must_use]
    pub fn inspect(&self) -> MindView {
        MindView {
            sizes: self.sizes.clone(),
            arch: self.arch,
            activation: self.activation,
            skip: self.skip.clone(),
            skip_up: self.skip_up,

            out_weights: self
                .w
                .last()
                .cloned()
                .unwrap_or_default(),

            hidden: self.sizes[self.sizes.len() - 2],
            outputs: self.sizes[self.sizes.len() - 1],

            last_out: self
                .learn_out
                .last()
                .cloned()
                .unwrap_or_default(),

            channels: self.last_input.clone(),

            hidden_acts: self
                .learn_out
                .get(self.learn_out.len().saturating_sub(2))
                .cloned()
                .unwrap_or_default(),

            hidden_layers: self.sizes[1..self.sizes.len() - 1].to_vec(),

            layers: self.w.clone(),

            layer_acts: self.learn_out.clone(),
        }
    }

    pub fn settle(&mut self, rate: f32) {
        for ws in &mut self.w {
            for v in ws.iter_mut() {
                *v *= 1.0 - rate;
            }
        }
    }
}
