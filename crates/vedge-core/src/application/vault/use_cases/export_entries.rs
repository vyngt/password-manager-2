//! `export_entries` — write the vault's active entries to a portable file
//! (slice 5.3a). Two formats, one door out:
//!
//! - **Encrypted** — every entry type, mapped to the permanent [`ExportEntry`]
//!   DTO, tarred with each document's decrypted bytes, and sealed in the export
//!   [`envelope`]. Opaque on disk (Decision ②: the plaintext tar never touches
//!   disk — it is assembled in RAM and only the sealed ciphertext is written).
//! - **CSV** — a **plaintext**, logins-only liberation file (Decision ⑤). The
//!   user chose plaintext for interop; the shell shows the honest warning
//!   (Decision ⑧) that it cannot be reliably wiped.
//!
//! One vault-level `Exported` audit row either way — readable-or-sealed, the
//! user's data left the vault as a portable copy. (`.vbk` backups use
//! `BackupCreated`; this is not that.)
//!
//! 🔴 This is a **one-way door for data, not a recovery path** — restore never
//! routes through here (Decision ①).

use std::path::PathBuf;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::session::VaultSession;
use crate::domain::export::{ExportBundle, payload_to_export};
use crate::domain::shared::{EntryId, StorageError};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::{CommonMeta, EntryPayload};
use crate::infrastructure::export::{archive, csv, envelope};

/// Which portable format to write.
pub enum ExportFormat {
    /// Encrypted JSON envelope — all entry types, opaque on disk.
    Encrypted { passphrase: Zeroizing<String> },
    /// Plaintext logins-only CSV. `spreadsheet_safe` apostrophe-prefixes
    /// formula-risky cells (never strips) — see [`csv`].
    Csv { spreadsheet_safe: bool },
}

pub struct ExportEntriesInput {
    pub dest: PathBuf,
    pub format: ExportFormat,
}

/// Non-secret result for the shell.
#[derive(Debug, Clone)]
pub struct ExportReport {
    pub dest: PathBuf,
    /// Entries written — for CSV this is logins only.
    pub entry_count: u64,
    /// True for the sealed envelope, false for the plaintext CSV.
    pub encrypted: bool,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Resolve a payload's tag IDs to tag names via the session index (a missing tag
/// is skipped — the index is authoritative for what exists now).
fn resolve_tags(session: &VaultSession, meta: &CommonMeta) -> Vec<String> {
    meta.tag_ids
        .iter()
        .filter_map(|tid| session.index.tags.get(tid).map(|t| t.name.clone()))
        .collect()
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn export_entries(
    session: &VaultSession,
    input: ExportEntriesInput,
) -> Result<ExportReport, VaultError> {
    // Snapshot the active (non-trashed) entry IDs from the index up front.
    let ids: Vec<EntryId> = session
        .index
        .all_active()
        .iter()
        .map(|e| e.id.clone())
        .collect();

    let report = match input.format {
        ExportFormat::Encrypted { passphrase } => {
            export_encrypted(session, &input.dest, &ids, &passphrase).await?
        }
        ExportFormat::Csv { spreadsheet_safe } => {
            export_csv(session, &input.dest, &ids, spreadsheet_safe).await?
        }
    };

    // One vault-level audit row (entry_id = None): the user exported. Best-effort
    // AFTER the file is written — a failed audit must not report a successful export
    // as failed (that would prompt a retry → a second plaintext export on disk).
    // Mirrors `backup_vault`'s post-commit discipline.
    if let Err(e) = super::create_entry::append_audit(session, AuditAction::Exported, None).await {
        tracing::warn!(error = %e, "export written but the audit row could not be recorded");
    }
    Ok(report)
}

async fn export_encrypted(
    session: &VaultSession,
    dest: &PathBuf,
    ids: &[EntryId],
    passphrase: &Zeroizing<String>,
) -> Result<ExportReport, VaultError> {
    let mut entries = Vec::with_capacity(ids.len());
    // Blob bytes must outlive the tar assembly (the members borrow them).
    let mut blob_members: Vec<(String, Zeroizing<Vec<u8>>)> = Vec::new();

    for id in ids {
        let row = session.repo.get_entry(id).await?;
        // Unwrap the DEK once so a Document can decrypt its blob with the same key.
        let dek = session
            .crypto
            .unwrap_dek(&row.dek_wrapped, session.kek.expose())?;
        let payload = super::refs::decrypt_row_with_dek(session.crypto.as_ref(), &dek, &row)?;

        if let EntryPayload::Document(doc) = &payload {
            let bytes = session.blob.read_blob(id, &dek, &doc.blob_nonce).await?;
            blob_members.push((format!("{}{}", archive::BLOBS_PREFIX, id.as_str()), bytes));
        }

        let tag_names = resolve_tags(session, payload.meta());
        entries.push(payload_to_export(id, &payload, tag_names));
    }

    let bundle = ExportBundle::new(entries);
    let entries_json = Zeroizing::new(
        serde_json::to_vec(&bundle)
            .map_err(|e| VaultError::MalformedPayload(format!("export bundle: {e}")))?,
    );

    // Assemble the tar in RAM: entries.json + each document's plaintext bytes.
    let mut members: Vec<(String, &[u8])> =
        Vec::with_capacity(blob_members.len().saturating_add(1));
    members.push((archive::ENTRIES_MEMBER.to_owned(), entries_json.as_slice()));
    for (name, bytes) in &blob_members {
        members.push((name.clone(), bytes.as_slice()));
    }
    let tar = archive::build_tar(&members)?;

    // Seal in memory; only the ciphertext is ever written to disk (Decision ②).
    let sealed = envelope::seal(passphrase.as_bytes(), &tar)?;
    std::fs::write(dest, &sealed).map_err(|e| io_ctx("write export", &e))?;

    Ok(ExportReport {
        dest: dest.clone(),
        entry_count: u64::try_from(ids.len()).unwrap_or(u64::MAX),
        encrypted: true,
    })
}

async fn export_csv(
    session: &VaultSession,
    dest: &PathBuf,
    ids: &[EntryId],
    spreadsheet_safe: bool,
) -> Result<ExportReport, VaultError> {
    let mut logins: Vec<csv::CsvLogin> = Vec::new();
    for id in ids {
        let row = session.repo.get_entry(id).await?;
        let payload = super::refs::decrypt_row_payload(session, &row)?;
        // Logins only — everything richer belongs in the encrypted envelope.
        if let EntryPayload::Login(l) = &payload {
            let tags = resolve_tags(session, &l.meta);
            logins.push(csv::CsvLogin {
                name: l.meta.name.clone(),
                username: l.username.clone(),
                password: l.password.clone(),
                url: l.meta.url.clone().unwrap_or_default(),
                notes: l.meta.notes.clone().unwrap_or_default(),
                tags,
            });
        }
    }

    let text = csv::write_csv(&logins, spreadsheet_safe)?;
    // PLAINTEXT to disk — the user's explicit choice (the shell shows the ⑧ warning).
    std::fs::write(dest, text.as_bytes()).map_err(|e| io_ctx("write csv export", &e))?;

    Ok(ExportReport {
        dest: dest.clone(),
        entry_count: u64::try_from(logins.len()).unwrap_or(u64::MAX),
        encrypted: false,
    })
}
