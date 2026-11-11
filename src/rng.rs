pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    #[inline]
    pub fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    #[inline]
    pub fn range(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }

    #[inline]
    pub fn range_i(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            lo
        } else {
            lo + (self.range((hi - lo) as u32)) as i32
        }
    }

    #[inline]
    pub fn sign(&mut self) -> i32 {
        if self.next_u32() & 1 == 0 {
            -1
        } else {
            1
        }
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.range((i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new(0x9E3779B97F4A7C15)
    }
}
