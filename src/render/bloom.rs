//! Bloom: the glow of the light sources.
//!
//! The bright parts of a picture bleed into what is around them: fire,
//! lava, crystals and wards glow into the dark. This is the glow.
//!
//! It is worked out at a quarter of the picture's size, because a glow
//! does not need any detail, and it is laid over the picture inside the
//! pixel loop itself. A separate pass over the whole picture would cost
//! twice as much.
//!
//! - mod.rs    the renderer, sky and terrain shading
//! - light.rs  the dynamic light field cast by emissive cells
//! - bloom.rs  the glow of the light sources

/// The glow is worked out at one cell for every four pixels.
pub const STEP: usize = 4;
/// How bright a light source is in the glow.
pub const GAIN: f32 = 130.0;
/// How much of the glow is laid over the picture.
pub const STRENGTH: f32 = 0.34;

/// The glow of a picture, waiting to be laid over it.
///
/// The glow is seeded from the light sources while the region is scanned,
/// so it costs nothing extra to gather.
pub struct Bloom {
    /// The glow, three floats a cell.
    pub buf: Vec<f32>,
    /// A second buffer, so the blur does not read and write the same
    /// memory.
    tmp: Vec<f32>,
    /// The size of the glow, in cells.
    pub w: usize,
    pub h: usize,
}

impl Bloom {
    pub fn new() -> Self {
        Bloom {
            buf: Vec::new(),
            tmp: Vec::new(),
            w: 0,
            h: 0,
        }
    }

    /// Make the glow the right size for a picture, and empty it.
    pub fn resize(&mut self, rw: usize, rh: usize) {
        // A margin of one cell on each side, so the blur has room to
        // spread without running off the edge.
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

    /// Add a light source to the glow.
    ///
    /// The cell is given in picture coordinates; the glow works at a
    /// quarter of the picture's size.
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

    /// Soften the glow.
    ///
    /// Three soft passes, one across and two down, make a glow that is
    /// wide and gentle instead of blocky.
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

    /// The glow at a cell of the picture, as (red, green, blue).
    ///
    /// The pixel loop reads the glow through this, so it is written to
    /// keep the picture's own arithmetic.
    #[inline]
    pub fn at(&self, row: usize, col: usize) -> [f32; 3] {
        let d = (row * self.w + col) * 3;
        [self.buf[d], self.buf[d + 1], self.buf[d + 2]]
    }
}

/// One box blur pass over the glow.
///
/// `horizontal` says which way the blur runs. The two passes together
/// make a wide, soft glow.
fn blur_box(src: &[f32], dst: &mut [f32], w: usize, h: usize, horizontal: bool) {
    // A 1-2-1 kernel: it is smooth enough for a glow and it costs a third
    // of what a full box would.
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
