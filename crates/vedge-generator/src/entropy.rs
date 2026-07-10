//! Honest process-entropy math for every mode.
//!
//! All values are tiny and bounded (alphabet ≤ ~95, length ≤ 128, ≤ 4 classes →
//! ≤ 16 inclusion-exclusion terms; wordlist ≤ 7776), so the denied float/int
//! arithmetic lints are allowed here with that bound — mirroring the accepted
//! `vedge-tauri/src/pdf/emergency_kit.rs` presentation-math precedent. Every
//! mode routes its entropy through this one module, so a mode's `*_with`
//! generate fn and its `*_entropy` preview fn can never disagree.
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

/// `count × log2(pool)` — the entropy of `count` iid uniform draws from a pool of
/// `pool` distinct symbols (PIN digits, passphrase words).
pub fn uniform_bits(count: u32, pool: usize) -> f64 {
    f64::from(count) * (pool as f64).log2()
}

/// `Σ log2(size_i)` — independent draws from per-position pools (pattern tokens).
/// Literal positions are excluded by the caller, so they contribute 0.
pub fn log2_sum(sizes: &[usize]) -> f64 {
    sizes.iter().map(|&n| (n as f64).log2()).sum()
}

/// Passphrase entropy: `words × log2(wordlist_len)`, plus `log2(10)` when a
/// single uniform digit is appended at a **fixed** position (the position is
/// deterministic, so it is not credited — only the digit value's 10 outcomes).
pub fn passphrase_bits(words: u32, wordlist_len: usize, include_number: bool) -> f64 {
    let mut b = uniform_bits(words, wordlist_len);
    if include_number {
        b += 10.0_f64.log2();
    }
    b
}

/// Pronounceable entropy: `⌈len/2⌉·log2(cons) + ⌊len/2⌋·log2(vow)` for a
/// consonant/vowel alternation starting with a consonant. `div_ceil` + a
/// subtraction give the floor without an `integer_division` site.
pub fn pronounceable_bits(length: u32, cons: usize, vow: usize) -> f64 {
    let consonant_positions = length.div_ceil(2);
    let vowel_positions = length - consonant_positions;
    uniform_bits(consonant_positions, cons) + uniform_bits(vowel_positions, vow)
}
