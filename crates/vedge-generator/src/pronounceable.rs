//! Pronounceable mode — strict consonant/vowel alternation starting with a
//! consonant, each position drawn uniformly from a fixed set. This is a
//! *structured subset* of random strings, so its analytic entropy
//! `⌈len/2⌉·log2(20) + ⌊len/2⌋·log2(6)` is well under the naïve `len·log2(26)`
//! (≈ 41 vs 56 bits at length 12) — reported honestly, never as random chars.

use crate::{GenError, GeneratedSecret, entropy, validate_length};
use rand::Rng;
use rand::seq::IndexedRandom;
use zeroize::Zeroizing;

/// Consonants (20) — one per even position. Excludes `y` (treated as a vowel).
const CONSONANTS: &str = "bcdfghjklmnpqrstvwxz";
/// Vowels (6) — one per odd position. Includes `y`.
const VOWELS: &str = "aeiouy";

/// Configuration for the pronounceable mode. `Copy` and ≤ 8 bytes → by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PronounceableConfig {
    /// Total length; must be within `[MIN_LEN, MAX_LEN]`.
    pub length: u32,
}

impl Default for PronounceableConfig {
    fn default() -> Self {
        Self { length: 12 }
    }
}

/// Entropy preview: `⌈len/2⌉·log2(20) + ⌊len/2⌋·log2(6)`.
///
/// # Errors
/// [`GenError::LengthOutOfRange`] if `length` is outside `[MIN_LEN, MAX_LEN]`.
pub fn pronounceable_entropy(cfg: PronounceableConfig) -> Result<f64, GenError> {
    validate_length(cfg.length)?;
    Ok(entropy::pronounceable_bits(
        cfg.length,
        CONSONANTS.chars().count(),
        VOWELS.chars().count(),
    ))
}

/// Draw a CV-alternating string. Shares [`pronounceable_entropy`]'s one number.
pub fn pronounceable_with<R: Rng>(
    cfg: PronounceableConfig,
    rng: &mut R,
) -> Result<GeneratedSecret, GenError> {
    let entropy_bits = pronounceable_entropy(cfg)?;
    let consonants: Vec<char> = CONSONANTS.chars().collect();
    let vowels: Vec<char> = VOWELS.chars().collect();
    let mut out = Zeroizing::new(String::with_capacity(cfg.length as usize));
    // Toggle bool instead of `i % 2` — avoids an arithmetic-side-effect site.
    let mut want_consonant = true;
    for _ in 0..cfg.length {
        let pool = if want_consonant { &consonants } else { &vowels };
        if let Some(&c) = pool.choose(rng) {
            out.push(c);
        }
        want_consonant = !want_consonant;
    }
    Ok(GeneratedSecret {
        secret: out,
        entropy_bits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenError;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn pronounceable_entropy_closed_form() {
        // Even length 12: 6·log2(20) + 6·log2(6).
        let bits = pronounceable_entropy(PronounceableConfig { length: 12 }).unwrap();
        let expect = 6.0 * 20.0_f64.log2() + 6.0 * 6.0_f64.log2();
        assert!((bits - expect).abs() < 1e-9);

        // Odd length 7: 4·log2(20) + 3·log2(6).
        let bits = pronounceable_entropy(PronounceableConfig { length: 7 }).unwrap();
        let expect = 4.0 * 20.0_f64.log2() + 3.0 * 6.0_f64.log2();
        assert!((bits - expect).abs() < 1e-9);
    }

    #[test]
    fn pronounceable_preview_matches_generate() {
        let cfg = PronounceableConfig { length: 14 };
        let mut rng = StdRng::seed_from_u64(11);
        assert_eq!(
            pronounceable_entropy(cfg).unwrap(),
            pronounceable_with(cfg, &mut rng).unwrap().entropy_bits
        );
    }

    #[test]
    fn pronounceable_cv_alternation() {
        let cfg = PronounceableConfig { length: 16 };
        let mut rng = StdRng::seed_from_u64(5);
        let out = pronounceable_with(cfg, &mut rng).unwrap();
        let cons: Vec<char> = CONSONANTS.chars().collect();
        let vow: Vec<char> = VOWELS.chars().collect();
        for (i, c) in out.secret.chars().enumerate() {
            if i % 2 == 0 {
                assert!(cons.contains(&c), "pos {i} should be a consonant, got {c}");
            } else {
                assert!(vow.contains(&c), "pos {i} should be a vowel, got {c}");
            }
        }
    }

    #[test]
    fn pronounceable_length_range() {
        assert_eq!(
            pronounceable_entropy(PronounceableConfig { length: 3 }).err(),
            Some(GenError::LengthOutOfRange)
        );
        assert_eq!(
            pronounceable_entropy(PronounceableConfig { length: 200 }).err(),
            Some(GenError::LengthOutOfRange)
        );
    }
}
