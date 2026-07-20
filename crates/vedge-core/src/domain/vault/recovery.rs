//! Recovery-key format — Crockford base32 + HMAC-SHA256 checksum.
//!
//! The Emergency Kit stores a 16-byte Secret Key in a human-typable form:
//!
//! ```text
//! A3-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX
//! ```
//!
//! - `A3` — literal version prefix. Bumped to `B1`, `C1`, … on format
//!   changes. Not part of the base32 payload.
//! - 6 groups × 5 chars = 30 base32 data chars → 18-byte payload
//!   (6 trailing padding bits).
//! - Payload layout: `[0..16]` Secret Key, `[16..18]` checksum.
//!
//! Checksum: first 2 bytes of `HMAC-SHA256(key = secret_key, data =
//! b"vedge-v1-kit-checksum")`. A single random character flip fails
//! ~`1 - 2^-16` of the time — good enough to reject typos before the
//! caller wastes ~500 ms on Argon2id.
//!
//! Parser tolerance: case-insensitive; `I`/`L` → `1` and `O` → `0`
//! (Crockford rules); hyphens and whitespace stripped in any position.
//! The canonical display always uses the strict alphabet.
//!
//! Error messages never echo the user's input.

use hmac::{Hmac, Mac, digest::KeyInit};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

/// Display-facing version prefix. Lives here (not `crypto_constants`) because
/// it is a user-visible token rather than a crypto constant.
pub const RECOVERY_FORMAT_PREFIX: &str = "A3";

/// HMAC personalization — bound to this format version.
const CHECKSUM_CONTEXT: &[u8] = b"vedge-v1-kit-checksum";

/// Length of the truncated HMAC appended to the payload.
const CHECKSUM_LEN: usize = 2;

/// Crockford base32 alphabet (no I/L/O/U).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Secret Key + checksum. No version byte inside the payload — the `A3`
/// display prefix carries the version indicator.
const PAYLOAD_LEN: usize = SECRET_KEY_LEN + CHECKSUM_LEN;

/// Number of base32 characters that follow the literal `A3` prefix.
/// 6 groups × 5 chars.
const DATA_CHARS: usize = 30;

/// Total characters in the canonical display after stripping hyphens and
/// whitespace: `A3` prefix + [`DATA_CHARS`] base32 data chars.
const ENCODED_CHARS: usize = 2 + DATA_CHARS;

/// Encode a Secret Key as a printable `A3-XXXXX-XXXXX-…-XXXXX` string.
///
/// The returned `String` is safe to display (that's the point of an
/// Emergency Kit). Callers are responsible for not lingering on the string
/// after the user has stored it — zeroization is their problem, because the
/// whole point is that it reaches a user-readable surface.
#[must_use]
pub fn format_secret_key(key: &[u8; SECRET_KEY_LEN]) -> String {
    let mut payload = [0u8; PAYLOAD_LEN];
    if let Some(slot) = payload.get_mut(..SECRET_KEY_LEN) {
        slot.copy_from_slice(key);
    }
    let checksum = compute_checksum(key);
    if let Some(slot) = payload.get_mut(SECRET_KEY_LEN..) {
        slot.copy_from_slice(&checksum);
    }

    let data = encode_fixed(&payload);

    // Layout: A3-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX
    let mut out = String::with_capacity(ENCODED_CHARS.saturating_add(6));
    out.push_str(RECOVERY_FORMAT_PREFIX);
    for chunk_idx in 0_usize..6 {
        out.push('-');
        let start = chunk_idx.saturating_mul(5);
        let end = start.saturating_add(5);
        if let Some(slice) = data.get(start..end) {
            out.push_str(slice);
        }
    }
    out
}

/// Parse a display string back into a 16-byte Secret Key.
///
/// Accepts upper- or lower-case, any hyphens or whitespace layout, and
/// Crockford confusables (`I`/`L` → `1`, `O` → `0`).
///
/// Rejects:
/// - Unknown version prefix (anything other than `A3`).
/// - Wrong length after normalization.
/// - Characters outside the Crockford alphabet (after substitution).
/// - Checksum mismatch.
///
/// Errors never echo the caller's input.
pub fn parse_secret_key(s: &str) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
    let normalized: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .map(canonicalize_char)
        .collect();

    if normalized.len() != ENCODED_CHARS {
        return Err(VaultError::InvalidRecoveryKey(format!(
            "expected {ENCODED_CHARS} characters after normalization, got {}",
            normalized.len(),
        )));
    }

    let (prefix, data) = normalized.split_at(2);
    if prefix != RECOVERY_FORMAT_PREFIX {
        return Err(VaultError::InvalidRecoveryKey(
            "unknown format prefix — upgrade Vedge or check for typos".to_owned(),
        ));
    }

    let decoded = decode_fixed(data)?;

    // Strict padding check: re-encode the decoded bytes and compare against
    // the canonicalized input. This rejects flips of the 6 trailing padding
    // bits in the last two data chars, which otherwise decode to the same
    // payload and would pass the checksum. Because `normalized` already had
    // confusables resolved, the comparison is against the strict alphabet.
    if encode_fixed(&decoded) != data {
        return Err(VaultError::InvalidRecoveryKey(
            "non-canonical encoding — trailing padding bits must be zero".to_owned(),
        ));
    }

    let key_slice = decoded
        .get(..SECRET_KEY_LEN)
        .ok_or_else(|| VaultError::InvalidRecoveryKey("truncated payload".to_owned()))?;
    let mut key = [0u8; SECRET_KEY_LEN];
    key.copy_from_slice(key_slice);

    let checksum_slice = decoded
        .get(SECRET_KEY_LEN..PAYLOAD_LEN)
        .ok_or_else(|| VaultError::InvalidRecoveryKey("truncated payload".to_owned()))?;
    let expected = compute_checksum(&key);

    if checksum_slice.ct_eq(&expected).unwrap_u8() != 1 {
        return Err(VaultError::InvalidRecoveryKey(
            "checksum mismatch — likely a typo".to_owned(),
        ));
    }

    Ok(Zeroizing::new(key))
}

fn compute_checksum(key: &[u8; SECRET_KEY_LEN]) -> [u8; CHECKSUM_LEN] {
    // HMAC-SHA256 accepts keys of any length, so `new_from_slice` never
    // actually fails for this type. The `Err` arm is mathematically
    // unreachable; returning zeros rather than panicking keeps the call
    // site free of `unwrap`/`expect` while being harmless in the
    // impossible fallback case (encode and decode both use the same
    // function so any fixed fallback round-trips).
    let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(key) else {
        return [0u8; CHECKSUM_LEN];
    };
    mac.update(CHECKSUM_CONTEXT);
    let tag = mac.finalize().into_bytes();
    let mut out = [0u8; CHECKSUM_LEN];
    if let Some(slice) = tag.get(..CHECKSUM_LEN) {
        out.copy_from_slice(slice);
    }
    out
}

/// Encode [`PAYLOAD_LEN`] bytes to exactly [`DATA_CHARS`] Crockford base32
/// chars, padding the trailing partial group (and any char beyond the
/// natural output length) with the zero symbol `0`.
#[allow(clippy::cast_possible_truncation)]
fn encode_fixed(input: &[u8; PAYLOAD_LEN]) -> String {
    let mut out = String::with_capacity(DATA_CHARS);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in input {
        buffer = (buffer << 8) | u32::from(byte);
        bits = bits.saturating_add(8);
        while bits >= 5 {
            bits = bits.saturating_sub(5);
            let idx = ((buffer >> bits) & 0x1f) as usize;
            if let Some(&c) = ALPHABET.get(idx) {
                out.push(c as char);
            }
        }
    }
    if bits > 0 {
        let shift = 5u32.saturating_sub(bits);
        let idx = ((buffer << shift) & 0x1f) as usize;
        if let Some(&c) = ALPHABET.get(idx) {
            out.push(c as char);
        }
    }
    // Pad to fixed length with `0` (index 0 of the Crockford alphabet).
    while out.len() < DATA_CHARS {
        out.push('0');
    }
    out
}

/// Apply Crockford confusable substitutions on input only: `I`/`L` → `1`,
/// `O` → `0`. Canonical output never emits these chars.
const fn canonicalize_char(c: char) -> char {
    match c {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    }
}

/// Decode exactly [`DATA_CHARS`] canonical Crockford base32 chars into a
/// [`PAYLOAD_LEN`]-byte payload. Caller is expected to apply
/// [`canonicalize_char`] first.
#[allow(clippy::cast_possible_truncation)]
fn decode_fixed(data: &str) -> Result<[u8; PAYLOAD_LEN], VaultError> {
    if data.len() != DATA_CHARS {
        return Err(VaultError::InvalidRecoveryKey(format!(
            "data section length mismatch (expected {DATA_CHARS}, got {})",
            data.len(),
        )));
    }

    let mut out = [0u8; PAYLOAD_LEN];
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    let mut cursor: usize = 0;

    for ch in data.chars() {
        let Some(value) = ALPHABET.iter().position(|&b| (b as char) == ch) else {
            return Err(VaultError::InvalidRecoveryKey(
                "invalid character in recovery key".to_owned(),
            ));
        };
        buffer = (buffer << 5) | value as u32;
        bits = bits.saturating_add(5);
        if bits >= 8 {
            bits = bits.saturating_sub(8);
            let byte = ((buffer >> bits) & 0xff) as u8;
            if let Some(slot) = out.get_mut(cursor) {
                *slot = byte;
                cursor = cursor.saturating_add(1);
            }
        }
    }

    if cursor != PAYLOAD_LEN {
        return Err(VaultError::InvalidRecoveryKey(format!(
            "decoded {cursor} bytes, expected {PAYLOAD_LEN}",
        )));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key() -> [u8; SECRET_KEY_LEN] {
        [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ]
    }

    #[test]
    fn round_trip_sample_key() {
        let key = sample_key();
        let display = format_secret_key(&key);
        let parsed = parse_secret_key(&display).expect("valid display parses");
        assert_eq!(*parsed, key);
    }

    #[test]
    fn round_trip_all_zeros() {
        let key = [0u8; SECRET_KEY_LEN];
        let display = format_secret_key(&key);
        let parsed = parse_secret_key(&display).expect("zero key round-trips");
        assert_eq!(*parsed, key);
    }

    #[test]
    fn round_trip_all_ones() {
        let key = [0xffu8; SECRET_KEY_LEN];
        let display = format_secret_key(&key);
        let parsed = parse_secret_key(&display).expect("all-ones key round-trips");
        assert_eq!(*parsed, key);
    }

    #[test]
    fn display_shape_is_a3_then_six_groups_of_five() {
        let display = format_secret_key(&sample_key());
        let groups: Vec<&str> = display.split('-').collect();
        assert_eq!(groups.len(), 7, "got {display}");
        assert_eq!(groups[0], "A3");
        for group in &groups[1..] {
            assert_eq!(group.len(), 5, "group {group} wrong length");
        }
    }

    #[test]
    fn canonical_display_uses_no_excluded_chars() {
        // Crockford excludes I, L, O, U.
        let display = format_secret_key(&sample_key());
        for c in display.chars() {
            if c == '-' {
                continue;
            }
            assert!(
                !matches!(c, 'I' | 'L' | 'O' | 'U'),
                "canonical display must not emit `{c}`",
            );
        }
    }

    #[test]
    fn lowercase_input_parses_identically() {
        let display = format_secret_key(&sample_key());
        let lowered = display.to_lowercase();
        let parsed = parse_secret_key(&lowered).expect("lowercase parses");
        assert_eq!(*parsed, sample_key());
    }

    #[test]
    fn whitespace_and_missing_hyphens_tolerated() {
        let display = format_secret_key(&sample_key());
        let stripped: String = display.chars().filter(|c| *c != '-').collect();
        let with_spaces = stripped
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && i % 5 == 0 {
                    vec![' ', c]
                } else {
                    vec![c]
                }
            })
            .collect::<String>();
        let parsed = parse_secret_key(&with_spaces).expect("spaces tolerated");
        assert_eq!(*parsed, sample_key());
    }

    #[test]
    fn crockford_confusables_are_tolerated_on_input() {
        let display = format_secret_key(&sample_key());
        let mutated: String = display
            .chars()
            .map(|c| match c {
                '0' => 'O',
                '1' => 'I',
                other => other,
            })
            .collect();
        let parsed = parse_secret_key(&mutated).expect("confusables tolerated");
        assert_eq!(*parsed, sample_key());

        let mutated_l: String = display
            .chars()
            .map(|c| if c == '1' { 'L' } else { c })
            .collect();
        let parsed_l = parse_secret_key(&mutated_l).expect("L tolerated as 1");
        assert_eq!(*parsed_l, sample_key());
    }

    #[test]
    fn wrong_prefix_rejected() {
        let mut display = format_secret_key(&sample_key());
        display.replace_range(0..2, "B3");
        let err = parse_secret_key(&display).expect_err("wrong prefix must fail");
        match err {
            VaultError::InvalidRecoveryKey(msg) => {
                assert!(msg.contains("prefix"), "got: {msg}");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn truncated_input_rejected() {
        let display = format_secret_key(&sample_key());
        let truncated = &display[..display.len().saturating_sub(1)];
        assert!(parse_secret_key(truncated).is_err());
    }

    #[test]
    fn overlong_input_rejected() {
        let mut display = format_secret_key(&sample_key());
        display.push('Z');
        assert!(parse_secret_key(&display).is_err());
    }

    #[test]
    fn invalid_character_rejected() {
        let mut display = format_secret_key(&sample_key());
        // `U` is explicitly excluded by Crockford and has no confusable
        // mapping. Replace the first data char with it.
        let first_data_idx = 3;
        display.replace_range(first_data_idx..=first_data_idx, "U");
        let err = parse_secret_key(&display).expect_err("U must fail");
        match err {
            VaultError::InvalidRecoveryKey(msg) => {
                assert!(
                    msg.contains("invalid character") || msg.contains("checksum"),
                    "got: {msg}",
                );
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn single_character_flip_breaks_checksum() {
        // Deterministically flip every data-section character to the
        // alphabet's first non-matching char. With a 16-bit HMAC tail the
        // collision rate is ~2⁻¹⁶; all flips must fail.
        let key = sample_key();
        let display = format_secret_key(&key);
        let chars: Vec<char> = display.chars().collect();

        let mut flips_tested: u32 = 0;
        let mut flips_rejected: u32 = 0;

        for (pos, &orig) in chars.iter().enumerate() {
            if pos < 2 || orig == '-' {
                continue;
            }
            let alt = ALPHABET
                .iter()
                .map(|&b| b as char)
                .find(|c| *c != orig)
                .unwrap_or('Z');
            let mut mutated = chars.clone();
            if let Some(slot) = mutated.get_mut(pos) {
                *slot = alt;
            }
            let mutated_str: String = mutated.into_iter().collect();
            flips_tested = flips_tested.saturating_add(1);
            if parse_secret_key(&mutated_str).is_err() {
                flips_rejected = flips_rejected.saturating_add(1);
            }
        }

        assert!(flips_tested > 0);
        assert_eq!(
            flips_tested, flips_rejected,
            "every single-char flip must fail checksum",
        );
    }

    #[test]
    fn checksum_is_stable_across_calls() {
        let key = sample_key();
        let a = compute_checksum(&key);
        let b = compute_checksum(&key);
        assert_eq!(a, b);
    }

    #[test]
    fn different_keys_produce_different_checksums() {
        let a = compute_checksum(&sample_key());
        let mut other = sample_key();
        other[0] = other[0].wrapping_add(1);
        let b = compute_checksum(&other);
        assert_ne!(a, b);
    }

    #[test]
    fn error_message_does_not_echo_input() {
        let probe = "A3-SECRETSECRET-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX";
        let err = parse_secret_key(probe).expect_err("invalid input must fail");
        let msg = err.to_string();
        assert!(!msg.contains("SECRETSECRET"), "error echoed input: {msg}");
    }
}
