//! Hand-rolled RFC 6238 (TOTP) / RFC 4226 (HOTP truncation) code generation.
//!
//! Vedge is a *generator*, never a verifier — no ±window tolerance, no
//! constant-time compare (RFC 6238 §5.2/§6 are verifier-side). The whole
//! contract is: an accurate clock (**injected** — the engine never reads it),
//! `counter = floor(now / period)`, HMAC over the big-endian counter, RFC 4226
//! dynamic truncation, `mod 10^digits`.
//!
//! Built directly on the crypto core's `RustCrypto` primitives (`hmac` + `sha1`/
//! `sha2`) + `data-encoding` for Base32 — no `totp-rs` wrapper, so the decoded
//! seed's whole lifetime stays inside a `Zeroizing` buffer we own, and the code
//! path is shared verbatim by `copy_field` and `reveal_totp` (the 3.1 "one code
//! path" rule). The only correctness gate is the RFC test vectors below.

use std::ops::RangeInclusive;

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, KeyInit, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use zeroize::Zeroizing;

use crate::domain::vault::errors::VaultError;

/// HMAC hash for the OTP.
///
/// SHA-1 dominates (Google Authenticator / GitHub / AWS); SHA-256/512 appear in
/// enterprise (`HashiCorp` Vault, some banks). Serialized `PascalCase` to match
/// the `otpauth://` `algorithm=` values and the wire DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TotpAlgorithm {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

/// Per-entry TOTP parameters. Defaults are the near-universal SHA-1 / 6 / 30.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotpParams {
    pub algorithm: TotpAlgorithm,
    pub digits: u8,
    pub period: u32,
}

impl Default for TotpParams {
    fn default() -> Self {
        Self {
            algorithm: TotpAlgorithm::Sha1,
            digits: 6,
            period: 30,
        }
    }
}

/// Accepted digit counts (RFC allows 6–8; Vedge does not offer more).
pub const DIGITS_RANGE: RangeInclusive<u8> = 6..=8;
/// Accepted period in seconds. Lower bound keeps `now / period` well-defined and
/// avoids absurdly short windows; upper bound is generous for enterprise codes.
pub const PERIOD_RANGE: RangeInclusive<u32> = 5..=300;

/// A generated one-time code plus how long it stays valid.
///
/// Deliberately **not** `Debug`/`PartialEq` — the code must never reach a log or
/// an assertion diff (mirrors `vedge-generator`'s `GeneratedSecret`). The code
/// string is `Zeroizing`; `seconds_remaining` is derived from the *same* `now`
/// as the counter, so the countdown physically cannot disagree with the code.
pub struct TotpCode {
    pub code: Zeroizing<String>,
    pub seconds_remaining: u32,
    /// The period this code was generated for — lets the UI size the countdown
    /// ring correctly for non-default (e.g. 60 s) codes.
    pub period: u32,
}

/// Enrolment intent carried on `UpdateEntryInput` (Login only).
///
/// Lets the edit form preserve a stored seed it no longer holds (the door — the
/// seed does not cross to WASM). `Unchanged` is the safe default: a form that
/// never touches TOTP keeps it.
#[derive(Debug, Clone, Default)]
pub enum TotpUpdate {
    #[default]
    Unchanged,
    Set(SecretString),
    Clear,
}

/// Generate the current code for a Base32 `seed` under `p`, at unix time `now`
/// (seconds). The clock is injected; the engine never reads it.
///
/// Validates `digits`/`period` against the accepted ranges up front so a
/// corrupt stored payload degrades to an error, never a panic.
pub fn generate(seed: &SecretString, p: TotpParams, now: u64) -> Result<TotpCode, VaultError> {
    if !DIGITS_RANGE.contains(&p.digits) {
        return Err(VaultError::InvalidTotpParams(format!(
            "digits must be {}–{}, got {}",
            DIGITS_RANGE.start(),
            DIGITS_RANGE.end(),
            p.digits
        )));
    }
    if !PERIOD_RANGE.contains(&p.period) {
        return Err(VaultError::InvalidTotpParams(format!(
            "period must be {}–{} seconds, got {}",
            PERIOD_RANGE.start(),
            PERIOD_RANGE.end(),
            p.period
        )));
    }

    let key = decode_secret(seed)?;
    let period = u64::from(p.period);

    // `period >= 5`, so div/rem cannot fault; `checked_*` satisfies the workspace
    // `integer_division` + `arithmetic_side_effects` denies without an `#[allow]`.
    let counter = now
        .checked_div(period)
        .ok_or_else(|| VaultError::MalformedPayload("TOTP period is zero".into()))?;
    let mac = hmac_sha(p.algorithm, &key, &counter.to_be_bytes())?;
    drop(key);

    let code_num = dynamic_truncation(&mac, p.digits)?;
    let width = usize::from(p.digits);
    let code = Zeroizing::new(format!("{code_num:0width$}"));

    let elapsed = now
        .checked_rem(period)
        .ok_or_else(|| VaultError::MalformedPayload("TOTP period is zero".into()))?;
    // `elapsed < period <= 300`, so the narrowing is exact.
    let elapsed = u32::try_from(elapsed).unwrap_or(0);
    let seconds_remaining = p.period.saturating_sub(elapsed);

    Ok(TotpCode {
        code,
        seconds_remaining,
        period: p.period,
    })
}

/// Lenient Base32 normalize: uppercase, strip whitespace + `=` padding. Shared by
/// enrolment validation (`otpauth`) and generation (`decode_secret`) so the two
/// can't drift on the accepted form. Returns a `Zeroizing` buffer — the
/// normalized Base32 is secret-equivalent.
pub(crate) fn normalize_b32(raw: &str) -> Zeroizing<String> {
    Zeroizing::new(
        raw.chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '=')
            .flat_map(char::to_uppercase)
            .collect(),
    )
}

/// Normalize then **strict** RFC 4648 Base32 decode into a zeroizing key buffer.
fn decode_secret(seed: &SecretString) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let normalized = normalize_b32(seed.expose_secret());
    if normalized.is_empty() {
        return Err(VaultError::MalformedPayload("TOTP secret is empty".into()));
    }
    BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map(Zeroizing::new)
        .map_err(|e| VaultError::MalformedPayload(format!("invalid Base32 TOTP secret: {e}")))
}

/// HMAC(key, msg) dispatched over the three supported hashes. Explicit arms (not
/// a generic) to sidestep the `Hmac<D>` trait-bound thicket; each result lives
/// in a zeroizing buffer.
fn hmac_sha(
    alg: TotpAlgorithm,
    key: &[u8],
    msg: &[u8],
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let bytes = match alg {
        TotpAlgorithm::Sha1 => {
            let mut m = Hmac::<Sha1>::new_from_slice(key)
                .map_err(|_| VaultError::MalformedPayload("TOTP HMAC init failed".into()))?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        TotpAlgorithm::Sha256 => {
            let mut m = Hmac::<Sha256>::new_from_slice(key)
                .map_err(|_| VaultError::MalformedPayload("TOTP HMAC init failed".into()))?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        TotpAlgorithm::Sha512 => {
            let mut m = Hmac::<Sha512>::new_from_slice(key)
                .map_err(|_| VaultError::MalformedPayload("TOTP HMAC init failed".into()))?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
    };
    Ok(Zeroizing::new(bytes))
}

/// RFC 4226 §5.3 dynamic truncation. `offset` is the low nibble of the **last**
/// byte (Errata 5130 — not a hardcoded 19, which only coincides for SHA-1).
fn dynamic_truncation(mac: &[u8], digits: u8) -> Result<u32, VaultError> {
    let trunc_err = || VaultError::MalformedPayload("TOTP truncation out of range".into());
    let last = *mac.last().ok_or_else(trunc_err)?;
    let offset = usize::from(last & 0x0f);
    let end = offset.checked_add(4).ok_or_else(trunc_err)?;
    let window: [u8; 4] = mac
        .get(offset..end)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .ok_or_else(trunc_err)?;
    let bin = u32::from_be_bytes(window) & 0x7fff_ffff;
    let modulus = 10u32
        .checked_pow(u32::from(digits))
        .ok_or_else(|| VaultError::MalformedPayload("TOTP digit count too large".into()))?;
    bin.checked_rem(modulus)
        .ok_or_else(|| VaultError::MalformedPayload("TOTP modulus is zero".into()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use data_encoding::BASE32_NOPAD;
    use secrecy::SecretString;

    use super::{TotpAlgorithm, TotpParams, generate};

    // RFC 6238 Appendix B seeds, per Errata 2866: the SHA-256/512 vectors use a
    // 32- / 64-byte seed (the 20-byte ASCII pattern extended), NOT the 20-byte
    // seed the prose implies. Encoding all three explicitly is the whole test.
    const K_SHA1: &[u8] = b"12345678901234567890";
    const K_SHA256: &[u8] = b"12345678901234567890123456789012";
    const K_SHA512: &[u8] =
        b"1234567890123456789012345678901234567890123456789012345678901234";

    fn code_at(key: &[u8], alg: TotpAlgorithm, now: u64) -> String {
        let seed = SecretString::from(BASE32_NOPAD.encode(key));
        let p = TotpParams {
            algorithm: alg,
            digits: 8,
            period: 30,
        };
        generate(&seed, p, now).unwrap().code.to_string()
    }

    #[test]
    fn totp_rfc6238_vectors_sha1() {
        assert_eq!(code_at(K_SHA1, TotpAlgorithm::Sha1, 59), "94287082");
        assert_eq!(code_at(K_SHA1, TotpAlgorithm::Sha1, 1_111_111_109), "07081804");
        assert_eq!(code_at(K_SHA1, TotpAlgorithm::Sha1, 1_234_567_890), "89005924");
    }

    #[test]
    fn totp_rfc6238_vectors_sha256() {
        assert_eq!(code_at(K_SHA256, TotpAlgorithm::Sha256, 59), "46119246");
        assert_eq!(
            code_at(K_SHA256, TotpAlgorithm::Sha256, 1_111_111_109),
            "68084774"
        );
        assert_eq!(
            code_at(K_SHA256, TotpAlgorithm::Sha256, 1_234_567_890),
            "91819424"
        );
    }

    #[test]
    fn totp_rfc6238_vectors_sha512() {
        assert_eq!(code_at(K_SHA512, TotpAlgorithm::Sha512, 59), "90693936");
        assert_eq!(
            code_at(K_SHA512, TotpAlgorithm::Sha512, 1_111_111_109),
            "25091201"
        );
        assert_eq!(
            code_at(K_SHA512, TotpAlgorithm::Sha512, 1_234_567_890),
            "93441116"
        );
    }

    #[test]
    fn totp_rfc4226_vectors() {
        // RFC 4226 Appendix D, HOTP(K_SHA1, counter) 6 digits, counters 0..=9.
        // With period=30, `now = counter * 30` yields `floor(now/30) == counter`,
        // so TOTP reuses the HOTP truncation over exactly those counters.
        const EXPECTED: [&str; 10] = [
            "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583",
            "399871", "520489",
        ];
        for (counter, want) in EXPECTED.iter().enumerate() {
            let seed = SecretString::from(BASE32_NOPAD.encode(K_SHA1));
            let p = TotpParams {
                algorithm: TotpAlgorithm::Sha1,
                digits: 6,
                period: 30,
            };
            let now = u64::try_from(counter).unwrap_or(0).checked_mul(30).unwrap_or(0);
            let got = generate(&seed, p, now).unwrap().code.to_string();
            assert_eq!(&got, want, "counter {counter}");
        }
    }

    #[test]
    fn totp_seconds_remaining_consistent() {
        let seed = SecretString::from(BASE32_NOPAD.encode(K_SHA1));
        let p = TotpParams::default(); // period 30
        // At the exact boundary, a fresh window just started → full period ahead.
        assert_eq!(generate(&seed, p, 30).unwrap().seconds_remaining, 30);
        assert_eq!(generate(&seed, p, 0).unwrap().seconds_remaining, 30);
        assert_eq!(generate(&seed, p, 45).unwrap().seconds_remaining, 15);
        assert_eq!(generate(&seed, p, 59).unwrap().seconds_remaining, 1);
    }

    #[test]
    fn generate_rejects_out_of_range_params() {
        let seed = SecretString::from(BASE32_NOPAD.encode(K_SHA1));
        let bad_digits = TotpParams {
            algorithm: TotpAlgorithm::Sha1,
            digits: 5,
            period: 30,
        };
        assert!(generate(&seed, bad_digits, 0).is_err());
        let bad_period = TotpParams {
            algorithm: TotpAlgorithm::Sha1,
            digits: 6,
            period: 0,
        };
        assert!(generate(&seed, bad_period, 0).is_err());
    }
}
