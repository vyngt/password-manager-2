//! Hand-rolled `otpauth://` URI + bare-Base32 enrolment parser (zero deps).
//!
//! Normative reference: `05 Research/02 - TOTP & OTP Standards`. Rules:
//! - `totp` only; `hotp` → [`VaultError::HotpNotSupported`] (not "malformed").
//! - `otpauth-migration://` → [`VaultError::TotpMigrationNotSupported`].
//! - `secret` is required; `algorithm`/`digits`/`period` are **honored** and
//!   validated against [`DIGITS_RANGE`]/[`PERIOD_RANGE`] — never silently coerced.
//! - Issuer conflict: the `issuer` **parameter** wins over the label prefix.
//! - Base32 is lenient-normalized (uppercase, strip whitespace + `=`) then
//!   strict-decoded to validate; the normalized string is what we store.
//! - No `unwrap`/`expect`/`panic` (workspace lints deny them). Percent-decoding
//!   is hand-rolled (`%XX` → byte, UTF-8 validated) — no `urlencoding` dep.

use data_encoding::BASE32_NOPAD;
use secrecy::SecretString;

use crate::domain::vault::errors::VaultError;
use crate::domain::vault::totp::{DIGITS_RANGE, PERIOD_RANGE, TotpAlgorithm, TotpParams};

/// A parsed enrolment: the normalized Base32 secret + honored params + optional
/// display context. `issuer`/`account` are prefill-only.
#[derive(Debug, Clone)]
pub struct TotpEnrolment {
    pub secret: SecretString,
    pub params: TotpParams,
    pub issuer: Option<String>,
    pub account: Option<String>,
}

const SCHEME: &str = "otpauth://";
const MIGRATION_SCHEME: &str = "otpauth-migration://";

/// Parse an `otpauth://totp/...` URI **or** a bare Base32 secret.
///
/// The raw string already originated in WASM (the user pasted it), so echoing the
/// normalized secret back is not a core→WASM disclosure.
pub fn parse_totp_input(raw: &str) -> Result<TotpEnrolment, VaultError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(VaultError::MalformedPayload("empty TOTP input".into()));
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with(MIGRATION_SCHEME) {
        return Err(VaultError::TotpMigrationNotSupported);
    }
    if lower.starts_with(SCHEME) {
        // The prefix is ASCII, so byte offset == char boundary.
        let rest = trimmed.get(SCHEME.len()..).unwrap_or("");
        return parse_uri(rest);
    }

    // No scheme → treat the whole thing as a bare Base32 secret.
    Ok(TotpEnrolment {
        secret: normalize_and_validate_secret(trimmed)?,
        params: TotpParams::default(),
        issuer: None,
        account: None,
    })
}

/// Parse the post-scheme remainder `TYPE/LABEL?QUERY`.
fn parse_uri(rest: &str) -> Result<TotpEnrolment, VaultError> {
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (type_str, label) = path.split_once('/').unwrap_or((path, ""));

    match type_str.to_ascii_lowercase().as_str() {
        "totp" => {}
        "hotp" => return Err(VaultError::HotpNotSupported),
        other => {
            return Err(VaultError::MalformedPayload(format!(
                "unsupported otpauth type: {other}"
            )));
        }
    }

    let mut secret_raw: Option<String> = None;
    let mut issuer_param: Option<String> = None;
    let mut algorithm = TotpAlgorithm::Sha1;
    let mut digits: u8 = 6;
    let mut period: u32 = 30;

    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k)?.to_ascii_lowercase();
        match key.as_str() {
            "secret" => secret_raw = Some(percent_decode(v)?),
            "issuer" => issuer_param = Some(percent_decode(v)?),
            "algorithm" => algorithm = parse_algorithm(&percent_decode(v)?)?,
            "digits" => digits = parse_digits(&percent_decode(v)?)?,
            "period" => period = parse_period(&percent_decode(v)?)?,
            _ => {} // ignore unknown params (issuer image, etc.)
        }
    }

    let secret_raw = secret_raw
        .ok_or_else(|| VaultError::MalformedPayload("otpauth URI missing secret".into()))?;
    let secret = normalize_and_validate_secret(&secret_raw)?;

    let (label_issuer, account) = split_label(&percent_decode(label)?);
    // The `issuer` parameter wins over the label prefix — but an *empty* param
    // must fall through to the label, not null it out.
    let issuer = issuer_param
        .filter(|s| !s.is_empty())
        .or(label_issuer)
        .filter(|s| !s.is_empty());

    Ok(TotpEnrolment {
        secret,
        params: TotpParams {
            algorithm,
            digits,
            period,
        },
        issuer,
        account: account.filter(|s| !s.is_empty()),
    })
}

fn parse_algorithm(s: &str) -> Result<TotpAlgorithm, VaultError> {
    match s.trim().to_ascii_uppercase().as_str() {
        "SHA1" => Ok(TotpAlgorithm::Sha1),
        "SHA256" => Ok(TotpAlgorithm::Sha256),
        "SHA512" => Ok(TotpAlgorithm::Sha512),
        other => Err(VaultError::InvalidTotpParams(format!(
            "unsupported algorithm: {other}"
        ))),
    }
}

fn parse_digits(s: &str) -> Result<u8, VaultError> {
    let d: u8 = s
        .trim()
        .parse()
        .map_err(|_| VaultError::InvalidTotpParams(format!("invalid digits: {s}")))?;
    if DIGITS_RANGE.contains(&d) {
        Ok(d)
    } else {
        Err(VaultError::InvalidTotpParams(format!(
            "digits must be {}–{}, got {d}",
            DIGITS_RANGE.start(),
            DIGITS_RANGE.end()
        )))
    }
}

fn parse_period(s: &str) -> Result<u32, VaultError> {
    let p: u32 = s
        .trim()
        .parse()
        .map_err(|_| VaultError::InvalidTotpParams(format!("invalid period: {s}")))?;
    if PERIOD_RANGE.contains(&p) {
        Ok(p)
    } else {
        Err(VaultError::InvalidTotpParams(format!(
            "period must be {}–{} seconds, got {p}",
            PERIOD_RANGE.start(),
            PERIOD_RANGE.end()
        )))
    }
}

/// `Issuer:Account` → `(Some(issuer), Some(account))`; a lone `Account` →
/// `(None, Some(account))`. Leading spaces on the account are conventional.
fn split_label(label: &str) -> (Option<String>, Option<String>) {
    if let Some((iss, acc)) = label.split_once(':') {
        (
            Some(iss.trim().to_owned()),
            Some(acc.trim_start().to_owned()),
        )
    } else {
        let a = label.trim();
        (None, (!a.is_empty()).then(|| a.to_owned()))
    }
}

fn normalize_and_validate_secret(raw: &str) -> Result<SecretString, VaultError> {
    // Same normalize the engine's `decode_secret` uses — shared so they can't drift.
    let normalized = crate::domain::vault::totp::normalize_b32(raw);
    if normalized.is_empty() {
        return Err(VaultError::MalformedPayload("TOTP secret is empty".into()));
    }
    // Strict-decode to validate; store the normalized Base32 (what the engine
    // re-decodes). A bad character surfaces here at enrol, not at first display.
    BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|e| VaultError::MalformedPayload(format!("invalid Base32 TOTP secret ({e})")))?;
    Ok(SecretString::from(normalized.to_string()))
}

/// Decode `%XX` escapes (UTF-8 validated). Leaves `+` literal — `otpauth`
/// labels use percent-encoding for spaces, not form-encoding.
fn percent_decode(s: &str) -> Result<String, VaultError> {
    let bad = || VaultError::MalformedPayload("malformed percent-escape in otpauth URI".into());
    let mut out: Vec<u8> = Vec::with_capacity(s.len());
    let mut it = s.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let hi = it.next().ok_or_else(bad)?;
            let lo = it.next().ok_or_else(bad)?;
            let hex = [hi, lo];
            let hex_str = core::str::from_utf8(&hex).map_err(|_| bad())?;
            let byte = u8::from_str_radix(hex_str, 16).map_err(|_| bad())?;
            out.push(byte);
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out)
        .map_err(|_| VaultError::MalformedPayload("invalid UTF-8 in otpauth URI".into()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use secrecy::ExposeSecret;

    use super::parse_totp_input;
    use crate::domain::vault::errors::VaultError;
    use crate::domain::vault::totp::TotpAlgorithm;

    const SECRET: &str = "JBSWY3DPEHPK3PXP";

    #[test]
    fn otpauth_happy_param_priority() {
        let e = parse_totp_input(&format!(
            "otpauth://totp/Example:alice@x?secret={SECRET}&algorithm=SHA256&digits=8&period=60"
        ))
        .unwrap();
        assert_eq!(e.params.algorithm, TotpAlgorithm::Sha256);
        assert_eq!(e.params.digits, 8);
        assert_eq!(e.params.period, 60);
        assert_eq!(e.secret.expose_secret(), SECRET);
        assert_eq!(e.account.as_deref(), Some("alice@x"));
    }

    #[test]
    fn otpauth_issuer_param_beats_label() {
        let e = parse_totp_input(&format!(
            "otpauth://totp/LabelIssuer:alice@x?secret={SECRET}&issuer=RealIssuer"
        ))
        .unwrap();
        assert_eq!(e.issuer.as_deref(), Some("RealIssuer"));
        assert_eq!(e.account.as_deref(), Some("alice@x"));
    }

    #[test]
    fn otpauth_label_issuer_when_no_param() {
        let e = parse_totp_input(&format!("otpauth://totp/GitHub:bob?secret={SECRET}")).unwrap();
        assert_eq!(e.issuer.as_deref(), Some("GitHub"));
        assert_eq!(e.account.as_deref(), Some("bob"));
        // Defaults honored.
        assert_eq!(e.params.algorithm, TotpAlgorithm::Sha1);
        assert_eq!(e.params.digits, 6);
        assert_eq!(e.params.period, 30);
    }

    #[test]
    fn otpauth_empty_issuer_param_falls_back_to_label() {
        // An explicit but empty `issuer=` must not null out the label issuer.
        let e = parse_totp_input(&format!(
            "otpauth://totp/GitHub:bob?secret={SECRET}&issuer="
        ))
        .unwrap();
        assert_eq!(e.issuer.as_deref(), Some("GitHub"));
    }

    #[test]
    fn otpauth_percent_decoded_label() {
        let e = parse_totp_input(&format!(
            "otpauth://totp/ACME%20Co%3A%20alice%40acme.com?secret={SECRET}&issuer=ACME%20Co"
        ))
        .unwrap();
        assert_eq!(e.issuer.as_deref(), Some("ACME Co"));
        assert_eq!(e.account.as_deref(), Some("alice@acme.com"));
    }

    #[test]
    fn otpauth_rejects_hotp() {
        let err =
            parse_totp_input(&format!("otpauth://hotp/x?secret={SECRET}&counter=0")).unwrap_err();
        assert!(matches!(err, VaultError::HotpNotSupported));
    }

    #[test]
    fn otpauth_rejects_bad_digits() {
        let err =
            parse_totp_input(&format!("otpauth://totp/x?secret={SECRET}&digits=5")).unwrap_err();
        assert!(matches!(err, VaultError::InvalidTotpParams(m) if m.contains("digits")));
    }

    #[test]
    fn otpauth_rejects_bad_period() {
        let err =
            parse_totp_input(&format!("otpauth://totp/x?secret={SECRET}&period=1")).unwrap_err();
        assert!(matches!(err, VaultError::InvalidTotpParams(m) if m.contains("period")));
    }

    #[test]
    fn otpauth_rejects_migration_uri() {
        let err = parse_totp_input("otpauth-migration://offline?data=abc").unwrap_err();
        assert!(matches!(err, VaultError::TotpMigrationNotSupported));
    }

    #[test]
    fn otpauth_requires_secret() {
        let err = parse_totp_input("otpauth://totp/x?issuer=Y").unwrap_err();
        assert!(matches!(err, VaultError::MalformedPayload(_)));
    }

    #[test]
    fn base32_lenient_normalize() {
        // Lowercase + spaced + unpadded, bare (no scheme) → normalized uppercase.
        let e = parse_totp_input("jbsw y3dp ehpk 3pxp").unwrap();
        assert_eq!(e.secret.expose_secret(), SECRET);
        assert_eq!(e.params.digits, 6);
    }

    #[test]
    fn base32_rejects_bad_chars() {
        // 0/1/8/9 and punctuation are not in the RFC 4648 Base32 alphabet.
        let err = parse_totp_input("JBSW0189!!!").unwrap_err();
        assert!(matches!(err, VaultError::MalformedPayload(_)));
    }
}
