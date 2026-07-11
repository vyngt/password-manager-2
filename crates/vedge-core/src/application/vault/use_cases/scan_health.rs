//! `scan_health` — vault-wide secret health (slice 4.3).
//!
//! **The codebase's first bulk decrypt.** It reads every entry, decrypts one at
//! a time, and returns **only non-invertible derivatives** (a zxcvbn score, a
//! reuse-group ordinal, an age in days). No secret — and no reuse digest — ever
//! leaves core.
//!
//! ## Audit
//!
//! **Audit-silent per entry**, following the `list_audit` / `list_history`
//! precedent (server-side reads that decrypt to derive but expose no secret
//! value). It writes no `Viewed` rows and never touches `accessed_at` — doing
//! either would poison 4.1's audit page and the "recently used" sort. It DOES
//! emit exactly **one** vault-level `HealthScanned` row per run, mirroring
//! `Exported`: a full-vault decrypt is a security-relevant event, and the
//! last-scan time then comes free from the audit trail.
//!
//! ## Memory
//!
//! Peak plaintext is **one** payload — each is decrypted, derived-from, and
//! dropped (zeroizing) before the next. Never hold N. (This is why age uses only
//! the `secret_changed_at` stamp or `created_at`, never a history walk — the
//! latter would materialize N snapshot plaintexts; see the 4.3 tracking note.)
//!
//! ## Reuse — keyed, cross-entry, not a plain digest
//!
//! A raw SHA-256 of a low-entropy password is an offline-crackable oracle. A
//! per-scan random HMAC key makes the digest meaningless outside the scan; the
//! digest never leaves this function (findings carry a sequential `group: u32`).
//! A reuse group is a secret shared across **≥ 2 distinct entries** — two fields
//! of one entry sharing a value is not cross-entry reuse.

use std::collections::HashMap;

use hmac::{Hmac, KeyInit, Mac};
use rand::{Rng, rng};
use secrecy::ExposeSecret;
use sha2::Sha256;
use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, Timestamp, now};
use crate::domain::vault::entities::{AuditAction, EntryRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::health::{
    AgeConfidence, Finding, FindingKind, HealthReport, HealthScanInput, HealthSummary, SecretField,
    Severity, SkipReason, Skipped, WeakPolicy, scoring, secret_fields, user_inputs, weak_severity,
};
use crate::domain::vault::payloads::EntryPayload;

type HmacSha256 = Hmac<Sha256>;

#[instrument(skip_all)]
pub async fn scan_health(
    session: &VaultSession,
    input: HealthScanInput,
) -> Result<HealthReport, VaultError> {
    // `all_entries()` includes trashed rows and is unpaginated — we filter trash
    // in memory below. The full-ciphertext footprint is precedented by unlock.
    let rows = session.repo.all_entries().await?;

    // Per-scan random HMAC key (see the module doc). Zeroized on drop.
    let mut key = Zeroizing::new([0u8; 32]);
    rng().fill_bytes(&mut *key);

    let scanned_at = now();
    let mut findings: Vec<Finding> = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    let mut summary = HealthSummary::default();
    let mut entries_scanned: u32 = 0;
    let mut secrets_scanned: u32 = 0;
    let mut groups: HashMap<[u8; 32], Vec<(EntryId, SecretField)>> = HashMap::new();

    for row in &rows {
        if row.is_trashed {
            continue;
        }
        let Ok(payload) = super::refs::decrypt_row_payload(session, row) else {
            skipped.push(Skipped {
                entry_id: row.id.clone(),
                reason: SkipReason::DecryptError,
            });
            continue;
        };
        // Never scan `unknown_fields` — unclassified JSON that may hold secrets
        // from a future schema.
        if matches!(payload, EntryPayload::Unknown(_)) {
            skipped.push(Skipped {
                entry_id: row.id.clone(),
                reason: SkipReason::UnknownPayload,
            });
            continue;
        }
        entries_scanned = entries_scanned.saturating_add(1);

        let ui_owned = user_inputs(&payload);
        let ui: Vec<&str> = ui_owned.iter().map(String::as_str).collect();

        for (field, secret) in secret_fields(&payload) {
            let exposed = secret.expose_secret();
            // An empty/blank field is "not set" — scanning it produces false
            // findings (two blank passwords are not "reused"). Skip entirely.
            if exposed.is_empty() {
                continue;
            }
            secrets_scanned = secrets_scanned.saturating_add(1);

            // Reuse: keyed HMAC of the live secret — every non-empty secret participates.
            let digest = hmac_sha256(&key, exposed.as_bytes())?;
            groups
                .entry(digest)
                .or_default()
                .push((row.id.clone(), field.clone()));

            // Weak: only credential-like fields; exemptions are counted, not hidden.
            match field.weak_policy() {
                WeakPolicy::Scored => {
                    let assessed = scoring::score(secret, &ui);
                    if assessed.score <= input.weak_threshold {
                        summary.weak = summary.weak.saturating_add(1);
                        findings.push(Finding {
                            entry_id: row.id.clone(),
                            field: Some(field.clone()),
                            kind: FindingKind::Weak {
                                score: assessed.score,
                                guesses_log10: assessed.guesses_log10,
                            },
                            severity: weak_severity(assessed.score),
                        });
                    }
                }
                WeakPolicy::ExemptByDesign | WeakPolicy::ExemptNotCredential => {
                    summary.exempt_not_scored = summary.exempt_not_scored.saturating_add(1);
                }
            }
        }

        // Age is per-entry (the stamp lives on CommonMeta). Synchronous — no
        // history walk (see the Memory note).
        if let Some((age_days, confidence)) = assess_age(&payload, row, input, scanned_at) {
            summary.old = summary.old.saturating_add(1);
            findings.push(Finding {
                entry_id: row.id.clone(),
                field: None,
                kind: FindingKind::Old {
                    age_days,
                    confidence,
                },
                severity: Severity::Low,
            });
        }

        drop(payload);
    }
    drop(key);

    emit_reuse_findings(groups, &mut findings, &mut summary);

    // Exactly one vault-level row — the full-vault decrypt itself.
    super::create_entry::append_audit(session, AuditAction::HealthScanned, None).await?;

    Ok(HealthReport {
        scanned_at,
        entries_scanned,
        secrets_scanned,
        findings,
        skipped,
        summary,
    })
}

/// HMAC-SHA256 of `msg` under the per-scan key. `new_from_slice` never fails for
/// HMAC (it accepts any key length), but the API is fallible, so we map the
/// impossible error rather than `unwrap`.
fn hmac_sha256(key: &[u8; 32], msg: &[u8]) -> Result<[u8; 32], VaultError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| VaultError::MalformedPayload("hmac key length".into()))?;
    mac.update(msg);
    Ok(mac.finalize().into_bytes().into())
}

/// The distinct entries sharing a digest group.
fn distinct_entry_count(members: &[(EntryId, SecretField)]) -> usize {
    let mut ids: Vec<&str> = members.iter().map(|(id, _)| id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.len()
}

/// Reuse findings from the digest map: every secret shared across ≥ 2 **distinct**
/// entries. Sorted for a deterministic sequential `group` ordinal (member sets
/// are disjoint across groups, so ordering by the sorted member list is a total
/// order). The digest is dropped here and never surfaces.
fn emit_reuse_findings(
    groups: HashMap<[u8; 32], Vec<(EntryId, SecretField)>>,
    findings: &mut Vec<Finding>,
    summary: &mut HealthSummary,
) {
    let mut reused: Vec<Vec<(EntryId, SecretField)>> = groups
        .into_values()
        .filter(|members| distinct_entry_count(members) >= 2)
        .collect();
    // Sort members within each group, then groups by their full member list.
    // `EntryId` isn't `Ord` (ulid newtype), so key on `as_str()` + the (now `Ord`)
    // `SecretField`; group ordering compares the member iterators lazily.
    for members in &mut reused {
        members.sort_by(|(a, fa), (b, fb)| (a.as_str(), fa).cmp(&(b.as_str(), fb)));
    }
    reused.sort_by(|a, b| {
        a.iter()
            .map(|(id, f)| (id.as_str(), f))
            .cmp(b.iter().map(|(id, f)| (id.as_str(), f)))
    });

    let mut group_id: u32 = 0;
    for members in reused {
        group_id = group_id.saturating_add(1);
        let count = u32::try_from(distinct_entry_count(&members)).unwrap_or(u32::MAX);
        for (entry_id, field) in members {
            summary.reused = summary.reused.saturating_add(1);
            findings.push(Finding {
                entry_id,
                field: Some(field),
                kind: FindingKind::Reused {
                    group: group_id,
                    count,
                },
                severity: Severity::Medium,
            });
        }
    }
}

/// Age of an entry's secret material, as `(age_days, confidence)` — `Some` only
/// when older than the threshold.
///
/// Two tiers: `secret_changed_at` (Exact); else `created_at` (Estimated). **Never
/// `updated_at`** — metadata mutations bump it. The `created_at` fallback
/// *over*-reports age for a pre-4.3 entry whose secret was rotated after creation
/// but never touched under 4.3; that self-heals as the entry is edited (the stamp
/// then makes it Exact). A history-based estimate was dropped after review — see
/// the 4.3 tracking note (`Gotchas`).
fn assess_age(
    payload: &EntryPayload,
    row: &EntryRow,
    input: HealthScanInput,
    scanned_at: Timestamp,
) -> Option<(u32, AgeConfidence)> {
    let (changed_at, confidence) = payload
        .meta()
        .secret_changed_at
        .map_or((row.created_at, AgeConfidence::Estimated), |stamp| {
            (stamp, AgeConfidence::Exact)
        });
    // Method form (not `-`) keeps the denied `arithmetic_side_effects` lint happy.
    let age_days_signed = scanned_at.signed_duration_since(changed_at).num_days();
    let age_days = u32::try_from(age_days_signed).unwrap_or(0);
    (age_days > input.max_age_days).then_some((age_days, confidence))
}
