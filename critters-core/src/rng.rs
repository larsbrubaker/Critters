//! Tiny pseudo-random source standing in for `Math.random()`. The original
//! never seeds anything, so this is an xorshift64* generator seeded from the
//! wall clock; `scenery.rs`, `game.rs` (shape picks, wind direction) and
//! `piece.rs` (animation phases) draw from one shared instance.

/// Uniform `[0, 1)` generator equivalent to `Math.random()`.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seed from the wall clock, like the browser's implicit seeding.
    pub fn from_time() -> Self {
        let nanos = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self::seeded(nanos)
    }

    /// Deterministic seed (tests).
    pub fn seeded(seed: u64) -> Self {
        // xorshift needs a non-zero state; splitmix the seed first so nearby
        // seeds diverge immediately.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Self {
            state: if z == 0 { 1 } else { z },
        }
    }

    /// `Math.random()`: a float in `[0, 1)`.
    pub fn random(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let out = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        // 53 random mantissa bits, same resolution as a JS double.
        (out >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `Math.random() * (max - min) + min`.
    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        min + self.random() * (max - min)
    }

    /// `Math.random() < 0.5 ? -1 : 1`.
    pub fn sign(&mut self) -> f64 {
        if self.random() < 0.5 {
            -1.0
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_stay_in_unit_interval_and_vary() {
        let mut rng = Rng::seeded(42);
        let mut distinct = std::collections::HashSet::new();
        for _ in 0..1000 {
            let v = rng.random();
            assert!((0.0..1.0).contains(&v), "out of range: {v}");
            distinct.insert(v.to_bits());
        }
        assert!(distinct.len() > 990);
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::seeded(7);
        let mut b = Rng::seeded(7);
        for _ in 0..50 {
            assert_eq!(a.random().to_bits(), b.random().to_bits());
        }
    }

    #[test]
    fn zero_seed_is_valid() {
        let mut rng = Rng::seeded(0);
        assert!(rng.random() < 1.0);
    }
}
