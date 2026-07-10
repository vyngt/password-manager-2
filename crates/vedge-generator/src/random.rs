//! Random charset mode — the shipped slice-3.1 engine, unchanged. Draws
//! `length` characters uniformly from the enabled character classes, optionally
//! excluding ambiguous glyphs and requiring at least one char from each class.

use crate::charset::{self, AMBIGUOUS, sample};
use crate::{GenError, GeneratedSecret, entropy, validate_length};
use rand::Rng;

/// Safety cap on `require_each_selected` reject-sampling. With `MIN_LEN == 4` and
/// at most 4 classes (each ≥ 8 chars after ambiguous exclusion), a valid draw is
/// found in a handful of tries — this bound is only a guard against an
/// unbounded loop, never hit in practice.
const MAX_DRAW_ATTEMPTS: u32 = 10_000;

/// Which character classes the random mode may draw from.
// Four bools is the domain (four character classes), not a modelling smell.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharClasses {
    /// `a`–`z`
    pub lowercase: bool,
    /// `A`–`Z`
    pub uppercase: bool,
    /// `0`–`9`
    pub digits: bool,
    /// the fixed symbol set
    pub symbols: bool,
}

impl Default for CharClasses {
    fn default() -> Self {
        Self {
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
        }
    }
}

/// Configuration for the random charset mode. All-`Copy`, so the UI holds it in
/// a single `RwSignal<RandomConfig>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RandomConfig {
    /// Desired length; must be within `[MIN_LEN, MAX_LEN]` or generation errs.
    pub length: u32,
    /// Enabled character classes (≥ 1 required).
    pub classes: CharClasses,
    /// Require at least one character from every enabled class.
    pub require_each_selected: bool,
    /// Drop the visually ambiguous characters from the effective alphabet.
    pub exclude_ambiguous: bool,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            length: 20,
            classes: CharClasses::default(),
            require_each_selected: false,
            exclude_ambiguous: false,
        }
    }
}

/// Generate a random secret from the OS/browser CSPRNG.
///
/// # Errors
/// Returns [`GenError`] if no class is selected, the length is out of range, or
/// the effective alphabet is empty.
pub fn generate_random(cfg: &RandomConfig) -> Result<GeneratedSecret, GenError> {
    generate_with(cfg, &mut rand::rng())
}

/// Entropy preview for `cfg` **without drawing a secret**.
///
/// Used by the live meter while the user is still adjusting the config. Derived
/// from the exact same alphabet/length logic [`generate_random`] uses, so it can
/// never lie.
///
/// # Errors
/// Same conditions as [`generate_random`].
pub fn random_entropy_bits(cfg: &RandomConfig) -> Result<f64, GenError> {
    validate_length(cfg.length)?;
    let alphabet = Alphabet::build(cfg)?;
    Ok(entropy::bits(
        cfg.length,
        &alphabet.sizes(),
        cfg.require_each_selected,
    ))
}

/// The effective alphabet, kept partitioned by enabled class so require-each and
/// entropy accounting can reason per-class. Classes emptied by ambiguous
/// exclusion are dropped (so they aren't required and don't skew entropy).
struct Alphabet {
    classes: Vec<Vec<char>>,
}

impl Alphabet {
    fn build(cfg: &RandomConfig) -> Result<Self, GenError> {
        let cl = &cfg.classes;
        if !(cl.lowercase || cl.uppercase || cl.digits || cl.symbols) {
            return Err(GenError::NoClassSelected);
        }
        let ex = cfg.exclude_ambiguous;
        let keep = |c: char| !(ex && AMBIGUOUS.contains(&c));
        let mut classes: Vec<Vec<char>> = Vec::new();
        if cl.lowercase {
            classes.push(
                charset::class_lower()
                    .into_iter()
                    .filter(|&c| keep(c))
                    .collect(),
            );
        }
        if cl.uppercase {
            classes.push(
                charset::class_upper()
                    .into_iter()
                    .filter(|&c| keep(c))
                    .collect(),
            );
        }
        if cl.digits {
            classes.push(
                charset::class_digit()
                    .into_iter()
                    .filter(|&c| keep(c))
                    .collect(),
            );
        }
        if cl.symbols {
            classes.push(
                charset::class_symbol()
                    .into_iter()
                    .filter(|&c| keep(c))
                    .collect(),
            );
        }
        classes.retain(|cls| !cls.is_empty());
        if classes.is_empty() {
            return Err(GenError::EmptyEffectiveAlphabet);
        }
        Ok(Self { classes })
    }

    fn flat(&self) -> Vec<char> {
        self.classes.iter().flatten().copied().collect()
    }

    fn sizes(&self) -> Vec<usize> {
        self.classes.iter().map(Vec::len).collect()
    }
}

/// The generic core — `generate_random` uses the OS CSPRNG; tests inject a
/// seeded RNG so the unbiased-sampling check is deterministic.
pub fn generate_with<R: Rng>(cfg: &RandomConfig, rng: &mut R) -> Result<GeneratedSecret, GenError> {
    validate_length(cfg.length)?;
    let alphabet = Alphabet::build(cfg)?;
    let flat = alphabet.flat();
    let entropy_bits = entropy::bits(cfg.length, &alphabet.sizes(), cfg.require_each_selected);

    let secret = if cfg.require_each_selected {
        // Reject-and-redraw the whole string until every enabled class appears.
        // Conditioning the uniform draw on the constraint keeps it uniform over
        // the valid set, so `entropy::bits`' inclusion-exclusion count is exact.
        let mut attempts: u32 = 0;
        loop {
            let candidate = sample(&flat, cfg.length, rng);
            if satisfies_require_each(&candidate, &alphabet.classes) {
                break candidate;
            }
            attempts = attempts.saturating_add(1);
            if attempts >= MAX_DRAW_ATTEMPTS {
                return Err(GenError::LengthOutOfRange);
            }
        }
    } else {
        sample(&flat, cfg.length, rng)
    };

    Ok(GeneratedSecret {
        secret,
        entropy_bits,
    })
}

fn satisfies_require_each(s: &str, classes: &[Vec<char>]) -> bool {
    classes
        .iter()
        .all(|cls| s.chars().any(|c| cls.contains(&c)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charset::SYMBOLS;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashMap;

    fn cfg(length: u32, l: bool, u: bool, d: bool, s: bool) -> RandomConfig {
        RandomConfig {
            length,
            classes: CharClasses {
                lowercase: l,
                uppercase: u,
                digits: d,
                symbols: s,
            },
            require_each_selected: false,
            exclude_ambiguous: false,
        }
    }

    #[test]
    fn entropy_random_matches_formula() {
        // lowercase + digits: A = 36.
        let bits = random_entropy_bits(&cfg(10, true, false, true, false)).unwrap();
        assert!((bits - 10.0 * 36.0_f64.log2()).abs() < 1e-9);

        // exclude_ambiguous drops o,l (lower) + 0,1 (digit) → A = 32 → 10·5 = 50.
        let mut c = cfg(10, true, false, true, false);
        c.exclude_ambiguous = true;
        let bits = random_entropy_bits(&c).unwrap();
        assert!((bits - 50.0).abs() < 1e-9);
    }

    #[test]
    fn generate_respects_length_and_classes() {
        // Only lowercase, exact length.
        let out = generate_random(&cfg(12, true, false, false, false)).unwrap();
        assert_eq!(out.secret.chars().count(), 12);
        assert!(out.secret.chars().all(|c| c.is_ascii_lowercase()));

        // require_each over all four classes → at least one of each.
        let mut c = cfg(16, true, true, true, true);
        c.require_each_selected = true;
        let out = generate_random(&c).unwrap();
        let s = out.secret.as_str();
        assert!(s.chars().any(|c| c.is_ascii_lowercase()));
        assert!(s.chars().any(|c| c.is_ascii_uppercase()));
        assert!(s.chars().any(|c| c.is_ascii_digit()));
        assert!(s.chars().any(|c| SYMBOLS.contains(c)));
    }

    #[test]
    fn no_class_selected_errs() {
        assert_eq!(
            generate_random(&cfg(12, false, false, false, false)).err(),
            Some(GenError::NoClassSelected)
        );
        assert_eq!(
            random_entropy_bits(&cfg(12, false, false, false, false)).err(),
            Some(GenError::NoClassSelected)
        );
    }

    #[test]
    fn length_out_of_range_errs() {
        assert_eq!(
            generate_random(&cfg(3, true, false, false, false)).err(),
            Some(GenError::LengthOutOfRange)
        );
        assert_eq!(
            generate_random(&cfg(129, true, false, false, false)).err(),
            Some(GenError::LengthOutOfRange)
        );
    }

    #[test]
    fn sampling_is_unbiased() {
        // Deterministic (seeded) so it never flakes. A `% 26` modulo bias would
        // skew the low letters — this guards that regression.
        let mut rng = StdRng::seed_from_u64(0xC0FF_EE42);
        let mut counts: HashMap<char, u32> = HashMap::new();
        for _ in 0..500 {
            let out = generate_with(&cfg(100, true, false, false, false), &mut rng).unwrap();
            for c in out.secret.chars() {
                *counts.entry(c).or_insert(0) += 1;
            }
        }
        assert_eq!(counts.len(), 26, "every lowercase letter must appear");
        let max = *counts.values().max().unwrap() as f64;
        let min = *counts.values().min().unwrap() as f64;
        assert!(
            max / min < 1.5,
            "distribution not roughly uniform: max/min = {}",
            max / min
        );
    }

    #[test]
    fn preview_matches_generate() {
        // The one-code-path guarantee: the preview equals the generated bits
        // exactly (both call `entropy::bits`), incl. the require_each correction.
        for &(len, req, ex) in &[(20, false, false), (16, true, false), (24, true, true)] {
            let mut c = cfg(len, true, true, true, true);
            c.require_each_selected = req;
            c.exclude_ambiguous = ex;
            let preview = random_entropy_bits(&c).unwrap();
            let generated = generate_random(&c).unwrap().entropy_bits;
            assert_eq!(preview, generated);
        }
    }

    #[test]
    fn require_each_reduces_entropy() {
        // At a short length the constraint measurably shrinks the sample space.
        let mut c = cfg(6, true, true, true, true);
        let unconstrained = random_entropy_bits(&c).unwrap();
        c.require_each_selected = true;
        let constrained = random_entropy_bits(&c).unwrap();
        assert!(constrained < unconstrained);
        assert!(constrained > 0.0);
    }
}
