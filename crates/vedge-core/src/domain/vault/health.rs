//! Password-health domain types (slice 4.3).
//!
//! The vocabulary for the vault-wide secret scan: the **secret-field taxonomy**
//! (`SecretField` + its split `WeakPolicy`), the **findings** it produces, and
//! the **report** it returns. Every type here is a *derivative* — it carries a
//! zxcvbn score, an entry id, a field discriminant, a reuse-group ordinal, or an
//! age in days. **No type in this module carries secret material**:
//! `SecretField::EnvVar(String)` holds the variable's *key* (`"AWS_SECRET_KEY"`),
//! never its value; `LoginRecoveryCode(u32)` holds an *index*. Pinned by
//! `reuse_never_emits_digest` (core) and `health_dtos_carry_no_secrets` (ipc).

use secrecy::{ExposeSecret, SecretString};
use sha1::{Digest, Sha1};
use zeroize::Zeroizing;

use crate::domain::shared::{EntryId, Timestamp};
use crate::domain::vault::index::registrable_domain;
use crate::domain::vault::payloads::EntryPayload;

pub mod scoring;

/// Length of the HIBP k-anonymity prefix — the only slice of a password hash
/// that ever leaves the process (slice 4.4).
pub const HIBP_PREFIX_LEN: usize = 5;

/// Default staleness horizon: a secret unchanged for a year is "old".
pub const DEFAULT_MAX_AGE_DAYS: u32 = 365;
/// Default weak cut-off on the zxcvbn 0..=4 scale (score <= this is "weak").
pub const DEFAULT_WEAK_THRESHOLD: u8 = 2;

/// Whether a secret field is subject to zxcvbn weak-scoring, and if not, *why*
/// — so the exemption is reported, never silently skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakPolicy {
    /// User-chosen credential material — zxcvbn-scored.
    Scored,
    /// Structurally short by design (PIN / CVV) — a low score is expected, not
    /// a finding.
    ExemptByDesign,
    /// Machine-generated or freeform (key blobs, recovery codes, notes, PII) —
    /// guessability scoring is meaningless.
    ExemptNotCredential,
}

/// A secret-bearing field, addressed structurally (never by value). `Ord` gives
/// the health scan a deterministic reuse-group ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecretField {
    LoginPassword,
    /// Index into `recovery_codes` — not the code itself.
    LoginRecoveryCode(u32),
    CardNumber,
    CardCvv,
    CardPin,
    SshPrivateKey,
    SshPassphrase,
    ApiKeyKey,
    ApiKeySecret,
    /// The variable's *key* (e.g. `"AWS_SECRET_KEY"`) — never its value.
    EnvVar(String),
    NoteContent,
    IdentityNationalId,
}

impl SecretField {
    /// The weak-scoring policy for this field. **Exhaustive** — a new secret
    /// field cannot be added without classifying it. Pinned by
    /// `weak_policy_is_total`.
    #[must_use]
    pub const fn weak_policy(&self) -> WeakPolicy {
        match self {
            Self::LoginPassword
            | Self::SshPassphrase
            | Self::ApiKeyKey
            | Self::ApiKeySecret
            | Self::EnvVar(_) => WeakPolicy::Scored,
            Self::CardCvv | Self::CardPin => WeakPolicy::ExemptByDesign,
            Self::LoginRecoveryCode(_)
            | Self::CardNumber
            | Self::SshPrivateKey
            | Self::NoteContent
            | Self::IdentityNationalId => WeakPolicy::ExemptNotCredential,
        }
    }
}

/// Finding severity — how loudly the UI should flag it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// How confidently we know a secret's age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeConfidence {
    /// From `secret_changed_at` — the secret was written by 4.3-aware code.
    Exact,
    /// Derived from `entry_history` or `created_at` — a lower bound on age.
    Estimated,
}

/// The three health dimensions. `Weak.guesses_log10` is a float, so this (and
/// everything that contains it) is `PartialEq` but not `Eq`.
#[derive(Debug, Clone, PartialEq)]
pub enum FindingKind {
    /// zxcvbn 0..=4 score below the threshold.
    Weak { score: u8, guesses_log10: f64 },
    /// Shares its exact secret with `count - 1` other fields. `group` is a
    /// per-scan sequential ordinal — **never** the reuse digest.
    Reused { group: u32, count: u32 },
    /// The secret has not changed in `age_days`.
    Old {
        age_days: u32,
        confidence: AgeConfidence,
    },
    /// The secret appears in the `HaveIBeenPwned` corpus `count` times (slice 4.4).
    /// Only emitted for `WeakPolicy::Scored` fields, and only when the opt-in
    /// breach check ran. Severity is always `High`.
    Breached { count: u32 },
}

/// One health finding on one field (or one entry, for `Old`).
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub entry_id: EntryId,
    /// `None` for entry-level findings (`Old` — the stamp lives on `CommonMeta`).
    pub field: Option<SecretField>,
    pub kind: FindingKind,
    pub severity: Severity,
}

/// Why an entry was not scanned. Carries no detail beyond the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Unrecognized `entry_type` — `unknown_fields` is never scanned.
    UnknownPayload,
    /// The row failed to decrypt (corrupt or mis-keyed) — skipped, not fatal.
    DecryptError,
}

/// An entry the scan could not process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub entry_id: EntryId,
    pub reason: SkipReason,
}

/// Roll-up counts for the summary header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HealthSummary {
    pub weak: u32,
    pub reused: u32,
    pub old: u32,
    /// Fields exempt from weak-scoring by policy (reported, not hidden).
    pub exempt_not_scored: u32,
    /// Fields found in the `HaveIBeenPwned` corpus (slice 4.4). Zero when the
    /// breach check is disabled or unavailable.
    pub breached: u32,
}

/// The full scan result — derivatives only, no secrets.
#[derive(Debug, Clone, PartialEq)]
pub struct HealthReport {
    pub scanned_at: Timestamp,
    pub entries_scanned: u32,
    pub secrets_scanned: u32,
    pub findings: Vec<Finding>,
    pub skipped: Vec<Skipped>,
    pub summary: HealthSummary,
    /// True when a breach check was run this scan (the opt-in was on). Lets the UI
    /// distinguish "checked, none breached" from "never checked" — without it a
    /// `breached: 0` reads as a false all-clear for the off-by-default case. Slice 4.4.
    pub breach_checked: bool,
    /// True when the breach check was attempted but the network phase failed
    /// (offline, proxy, rate limit, parse). Zero `Breached` findings are emitted;
    /// the rest of the scan is still valid. Run-level, not per-entry (`SkipReason`
    /// is entry-level, so it is deliberately not reused). Slice 4.4.
    pub breach_check_failed: bool,
}

/// Scan tuning. Defaults come from the `DEFAULT_*` constants above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthScanInput {
    pub max_age_days: u32,
    pub weak_threshold: u8,
}

impl Default for HealthScanInput {
    fn default() -> Self {
        Self {
            max_age_days: DEFAULT_MAX_AGE_DAYS,
            weak_threshold: DEFAULT_WEAK_THRESHOLD,
        }
    }
}

/// Severity for a weak finding: the weaker the score, the louder.
#[must_use]
pub const fn weak_severity(score: u8) -> Severity {
    match score {
        0 | 1 => Severity::High,
        _ => Severity::Medium,
    }
}

/// Uppercase-hex SHA-1 of `input`.
///
/// For a live secret this is a full password hash, so it is returned `Zeroizing`.
/// The HIBP k-anonymity split is the first [`HIBP_PREFIX_LEN`] chars (the prefix,
/// sent to the API) and the remaining 35 (the suffix, matched locally). Slice 4.4.
#[must_use]
pub fn sha1_upper_hex(input: &str) -> Zeroizing<String> {
    use std::fmt::Write as _;
    let digest = Sha1::digest(input.as_bytes());
    let mut hex = String::with_capacity(40);
    for b in &digest {
        // Writing to a String is infallible; `.ok()` consumes the `#[must_use]`.
        write!(hex, "{b:02X}").ok();
    }
    Zeroizing::new(hex)
}

/// Every secret-bearing field of a payload, paired with its live secret.
///
/// The secret is a borrow tied to `payload`. Taxonomy-driven: `Login.totp_secret`
/// is **excluded entirely** (a high-entropy machine seed — it cannot be weak and
/// will never collide; TOTP has its own slice). `Document` / `Folder` / `Unknown`
/// carry no inline secret and yield nothing.
#[must_use]
pub fn secret_fields(payload: &EntryPayload) -> Vec<(SecretField, &SecretString)> {
    let mut out: Vec<(SecretField, &SecretString)> = Vec::new();
    match payload {
        EntryPayload::Login(p) => {
            out.push((SecretField::LoginPassword, &p.password));
            for (i, code) in p.recovery_codes.iter().enumerate() {
                let idx = u32::try_from(i).unwrap_or(u32::MAX);
                out.push((SecretField::LoginRecoveryCode(idx), code));
            }
            // totp_secret is deliberately excluded — see the doc above.
        }
        EntryPayload::Card(p) => {
            out.push((SecretField::CardNumber, &p.number));
            out.push((SecretField::CardCvv, &p.cvv));
            if let Some(pin) = &p.pin {
                out.push((SecretField::CardPin, pin));
            }
        }
        EntryPayload::SshKey(p) => {
            out.push((SecretField::SshPrivateKey, &p.private_key_pem));
            if let Some(pass) = &p.passphrase {
                out.push((SecretField::SshPassphrase, pass));
            }
        }
        EntryPayload::ApiKey(p) => {
            out.push((SecretField::ApiKeyKey, &p.key));
            if let Some(secret) = &p.secret {
                out.push((SecretField::ApiKeySecret, secret));
            }
        }
        EntryPayload::EnvVars(p) => {
            for var in &p.vars {
                out.push((SecretField::EnvVar(var.key.clone()), &var.value));
            }
        }
        EntryPayload::Note(p) => {
            out.push((SecretField::NoteContent, &p.content));
        }
        EntryPayload::Identity(p) => {
            if let Some(id) = &p.national_id {
                out.push((SecretField::IdentityNationalId, id));
            }
        }
        EntryPayload::Document(_) | EntryPayload::Folder(_) | EntryPayload::Unknown(_) => {}
    }
    out
}

/// True when the **multiset of secret values** differs between two payloads.
///
/// Used by `update_entry` to decide whether to bump `secret_changed_at`. Compares
/// only the taxonomy's `SecretString` *values* (order- and field-independent), so
/// a rename, a favorite toggle, reordering env vars, or renaming an env-var key
/// (the key is not secret) does **not** count as a secret change; changing,
/// adding, or removing a secret value does. `Login.totp_secret` is outside the
/// taxonomy, so a TOTP-only change does not bump password age. Constant-time
/// comparison is **not** required — both sides are already plaintext in-process.
#[must_use]
pub fn secrets_differ(old: &EntryPayload, new: &EntryPayload) -> bool {
    let of = secret_fields(old);
    let nf = secret_fields(new);
    let mut a: Vec<&str> = of.iter().map(|(_, s)| s.expose_secret()).collect();
    let mut b: Vec<&str> = nf.iter().map(|(_, s)| s.expose_secret()).collect();
    a.sort_unstable();
    b.sort_unstable();
    a != b
}

/// Non-secret tokens fed to zxcvbn so `"github2024"` scores weak *for an entry
/// named GitHub*: the entry name, a Login's username, and the URL host. All are
/// already-non-secret metadata.
#[must_use]
pub fn user_inputs(payload: &EntryPayload) -> Vec<String> {
    let meta = payload.meta();
    let mut inputs: Vec<String> = Vec::new();
    if !meta.name.is_empty() {
        inputs.push(meta.name.clone());
    }
    if let EntryPayload::Login(p) = payload {
        if !p.username.is_empty() {
            inputs.push(p.username.clone());
        }
    }
    if let Some(url) = &meta.url {
        // Reuse the autofill matcher's host parser so the health feed and the
        // browser matcher never disagree about a URL's host.
        if let Some(host) = registrable_domain(url) {
            inputs.push(host);
        }
    }
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_policy_is_total() {
        // Exhaustive over every SecretField variant — a new field cannot go
        // unclassified (the const fn match is wildcard-free; this pins the map).
        use SecretField as F;
        use WeakPolicy as W;
        let cases = [
            (F::LoginPassword, W::Scored),
            (F::LoginRecoveryCode(0), W::ExemptNotCredential),
            (F::CardNumber, W::ExemptNotCredential),
            (F::CardCvv, W::ExemptByDesign),
            (F::CardPin, W::ExemptByDesign),
            (F::SshPrivateKey, W::ExemptNotCredential),
            (F::SshPassphrase, W::Scored),
            (F::ApiKeyKey, W::Scored),
            (F::ApiKeySecret, W::Scored),
            (F::EnvVar("AWS_SECRET_KEY".to_owned()), W::Scored),
            (F::NoteContent, W::ExemptNotCredential),
            (F::IdentityNationalId, W::ExemptNotCredential),
        ];
        for (field, expected) in cases {
            assert_eq!(field.weak_policy(), expected, "policy for {field:?}");
        }
    }

    #[test]
    fn secrets_differ_ignores_env_var_reorder_and_key_rename() {
        use crate::domain::vault::payloads::{CommonMeta, EntryType, EnvVar, EnvVarsPayload};
        let mk = |vars: Vec<(&str, &str)>| {
            EntryPayload::EnvVars(EnvVarsPayload {
                meta: CommonMeta::new("env", EntryType::EnvVars),
                vars: vars
                    .into_iter()
                    .map(|(k, v)| EnvVar {
                        key: k.to_owned(),
                        value: SecretString::from(v),
                    })
                    .collect(),
            })
        };
        let a = mk(vec![("A", "1"), ("B", "2")]);
        let reordered = mk(vec![("B", "2"), ("A", "1")]);
        let key_renamed = mk(vec![("A", "1"), ("RENAMED", "2")]);
        let value_changed = mk(vec![("A", "1"), ("B", "9")]);
        assert!(
            !secrets_differ(&a, &reordered),
            "reorder is not a secret change"
        );
        assert!(
            !secrets_differ(&a, &key_renamed),
            "key rename is not a secret change"
        );
        assert!(
            secrets_differ(&a, &value_changed),
            "a value change IS a secret change"
        );
    }

    #[test]
    fn sha1_hex_prefix_suffix_split() {
        // The canonical FIPS-180 vector: SHA-1("abc") =
        // A9993E364706816ABA3E25717850C26C9CD0D89D. (A neutral input, not a
        // password-shaped literal — the string+hash otherwise trips secret scanners.)
        let hex = sha1_upper_hex("abc");
        assert_eq!(hex.len(), 40, "SHA-1 hex is 40 chars");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "hex only");
        assert_eq!(*hex, hex.to_uppercase(), "uppercase hex");
        assert_eq!(&hex[..HIBP_PREFIX_LEN], "A9993", "5-hex prefix");
        assert_eq!(
            &hex[HIBP_PREFIX_LEN..],
            "E364706816ABA3E25717850C26C9CD0D89D",
            "35-hex suffix"
        );
    }
}
