//! Password-health DTO conversion (slice 4.3).
//!
//! A pure structural mapping: the domain `HealthReport` already carries only
//! derivatives (scores, group ordinals, ages), so there is no secret to guard
//! here — only enum-for-enum translation and the timestamp format.

pub use vedge_ipc::{
    AgeConfidenceDto, FindingDto, FindingKindDto, HealthReportDto, HealthScanInputDto,
    HealthSummaryDto, SecretFieldDto, SeverityDto, SkipReasonDto, SkippedDto,
};

use vedge_core::{
    AgeConfidence, Finding, FindingKind, HealthReport, HealthScanInput, HealthSummary, SecretField,
    Severity, SkipReason, Skipped,
};

#[must_use]
pub const fn health_scan_input_from_dto(d: HealthScanInputDto) -> HealthScanInput {
    HealthScanInput {
        max_age_days: d.max_age_days,
        weak_threshold: d.weak_threshold,
    }
}

fn secret_field_to_dto(f: &SecretField) -> SecretFieldDto {
    match f {
        SecretField::LoginPassword => SecretFieldDto::LoginPassword,
        SecretField::LoginRecoveryCode(i) => SecretFieldDto::LoginRecoveryCode(*i),
        SecretField::CardNumber => SecretFieldDto::CardNumber,
        SecretField::CardCvv => SecretFieldDto::CardCvv,
        SecretField::CardPin => SecretFieldDto::CardPin,
        SecretField::SshPrivateKey => SecretFieldDto::SshPrivateKey,
        SecretField::SshPassphrase => SecretFieldDto::SshPassphrase,
        SecretField::ApiKeyKey => SecretFieldDto::ApiKeyKey,
        SecretField::ApiKeySecret => SecretFieldDto::ApiKeySecret,
        SecretField::EnvVar(k) => SecretFieldDto::EnvVar(k.clone()),
        SecretField::NoteContent => SecretFieldDto::NoteContent,
        SecretField::IdentityNationalId => SecretFieldDto::IdentityNationalId,
    }
}

const fn severity_to_dto(s: Severity) -> SeverityDto {
    match s {
        Severity::High => SeverityDto::High,
        Severity::Medium => SeverityDto::Medium,
        Severity::Low => SeverityDto::Low,
    }
}

const fn age_confidence_to_dto(c: AgeConfidence) -> AgeConfidenceDto {
    match c {
        AgeConfidence::Exact => AgeConfidenceDto::Exact,
        AgeConfidence::Estimated => AgeConfidenceDto::Estimated,
    }
}

const fn finding_kind_to_dto(k: &FindingKind) -> FindingKindDto {
    match k {
        FindingKind::Weak {
            score,
            guesses_log10,
        } => FindingKindDto::Weak {
            score: *score,
            guesses_log10: *guesses_log10,
        },
        FindingKind::Reused { group, count } => FindingKindDto::Reused {
            group: *group,
            count: *count,
        },
        FindingKind::Old {
            age_days,
            confidence,
        } => FindingKindDto::Old {
            age_days: *age_days,
            confidence: age_confidence_to_dto(*confidence),
        },
        FindingKind::Breached { count } => FindingKindDto::Breached { count: *count },
    }
}

fn finding_to_dto(f: &Finding) -> FindingDto {
    FindingDto {
        entry_id: f.entry_id.as_str().to_owned(),
        field: f.field.as_ref().map(secret_field_to_dto),
        kind: finding_kind_to_dto(&f.kind),
        severity: severity_to_dto(f.severity),
    }
}

const fn skip_reason_to_dto(r: SkipReason) -> SkipReasonDto {
    match r {
        SkipReason::UnknownPayload => SkipReasonDto::UnknownPayload,
        SkipReason::DecryptError => SkipReasonDto::DecryptError,
    }
}

fn skipped_to_dto(s: &Skipped) -> SkippedDto {
    SkippedDto {
        entry_id: s.entry_id.as_str().to_owned(),
        reason: skip_reason_to_dto(s.reason),
    }
}

const fn summary_to_dto(s: &HealthSummary) -> HealthSummaryDto {
    HealthSummaryDto {
        weak: s.weak,
        reused: s.reused,
        old: s.old,
        exempt_not_scored: s.exempt_not_scored,
        breached: s.breached,
    }
}

#[must_use]
pub fn health_report_to_dto(r: &HealthReport) -> HealthReportDto {
    HealthReportDto {
        scanned_at: crate::dto::common::ts_to_string(r.scanned_at),
        entries_scanned: r.entries_scanned,
        secrets_scanned: r.secrets_scanned,
        findings: r.findings.iter().map(finding_to_dto).collect(),
        skipped: r.skipped.iter().map(skipped_to_dto).collect(),
        summary: summary_to_dto(&r.summary),
        breach_checked: r.breach_checked,
        breach_check_failed: r.breach_check_failed,
    }
}
