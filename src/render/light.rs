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
    /// Prefix sums for the blur, reused between frames.
    prefix: Vec<[f32; 3]>,
}

impl LightField {
    pub fn new() -> Self {
        LightField {
            w: 0,
            h: 0,
            buf: Vec::new(),
            tmp: Vec::new(),
            prefix: Vec::new(),
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
        self.prefix.clear();
        self.prefix.resize(n.max(w.max(h)) + 1, [0.0; 3]);
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

    /// Spread the light with a box blur, run once horizontally and once
    /// vertically.
    ///
    /// The edges are clamped and the average is normalised by the number of
    /// samples that were actually taken. A plain running sum gets the edges
    /// wrong, and because the field's origin moves with the camera, that
    /// error showed up as a flash of lighting whenever the view moved.
    pub fn blur(&mut self, radius: usize) {
        if radius == 0 || self.w == 0 || self.h == 0 {
            return;
        }
        self.blur_horizontal(radius);
        self.blur_vertical(radius);
    }

    fn blur_horizontal(&mut self, radius: usize) {
        let w = self.w;
        let h = self.h;
        let r = radius.min(w.saturating_sub(1));
        if r == 0 {
            return;
        }
        // A prefix sum over the row gives the window total in constant time.
        // The window is clamped at the edges and the divisor is the number of
        // samples actually taken, so the result does not depend on where the
        // field starts. A plain running sum gets the clipped edges subtly
        // wrong, and because the field's origin moves with the camera, that
        // error showed up as a flash of lighting whenever the view moved.
        let mut prefix = std::mem::take(&mut self.prefix);
        prefix.resize(w + 1, [0.0; 3]);
        for y in 0..h {
            let row = y * w;
            prefix[0] = [0.0; 3];
            for x in 0..w {
                let c = self.buf[row + x];
                let p = prefix[x];
                prefix[x + 1] = [p[0] + c[0], p[1] + c[1], p[2] + c[2]];
            }
            for x in 0..w {
                let lo = x.saturating_sub(r);
                let hi = (x + r).min(w - 1);
                let a = prefix[lo];
                let b = prefix[hi + 1];
                let inv = 1.0 / (hi - lo + 1) as f32;
                self.tmp[row + x] = [
                    (b[0] - a[0]) * inv,
                    (b[1] - a[1]) * inv,
                    (b[2] - a[2]) * inv,
                ];
            }
        }
        std::mem::swap(&mut self.buf, &mut self.tmp);
        self.prefix = prefix;
    }

    fn blur_vertical(&mut self, radius: usize) {
        let w = self.w;
        let h = self.h;
        let r = radius.min(h.saturating_sub(1));
        if r == 0 {
            return;
        }
        // A prefix sum down the column.
        let mut prefix = std::mem::take(&mut self.prefix);
        prefix.resize(h + 1, [0.0; 3]);
        for x in 0..w {
            prefix[0] = [0.0; 3];
            for y in 0..h {
                let c = self.buf[y * w + x];
                let p = prefix[y];
                prefix[y + 1] = [p[0] + c[0], p[1] + c[1], p[2] + c[2]];
            }
            for y in 0..h {
                let lo = y.saturating_sub(r);
                let hi = (y + r).min(h - 1);
                let a = prefix[lo];
                let b = prefix[hi + 1];
                let inv = 1.0 / (hi - lo + 1) as f32;
                self.buf[y * w + x] = [
                    (b[0] - a[0]) * inv,
                    (b[1] - a[1]) * inv,
                    (b[2] - a[2]) * inv,
                ];
            }
        }
        self.prefix = prefix;
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
}
