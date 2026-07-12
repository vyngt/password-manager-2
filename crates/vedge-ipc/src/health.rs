//! Password-health wire types (slice 4.3).
//!
//! **Derivatives only.** No secret and no reuse digest crosses the boundary:
//! `SecretFieldDto::EnvVar(String)` holds the variable's *key*,
//! `LoginRecoveryCode(u32)` an *index*, and reuse is a sequential `group`
//! ordinal. Pinned by `health_dtos_carry_no_secrets`.

use serde::{Deserialize, Serialize};

/// A secret-bearing field on the wire. Adjacently tagged, mirroring
/// `FieldSelectorDto`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SecretFieldDto {
    LoginPassword,
    LoginRecoveryCode(u32),
    CardNumber,
    CardCvv,
    CardPin,
    SshPrivateKey,
    SshPassphrase,
    ApiKeyKey,
    ApiKeySecret,
    EnvVar(String),
    NoteContent,
    IdentityNationalId,
}

/// Finding severity — `"High"` / `"Medium"` / `"Low"` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeverityDto {
    High,
    Medium,
    Low,
}

/// How confidently the secret's age is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeConfidenceDto {
    Exact,
    Estimated,
}

/// The three health dimensions. `Weak.guesses_log10` is a float → not `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum FindingKindDto {
    Weak {
        score: u8,
        guesses_log10: f64,
    },
    Reused {
        group: u32,
        count: u32,
    },
    Old {
        age_days: u32,
        confidence: AgeConfidenceDto,
    },
    /// Found in the `HaveIBeenPwned` corpus `count` times (slice 4.4).
    Breached {
        count: u32,
    },
}

/// One finding on one field (or one entry, for `Old`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingDto {
    pub entry_id: String,
    #[serde(default)]
    pub field: Option<SecretFieldDto>,
    pub kind: FindingKindDto,
    pub severity: SeverityDto,
}

/// Why an entry was not scanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReasonDto {
    UnknownPayload,
    DecryptError,
}

/// An entry the scan could not process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedDto {
    pub entry_id: String,
    pub reason: SkipReasonDto,
}

/// Roll-up counts for the summary header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthSummaryDto {
    pub weak: u32,
    pub reused: u32,
    pub old: u32,
    pub exempt_not_scored: u32,
    /// Fields found in the `HaveIBeenPwned` corpus (slice 4.4).
    #[serde(default)]
    pub breached: u32,
}

/// The full scan result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthReportDto {
    /// RFC 3339 timestamp of the scan.
    pub scanned_at: String,
    pub entries_scanned: u32,
    pub secrets_scanned: u32,
    pub findings: Vec<FindingDto>,
    pub skipped: Vec<SkippedDto>,
    pub summary: HealthSummaryDto,
    /// True when a breach check ran this scan. Lets the UI show the breached stat
    /// only when it means something (avoids a false all-clear for the off-by-default
    /// case). Slice 4.4.
    #[serde(default)]
    pub breach_checked: bool,
    /// True when the breach check ran but the network phase failed (offline /
    /// proxy / rate limit). Zero `Breached` findings; the rest of the report is
    /// valid. The UI shows "breach check unavailable". Slice 4.4.
    #[serde(default)]
    pub breach_check_failed: bool,
}

const fn default_max_age_days() -> u32 {
    365
}
const fn default_weak_threshold() -> u8 {
    2
}

/// Inbound scan tuning. Both fields default (a form may send `{}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthScanInputDto {
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,
    #[serde(default = "default_weak_threshold")]
    pub weak_threshold: u8,
}

impl Default for HealthScanInputDto {
    fn default() -> Self {
        Self {
            max_age_days: default_max_age_days(),
            weak_threshold: default_weak_threshold(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> HealthReportDto {
        HealthReportDto {
            scanned_at: "2026-07-12T09:14:03.000Z".into(),
            entries_scanned: 3,
            secrets_scanned: 5,
            findings: vec![
                FindingDto {
                    entry_id: "01HENTRY0000000000000000A".into(),
                    field: Some(SecretFieldDto::LoginPassword),
                    kind: FindingKindDto::Weak {
                        score: 1,
                        guesses_log10: 4.2,
                    },
                    severity: SeverityDto::High,
                },
                FindingDto {
                    entry_id: "01HENTRY0000000000000000B".into(),
                    field: Some(SecretFieldDto::EnvVar("AWS_SECRET_KEY".into())),
                    kind: FindingKindDto::Reused { group: 1, count: 2 },
                    severity: SeverityDto::Medium,
                },
                FindingDto {
                    entry_id: "01HENTRY0000000000000000C".into(),
                    field: None,
                    kind: FindingKindDto::Old {
                        age_days: 812,
                        confidence: AgeConfidenceDto::Estimated,
                    },
                    severity: SeverityDto::Low,
                },
                FindingDto {
                    entry_id: "01HENTRY0000000000000000E".into(),
                    field: Some(SecretFieldDto::LoginPassword),
                    kind: FindingKindDto::Breached { count: 1337 },
                    severity: SeverityDto::High,
                },
            ],
            skipped: vec![SkippedDto {
                entry_id: "01HENTRY0000000000000000D".into(),
                reason: SkipReasonDto::UnknownPayload,
            }],
            summary: HealthSummaryDto {
                weak: 1,
                reused: 1,
                old: 1,
                exempt_not_scored: 2,
                breached: 1,
            },
            breach_checked: true,
            breach_check_failed: false,
        }
    }

    #[test]
    fn report_round_trips() {
        let dto = sample_report();
        let json = serde_json::to_string(&dto).unwrap();
        let back: HealthReportDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dto);
    }

    #[test]
    fn finding_kind_adjacent_tagging() {
        let k = FindingKindDto::Reused { group: 3, count: 2 };
        assert_eq!(
            serde_json::to_string(&k).unwrap(),
            r#"{"kind":"Reused","value":{"group":3,"count":2}}"#
        );
        let b = FindingKindDto::Breached { count: 42 };
        assert_eq!(
            serde_json::to_string(&b).unwrap(),
            r#"{"kind":"Breached","value":{"count":42}}"#
        );
    }

    /// The DTO **cannot** carry a secret by construction: `EnvVar` holds a key,
    /// `LoginRecoveryCode` an index, reuse a group ordinal. Serializing a report
    /// built from known-secret-adjacent data must expose no digest or value.
    #[test]
    fn health_dtos_carry_no_secrets() {
        let json = serde_json::to_string(&sample_report()).unwrap();
        // The env-var KEY is allowed; a value never appears.
        assert!(
            json.contains("AWS_SECRET_KEY"),
            "the key is a non-secret label"
        );
        assert!(!json.contains("group\":[") && !json.contains("digest"));
        // Reuse carries an ordinal, not bytes.
        assert!(json.contains(r#""group":1"#));
        // Breach carries only a corpus count — never a SHA-1 prefix, suffix, or
        // full hash. Assert no run of 20+ hex digits: catches the 35-hex suffix and
        // the full 40-hex hash while staying above the longest hex run in real
        // fields (the fixture's 17-char `0000000000000000A` ULID tail).
        let has_hash_run = json
            .as_bytes()
            .windows(20)
            .any(|w| w.iter().all(u8::is_ascii_hexdigit));
        assert!(
            !has_hash_run,
            "no long hex run (a SHA-1 fragment) may cross the boundary"
        );
    }

    #[test]
    fn scan_input_defaults_when_empty() {
        let dto: HealthScanInputDto = serde_json::from_str("{}").unwrap();
        assert_eq!(dto.max_age_days, 365);
        assert_eq!(dto.weak_threshold, 2);
    }
}
