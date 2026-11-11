//! Dynamic light field.
//!
//! Emissive cells (lava, fire, crystals, mana) cast light into the world.
//! The field is built at a quarter of the render resolution - light is a soft
//! quantity, so the low resolution is invisible and the whole pass costs a
//! fraction of a per-cell computation at full resolution.

/// One light cell covers this many world cells in each direction.
pub const LIGHT_SCALE: usize = 4;

/// RGB light buffer, one entry per light cell.
pub struct LightField {
    pub w: usize,
    pub h: usize,
    pub buf: Vec<[f32; 3]>,
    /// Scratch buffer for the separable blur.
    tmp: Vec<[f32; 3]>,
}

impl LightField {
    pub fn new() -> Self {
        LightField {
            w: 0,
            h: 0,
            buf: Vec::new(),
            tmp: Vec::new(),
        }
    }

    /// Resize the field, keeping the buffers allocated.
    pub fn resize(&mut self, w: usize, h: usize) {
        if self.w == w && self.h == h {
            return;
        }
        self.w = w;
        self.h = h;
        let n = w * h;
        self.buf.clear();
        self.buf.resize(n, [0.0; 3]);
        self.tmp.clear();
        self.tmp.resize(n, [0.0; 3]);
    }

    pub fn clear(&mut self) {
        for c in self.buf.iter_mut() {
            *c = [0.0; 3];
        }
    }

    /// Add light at a light-cell coordinate. Contributions are clamped so a
    /// huge lava chamber cannot blow the whole cave out to white.
    #[inline]
    pub fn add(&mut self, x: usize, y: usize, c: [f32; 3]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let e = &mut self.buf[y * self.w + x];
        e[0] = (e[0] + c[0]).min(3.0);
        e[1] = (e[1] + c[1]).min(3.0);
        e[2] = (e[2] + c[2]).min(3.0);
    }

    /// Separable box blur, run once for a smooth falloff. A running sum
    /// keeps this O(cells) per pass instead of O(cells * radius).
    pub fn blur(&mut self, radius: usize) {
        if radius == 0 || self.w == 0 || self.h == 0 {
            return;
        }
        self.blur_horizontal(radius);
        self.blur_vertical(radius);
    }

    /// Spread the light outwards, keeping the brightest value at each cell.
    ///
    /// A box blur would *average* the light, which dilutes a small emitter
    /// until it is invisible. Taking the maximum instead preserves the
    /// brightness of the source, so a lava chamber really does light the
    /// cave around it.
    pub fn spread(&mut self, radius: usize, decay: f32) {
        if radius == 0 || self.w == 0 || self.h == 0 {
            return;
        }
        for _ in 0..radius {
            self.spread_pass(decay);
        }
    }

    /// One expansion step: every cell takes the brightest of its neighbours,
    /// attenuated by `decay`.
    fn spread_pass(&mut self, decay: f32) {
        let w = self.w;
        let h = self.h;
        // Horizontal expansion into the scratch buffer.
        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let mut best = self.buf[row + x];
                let left = if x > 0 {
                    self.buf[row + x - 1]
                } else {
                    best
                };
                let right = if x + 1 < w {
                    self.buf[row + x + 1]
                } else {
                    best
                };
                for i in 0..3 {
                    let v = left[i].max(right[i]) * decay;
                    if v > best[i] {
                        best[i] = v;
                    }
                }
                self.tmp[row + x] = best;
            }
        }
        // Vertical expansion, back into the main buffer. The horizontal pass
        // already moved the light up and down by one cell, so this pass
        // doubles the reach in both directions.
        for x in 0..w {
            for y in 0..h {
                let mut best = self.tmp[y * w + x];
                let up = if y > 0 {
                    self.tmp[(y - 1) * w + x]
                } else {
                    best
                };
                let down = if y + 1 < h {
                    self.tmp[(y + 1) * w + x]
                } else {
                    best
                };
                for i in 0..3 {
                    let v = up[i].max(down[i]) * decay;
                    if v > best[i] {
                        best[i] = v;
                    }
                }
                self.buf[y * w + x] = best;
            }
        }
    }

    /// Convert the raw light into the final additive colour, in place.
    ///
    /// Doing this once per light cell instead of once per pixel is what keeps
    /// the pixel loop cheap: the inner loop only adds three numbers.
    pub fn finalize(&mut self) {
        for c in self.buf.iter_mut() {
            // Warm light dominates; the blue channel keeps it from turning
            // everything orange.
            let lr = c[0].max(c[1] * 0.85).max(c[2] * 0.6);
            let lg = c[1].max(c[0] * 0.45);
            let lb = c[2].max(c[0] * 0.25);
            *c = [lr * 62.0, lg * 62.0, lb * 62.0];
        }
    }

    fn blur_horizontal(&mut self, radius: usize) {
        let w = self.w;
        let h = self.h;
        if w == 0 || h == 0 {
            return;
        }
        let r = radius.min(w.saturating_sub(1) / 2);
        if r == 0 {
            return;
        }
        let win = (2 * r + 1) as f32;
        for y in 0..h {
            let row = y * w;
            // Seed the running sum with the first window.
            let mut acc = [0.0f32; 3];
            for x in 0..=r.min(w - 1) {
                let c = self.buf[row + x];
                acc[0] += c[0];
                acc[1] += c[1];
                acc[2] += c[2];
            }
            for x in 0..w {
                self.tmp[row + x] = [acc[0] / win, acc[1] / win, acc[2] / win];
                // Slide the window one cell to the right.
                let out = x.saturating_sub(r);
                let add = (x + r + 1).min(w - 1);
                let outc = self.buf[row + out];
                let addc = self.buf[row + add];
                acc[0] += addc[0] - outc[0];
                acc[1] += addc[1] - outc[1];
                acc[2] += addc[2] - outc[2];
            }
        }
        std::mem::swap(&mut self.buf, &mut self.tmp);
    }

    fn blur_vertical(&mut self, radius: usize) {
        let w = self.w;
        let h = self.h;
        if w == 0 || h == 0 {
            return;
        }
        let r = radius.min(h.saturating_sub(1) / 2);
        if r == 0 {
            return;
        }
        let win = (2 * r + 1) as f32;
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for y in 0..=r.min(h - 1) {
                let c = self.buf[y * w + x];
                acc[0] += c[0];
                acc[1] += c[1];
                acc[2] += c[2];
            }
            for y in 0..h {
                self.tmp[y * w + x] = [acc[0] / win, acc[1] / win, acc[2] / win];
                let out = y.saturating_sub(r);
                let add = (y + r + 1).min(h - 1);
                let outc = self.buf[out * w + x];
                let addc = self.buf[add * w + x];
                acc[0] += addc[0] - outc[0];
                acc[1] += addc[1] - outc[1];
                acc[2] += addc[2] - outc[2];
            }
        }
        std::mem::swap(&mut self.buf, &mut self.tmp);
    }
}
