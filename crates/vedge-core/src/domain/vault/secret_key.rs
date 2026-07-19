//! Kit codecs — Crockford base32 + HMAC-SHA256 checksum.
//!
//! Two credentials share one codec, distinguished only by a [`KeyCodec`]
//! descriptor (prefix, key length, checksum context):
//!
//! ```text
//! A3-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX               (Emergency Kit — 16-byte Secret Key)
//! RK1-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX  (Recovery Kit — 32-byte Recovery Key, slice 5.7)
//! ```
//!
//! - The literal prefix is the version/type indicator, **not** part of the
//!   base32 payload. `A3` = Secret Key (bumped to `A4`, … on SK format
//!   changes); `RK1` = Recovery Key (slice 5.7). The two are deliberately in
//!   different namespaces so a Secret Key never parses as a Recovery Key and
//!   vice-versa (distinct length *and* distinct checksum context).
//! - Payload layout: `[0..key_len]` the key, `[key_len..]` a 2-byte checksum.
//! - `groups` × 5 chars of base32 encode the payload (trailing bits padded
//!   with `0`).
//!
//! Checksum: first 2 bytes of `HMAC-SHA256(key = raw_key, data =
//! checksum_context)`. A single random character flip fails ~`1 - 2^-16` of
//! the time — good enough to reject typos before the caller wastes ~500 ms on
//! Argon2id. The checksum context is per-codec, so an `A3-` string fed to the
//! recovery parser (or the reverse) fails on both length and checksum.
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

use crate::domain::vault::crypto_constants::{RECOVERY_KEY_LEN, SECRET_KEY_LEN};
use crate::domain::vault::errors::VaultError;

/// Display-facing version prefix for the Emergency Kit's Secret Key. Lives
/// here (not `crypto_constants`) because it is a user-visible token rather than
/// a crypto constant.
pub const SECRET_KEY_FORMAT_PREFIX: &str = "A3";

/// Display-facing version prefix for the Recovery Kit's Recovery Key (slice
/// 5.7). Deliberately outside the Secret Key's `A3`/`A4`/… namespace so the two
/// kits are unmistakable and mutually unparseable.
pub const RECOVERY_KEY_FORMAT_PREFIX: &str = "RK1";

/// Length of the truncated HMAC appended to the payload (both codecs).
const CHECKSUM_LEN: usize = 2;

/// Crockford base32 alphabet (no I/L/O/U).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Describes one kit codec. The bit-packing helpers are shared; only these
/// parameters differ between the Secret Key and the Recovery Key.
struct KeyCodec {
    /// Literal display prefix (`A3` / `RK1`) — the version/type indicator.
    prefix: &'static str,
    /// Raw key length in bytes (16 / 32).
    key_len: usize,
    /// Number of base32 data chars after the prefix = `groups * 5`.
    data_chars: usize,
    /// Number of hyphen-separated 5-char groups in the display.
    groups: usize,
    /// HMAC personalization — bound to this format version, distinct per codec.
    checksum_context: &'static [u8],
    /// Variant constructor for a parse failure (`InvalidSecretKey` /
    /// `InvalidRecoveryKey`) — mirrors `build_index`'s `remap_err` pattern.
    err: fn(String) -> VaultError,
}

impl KeyCodec {
    /// Payload = raw key + 2-byte checksum.
    const fn payload_len(&self) -> usize {
        self.key_len.saturating_add(CHECKSUM_LEN)
    }
    /// Canonical char count after stripping hyphens/whitespace: prefix + data.
    const fn encoded_chars(&self) -> usize {
        self.prefix.len().saturating_add(self.data_chars)
    }
}

/// The Emergency Kit codec — 16-byte Secret Key, 6 groups → 30 data chars
/// (6 trailing padding bits). **Frozen** (`golden_a3_output_is_frozen`).
const SECRET_KEY_CODEC: KeyCodec = KeyCodec {
    prefix: SECRET_KEY_FORMAT_PREFIX,
    key_len: SECRET_KEY_LEN,
    data_chars: 30,
    groups: 6,
    checksum_context: b"vedge-v1-kit-checksum",
    err: VaultError::InvalidSecretKey,
};

/// The Recovery Kit codec (slice 5.7) — 32-byte Recovery Key, 11 groups → 55
/// data chars (3 trailing padding bits). Frozen at introduction.
const RECOVERY_KEY_CODEC: KeyCodec = KeyCodec {
    prefix: RECOVERY_KEY_FORMAT_PREFIX,
    key_len: RECOVERY_KEY_LEN,
    data_chars: 55,
    groups: 11,
    checksum_context: b"vedge-v1-recovery-checksum",
    err: VaultError::InvalidRecoveryKey,
};

/// Encode a Secret Key as a printable `A3-XXXXX-…-XXXXX` string.
///
/// The returned `String` is safe to display (that's the point of an
/// Emergency Kit). Callers are responsible for not lingering on the string
/// after the user has stored it — zeroization is their problem, because the
/// whole point is that it reaches a user-readable surface.
#[must_use]
pub fn format_secret_key(key: &[u8; SECRET_KEY_LEN]) -> String {
    format_key(&SECRET_KEY_CODEC, key)
}

/// Parse a display string back into a 16-byte Secret Key.
///
/// Rejects an unknown prefix, wrong length, out-of-alphabet chars,
/// non-canonical padding, or a checksum mismatch. Errors never echo the
/// caller's input.
pub fn parse_secret_key(s: &str) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
    let bytes = parse_key(&SECRET_KEY_CODEC, s)?;
    let mut key = Zeroizing::new([0u8; SECRET_KEY_LEN]);
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Encode a 32-byte Recovery Key as a printable `RK1-XXXXX-…-XXXXX` string
/// (slice 5.7). Show-once, like `format_secret_key`.
#[must_use]
pub fn format_recovery_key(key: &[u8; RECOVERY_KEY_LEN]) -> String {
    format_key(&RECOVERY_KEY_CODEC, key)
}

/// Parse a display string back into a 32-byte Recovery Key (slice 5.7). Same
/// tolerance and rejections as [`parse_secret_key`], with the `RK1` prefix and
/// the recovery checksum context.
pub fn parse_recovery_key(s: &str) -> Result<Zeroizing<[u8; RECOVERY_KEY_LEN]>, VaultError> {
    let bytes = parse_key(&RECOVERY_KEY_CODEC, s)?;
    let mut key = Zeroizing::new([0u8; RECOVERY_KEY_LEN]);
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Encode `key` (length must equal `codec.key_len` — the public wrappers pass
/// a fixed-size array) as `PREFIX-XXXXX-…-XXXXX`.
fn format_key(codec: &KeyCodec, key: &[u8]) -> String {
    let mut payload = vec![0u8; codec.payload_len()];
    if let Some(slot) = payload.get_mut(..codec.key_len) {
        slot.copy_from_slice(key);
    }
    let checksum = compute_checksum(key, codec.checksum_context);
    if let Some(slot) = payload.get_mut(codec.key_len..) {
        slot.copy_from_slice(&checksum);
    }

    let data = encode_fixed(&payload, codec.data_chars);

    let mut out = String::with_capacity(codec.encoded_chars().saturating_add(codec.groups));
    out.push_str(codec.prefix);
    for chunk_idx in 0..codec.groups {
        out.push('-');
        let start = chunk_idx.saturating_mul(5);
        let end = start.saturating_add(5);
        if let Some(slice) = data.get(start..end) {
            out.push_str(slice);
        }
    }
    out
}

/// Parse a display string into the raw key bytes (length `codec.key_len`),
/// verifying prefix, length, alphabet, canonical padding, and checksum.
fn parse_key(codec: &KeyCodec, s: &str) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let err = codec.err;
    let normalized: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .map(canonicalize_char)
        .collect();

    if normalized.len() != codec.encoded_chars() {
        return Err(err(format!(
            "expected {} characters after normalization, got {}",
            codec.encoded_chars(),
            normalized.len(),
        )));
    }

    let (prefix, data) = normalized.split_at(codec.prefix.len());
    if prefix != codec.prefix {
        return Err(err(
            "unknown format prefix — upgrade Vedge or check for typos".to_owned(),
        ));
    }

    let decoded = decode_fixed(data, codec.payload_len(), err)?;

    // Strict padding check: re-encode the decoded bytes and compare against the
    // canonicalized input. This rejects flips of the trailing padding bits in
    // the last data char, which otherwise decode to the same payload and would
    // pass the checksum. `normalized` already had confusables resolved, so the
    // comparison is against the strict alphabet.
    if encode_fixed(decoded.as_slice(), codec.data_chars) != data {
        return Err(err(
            "non-canonical encoding — trailing padding bits must be zero".to_owned(),
        ));
    }

    let key_slice = decoded
        .get(..codec.key_len)
        .ok_or_else(|| err("truncated payload".to_owned()))?;
    let checksum_slice = decoded
        .get(codec.key_len..codec.payload_len())
        .ok_or_else(|| err("truncated payload".to_owned()))?;
    let expected = compute_checksum(key_slice, codec.checksum_context);

    if checksum_slice.ct_eq(&expected).unwrap_u8() != 1 {
        return Err(err("checksum mismatch — likely a typo".to_owned()));
    }

    Ok(Zeroizing::new(key_slice.to_vec()))
}

fn compute_checksum(key: &[u8], context: &[u8]) -> [u8; CHECKSUM_LEN] {
    // HMAC-SHA256 accepts keys of any length, so `new_from_slice` never
    // actually fails. The `Err` arm is mathematically unreachable; returning
    // zeros rather than panicking keeps the call site free of `unwrap`/`expect`
    // (encode and decode both use this function so any fixed fallback
    // round-trips within one codec).
    let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(key) else {
        return [0u8; CHECKSUM_LEN];
    };
    mac.update(context);
    let tag = mac.finalize().into_bytes();
    let mut out = [0u8; CHECKSUM_LEN];
    if let Some(slice) = tag.get(..CHECKSUM_LEN) {
        out.copy_from_slice(slice);
    }
    out
}

/// Encode `input` to exactly `data_chars` Crockford base32 chars, padding the
/// trailing partial group (and any char beyond the natural output length) with
/// the zero symbol `0`. `buffer` never holds more than ~13 meaningful bits (it
/// flushes every 5), so `u32` is sufficient for any payload length.
#[allow(clippy::cast_possible_truncation)]
fn encode_fixed(input: &[u8], data_chars: usize) -> String {
    let mut out = String::with_capacity(data_chars);
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
    while out.len() < data_chars {
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

/// Decode exactly `data.len()` canonical Crockford base32 chars into a
/// `payload_len`-byte payload. Caller is expected to apply [`canonicalize_char`]
/// first and to have length-checked `data`. The result is `Zeroizing` because a
/// recovery/secret key payload is sensitive.
#[allow(clippy::cast_possible_truncation)]
fn decode_fixed(
    data: &str,
    payload_len: usize,
    err: fn(String) -> VaultError,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let mut out = Zeroizing::new(Vec::with_capacity(payload_len));
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for ch in data.chars() {
        let Some(value) = ALPHABET.iter().position(|&b| (b as char) == ch) else {
            return Err(err("invalid character in key".to_owned()));
        };
        buffer = (buffer << 5) | value as u32;
        bits = bits.saturating_add(5);
        if bits >= 8 {
            bits = bits.saturating_sub(8);
            let byte = ((buffer >> bits) & 0xff) as u8;
            if out.len() < payload_len {
                out.push(byte);
            }
        }
    }

    if out.len() != payload_len {
        return Err(err(format!(
            "decoded {} bytes, expected {payload_len}",
            out.len(),
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

    /// 🔴 The `A3-` format is FROZEN — a shipped Emergency Kit is permanent. This
    /// pins the exact output for two known keys so the shared-codec refactor (and
    /// any future edit to the bit helpers) cannot silently change what `A3-`
    /// encodes and orphan already-printed kits.
    #[test]
    fn golden_a3_output_is_frozen() {
        assert_eq!(
            format_secret_key(&sample_key()),
            "A3-008J4-CT4AN-K7F24-SNAXW-SQFEZ-ZAVT0",
        );
        assert_eq!(
            format_secret_key(&[0u8; SECRET_KEY_LEN]),
            "A3-00000-00000-00000-00000-00000-35CY0",
        );
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
            VaultError::InvalidSecretKey(msg) => {
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
            VaultError::InvalidSecretKey(msg) => {
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
        let ctx = SECRET_KEY_CODEC.checksum_context;
        let a = compute_checksum(&key, ctx);
        let b = compute_checksum(&key, ctx);
        assert_eq!(a, b);
    }

    #[test]
    fn different_keys_produce_different_checksums() {
        let ctx = SECRET_KEY_CODEC.checksum_context;
        let a = compute_checksum(&sample_key(), ctx);
        let mut other = sample_key();
        other[0] = other[0].wrapping_add(1);
        let b = compute_checksum(&other, ctx);
        assert_ne!(a, b);
    }

    #[test]
    fn different_contexts_produce_different_checksums() {
        // The domain-separation property: the same raw bytes checksum
        // differently under the Secret-Key vs Recovery-Key context, so an
        // `A3-` string can never masquerade as an `RK1-` one at the checksum.
        let bytes = sample_key();
        let a = compute_checksum(&bytes, SECRET_KEY_CODEC.checksum_context);
        let b = compute_checksum(&bytes, RECOVERY_KEY_CODEC.checksum_context);
        assert_ne!(a, b);
    }

    #[test]
    fn error_message_does_not_echo_input() {
        let probe = "A3-SECRETSECRET-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX";
        let err = parse_secret_key(probe).expect_err("invalid input must fail");
        let msg = err.to_string();
        assert!(!msg.contains("SECRETSECRET"), "error echoed input: {msg}");
    }

    // ---- Recovery Key (RK1) codec — slice 5.7 --------------------------------

    fn recovery_sample_key() -> [u8; RECOVERY_KEY_LEN] {
        let mut k = [0u8; RECOVERY_KEY_LEN];
        let mut n: u8 = 0;
        for b in &mut k {
            *b = n.wrapping_mul(7).wrapping_add(3);
            n = n.wrapping_add(1);
        }
        k
    }

    #[test]
    fn recovery_round_trips() {
        for key in [
            recovery_sample_key(),
            [0u8; RECOVERY_KEY_LEN],
            [0xffu8; RECOVERY_KEY_LEN],
        ] {
            let display = format_recovery_key(&key);
            let parsed = parse_recovery_key(&display).expect("valid recovery display parses");
            assert_eq!(*parsed, key, "round-trip must recover the exact 32 bytes");
        }
    }

    #[test]
    fn recovery_display_shape_is_rk1_then_eleven_groups_of_five() {
        let display = format_recovery_key(&recovery_sample_key());
        let groups: Vec<&str> = display.split('-').collect();
        assert_eq!(groups.len(), 12, "got {display}");
        assert_eq!(groups[0], "RK1");
        for group in &groups[1..] {
            assert_eq!(group.len(), 5, "group {group} wrong length");
        }
    }

    #[test]
    fn recovery_canonical_display_uses_no_excluded_chars() {
        let display = format_recovery_key(&recovery_sample_key());
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
    fn recovery_tolerates_lowercase_confusables_and_spacing() {
        let display = format_recovery_key(&recovery_sample_key());
        let mangled: String = display
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-')
            .map(|c| match c {
                '0' => 'o',
                '1' => 'l',
                other => other,
            })
            .collect();
        let parsed = parse_recovery_key(&mangled).expect("confusables + no hyphens tolerated");
        assert_eq!(*parsed, recovery_sample_key());
    }

    #[test]
    fn secret_key_and_recovery_key_are_mutually_unparseable() {
        // A `RK1-` string must NOT parse as a Secret Key, and an `A3-` string
        // must NOT parse as a Recovery Key — distinct length AND checksum
        // context (spec test #6).
        let rk = format_recovery_key(&recovery_sample_key());
        let sk = format_secret_key(&sample_key());

        let e1 = parse_secret_key(&rk).expect_err("RK1 must not parse as a Secret Key");
        assert!(matches!(e1, VaultError::InvalidSecretKey(_)), "got {e1:?}");

        let e2 = parse_recovery_key(&sk).expect_err("A3 must not parse as a Recovery Key");
        assert!(
            matches!(e2, VaultError::InvalidRecoveryKey(_)),
            "got {e2:?}"
        );
    }

    #[test]
    fn recovery_single_character_flip_breaks_checksum() {
        let key = recovery_sample_key();
        let display = format_recovery_key(&key);
        let chars: Vec<char> = display.chars().collect();

        let mut tested: u32 = 0;
        let mut rejected: u32 = 0;
        for (pos, &orig) in chars.iter().enumerate() {
            // Skip the 3-char "RK1" prefix and the hyphens.
            if pos < 3 || orig == '-' {
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
            tested = tested.saturating_add(1);
            if parse_recovery_key(&mutated_str).is_err() {
                rejected = rejected.saturating_add(1);
            }
        }
        assert!(tested > 0);
        assert_eq!(tested, rejected, "every single-char flip must fail");
    }

    #[test]
    fn recovery_wrong_prefix_rejected() {
        let mut display = format_recovery_key(&recovery_sample_key());
        display.replace_range(0..3, "RK2");
        let err = parse_recovery_key(&display).expect_err("wrong prefix must fail");
        match err {
            VaultError::InvalidRecoveryKey(msg) => assert!(msg.contains("prefix"), "got: {msg}"),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn recovery_error_message_does_not_echo_input() {
        let probe = "RK1-SECRETSECRET-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX";
        let err = parse_recovery_key(probe).expect_err("invalid input must fail");
        assert!(
            !err.to_string().contains("SECRETSECRET"),
            "error echoed input",
        );
    }
}
