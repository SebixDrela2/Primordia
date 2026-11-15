use super::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Personality {

    pub aggression: f32,

    pub curiosity: f32,

    pub sociability: f32,

    pub greed: f32,

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

    #[must_use]
    pub fn random(rng: &mut Rng) -> Self {
        Personality {
            aggression: rng.f32(),
            curiosity: rng.f32(),
            sociability: rng.f32(),
            greed: rng.f32(),
            caution: rng.f32(),
        }
    }

    #[must_use]
    pub fn inherit(parent: &Personality, rng: &mut Rng) -> Self {
        Personality {
            aggression: drift(parent.aggression, rng),
            curiosity: drift(parent.curiosity, rng),
            sociability: drift(parent.sociability, rng),
            greed: drift(parent.greed, rng),
            caution: drift(parent.caution, rng),
        }
    }

    #[must_use]
    pub fn blend(a: &Personality, b: &Personality, rng: &mut Rng) -> Self {
        Personality {
            aggression: drift((a.aggression + b.aggression) * 0.5, rng),
            curiosity: drift((a.curiosity + b.curiosity) * 0.5, rng),
            sociability: drift((a.sociability + b.sociability) * 0.5, rng),
            greed: drift((a.greed + b.greed) * 0.5, rng),
            caution: drift((a.caution + b.caution) * 0.5, rng),
        }
    }

    #[must_use]
    pub fn distance(&self, other: &Personality) -> f32 {
        let d = (self.aggression - other.aggression).abs()
            + (self.curiosity - other.curiosity).abs()
            + (self.sociability - other.sociability).abs()
            + (self.greed - other.greed).abs()
            + (self.caution - other.caution).abs();
        (d / 5.0).clamp(0.0, 1.0)
    }

    #[must_use]
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

fn drift(v: f32, rng: &mut Rng) -> f32 {
    (v + (rng.f32() * 2.0 - 1.0) * 0.09).clamp(0.0, 1.0)
}
