//! Embedded EFF Large Wordlist (Diceware) for the passphrase mode — the repo's
//! first embedded asset.
//!
//! Source: Electronic Frontier Foundation, 2016 —
//! <https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt>. Committed
//! **words-only** (the leading `NNNNN\t` dice column stripped), 7776 entries,
//! one per line, LF endings (pinned by `.gitattributes` so the hash is stable
//! across checkouts regardless of `core.autocrlf`).
//!
//! Integrity: the SHA-256 of the normalized list — `WORDS.join("\n")`, i.e. the
//! committed file with its trailing newline removed —
//! is `abae49761b88f3f1ba31ef944bea1f61b795a3cd7e1cfb7d276ed45bf77967ba`, pinned
//! and asserted by the `wordlist_sha256` test.

use std::sync::LazyLock;

const RAW: &str = include_str!("wordlists/eff_large.txt");

/// The parsed wordlist — trimmed, non-empty lines borrowed from `RAW` (zero-copy).
/// Built once on first use; safe on `wasm32` (single-threaded, no lock contention).
pub static WORDS: LazyLock<Box<[&'static str]>> = LazyLock::new(|| {
    RAW.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
});

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;

    /// EFF Large Wordlist entry count (`6^5`).
    const WORD_COUNT: usize = 7776;
    /// SHA-256 of `WORDS.join("\n")`.
    const SHA256_HEX: &str = "abae49761b88f3f1ba31ef944bea1f61b795a3cd7e1cfb7d276ed45bf77967ba";

    #[test]
    fn wordlist_count_and_unique() {
        assert_eq!(
            WORDS.len(),
            WORD_COUNT,
            "wordlist must hold exactly 7776 entries"
        );
        let unique: HashSet<&&str> = WORDS.iter().collect();
        assert_eq!(unique.len(), WORD_COUNT, "wordlist entries must be unique");
    }

    #[test]
    fn wordlist_sha256() {
        use std::fmt::Write as _;
        let joined = WORDS.join("\n");
        let digest = Sha256::digest(joined.as_bytes());
        let mut hex = String::with_capacity(64);
        for b in digest {
            write!(hex, "{b:02x}").ok();
        }
        assert_eq!(hex, SHA256_HEX, "embedded wordlist tampered or re-encoded");
    }
}
