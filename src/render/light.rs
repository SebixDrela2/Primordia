pub const LIGHT_SCALE: usize = 4;

pub struct LightField {
    pub w: usize,
    pub h: usize,
    pub buf: Vec<[f32; 3]>,

    tmp: Vec<[f32; 3]>,

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

    pub fn finalize(&mut self) {
        for c in self.buf.iter_mut() {

            let lr = c[0].max(c[1] * 0.85).max(c[2] * 0.6);
            let lg = c[1].max(c[0] * 0.45);
            let lb = c[2].max(c[0] * 0.25);
            *c = [lr * 62.0, lg * 62.0, lb * 62.0];
        }
    }
}
