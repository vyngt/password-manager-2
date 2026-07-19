//! `rekey_vault` — the full re-encrypt-everything superset of `change_password` (slice 5.8).
//!
//! ## Why re-key exists (what a rewrap CANNOT do)
//!
//! `change_password` (5.6) rewraps every per-entry DEK from the old KEK to the new one, but the
//! **DEK bytes and every ciphertext are byte-identical** — only their KEK-wrapping rotates. So an
//! attacker who copied the `.vdb` AND opened it once holds the DEKs themselves, and those DEKs
//! still open every payload — a later password change buys nothing for existing data. Re-key is
//! the ONLY operation that revokes a captured DEK: it mints a **fresh DEK per entry** and
//! **re-encrypts every ciphertext surface** — entries, **every `entry_history` version**, document
//! blobs, and tags — under new DEKs + a new KEK. The old DEKs decrypt nothing that exists going
//! forward.
//!
//! ## Crash-safety — stage-and-swap (spec ②), never in-place
//!
//! Re-key touches DB rows AND external blob files — two storage domains with no shared
//! transaction — so an in-place mutation has no atomic boundary. Instead it rebuilds the whole
//! home under new DEKs into a staging dir and swaps it in via the 5.2 restore journal
//! (`journal::commit_preserving`). A crash before the commit point leaves the live vault
//! untouched (the journal rolls back); only a fully re-encrypted staged home ever swaps in.
//!
//! ## Retire old snapshots (spec ③) — a SECURITY requirement
//!
//! A pre-re-key snapshot's bodies stay on the OLD DEKs, and reverting to it would restore the
//! exact state re-key burned. So the swap uses `preserve = &[]` (the ONE swap that does NOT carry
//! `snapshots/` across, unlike revert/replace) — every pre-re-key snapshot is retired.
//!
//! ## Locks on success (spec ⑤)
//!
//! Re-key crosses a credential boundary and rebuilds the whole home, so on success the caller
//! LOCKS the vault (the 5.7 revert-locks precedent) and the user re-unlocks against the re-keyed
//! state. The core closes the live DB handle before the swap (Windows handle release) and returns
//! an outcome telling the caller whether it must lock.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{instrument, warn};
use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::blob_store::BlobStore;
use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, SNAPSHOTS_DIR, StorageError, format_rfc3339_millis, now};
use crate::domain::vault::aad::{entry_aad, tag_aad};
use crate::domain::vault::crypto_constants::{DEK_LEN, KEK_LEN, SECRET_KEY_LEN, VERIFY_HASH_LEN};
use crate::domain::vault::entities::{AuditAction, AuditEvent, EntryHistoryRow, EntryRow, TagRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::entry_payload::EntryPayload;
use crate::infrastructure::backup::{archive, journal};
use crate::infrastructure::blob::FilesystemBlobStore;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use super::credential_common::{derive_kek_and_verify, restore_keychain_and_biometric};
use super::lock_vault::close_session_db;
use super::tag_crypto::{open_tag_row, seal_tag_payload};

/// Re-key parameters. **New password is required** (re-key is a credential-reset moment); a new
/// Secret Key is optional (`None` keeps the existing one).
#[derive(Debug)]
pub struct RekeyVaultInput {
    pub new_password: Zeroizing<String>,
    pub new_secret_key: Option<Zeroizing<[u8; SECRET_KEY_LEN]>>,
}

/// Non-invertible facts surfaced to the UI once the swap has committed.
#[derive(Debug, Clone)]
pub struct RekeyReport {
    pub entry_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub reencrypted_at: String,
}

/// The outcome of a re-key.
///
/// Mirrors [`SeamlessRevertOutcome`](super::revert_to_snapshot::SeamlessRevertOutcome): the variant
/// tells the caller whether the session's DB handle was already closed (and so the caller MUST
/// lock) or whether the session is still fully alive.
pub enum RekeyOutcome {
    /// The swap committed — the vault is re-keyed. The session's DB is closed; the caller MUST
    /// lock (drop the session + remove it from state) and the user re-unlocks with the new
    /// password.
    Rekeyed { report: RekeyReport },
    /// The user cancelled BEFORE the commit point. Staging was discarded, the live vault is
    /// untouched, and the session is still fully alive — the caller keeps the user unlocked.
    Cancelled,
    /// The re-encryption completed but the write-ahead swap FAILED (rare — e.g. a full disk after
    /// staging). The journal rolled back: the live vault is unchanged (opens on the OLD password).
    /// The session's DB is already closed, so the caller must lock and surface the error.
    CommitFailed { error: VaultError },
}

/// Internal result of the staged re-encryption pass.
enum ReencryptStatus {
    Completed { entry_count: u64 },
    Cancelled,
}

/// The new credential material threaded through the re-encryption pass (bundled to keep the
/// argument counts sane).
struct NewCreds<'a> {
    old_kek: &'a [u8; KEK_LEN],
    new_kek: &'a [u8; KEK_LEN],
    verify_hash: &'a [u8; VERIFY_HASH_LEN],
}

/// Everything a single entry's re-encryption needs: the crypto, both KEKs, and both blob stores
/// (the LIVE one to read the old blob, the STAGED one to write the new blob).
struct RekeyCtx<'a> {
    crypto: &'a dyn CryptoProvider,
    old_kek: &'a [u8; KEK_LEN],
    new_kek: &'a [u8; KEK_LEN],
    verify_hash: &'a [u8; VERIFY_HASH_LEN],
    live_blob: &'a dyn BlobStore,
    staged_blob: &'a FilesystemBlobStore,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Re-key an unlocked vault: fresh DEK per entry, every ciphertext surface re-encrypted, swapped
/// in crash-safely, old snapshots retired.
///
/// `on_progress(done, total)` is called after each entry; `cancel` is polled before each entry —
/// a cancel BEFORE the commit point discards staging with the live vault untouched (a cancel is
/// impossible once the swap begins).
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn rekey_vault(
    session: &VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    input: RekeyVaultInput,
    on_progress: &(dyn Fn(u64, u64) + Send + Sync),
    cancel: &AtomicBool,
) -> Result<RekeyOutcome, VaultError> {
    let home = session.vault_id().path().to_path_buf();

    // 0. Reconcile any prior interrupted swap of this home before staging a fresh one.
    journal::recover_if_pending(&home)?;

    // 1. Resolve the Secret Key (rotated OR existing) and derive the NEW KEK + verify_hash.
    let secret_key = if let Some(sk) = input.new_secret_key.clone() {
        sk
    } else {
        let uuid = session
            .vault_uuid()
            .ok_or(VaultError::KeychainEntryNotFound)?;
        keychain.read_secret_key(uuid)?
    };
    let (new_kek_z, new_verify_hash) = derive_kek_and_verify(
        kdf,
        input.new_password.clone(),
        secret_key,
        session.config.vault_salt,
        session.config.kdf_params.clone(),
    )
    .await?;
    let old_kek_z: Zeroizing<[u8; KEK_LEN]> = Zeroizing::new(*session.kek.expose());

    // 2. Stage a fresh copy of the home (VACUUM the vault + an EMPTY snapshots/) to re-encrypt.
    let staging = journal::staging_path(&home);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| io_ctx("create staging", &e))?;
    let staged_vault = journal::staged_vault(&staging);
    let staged_blobs = journal::staged_blobs(&staging);
    std::fs::create_dir_all(&staged_blobs).map_err(|e| io_ctx("create staged blobs", &e))?;
    // The retired snapshot store: staging ships an EMPTY `snapshots/`, and the `&[]` preserve-list
    // (step 4) drops the live one on swap — a re-key retires all pre-re-key snapshots (spec ③).
    std::fs::create_dir_all(staging.join(SNAPSHOTS_DIR))
        .map_err(|e| io_ctx("create staged snapshots", &e))?;
    // Live-safe VACUUM INTO the staging dir (dest must not pre-exist — it is fresh).
    session.repo.vacuum_into(&staged_vault).await?;

    // 3. Re-encrypt every surface in the staged copy. A cancel or error here leaves the LIVE vault
    //    untouched (its DB is still open); we just discard staging.
    let creds = NewCreds {
        old_kek: &old_kek_z,
        new_kek: &new_kek_z,
        verify_hash: &new_verify_hash,
    };
    let status = match reencrypt_into_staging(
        &staged_vault,
        &staging,
        session,
        &creds,
        on_progress,
        cancel,
    )
    .await
    {
        Ok(status) => status,
        Err(e) => {
            std::fs::remove_dir_all(&staging).ok();
            return Err(e);
        }
    };
    let entry_count = match status {
        ReencryptStatus::Completed { entry_count } => entry_count,
        ReencryptStatus::Cancelled => {
            std::fs::remove_dir_all(&staging).ok();
            return Ok(RekeyOutcome::Cancelled);
        }
    };

    // 4. COMMIT: hash the staged vault (anchors the journal), close the LIVE DB handle so the home
    //    rename does not race an open `.vdb` on Windows, then run the swap RETIRING snapshots.
    let (_size, staged_hash) = archive::hash_file(&staged_vault)?;
    let started_at = format_rfc3339_millis(now());
    close_session_db(session).await;
    if let Err(error) = commit_swap_retiring(&home, &staging, &staged_hash, &started_at).await {
        // The journal rolled back: the live vault is intact on the OLD credentials. The session's
        // DB is already closed, so the caller must lock either way.
        return Ok(RekeyOutcome::CommitFailed { error });
    }

    // 5. POST-SWAP (best-effort): re-store the rotated Secret Key + biometric KEK. The vault is
    //    already re-keyed and openable with the new password; a failure here is not fatal.
    if let Some(uuid) = session.vault_uuid() {
        if let Err(e) = restore_keychain_and_biometric(
            keychain.as_ref(),
            biometric.as_ref(),
            uuid,
            &new_kek_z,
            input.new_secret_key.as_deref(),
        ) {
            warn!(error = %e, "re-key committed but re-storing keychain/biometric failed — re-enrol as needed");
        }
    }

    Ok(RekeyOutcome::Rekeyed {
        report: RekeyReport {
            entry_count,
            reencrypted_at: started_at,
        },
    })
}

/// Open the staged DB, re-encrypt every surface, and ALWAYS release the DB handle before
/// returning (the caller may remove or swap the staging dir, and Windows will not touch a
/// directory holding an open `.vdb`).
async fn reencrypt_into_staging(
    staged_vault: &Path,
    staging: &Path,
    session: &VaultSession,
    creds: &NewCreds<'_>,
    on_progress: &(dyn Fn(u64, u64) + Send + Sync),
    cancel: &AtomicBool,
) -> Result<ReencryptStatus, VaultError> {
    let db = VaultDbConnection::open(staged_vault)
        .await
        .map_err(VaultError::Storage)?;
    let staged_repo = SqliteVaultRepository::new(db.handle());
    let staged_blob = FilesystemBlobStore::new(staging, Arc::clone(&session.crypto))?;
    let ctx = RekeyCtx {
        crypto: session.crypto.as_ref(),
        old_kek: creds.old_kek,
        new_kek: creds.new_kek,
        verify_hash: creds.verify_hash,
        live_blob: session.blob.as_ref(),
        staged_blob: &staged_blob,
    };

    let result = reencrypt_rows(&staged_repo, &ctx, on_progress, cancel).await;

    // Release the handle on EVERY path. `drop(staged_repo)` frees the repo's Arc clone so
    // `db.close()`'s `Arc::try_unwrap` can actually close the pool (the create_snapshot idiom).
    drop(staged_repo);
    let close_result = db.close().await;
    let status = result?; // surface a re-encryption error first (the DB is already closed)
    close_result.map_err(VaultError::Storage)?;
    Ok(status)
}

/// The re-encryption pass over the staged DB: every entry (+ its history + blob), every tag, the
/// config, the audit rows, and a read-back validation under the new KEK.
async fn reencrypt_rows(
    staged_repo: &SqliteVaultRepository,
    ctx: &RekeyCtx<'_>,
    on_progress: &(dyn Fn(u64, u64) + Send + Sync),
    cancel: &AtomicBool,
) -> Result<ReencryptStatus, VaultError> {
    let rows = staged_repo.all_entries().await?;
    let total = u64::try_from(rows.len()).unwrap_or(u64::MAX);

    for (i, row) in rows.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(ReencryptStatus::Cancelled);
        }
        reencrypt_one_entry(staged_repo, ctx, &row).await?;
        let done = u64::try_from(i).unwrap_or(u64::MAX).saturating_add(1);
        on_progress(done, total);
    }

    // Tags: mint a FRESH tag DEK and re-encrypt the tag payload under the new KEK (the `Some`
    // branch of `rewrap_tag_row` only re-wraps — re-key must rotate the DEK, so re-seal instead).
    for tag in staged_repo.all_tags().await? {
        let plaintext = open_tag_row(ctx.crypto, ctx.old_kek, &tag)?;
        let aad = tag_aad(&tag.id)?;
        let (nonce, ciphertext, dek_wrapped) =
            seal_tag_payload(ctx.crypto, ctx.new_kek, &aad, &plaintext)?;
        let new_tag = TagRow {
            nonce,
            ciphertext,
            dek_wrapped: Some(dek_wrapped),
            ..tag.clone()
        };
        staged_repo.update_tag(&new_tag).await?;
    }

    // Config: the new `verify_hash`; NULL the recovery slot (a KEK change revokes recovery, 5.7 ③).
    // `save_config` deliberately OMITS `recovery_slot` from its UPDATE, so null it via the
    // targeted `clear_recovery_slot` afterwards.
    let mut cfg = staged_repo.load_config().await?;
    cfg.verify_hash = *ctx.verify_hash;
    cfg.last_unlocked_at = Some(now());
    cfg.recovery_slot = None;
    staged_repo.save_config(&cfg).await?;
    staged_repo.clear_recovery_slot().await?;

    // Audit — written INTO the STAGED (surviving) DB, not the live session repo (discarded on the
    // swap): `VaultRekeyed` then `PasswordChanged` (re-key is a superset of a password change).
    append_staged_audit(staged_repo, AuditAction::VaultRekeyed).await?;
    append_staged_audit(staged_repo, AuditAction::PasswordChanged).await?;

    // Validate the staged home opens under the NEW KEK before we swap it in: read back the first
    // entry and decrypt it under the new KEK (unwrap proves the new wrap; decrypt proves the
    // ciphertext/nonce/AAD are consistent). A failure aborts with the live vault untouched.
    if let Some(row) = staged_repo.all_entries().await?.into_iter().next() {
        let dek = ctx.crypto.unwrap_dek(&row.dek_wrapped, ctx.new_kek)?;
        let aad = entry_aad(&row.id, row.version)?;
        let _ = ctx
            .crypto
            .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)?;
    }

    Ok(ReencryptStatus::Completed { entry_count: total })
}

/// Re-encrypt one entry: mint a fresh DEK, re-encrypt its payload (+ document blob, in lockstep),
/// wrap the new DEK under the new KEK, and re-encrypt EVERY history version under the same DEK.
async fn reencrypt_one_entry(
    staged_repo: &SqliteVaultRepository,
    ctx: &RekeyCtx<'_>,
    row: &EntryRow,
) -> Result<(), VaultError> {
    let old_dek = ctx.crypto.unwrap_dek(&row.dek_wrapped, ctx.old_kek)?;
    let new_dek = ctx.crypto.generate_dek();

    // The live entry payload (+ its blob, if a Document).
    let aad = entry_aad(&row.id, row.version)?;
    let plaintext = ctx
        .crypto
        .decrypt_entry(&old_dek, &row.nonce, &row.ciphertext, &aad)?;
    let new_payload_bytes =
        reencrypt_payload_and_blob(ctx, &row.id, &old_dek, &new_dek, &plaintext).await?;
    let (new_nonce, new_ct) = ctx
        .crypto
        .encrypt_entry(&new_dek, &new_payload_bytes, &aad)?;
    let new_wrapped = ctx.crypto.wrap_dek(&new_dek, ctx.new_kek)?;
    let new_row = EntryRow {
        dek_wrapped: new_wrapped,
        nonce: new_nonce,
        ciphertext: new_ct,
        ..row.clone()
    };
    staged_repo.update_entry(&new_row).await?;

    // 🔴 The landmine (spec ①): `entry_history` has NO per-row DEK — every version is sealed under
    // the live entry's stable DEK. A fresh entry DEK orphans them all unless each is re-encrypted
    // under the new DEK with the SAME version-AAD (id/version unchanged, only the DEK + nonce
    // rotate). Delete-then-reinsert (there is no `update_history`); the version/changed_at are
    // preserved verbatim.
    let history = staged_repo.list_history(&row.id).await?;
    if !history.is_empty() {
        let mut new_history = Vec::with_capacity(history.len());
        for h in &history {
            let h_aad = entry_aad(&row.id, h.version)?;
            let h_plain = ctx
                .crypto
                .decrypt_entry(&old_dek, &h.nonce, &h.ciphertext, &h_aad)?;
            let (h_nonce, h_ct) = ctx.crypto.encrypt_entry(&new_dek, &h_plain, &h_aad)?;
            new_history.push(EntryHistoryRow {
                nonce: h_nonce,
                ciphertext: h_ct,
                ..h.clone()
            });
        }
        staged_repo.delete_history_for_entry(&row.id).await?;
        for h in &new_history {
            staged_repo.insert_history(h).await?;
        }
    }

    Ok(())
}

/// Re-encrypt an entry's payload bytes under the new DEK's regime, handling the ONE payload that
/// carries out-of-band ciphertext — a `Document`'s external blob.
///
/// 🔴 This is the `..`-less / no-`_` enumeration that makes a silently-partial re-key impossible
/// (spec ①, the 5.4.1 move): a future `EntryPayload` variant that carries its own external
/// ciphertext will fail to compile here until a maintainer decides how re-key must rotate it.
async fn reencrypt_payload_and_blob(
    ctx: &RekeyCtx<'_>,
    entry_id: &EntryId,
    old_dek: &[u8; DEK_LEN],
    new_dek: &[u8; DEK_LEN],
    plaintext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let payload = EntryPayload::from_decrypted_json(plaintext)?;
    match payload {
        // A Document's blob is sealed under the SAME entry DEK. Re-encrypt it under the new DEK (a
        // fresh on-disk nonce), patch the payload's `blob_nonce` to match, and re-serialize.
        // `read_blob` enforces disk-nonce == payload-nonce, so the two MUST move in lockstep.
        EntryPayload::Document(mut doc) => {
            let blob_plain = ctx
                .live_blob
                .read_blob(entry_id, old_dek, &doc.blob_nonce)
                .await?;
            let new_blob_nonce = ctx
                .staged_blob
                .write_blob(entry_id, new_dek, &blob_plain)
                .await?;
            doc.blob_nonce = new_blob_nonce;
            EntryPayload::Document(doc).to_encryptable_json()
        }
        // Every other variant carries no out-of-band ciphertext — re-encrypt the SAME plaintext
        // bytes verbatim. Verbatim (not a serde round-trip) is deliberate: `to_encryptable_json`
        // refuses `Unknown`, and re-serializing would drop a forward-compat entry's bytes.
        EntryPayload::Login(_)
        | EntryPayload::Card(_)
        | EntryPayload::SshKey(_)
        | EntryPayload::ApiKey(_)
        | EntryPayload::EnvVars(_)
        | EntryPayload::Note(_)
        | EntryPayload::Identity(_)
        | EntryPayload::Folder(_)
        | EntryPayload::Unknown(_) => Ok(Zeroizing::new(plaintext.to_vec())),
    }
}

/// Append one vault-level audit row to the STAGED repo.
async fn append_staged_audit(
    repo: &SqliteVaultRepository,
    action: AuditAction,
) -> Result<(), VaultError> {
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action,
        occurred_at: now(),
        device_id: None,
    };
    repo.append_audit(&event).await
}

/// Run the whole-home swap on a blocking thread, RETIRING the snapshot store.
///
/// 🔴 `preserve = &[]` — a re-key does NOT carry `snapshots/` across, the opposite of revert/replace
/// (`&[SNAPSHOTS_DIR]`): a pre-re-key snapshot's bodies are on the OLD DEKs, so keeping it
/// revertable would defeat the re-key (spec ③). Blocking so a lingering just-closed pool can finish
/// releasing the `.vdb` handle while the rename retries (M1, Windows).
async fn commit_swap_retiring(
    home: &Path,
    staging: &Path,
    vault_hash: &str,
    started_at: &str,
) -> Result<(), VaultError> {
    let (home, staging, vault_hash, started_at) = (
        home.to_owned(),
        staging.to_owned(),
        vault_hash.to_owned(),
        started_at.to_owned(),
    );
    tokio::task::spawn_blocking(move || {
        journal::commit_preserving(&home, &staging, &vault_hash, &started_at, &[])
    })
    .await
    .map_err(|e| VaultError::Storage(StorageError::Io(format!("swap task join: {e}"))))?
}
