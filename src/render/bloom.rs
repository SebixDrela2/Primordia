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
            for v in &mut self.buf {
                *v = 0.0;
            }
        }
    }

    pub fn add(&mut self, x: usize, y: usize, red: f32, green: f32, blue: f32) {
        let gx = x / STEP + 1;
        let gy = y / STEP + 1;
        if gx >= self.w || gy >= self.h {
            return;
        }
        let d = (gy * self.w + gx) * 3;
        self.buf[d] += red * GAIN;
        self.buf[d + 1] += green * GAIN;
        self.buf[d + 2] += blue * GAIN;
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
    #[must_use]
    pub fn at(&self, row: usize, col: usize) -> [f32; 3] {
        let d = (row * self.w + col) * 3;
        [self.buf[d], self.buf[d + 1], self.buf[d + 2]]
    }
}

fn blur_box(src: &[f32], dst: &mut [f32], width: usize, height: usize, horizontal: bool) {

    for y in 0..height {
        for x in 0..width {
            let mut red = 0.0f32;
            let mut green = 0.0f32;
            let mut blue = 0.0f32;
            let mut weight_sum = 0.0f32;
            for (k, weight) in [(-1i32, 1.0f32), (0, 2.0), (1, 1.0)] {
                let (sx, sy) = if horizontal {
                    (x as i32 + k, y as i32)
                } else {
                    (x as i32, y as i32 + k)
                };
                if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                    continue;
                }
                let s = ((sy as usize) * width + sx as usize) * 3;
                red += src[s] * weight;
                green += src[s + 1] * weight;
                blue += src[s + 2] * weight;
                weight_sum += weight;
            }
            let d = (y * width + x) * 3;
            if weight_sum > 0.0 {
                dst[d] = red / weight_sum;
                dst[d + 1] = green / weight_sum;
                dst[d + 2] = blue / weight_sum;
            } else {
                dst[d] = 0.0;
                dst[d + 1] = 0.0;
                dst[d + 2] = 0.0;
            }
        }
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Self::new()
    }
}
