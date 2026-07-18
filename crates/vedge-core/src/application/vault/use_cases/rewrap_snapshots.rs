//! Rewrap the local snapshot store under new credentials (slice 5.2.1, Decision ⑬).
//!
//! A snapshot's `vault.vdb` carries `verify_hash = f(password, SecretKey)` and per-entry DEKs
//! wrapped under the KEK **of its own moment**. Left alone, a credential change would leave
//! every snapshot openable only with the password the product just told the user to discard.
//!
//! So on every password change / Secret-Key rotation we rewrap each snapshot: unwrap each
//! `dek_wrapped` with the OLD KEK, rewrap with the NEW KEK, write the new `verify_hash`, in
//! ONE transaction per snapshot. **The object pool is never touched** — blobs are sealed under
//! DEKs, and a rewrap changes only the KEK-wrapping of those DEKs, never the DEK bytes
//! (Decision ⑬: snapshot DEKs are NEVER rotated — after a breach the honest advice is to
//! retire the old snapshots, not pretend a re-key un-rings the bell).
//!
//! 🔴 Two hard rules, enforced by the caller (`change_password`) and here:
//! - The live vault commits FIRST; this runs AFTER. A crash then leaves snapshots at the OLD
//!   generation — openable with the password the user just typed — plus a "needs updating"
//!   report, never stranded under a password that does not yet exist.
//! - A failed rewrap NEVER fails the credential change. Each failure is a `warn!` + a `failed`
//!   entry; the snapshot simply keeps the previous credentials (the ⚠ stale badge, ⑯).

use std::path::PathBuf;

use tracing::warn;

use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::use_cases::tag_crypto::rewrap_tag_row;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::snapshot::manifest::verify_hash_prefix;
use crate::infrastructure::snapshot::store::{self, SnapshotEntry};
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

/// The outcome of rewrapping a store.
///
/// How many landed, and which snapshots kept the old credentials (a stale-credential snapshot
/// the UI must badge and retention must not prune).
#[derive(Debug, Default, Clone)]
pub struct RewrapReport {
    pub rewrapped: u64,
    pub failed: Vec<PathBuf>,
}

/// Rewrap every snapshot in `snapshots_dir` from `old_kek` to `new_kek`.
///
/// Stamps `new_verify_hash`. Never returns `Err` — the credential change has already
/// committed; per-snapshot failures are collected into the report.
pub async fn rewrap_snapshots(
    crypto: &dyn CryptoProvider,
    snapshots_dir: &std::path::Path,
    old_kek: &[u8; KEK_LEN],
    new_kek: &[u8; KEK_LEN],
    new_verify_hash: &[u8; 32],
) -> RewrapReport {
    let mut report = RewrapReport::default();
    let snapshots = match store::list_snapshots(snapshots_dir) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "could not list snapshots to rewrap — leaving them at the old credentials");
            return report;
        }
    };
    for snap in snapshots {
        match rewrap_one(crypto, &snap, old_kek, new_kek, new_verify_hash).await {
            Ok(()) => report.rewrapped = report.rewrapped.saturating_add(1),
            Err(e) => {
                warn!(error = %e, snapshot = %snap.id(), "snapshot rewrap failed — it keeps the previous credentials");
                report.failed.push(snap.dir.clone());
            }
        }
    }
    report
}

/// Rewrap a single snapshot's `vault.vdb` in one transaction, then stamp its manifest's
/// `verify_hash_prefix`. Any unwrap failure — an entry DEK OR a tag (slice 5.6.0) that an
/// already-rewrapped or stale snapshot cannot yield — aborts BEFORE the write, so a snapshot
/// is never left half-rewrapped (which, for a tag, would re-brick it on revert: the revert
/// reopen goes through `build_index`, which hard-fails on a tag it cannot decrypt).
async fn rewrap_one(
    crypto: &dyn CryptoProvider,
    snap: &SnapshotEntry,
    old_kek: &[u8; KEK_LEN],
    new_kek: &[u8; KEK_LEN],
    new_verify_hash: &[u8; 32],
) -> Result<(), VaultError> {
    let db = VaultDbConnection::open(&snap.vault_file())
        .await
        .map_err(VaultError::Storage)?;
    let repo = SqliteVaultRepository::new(db.handle());

    let rows = repo.all_entries().await?;
    let mut updates: Vec<(crate::domain::shared::EntryId, [u8; 40])> =
        Vec::with_capacity(rows.len());
    for row in rows {
        // A failure here (the row is not wrapped under `old_kek`) aborts the whole snapshot —
        // no partial write. `dek` is `Zeroizing`, zeroized as the loop iterates.
        let dek = crypto.unwrap_dek(&row.dek_wrapped, old_kek)?;
        let new_wrapped = crypto.wrap_dek(&dek, new_kek)?;
        updates.push((row.id, new_wrapped));
    }

    // Tags rewrap/migrate the same way (slice 5.6.0): a DEK-sealed snapshot tag re-wraps its
    // DEK; a legacy (pre-5.6.0) snapshot tag is migrated — legacy-decrypt under `old_kek`
    // (which succeeds precisely when the entry rewrap does) then re-seal under `new_kek`. A
    // tag it genuinely cannot read aborts the WHOLE snapshot via `?` BEFORE any write, so
    // `verify_hash` never advances past a skipped tag (the revert re-brick, B2).
    let tag_rows = repo.all_tags().await?;
    let mut tag_updates = Vec::with_capacity(tag_rows.len());
    for tag in &tag_rows {
        tag_updates.push(rewrap_tag_row(crypto, tag, old_kek, new_kek)?);
    }

    let mut cfg = repo.load_config().await?;
    cfg.verify_hash = *new_verify_hash;
    // A snapshot carries no Recovery Key slot of its own (slice 5.7 ④) — you revert it with
    // the current session KEK, never recovery-unlock it. `create_snapshot` already nulls the
    // slot at capture, so this is belt-and-suspenders (it should already be `None`); nulling
    // it here as part of the rewrap guarantees a rewrapped snapshot never keeps a slot that
    // wraps the pre-rewrap KEK — the same brick as ③, one level down.
    cfg.recovery_slot = None;
    // One atomic transaction: every entry DEK + every tag key + the new `verify_hash`. The
    // object pool is NOT touched — DEK values are unchanged, only their KEK-wrapping.
    repo.rewrap_all_deks(&updates, &tag_updates, &cfg).await?;

    drop(repo);
    // Close so the WAL is checkpointed into the snapshot's own `vault.vdb` (self-contained).
    db.close().await.map_err(VaultError::Storage)?;

    // Stamp the manifest so the UI can confirm the rewrap landed (⑬) and clear the stale badge.
    let mut manifest = snap.manifest.clone();
    manifest.verify_hash_prefix = verify_hash_prefix(new_verify_hash);
    store::write_manifest(&snap.dir, &manifest)?;
    Ok(())
}
