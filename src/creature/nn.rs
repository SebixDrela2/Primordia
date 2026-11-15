//! The tight float kernels of the mind.
//!
//! The mind spends most of its time in two shapes of loop: adding a
//! scaled run of numbers to another run, and scaling a run of numbers
//! and putting it back where it came from. They live here so that they
//! are written once, in the way the compiler likes best, and so that
//! every mind in the game is worked on the same way.
//!
//! The compiler is told to use the wide registers of the processor
//! (see `.cargo/config.toml`), so these loops are carried out sixteen
//! numbers at a time where the processor has them.

/// Add a run of numbers, scaled, to another run: `dst += a * src`.
///
/// The two runs are the same length. The numbers are added one at a
/// time and in the order they come, so the answer is the same as the
/// one the plain loop gives.
#[inline]
pub fn axpy(dst: &mut [f32], src: &[f32], a: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += *s * a;
    }
}

/// Scale a run of numbers and add it to another, keeping the result
/// inside the limits: `dst = clamp(dst + a * src, lo, hi)`.
///
/// This is the shape the learning of the mind has: a weight is moved
/// towards what the mind wants, and it is never allowed to leave the
/// range the mind can think in.
#[inline]
pub fn scale_add_clamp(dst: &mut [f32], src: &[f32], a: f32, lo: f32, hi: f32) {
    debug_assert_eq!(dst.len(), src.len());
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d = (*d + *s * a).clamp(lo, hi);
    }
}
