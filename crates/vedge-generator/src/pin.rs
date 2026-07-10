//! PIN mode — uniform decimal digits. Honest entropy `length × log2(10)`
//! (≈ 3.32 bits/digit): a 6-digit PIN is ≈ 19.9 bits, correctly a *weak* band.
//! The UI pairs it with a context caption — PINs are for rate-limited devices,
//! not offline attack.

use crate::charset::{class_digit, sample};
use crate::{GenError, GeneratedSecret, entropy, validate_length};
use rand::Rng;

/// Configuration for the PIN mode. `Copy` and ≤ 8 bytes, so passed by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinConfig {
    /// Number of digits; must be within `[MIN_LEN, MAX_LEN]`.
    pub length: u32,
}

impl Default for PinConfig {
    fn default() -> Self {
        Self { length: 6 }
    }
}

/// Entropy preview: `length × log2(10)`.
///
/// # Errors
/// [`GenError::LengthOutOfRange`] if `length` is outside `[MIN_LEN, MAX_LEN]`.
pub fn pin_entropy(cfg: PinConfig) -> Result<f64, GenError> {
    validate_length(cfg.length)?;
    Ok(entropy::uniform_bits(cfg.length, class_digit().len()))
}

/// Draw `length` uniform digits. Shares [`pin_entropy`]'s one number + validation.
pub fn pin_with<R: Rng>(cfg: PinConfig, rng: &mut R) -> Result<GeneratedSecret, GenError> {
    let entropy_bits = pin_entropy(cfg)?;
    let secret = sample(&class_digit(), cfg.length, rng);
    Ok(GeneratedSecret {
        secret,
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
    fn pin_entropy_closed_form() {
        let bits = pin_entropy(PinConfig { length: 6 }).unwrap();
        assert!((bits - 6.0 * 10.0_f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn pin_preview_matches_generate() {
        let cfg = PinConfig { length: 8 };
        let mut rng = StdRng::seed_from_u64(7);
        let preview = pin_entropy(cfg).unwrap();
        let generated = pin_with(cfg, &mut rng).unwrap();
        assert_eq!(preview, generated.entropy_bits);
    }

    #[test]
    fn pin_all_digits_exact_len() {
        let cfg = PinConfig { length: 10 };
        let mut rng = StdRng::seed_from_u64(3);
        let out = pin_with(cfg, &mut rng).unwrap();
        assert_eq!(out.secret.chars().count(), 10);
        assert!(out.secret.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn pin_length_range() {
        assert_eq!(
            pin_entropy(PinConfig { length: 3 }).err(),
            Some(GenError::LengthOutOfRange)
        );
        assert_eq!(
            pin_entropy(PinConfig { length: 129 }).err(),
            Some(GenError::LengthOutOfRange)
        );
    }
}
