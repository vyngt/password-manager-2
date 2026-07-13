//! Live-safe backup of an unlocked vault to a single `.vbk` archive (slice 5.2).
//!
//! Snapshots the `.vdb` via `VACUUM INTO` (no forced lock), copies the whole
//! `.vedge_blobs/` directory, and tars them with a per-file + whole-archive
//! BLAKE3 manifest. The payload is already-encrypted ciphertext — no KEK, no DEK,
//! no plaintext ever touches this path (Decision ④). One `BackupCreated` audit
//! row, vault-level (`entry_id = None`), mirroring `export_emergency_kit`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{StorageError, format_rfc3339_millis, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive;
use crate::infrastructure::backup::manifest::{
    BACKUP_FORMAT_VERSION, BLOBS_PREFIX, BackupManifest, MANIFEST_NAME, ManifestFile, VAULT_MEMBER,
};

/// Where to write the archive. The `.blake3` sidecar goes beside it.
#[derive(Debug, Clone)]
pub struct BackupVaultInput {
    pub dest_archive: PathBuf,
}

/// The result surfaced to the UI (all non-invertible derivatives — counts,
/// hashes, timestamp; no key material, no plaintext).
#[derive(Debug, Clone)]
pub struct BackupReport {
    pub archive_path: PathBuf,
    pub archive_bytes: u64,
    pub archive_blake3: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub created_at: String,
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Append a literal suffix to a path (`x.vbk` + `.tmp` → `x.vbk.tmp`), unlike
/// `with_extension` which would *replace* the existing one.
fn append_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s: OsString = base.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn backup_vault(
    session: &VaultSession,
    input: BackupVaultInput,
) -> Result<BackupReport, VaultError> {
    let dest = input.dest_archive;
    let parent = dest
        .parent()
        .ok_or_else(|| io_msg("backup destination has no parent directory"))?;

    // Stage on the SAME volume as the destination — the archive lands by a single
    // intra-volume rename at the end. TempDir cleans up on drop (incl. on error).
    let staging =
        tempfile::TempDir::new_in(parent).map_err(|e| io_ctx("create staging dir", &e))?;
    let staging_path = staging.path();

    // 1. Live-safe snapshot of the vault DB (Decision ①). Dest must not exist →
    //    a fresh path inside the staging dir.
    let vault_snap = staging_path.join(VAULT_MEMBER);
    session.repo.vacuum_into(&vault_snap).await?;

    // 2. Hash the snapshot → its manifest row.
    let (vault_size, vault_hash) = archive::hash_file(&vault_snap)?;
    let mut files = vec![ManifestFile {
        name: VAULT_MEMBER.to_owned(),
        size: vault_size,
        blake3: vault_hash,
    }];

    // 3. Copy every `*.blob` from the DIRECTORY (source of truth, not the index —
    //    an orphan blob is still the user's bytes; `run_maintenance` reaps orphans,
    //    not backup). Stage + hash each; skip cleanly if the dir is absent.
    let blobs_staging = staging_path.join("blobs");
    let mut blob_members: Vec<(String, PathBuf)> = Vec::new();
    let mut blob_count: u64 = 0;

    let read_dir = match std::fs::read_dir(session.blob.blob_dir()) {
        Ok(rd) => Some(rd),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_ctx("read blob dir", &e)),
    };
    if let Some(rd) = read_dir {
        let mut blob_files: Vec<PathBuf> = Vec::new();
        for entry in rd {
            let path = entry.map_err(|e| io_ctx("read blob entry", &e))?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("blob") {
                blob_files.push(path);
            }
        }
        blob_files.sort(); // deterministic archive layout
        if !blob_files.is_empty() {
            std::fs::create_dir_all(&blobs_staging)
                .map_err(|e| io_ctx("create staging blobs dir", &e))?;
        }
        for src in blob_files {
            let file_name = src
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| io_msg("blob file has a non-UTF-8 name"))?
                .to_owned();
            let staged = blobs_staging.join(&file_name);
            std::fs::copy(&src, &staged).map_err(|e| io_ctx("copy blob", &e))?;
            let (size, blake3) = archive::hash_file(&staged)?;
            let member = format!("{BLOBS_PREFIX}{file_name}");
            files.push(ManifestFile {
                name: member.clone(),
                size,
                blake3,
            });
            blob_members.push((member, staged));
            blob_count = blob_count.saturating_add(1);
        }
    }

    // 4. Build + write the manifest.
    let created_at = format_rfc3339_millis(now());
    let entry_count = u64::try_from(session.index.entries.len()).unwrap_or(u64::MAX);
    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        vault_uuid: session.config.vault_uuid.clone(),
        schema_version: session.config.schema_version,
        created_at: created_at.clone(),
        entry_count,
        blob_count,
        files,
    };
    let manifest_path = staging_path.join(MANIFEST_NAME);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| VaultError::Storage(StorageError::Serialization(format!("manifest: {e}"))))?;
    std::fs::write(&manifest_path, &manifest_bytes).map_err(|e| io_ctx("write manifest", &e))?;

    // 5. Tar (manifest, vault, blobs…) → `dest.tmp`, hashing the stream.
    let mut members: Vec<(String, PathBuf)> =
        Vec::with_capacity(blob_members.len().saturating_add(2));
    members.push((MANIFEST_NAME.to_owned(), manifest_path));
    members.push((VAULT_MEMBER.to_owned(), vault_snap));
    members.extend(blob_members);

    let dest_tmp = append_suffix(&dest, ".tmp");
    let archive_blake3 = archive::write_archive(&members, &dest_tmp)?;

    // 6. Atomic move into place (intra-volume).
    std::fs::rename(&dest_tmp, &dest).map_err(|e| io_ctx("rename archive into place", &e))?;

    // 7. Advisory whole-archive hash sidecar (Decision ③).
    let sidecar = append_suffix(&dest, ".blake3");
    std::fs::write(&sidecar, format!("{archive_blake3}\n"))
        .map_err(|e| io_ctx("write archive hash sidecar", &e))?;

    let archive_bytes = std::fs::metadata(&dest)
        .map_err(|e| io_ctx("stat archive", &e))?
        .len();

    // 8. Exactly one vault-level audit row (ciphertext left — never `Exported`).
    super::create_entry::append_audit(session, AuditAction::BackupCreated, None).await?;

    Ok(BackupReport {
        archive_path: dest,
        archive_bytes,
        archive_blake3,
        entry_count,
        blob_count,
        created_at,
    })
}
