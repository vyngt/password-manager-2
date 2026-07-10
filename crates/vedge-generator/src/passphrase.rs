//! Passphrase mode — EFF Large Wordlist (7776 words), uniform word draws. Honest
//! entropy `words × log2(7776)` (≈ 12.9 bits/word, independent of word length),
//! plus `log2(10)` for the optional appended digit. Separator and capitalization
//! are deterministic transforms worth **0 bits** — never scored by length.

use crate::charset::class_digit;
use crate::wordlist::WORDS;
use crate::{GenError, GeneratedSecret, entropy};
use rand::Rng;
use rand::seq::IndexedRandom;
use zeroize::Zeroizing;

/// Word-count bounds for the passphrase mode.
const MIN_WORDS: u32 = 3;
const MAX_WORDS: u32 = 12;

/// How generated words are joined. Formatting only — contributes 0 entropy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Separator {
    /// `-` (default).
    #[default]
    Hyphen,
    /// ` `
    Space,
    /// `.`
    Dot,
    /// `_`
    Underscore,
    /// no separator
    None,
}

impl Separator {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Hyphen => "-",
            Self::Space => " ",
            Self::Dot => ".",
            Self::Underscore => "_",
            Self::None => "",
        }
    }
}

/// Configuration for the passphrase mode. `Copy` (≤ 8 bytes) → passed by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassphraseConfig {
    /// Number of words; must be within `[MIN_WORDS, MAX_WORDS]` (3..=12).
    pub words: u32,
    /// How the words are joined.
    pub separator: Separator,
    /// Title-case each word.
    pub capitalize: bool,
    /// Append one uniform digit `0–9` to the last word.
    pub include_number: bool,
}

impl Default for PassphraseConfig {
    fn default() -> Self {
        Self {
            words: 6,
            separator: Separator::Hyphen,
            capitalize: false,
            include_number: false,
        }
    }
}

fn validate_words(words: u32) -> Result<(), GenError> {
    if (MIN_WORDS..=MAX_WORDS).contains(&words) {
        Ok(())
    } else {
        Err(GenError::WordCountOutOfRange)
    }
}

/// Entropy preview: `words × log2(7776)` (+ `log2(10)` if a digit is appended).
///
/// # Errors
/// [`GenError::WordCountOutOfRange`] if `words` is outside `3..=12`.
pub fn passphrase_entropy(cfg: PassphraseConfig) -> Result<f64, GenError> {
    validate_words(cfg.words)?;
    Ok(entropy::passphrase_bits(
        cfg.words,
        WORDS.len(),
        cfg.include_number,
    ))
}

/// Draw `words` uniform words, join, optionally capitalize + append one digit.
/// Shares [`passphrase_entropy`]'s one number + validation.
pub fn passphrase_with<R: Rng>(
    cfg: PassphraseConfig,
    rng: &mut R,
) -> Result<GeneratedSecret, GenError> {
    let entropy_bits = passphrase_entropy(cfg)?;
    let mut words: Vec<String> = Vec::with_capacity(cfg.words as usize);
    for _ in 0..cfg.words {
        // WORDS is never empty; `unwrap_or` keeps this panic-free regardless.
        let word = WORDS.choose(rng).copied().unwrap_or("");
        words.push(if cfg.capitalize {
            capitalize_word(word)
        } else {
            word.to_owned()
        });
    }
    // Append one uniform digit to a FIXED word (the last) so the process entropy
    // is exactly +log2(10) — the position is deterministic, never credited.
    if cfg.include_number {
        let digits = class_digit();
        if let (Some(&d), Some(last)) = (digits.choose(rng), words.last_mut()) {
            last.push(d);
        }
    }
    let joined = words.join(cfg.separator.as_str());
    Ok(GeneratedSecret {
        secret: Zeroizing::new(joined),
        entropy_bits,
    })
}

/// Upper-case the first character of `w`, leaving the rest unchanged.
fn capitalize_word(w: &str) -> String {
    let mut chars = w.chars();
    chars.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_uppercase(), chars.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenError;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashSet;

    fn base() -> PassphraseConfig {
        PassphraseConfig {
            words: 5,
            separator: Separator::Hyphen,
            capitalize: false,
            include_number: false,
        }
    }

    #[test]
    fn passphrase_entropy_closed_form() {
        let bits = passphrase_entropy(base()).unwrap();
        assert!((bits - 5.0 * 7776.0_f64.log2()).abs() < 1e-9);

        // include_number adds exactly log2(10).
        let mut c = base();
        c.include_number = true;
        let bits2 = passphrase_entropy(c).unwrap();
        assert!((bits2 - (5.0 * 7776.0_f64.log2() + 10.0_f64.log2())).abs() < 1e-9);
    }

    #[test]
    fn passphrase_preview_matches_generate() {
        for (sep, cap, num) in [
            (Separator::Hyphen, false, false),
            (Separator::Space, true, false),
            (Separator::None, true, true),
        ] {
            let cfg = PassphraseConfig {
                words: 6,
                separator: sep,
                capitalize: cap,
                include_number: num,
            };
            let mut rng = StdRng::seed_from_u64(21);
            assert_eq!(
                passphrase_entropy(cfg).unwrap(),
                passphrase_with(cfg, &mut rng).unwrap().entropy_bits
            );
        }
    }

    #[test]
    fn passphrase_format_word_count_and_capitalize() {
        let cfg = PassphraseConfig {
            words: 4,
            separator: Separator::Hyphen,
            capitalize: true,
            include_number: false,
        };
        let mut rng = StdRng::seed_from_u64(2);
        let out = passphrase_with(cfg, &mut rng).unwrap();
        let parts: Vec<&str> = out.secret.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert!(
            parts
                .iter()
                .all(|w| w.chars().next().unwrap().is_ascii_uppercase())
        );
    }

    #[test]
    fn passphrase_include_number_appends_one_digit_to_last_word() {
        let cfg = PassphraseConfig {
            words: 3,
            separator: Separator::Hyphen,
            capitalize: false,
            include_number: true,
        };
        let mut rng = StdRng::seed_from_u64(9);
        let out = passphrase_with(cfg, &mut rng).unwrap();
        let digit_count = out.secret.chars().filter(char::is_ascii_digit).count();
        assert_eq!(digit_count, 1, "exactly one appended digit");
        let parts: Vec<&str> = out.secret.split('-').collect();
        assert!(
            parts
                .last()
                .unwrap()
                .chars()
                .last()
                .unwrap()
                .is_ascii_digit()
        );
    }

    #[test]
    fn passphrase_word_count_range() {
        assert_eq!(
            passphrase_entropy(PassphraseConfig { words: 2, ..base() }).err(),
            Some(GenError::WordCountOutOfRange)
        );
        assert_eq!(
            passphrase_entropy(PassphraseConfig {
                words: 13,
                ..base()
            })
            .err(),
            Some(GenError::WordCountOutOfRange)
        );
        assert!(passphrase_entropy(PassphraseConfig { words: 3, ..base() }).is_ok());
        assert!(
            passphrase_entropy(PassphraseConfig {
                words: 12,
                ..base()
            })
            .is_ok()
        );
    }

    #[test]
    fn word_sampling_is_broad() {
        // Seeded: 2000 × 3 = 6000 word draws from 7776 → expect wide coverage
        // (coupon-collector ≈ 4180 distinct). Guards against index bias.
        let cfg = PassphraseConfig {
            words: 3,
            separator: Separator::Space,
            capitalize: false,
            include_number: false,
        };
        let mut rng = StdRng::seed_from_u64(0xABCD);
        let mut seen: HashSet<String> = HashSet::new();
        for _ in 0..2000 {
            let out = passphrase_with(cfg, &mut rng).unwrap();
            for w in out.secret.split(' ') {
                seen.insert(w.to_owned());
            }
        }
        assert!(seen.len() > 3000, "distinct words too few: {}", seen.len());
    }
}
