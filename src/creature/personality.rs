//! Personality: the temperament of an individual creature.
//!
//! Traits are what a *species* evolves; personality is what makes one
//! creature different from its siblings. A trait says "this species is
//! ferocious"; a personality says "this particular animal is a coward".
//!
//! The five axes are deliberately simple, but each one shifts real numbers
//! in the behaviour code, so two creatures with identical genes still play
//! very differently.

use super::*;

/// The temperament of a single creature.
///
/// Every axis runs from 0.0 to 1.0. The values are inherited from the
/// parents and drift a little with every generation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Personality {
    /// How eagerly it starts a fight, and how hard it hits.
    pub aggression: f32,
    /// How eagerly it explores and tries the unknown.
    pub curiosity: f32,
    /// How much it likes the company of its own kind.
    pub sociability: f32,
    /// How much it hoards and competes for what it finds.
    pub greed: f32,
    /// How much it avoids risk.
    pub caution: f32,
}

impl Default for Personality {
    fn default() -> Self {
        Personality {
            aggression: 0.5,
            curiosity: 0.5,
            sociability: 0.5,
            greed: 0.5,
            caution: 0.5,
        }
    }
}

impl Personality {
    /// A completely random temperament.
    pub fn random(rng: &mut Rng) -> Self {
        Personality {
            aggression: rng.f32(),
            curiosity: rng.f32(),
            sociability: rng.f32(),
            greed: rng.f32(),
            caution: rng.f32(),
        }
    }

    /// The temperament a newborn inherits from one parent, with drift.
    pub fn inherit(parent: &Personality, rng: &mut Rng) -> Self {
        Personality {
            aggression: drift(parent.aggression, rng),
            curiosity: drift(parent.curiosity, rng),
            sociability: drift(parent.sociability, rng),
            greed: drift(parent.greed, rng),
            caution: drift(parent.caution, rng),
        }
    }

    /// The temperament a newborn inherits from *two* parents.
    ///
    /// The child sits between its parents and drifts a little further, so
    /// two different temperaments produce something new rather than a copy
    /// of one of them.
    pub fn blend(a: &Personality, b: &Personality, rng: &mut Rng) -> Self {
        Personality {
            aggression: drift((a.aggression + b.aggression) * 0.5, rng),
            curiosity: drift((a.curiosity + b.curiosity) * 0.5, rng),
            sociability: drift((a.sociability + b.sociability) * 0.5, rng),
            greed: drift((a.greed + b.greed) * 0.5, rng),
            caution: drift((a.caution + b.caution) * 0.5, rng),
        }
    }

    /// How different two temperaments are, in the range 0..1.
    ///
    /// This is the measure of *genetic distance* used for mate choice: a
    /// creature prefers a partner that is not too similar (inbreeding) and
    /// not too different (that would be a different species).
    pub fn distance(&self, other: &Personality) -> f32 {
        let d = (self.aggression - other.aggression).abs()
            + (self.curiosity - other.curiosity).abs()
            + (self.sociability - other.sociability).abs()
            + (self.greed - other.greed).abs()
            + (self.caution - other.caution).abs();
        (d / 5.0).clamp(0.0, 1.0)
    }

    /// A short label for the tooltip, e.g. "bold and curious".
    pub fn describe(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.aggression > 0.68 {
            parts.push("bold");
        } else if self.aggression < 0.32 {
            parts.push("mild");
        }
        if self.curiosity > 0.68 {
            parts.push("curious");
        } else if self.curiosity < 0.32 {
            parts.push("set in its ways");
        }
        if self.sociability > 0.68 {
            parts.push("gregarious");
        } else if self.sociability < 0.32 {
            parts.push("solitary");
        }
        if self.greed > 0.68 {
            parts.push("greedy");
        } else if self.greed < 0.32 {
            parts.push("generous");
        }
        if self.caution > 0.68 {
            parts.push("wary");
        } else if self.caution < 0.32 {
            parts.push("reckless");
        }
        if parts.is_empty() {
            "even-tempered".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// A single step of drift: the value wanders a little, and stays in range.
fn drift(v: f32, rng: &mut Rng) -> f32 {
    (v + (rng.f32() * 2.0 - 1.0) * 0.09).clamp(0.0, 1.0)
}
