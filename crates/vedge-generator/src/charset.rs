//! Character-class alphabets shared across the generator modes.
//!
//! The random mode partitions its alphabet by class (for require-each and
//! entropy accounting); the pattern mode's tokens and the PIN mode reuse the
//! very same class pools, so no alphabet is duplicated and entropy accounting
//! stays consistent across modes.

use rand::Rng;
use rand::seq::IndexedRandom;
use zeroize::Zeroizing;

// Every item here is `pub` inside a *private* module (declared `mod charset;` in
// lib.rs), so it is crate-visible only — `pub(crate)` would be redundant here
// (the `redundant_pub_crate` nursery lint), matching the shipped `mod entropy`.

/// The fixed, documented symbol set. Changing it changes entropy accounting, so
/// it is a constant, not a config knob.
pub const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}|;:,.<>?/";

/// The fixed "visually ambiguous" set removed when `exclude_ambiguous` is on:
/// capital-O / zero / lower-o, capital-I / lower-l / one, and pipe. Spans several
/// classes on purpose (`0` is a digit, `|` a symbol, the rest letters).
pub const AMBIGUOUS: [char; 7] = ['O', '0', 'o', 'I', 'l', '1', '|'];

/// `a`–`z` (26).
pub fn class_lower() -> Vec<char> {
    ('a'..='z').collect()
}

/// `A`–`Z` (26).
pub fn class_upper() -> Vec<char> {
    ('A'..='Z').collect()
}

/// `0`–`9` (10).
pub fn class_digit() -> Vec<char> {
    ('0'..='9').collect()
}

/// The fixed [`SYMBOLS`] set.
pub fn class_symbol() -> Vec<char> {
    SYMBOLS.chars().collect()
}

/// The full charset (lower + upper + digit + symbol) — the pattern `*` pool.
pub fn class_all() -> Vec<char> {
    let mut v = class_lower();
    v.extend(class_upper());
    v.extend(class_digit());
    v.extend(class_symbol());
    v
}

/// Draw `length` characters uniformly from `alphabet`. `choose` samples an index
/// via rejection (no modulo bias); callers guarantee a non-empty alphabet, so
/// every draw yields a character.
pub fn sample<R: Rng>(alphabet: &[char], length: u32, rng: &mut R) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(length as usize));
    for _ in 0..length {
        if let Some(&c) = alphabet.choose(rng) {
            out.push(c);
        }
    }
    out
}
