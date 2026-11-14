pub const STEP: usize = 4;

pub const GAIN: f32 = 130.0;

pub const STRENGTH: f32 = 0.34;

pub struct Bloom {

    pub buf: Vec<f32>,

    tmp: Vec<f32>,

    pub w: usize,
    pub h: usize,
}

impl Bloom {
    #[must_use]
    pub fn new() -> Self {
        Bloom {
            buf: Vec::new(),
            tmp: Vec::new(),
            w: 0,
            h: 0,
        }
    }

    pub fn resize(&mut self, rw: usize, rh: usize) {

        let w = rw / STEP + 3;
        let h = rh / STEP + 3;
        if self.w != w || self.h != h {
            self.w = w;
            self.h = h;
            self.buf.clear();
            self.buf.resize(w * h * 3, 0.0);
            self.tmp.clear();
            self.tmp.resize(w * h * 3, 0.0);
        } else {
            for v in self.buf.iter_mut() {
                *v = 0.0;
            }
        }
    }

    pub fn add(&mut self, x: usize, y: usize, r: f32, g: f32, b: f32) {
        let gx = x / STEP + 1;
        let gy = y / STEP + 1;
        if gx >= self.w || gy >= self.h {
            return;
        }
        let d = (gy * self.w + gx) * 3;
        self.buf[d] += r * GAIN;
        self.buf[d + 1] += g * GAIN;
        self.buf[d + 2] += b * GAIN;
    }

    pub fn blur(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let (w, h) = (self.w, self.h);
        self.tmp.copy_from_slice(&self.buf);
        blur_box(&self.tmp, &mut self.buf, w, h, true);
        blur_box(&self.buf, &mut self.tmp, w, h, false);
        blur_box(&self.tmp, &mut self.buf, w, h, false);
    }

    #[inline]
    pub fn at(&self, row: usize, col: usize) -> [f32; 3] {
        let d = (row * self.w + col) * 3;
        [self.buf[d], self.buf[d + 1], self.buf[d + 2]]
    }
}

fn blur_box(src: &[f32], dst: &mut [f32], w: usize, h: usize, horizontal: bool) {

    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            let mut n = 0.0f32;
            for (k, weight) in [(-1i32, 1.0f32), (0, 2.0), (1, 1.0)] {
                let (sx, sy) = if horizontal {
                    (x as i32 + k, y as i32)
                } else {
                    (x as i32, y as i32 + k)
                };
                if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
                    continue;
                }
                let s = ((sy as usize) * w + sx as usize) * 3;
                r += src[s] * weight;
                g += src[s + 1] * weight;
                b += src[s + 2] * weight;
                n += weight;
            }
            let d = (y * w + x) * 3;
            if n > 0.0 {
                dst[d] = r / n;
                dst[d + 1] = g / n;
                dst[d + 2] = b / n;
            } else {
                dst[d] = 0.0;
                dst[d + 1] = 0.0;
                dst[d + 2] = 0.0;
            }
        }
    }
}
