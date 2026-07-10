//! # vedge-generator
//!
//! Pure, stateless secret-generation engine.
//!
//! No `SQLite`, no OS integration, no serde/DB — just a CSPRNG and math.
//! Compiles to **both** native (OS RNG) and `wasm32` (browser
//! `crypto.getRandomValues`), so the Leptos/WASM UI calls it with zero IPC while
//! native CLI/server shells reuse the identical code.
//!
//! ## One code path (the load-bearing invariant)
//!
//! [`generate_random`] and [`random_entropy_bits`] compute the reported entropy
//! from the **same** [`entropy::bits`] helper, so the live meter can never
//! disagree with the generator — the entropy is a property of the generation
//! *process*, not of the output string.
//!
//! This slice ships the **random charset** mode; passphrase/PIN/pronounceable/
//! pattern modes land in slice 3.4 behind the same `(secret, entropy_bits)`
//! shape.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::float_cmp,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::cast_precision_loss,
        clippy::cast_lossless,
        clippy::fn_params_excessive_bools,
        clippy::suboptimal_flops
    )
)]

use rand::Rng;
use rand::seq::IndexedRandom;
use zeroize::Zeroizing;

/// Minimum secret length the engine will produce.
pub const MIN_LEN: u32 = 4;
/// Maximum secret length the engine will produce.
pub const MAX_LEN: u32 = 128;

/// The fixed, documented symbol set. Changing it changes entropy accounting, so
/// it is a constant, not a config knob.
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}|;:,.<>?/";

/// The fixed "visually ambiguous" set removed when `exclude_ambiguous` is on:
/// capital-O / zero / lower-o, capital-I / lower-l / one, and pipe. Spans several
/// classes on purpose (`0` is a digit, `|` a symbol, the rest letters).
const AMBIGUOUS: [char; 7] = ['O', '0', 'o', 'I', 'l', '1', '|'];

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
    /// the fixed [`SYMBOLS`] set
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
    /// Drop the visually [`AMBIGUOUS`] characters from the effective alphabet.
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

/// A generated secret plus the entropy of the process that produced it.
///
/// The secret is wiped on drop ([`Zeroizing`]). Intentionally **not** `Debug` —
/// it must never be formatted into a log.
pub struct GeneratedSecret {
    /// The generated characters.
    pub secret: Zeroizing<String>,
    /// Entropy of the generation process, in bits (not a property of the string).
    pub entropy_bits: f64,
}

/// Why generation (or an entropy preview) could not proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenError {
    /// No character class was enabled.
    NoClassSelected,
    /// `length` was outside `[MIN_LEN, MAX_LEN]` (or the require-each constraint
    /// was unsatisfiable at that length — unreachable with the built-in sets).
    LengthOutOfRange,
    /// The effective alphabet was empty (e.g. every enabled class was fully
    /// excluded as ambiguous). Not reachable with the built-in sets.
    EmptyEffectiveAlphabet,
}

impl core::fmt::Display for GenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NoClassSelected => "no character class selected",
            Self::LengthOutOfRange => "length out of range",
            Self::EmptyEffectiveAlphabet => "effective alphabet is empty",
        })
    }
}

impl std::error::Error for GenError {}

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

/// Map entropy bits to a 1..=4 band for the meter, calibrated to offline-attack
/// scale: `<45` weak (1) · `45–64` fair (2) · `64–90` strong (3) · `≥90`
/// excellent (4).
#[must_use]
pub const fn entropy_band(bits: f64) -> u8 {
    if bits < 45.0 {
        1
    } else if bits < 64.0 {
        2
    } else if bits < 90.0 {
        3
    } else {
        4
    }
}

// --- internals -----------------------------------------------------------

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
            classes.push(('a'..='z').filter(|&c| keep(c)).collect());
        }
        if cl.uppercase {
            classes.push(('A'..='Z').filter(|&c| keep(c)).collect());
        }
        if cl.digits {
            classes.push(('0'..='9').filter(|&c| keep(c)).collect());
        }
        if cl.symbols {
            classes.push(SYMBOLS.chars().filter(|&c| keep(c)).collect());
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

fn validate_length(length: u32) -> Result<(), GenError> {
    if (MIN_LEN..=MAX_LEN).contains(&length) {
        Ok(())
    } else {
        Err(GenError::LengthOutOfRange)
    }
}

/// The generic core — `generate_random` uses the OS CSPRNG; tests inject a
/// seeded RNG so the unbiased-sampling check is deterministic.
fn generate_with<R: Rng>(cfg: &RandomConfig, rng: &mut R) -> Result<GeneratedSecret, GenError> {
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

/// Draw `length` characters uniformly from `alphabet`. `choose` samples an index
/// via rejection (no modulo bias); the alphabet is guaranteed non-empty by
/// [`Alphabet::build`], so every draw yields a character.
fn sample<R: Rng>(alphabet: &[char], length: u32, rng: &mut R) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(length as usize));
    for _ in 0..length {
        if let Some(&c) = alphabet.choose(rng) {
            out.push(c);
        }
    }
    out
}

fn satisfies_require_each(s: &str, classes: &[Vec<char>]) -> bool {
    classes
        .iter()
        .all(|cls| s.chars().any(|c| cls.contains(&c)))
}

mod entropy {
    //! Honest process-entropy math. All values are tiny and bounded (alphabet
    //! ≤ ~95, length ≤ 128, ≤ 4 classes → ≤ 16 inclusion-exclusion terms), so
    //! the denied float/int-arithmetic lints are allowed here with that bound —
    //! mirroring the accepted `vedge-tauri/src/pdf/emergency_kit.rs`
    //! presentation-math precedent.
    #![allow(
        clippy::float_arithmetic,
        clippy::arithmetic_side_effects,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::suboptimal_flops
    )]

    /// Entropy in bits of drawing a `length`-char string over an alphabet
    /// partitioned into `class_sizes`.
    ///
    /// Base case: `length × log2(A)` where `A = Σ class_sizes` (every position
    /// iid uniform). With `require_each` and ≥ 2 classes, the sample space is the
    /// strings that include ≥ 1 char from every class; count it by
    /// inclusion-exclusion over "class *i* absent":
    ///
    /// ```text
    /// valid   = Σ_{S⊆classes} (-1)^|S| · (A − Σ_{i∈S} a_i)^length
    /// entropy = log2(valid) = length·log2(A) + log2(valid / A^length)
    /// ```
    ///
    /// `valid / A^length ∈ (0, 1]` for `length ≥ #classes`, so the correction is
    /// a small honest *reduction*, never a fabricated increase.
    pub fn bits(length: u32, class_sizes: &[usize], require_each: bool) -> f64 {
        let alphabet: usize = class_sizes.iter().sum();
        let a = alphabet as f64;
        let base = f64::from(length) * a.log2();
        if !require_each || class_sizes.len() < 2 {
            return base;
        }
        let k = class_sizes.len();
        let mut p_valid = 0.0_f64;
        for mask in 0u32..(1u32 << k) {
            let mut removed = 0usize;
            let mut picked = 0u32;
            for (i, &size) in class_sizes.iter().enumerate() {
                if mask & (1u32 << i) != 0 {
                    removed += size;
                    picked += 1;
                }
            }
            let term = ((alphabet - removed) as f64 / a).powi(length as i32);
            if picked % 2 == 0 {
                p_valid += term;
            } else {
                p_valid -= term;
            }
        }
        base + p_valid.log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn entropy_band_thresholds() {
        assert_eq!(entropy_band(44.9), 1);
        assert_eq!(entropy_band(45.0), 2);
        assert_eq!(entropy_band(63.9), 2);
        assert_eq!(entropy_band(64.0), 3);
        assert_eq!(entropy_band(89.9), 3);
        assert_eq!(entropy_band(90.0), 4);
        assert_eq!(entropy_band(256.0), 4);
    }
}
