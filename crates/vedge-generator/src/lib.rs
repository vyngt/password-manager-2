//! # vedge-generator
//!
//! Pure, stateless secret-generation engine.
//!
//! No `SQLite`, no OS integration, no serde/DB — just a CSPRNG and math.
//! Compiles to **both** native (OS RNG) and `wasm32` (browser
//! `crypto.getRandomValues`), so the Leptos/WASM UI calls it with zero IPC while
//! native CLI/server shells reuse the identical code.
//!
//! ## Modes
//!
//! Five modes dispatch through one [`GenSpec`] enum + [`generate`] /
//! [`entropy_bits`]: **random charset** ([`RandomConfig`]), **passphrase**
//! (EFF 7776 wordlist, [`PassphraseConfig`]), **PIN** ([`PinConfig`]),
//! **pronounceable** ([`PronounceableConfig`]), and **pattern**
//! ([`PatternConfig`]). The slice-3.1 free functions [`generate_random`] /
//! [`random_entropy_bits`] are retained as thin delegates to the `Random` arm.
//!
//! ## One code path (the load-bearing invariant)
//!
//! For **every** mode, the generate fn derives its reported entropy from the
//! same helper its preview fn returns (all routed through the private `entropy`
//! module), so the live meter can never disagree with the generator — the
//! entropy is a property of the generation *process*, not of the output string.

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
use zeroize::Zeroizing;

mod charset;
mod entropy;
mod passphrase;
mod pattern;
mod pin;
mod pronounceable;
mod random;
mod wordlist;

pub use passphrase::{PassphraseConfig, Separator};
pub use pattern::PatternConfig;
pub use pin::PinConfig;
pub use pronounceable::PronounceableConfig;
pub use random::{CharClasses, RandomConfig, generate_random, random_entropy_bits};

/// Minimum secret length the length-based modes will produce.
pub const MIN_LEN: u32 = 4;
/// Maximum secret length the length-based modes will produce.
pub const MAX_LEN: u32 = 128;
/// Maximum number of secrets a single [`generate_many`] batch will produce.
pub const MAX_BATCH: u32 = 50;

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

/// Why generation (or an entropy preview) could not proceed. Payload-free, so
/// the type stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenError {
    /// No character class was enabled (random mode).
    NoClassSelected,
    /// A length was outside `[MIN_LEN, MAX_LEN]` (random / PIN / pronounceable /
    /// pattern), or a require-each constraint was unsatisfiable at that length.
    LengthOutOfRange,
    /// The effective alphabet was empty (random mode; not reachable with the
    /// built-in sets).
    EmptyEffectiveAlphabet,
    /// Passphrase word count was outside the supported range.
    WordCountOutOfRange,
    /// A pattern had no random token (empty string or all literals).
    EmptyPattern,
    /// A pattern was malformed (a dangling escape).
    InvalidPattern,
    /// A bulk [`generate_many`] count was outside `[1, MAX_BATCH]`.
    BatchOutOfRange,
}

impl core::fmt::Display for GenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NoClassSelected => "no character class selected",
            Self::LengthOutOfRange => "length out of range",
            Self::EmptyEffectiveAlphabet => "effective alphabet is empty",
            Self::WordCountOutOfRange => "word count out of range",
            Self::EmptyPattern => "pattern has no random tokens",
            Self::InvalidPattern => "invalid pattern",
            Self::BatchOutOfRange => "batch count out of range",
        })
    }
}

impl std::error::Error for GenError {}

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

/// Shared length validation for the length-based modes.
pub(crate) fn validate_length(length: u32) -> Result<(), GenError> {
    if (MIN_LEN..=MAX_LEN).contains(&length) {
        Ok(())
    } else {
        Err(GenError::LengthOutOfRange)
    }
}

/// A full generation spec across all five modes. `Clone` (not `Copy`) — the
/// `Pattern` arm owns a `String`.
#[derive(Debug, Clone)]
pub enum GenSpec {
    /// Random charset mode.
    Random(RandomConfig),
    /// EFF-wordlist passphrase mode.
    Passphrase(PassphraseConfig),
    /// Digits-only PIN mode.
    Pin(PinConfig),
    /// Consonant/vowel pronounceable mode.
    Pronounceable(PronounceableConfig),
    /// Token-grammar pattern mode.
    Pattern(PatternConfig),
}

/// Generate a secret for `spec` from the OS/browser CSPRNG.
///
/// # Errors
/// The mode-specific [`GenError`] for the active arm (see each config type).
pub fn generate(spec: &GenSpec) -> Result<GeneratedSecret, GenError> {
    generate_with_spec(spec, &mut rand::rng())
}

/// Entropy preview for `spec` **without drawing a secret** — drives the live
/// meter. Shares each arm's math with [`generate`], so it can never lie.
///
/// # Errors
/// The mode-specific [`GenError`] for the active arm.
pub fn entropy_bits(spec: &GenSpec) -> Result<f64, GenError> {
    match spec {
        GenSpec::Random(c) => random::random_entropy_bits(c),
        GenSpec::Passphrase(c) => passphrase::passphrase_entropy(*c),
        GenSpec::Pin(c) => pin::pin_entropy(*c),
        GenSpec::Pronounceable(c) => pronounceable::pronounceable_entropy(*c),
        GenSpec::Pattern(c) => pattern::pattern_entropy(c),
    }
}

/// Generic dispatch core — `generate` uses the OS CSPRNG; tests inject a seeded
/// RNG for determinism.
pub(crate) fn generate_with_spec<R: Rng>(
    spec: &GenSpec,
    rng: &mut R,
) -> Result<GeneratedSecret, GenError> {
    match spec {
        GenSpec::Random(c) => random::generate_with(c, rng),
        GenSpec::Passphrase(c) => passphrase::passphrase_with(*c, rng),
        GenSpec::Pin(c) => pin::pin_with(*c, rng),
        GenSpec::Pronounceable(c) => pronounceable::pronounceable_with(*c, rng),
        GenSpec::Pattern(c) => pattern::pattern_with(c, rng),
    }
}

/// Generate `count` independent secrets for `spec` from the OS/browser CSPRNG.
///
/// Each item is an independent CSPRNG draw (the RNG is fetched once and looped),
/// so — for a fixed spec — every item carries the same `entropy_bits` but a
/// distinct secret. The whole batch is a deliberate, bounded set of live copies
/// (see the session-history memory-hygiene note); `count` is capped at
/// [`MAX_BATCH`].
///
/// # Errors
/// - [`GenError::BatchOutOfRange`] if `count` is not in `[1, MAX_BATCH]`.
/// - The mode-specific [`GenError`] for the active arm (validated once, before
///   the loop, so an invalid spec fails fast without allocating a partial batch).
pub fn generate_many(spec: &GenSpec, count: u32) -> Result<Vec<GeneratedSecret>, GenError> {
    generate_many_with(spec, count, &mut rand::rng())
}

/// Seeded core for [`generate_many`] — mirrors the [`generate`] /
/// [`generate_with_spec`] split so tests can inject a deterministic RNG.
pub(crate) fn generate_many_with<R: Rng>(
    spec: &GenSpec,
    count: u32,
    rng: &mut R,
) -> Result<Vec<GeneratedSecret>, GenError> {
    if !(1..=MAX_BATCH).contains(&count) {
        return Err(GenError::BatchOutOfRange);
    }
    // Validate the config once up front so an invalid spec fails fast without
    // allocating (the entropy preview shares every arm's validation with
    // `generate_with_spec`).
    entropy_bits(spec)?;
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(generate_with_spec(spec, rng)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn random_delegates_to_free_functions() {
        let cfg = RandomConfig::default();
        // Preview parity.
        assert_eq!(
            entropy_bits(&GenSpec::Random(cfg)).unwrap(),
            random_entropy_bits(&cfg).unwrap()
        );
        // Seeded generate parity: same seed → same secret + entropy via both paths.
        let mut a = StdRng::seed_from_u64(1);
        let mut b = StdRng::seed_from_u64(1);
        let via_spec = generate_with_spec(&GenSpec::Random(cfg), &mut a).unwrap();
        let via_free = random::generate_with(&cfg, &mut b).unwrap();
        assert_eq!(via_spec.secret.as_str(), via_free.secret.as_str());
        assert_eq!(via_spec.entropy_bits, via_free.entropy_bits);
    }

    #[test]
    fn dispatch_covers_all_modes_preview_equals_generate() {
        let specs = [
            GenSpec::Random(RandomConfig::default()),
            GenSpec::Passphrase(PassphraseConfig::default()),
            GenSpec::Pin(PinConfig::default()),
            GenSpec::Pronounceable(PronounceableConfig::default()),
            GenSpec::Pattern(PatternConfig::default()),
        ];
        for spec in specs {
            let preview = entropy_bits(&spec).unwrap();
            assert!(preview > 0.0, "preview must be positive for {spec:?}");
            let out = generate(&spec).unwrap();
            assert!(
                !out.secret.is_empty(),
                "secret must be non-empty for {spec:?}"
            );
            // The one-code-path invariant, across the whole dispatch.
            assert_eq!(out.entropy_bits, preview);
        }
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

    #[test]
    fn generate_many_returns_exact_count_and_per_item_entropy() {
        let spec = GenSpec::Random(RandomConfig::default());
        let expected_bits = entropy_bits(&spec).unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let batch = generate_many_with(&spec, 12, &mut rng).unwrap();
        assert_eq!(batch.len(), 12);
        for item in &batch {
            assert!(!item.secret.is_empty());
            // Fixed config → identical process entropy for every item.
            assert_eq!(item.entropy_bits, expected_bits);
        }
    }

    #[test]
    fn generate_many_is_deterministic_under_seed() {
        let spec = GenSpec::Passphrase(PassphraseConfig::default());
        let mut a = StdRng::seed_from_u64(99);
        let mut b = StdRng::seed_from_u64(99);
        let batch_a = generate_many_with(&spec, 8, &mut a).unwrap();
        let batch_b = generate_many_with(&spec, 8, &mut b).unwrap();
        let seq_a: Vec<&str> = batch_a.iter().map(|g| g.secret.as_str()).collect();
        let seq_b: Vec<&str> = batch_b.iter().map(|g| g.secret.as_str()).collect();
        assert_eq!(seq_a, seq_b);
    }

    #[test]
    fn generate_many_items_are_distinct_for_high_entropy_spec() {
        use std::collections::HashSet;
        let spec = GenSpec::Random(RandomConfig::default()); // 20 chars, all classes → ~130 bits
        let mut rng = StdRng::seed_from_u64(3);
        let batch = generate_many_with(&spec, MAX_BATCH, &mut rng).unwrap();
        let distinct: HashSet<&str> = batch.iter().map(|g| g.secret.as_str()).collect();
        assert_eq!(distinct.len(), batch.len(), "all secrets should differ");
    }

    #[test]
    fn generate_many_rejects_out_of_range_count() {
        // `GeneratedSecret` is deliberately not `Debug`/`PartialEq`, so assert on
        // the error via `matches!` rather than `assert_eq!` on the whole `Result`.
        let spec = GenSpec::Pin(PinConfig::default());
        let mut rng = StdRng::seed_from_u64(1);
        assert!(matches!(
            generate_many_with(&spec, 0, &mut rng),
            Err(GenError::BatchOutOfRange)
        ));
        assert!(matches!(
            generate_many_with(&spec, MAX_BATCH + 1, &mut rng),
            Err(GenError::BatchOutOfRange)
        ));
    }

    #[test]
    fn generate_many_fails_fast_on_invalid_config() {
        // Random with every class disabled → NoClassSelected, surfaced before the loop.
        let cfg = RandomConfig {
            classes: CharClasses {
                lowercase: false,
                uppercase: false,
                digits: false,
                symbols: false,
            },
            ..RandomConfig::default()
        };
        let spec = GenSpec::Random(cfg);
        let mut rng = StdRng::seed_from_u64(1);
        assert!(matches!(
            generate_many_with(&spec, 10, &mut rng),
            Err(GenError::NoClassSelected)
        ));
    }
}
