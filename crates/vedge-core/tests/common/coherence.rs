//! PG.1 "Vault Integrity Matrix": one reusable [`assert_vault_coherent`] that OPENS THE VAULT
//! HOME FRESH FROM DISK (new DB connection, KEK re-derived from the supplied creds — never the
//! live in-memory session) and asserts 13 invariants across the five tables + blobs + snapshots
//! + OS state. Called after every mutating-operation integration test.
//!
//! 🔴 The fresh open **is** the test. Phase 5's three worst bugs (5.6.0 tags, 5.7 snapshot
//! manifests, 5.8 history) all returned `Ok`, kept the live session working, and failed only at
//! the *next* unlock/revert — a helper inspecting the in-memory session would have caught none.
//! Generalizes `rekey_vault.rs`'s bespoke `dump()`/`derive()` pattern.
//!
//! Every failure NAMES the failing invariant + the offending row/file — `"vault incoherent"` is
//! unactionable; `"entry 01ARZ… history v2 does not decrypt under the live DEK"` is a fix.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines
)]

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use vedge_core::application::vault::ports::{
    BlobStore, CryptoProvider, KeyDerivationProvider, KeychainProvider, VaultRepository,
};
use vedge_core::domain::shared::{EntryId, SNAPSHOTS_DIR, TagId, VAULT_FILE, now};
use vedge_core::domain::vault::aad::{entry_aad, tag_aad};
use vedge_core::domain::vault::crypto_constants::{
    DEK_LEN, KEK_LEN, RECOVERY_KEY_LEN, SECRET_KEY_LEN, VERIFY_HASH_LEN,
};
use vedge_core::domain::vault::entities::{AuditQuery, EntryRow, VaultConfig};
use vedge_core::domain::vault::payloads::EntryPayload;
use vedge_core::infrastructure::backup::archive::hash_file;
use vedge_core::infrastructure::blob::FilesystemBlobStore;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::snapshot::store::{self, SnapshotEntry};
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use super::Harness;

/// The documents needed to re-derive the KEK from disk. Test-only — no `Creds` struct exists in
/// prod. Borrows to avoid cloning secrets.
pub struct Creds<'a> {
    pub master_password: &'a str,
    pub secret_key: &'a [u8; SECRET_KEY_LEN],
    /// Only a recovery test supplies this; gates the FULL invariant #7 unwrap-to-KEK check. When
    /// `None`, a present slot is left unverified (an enroll test may reuse the helper without it).
    pub recovery_key: Option<&'a [u8; RECOVERY_KEY_LEN]>,
}

/// State that is NOT on disk in the home — it lives in the harness's in-memory OS-double
/// providers. Optional: when `None`, invariants #12 (rollback baseline) and the keychain-SK-present
/// check are skipped (the pure-disk invariants still all run).
pub struct OsState<'a> {
    pub vault_uuid: &'a str,
    pub keychain: &'a dyn KeychainProvider,
}

impl Harness {
    /// The common case: assert coherence with THIS harness's (unchanged) creds + OS state. Use the
    /// free [`assert_vault_coherent`] directly when a credential op changed the creds or moved the
    /// home out from under the harness.
    pub async fn assert_coherent(&self) {
        let creds = Creds {
            master_password: self.master_password.as_str(),
            secret_key: &self.secret_key,
            recovery_key: None,
        };
        let os = OsState {
            vault_uuid: &self.vault_uuid,
            keychain: self.keychain.as_ref(),
        };
        assert_vault_coherent(&self.home, &creds, Some(&os)).await;
    }
}

/// Open `home` FRESH under `creds` and assert EVERY invariant. Panics with the failing invariant
/// named — never a bare `assert!(ok)`. `os = None` runs the pure-disk invariants only.
pub async fn assert_vault_coherent(home: &Path, creds: &Creds<'_>, os: Option<&OsState<'_>>) {
    let where_ = home.display();
    let crypto = XChaCha20CryptoProvider::new();
    let kdf = Argon2idKdfProvider::new();

    // ── Phase A: fresh open, read every raw row, close the handle cleanly ──
    // (`open` runs migrations + pins WAL, which mutates the *main* vault.vdb harmlessly — nothing
    // hashes it. Drop the repo, then close, before returning: Windows won't rename a home whose
    // `.vdb` is still open, and a later mutating op / TempDir drop must be able to.)
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .unwrap_or_else(|e| panic!("[coherence] cannot open vault {where_}: {e}"));
    let repo = SqliteVaultRepository::new(db.handle());
    let entries = repo.all_entries().await.unwrap();
    let tags = repo.all_tags().await.unwrap();
    let history = repo.all_history().await.unwrap();
    let config = repo.load_config().await.unwrap();
    let audit = repo
        .query_audit(&AuditQuery {
            limit: u32::MAX, // 🔴 Default is 0 — an unset limit makes invariant #11 vacuously pass.
            ..Default::default()
        })
        .await
        .unwrap();
    drop(repo);
    db.close().await.unwrap();

    // Re-derive (KEK, verify_hash) from the on-disk config + the supplied creds.
    let (kek, verify) = derive_kek(&kdf, &config, creds.secret_key, creds.master_password);

    // ── #6 verify_hash matches the re-derived KEK (constant-time; never `==`) ──
    assert!(
        crypto.verify_hash_matches(&verify, &config.verify_hash),
        "[coherence] invariant 6 (verify-hash): re-derived verify_hash != config.verify_hash in {where_}"
    );

    // ── #1 every entry decrypts (incl. TRASHED — all_entries has no filter) ──
    let entry_ids: HashSet<EntryId> = entries.iter().map(|r| r.id.clone()).collect();
    let mut deks: HashMap<EntryId, [u8; DEK_LEN]> = HashMap::with_capacity(entries.len());
    let mut decoded: Vec<(EntryRow, EntryPayload)> = Vec::with_capacity(entries.len());
    for row in &entries {
        let dek = crypto.unwrap_dek(&row.dek_wrapped, &kek).unwrap_or_else(|_| {
            panic!(
                "[coherence] invariant 1 (entry-decrypt): entry {} v{} DEK did not unwrap under the current KEK in {where_}",
                row.id, row.version
            )
        });
        let aad = entry_aad(&row.id, row.version).unwrap();
        let plain = crypto
            .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)
            .unwrap_or_else(|_| {
                panic!(
                    "[coherence] invariant 1 (entry-decrypt): entry {} v{} ciphertext did not decrypt in {where_}",
                    row.id, row.version
                )
            });
        let payload = EntryPayload::from_decrypted_json(&plain).unwrap_or_else(|e| {
            panic!(
                "[coherence] invariant 1 (entry-decrypt): entry {} v{} payload did not parse ({e}) in {where_}",
                row.id, row.version
            )
        });
        deks.insert(row.id.clone(), *dek);
        decoded.push((row.clone(), payload));
    }

    // ── #2 every history version decrypts under its LIVE entry's DEK ──
    //    (also #10's orphan-history arm: a history row whose entry has no live row.)
    for h in &history {
        let dek = deks.get(&h.entry_id).unwrap_or_else(|| {
            panic!(
                "[coherence] invariant 10 (ref-integrity): history row {} references entry {} which has no live row (orphan) in {where_}",
                h.id, h.entry_id
            )
        });
        let aad = entry_aad(&h.entry_id, h.version).unwrap();
        crypto
            .decrypt_entry(dek, &h.nonce, &h.ciphertext, &aad)
            .unwrap_or_else(|_| {
                panic!(
                    "[coherence] invariant 2 (history-decrypt): entry {} history v{} did not decrypt under the live DEK in {where_}",
                    h.entry_id, h.version
                )
            });
    }

    // ── #3 every Document entry's blob decrypts (read_blob checks disk-nonce == payload-nonce) ──
    let blob = FilesystemBlobStore::new(
        home,
        Arc::new(XChaCha20CryptoProvider::new()) as Arc<dyn CryptoProvider>,
    )
    .unwrap_or_else(|e| panic!("[coherence] cannot open blob store for {where_}: {e}"));
    for (row, payload) in &decoded {
        if let EntryPayload::Document(doc) = payload {
            let dek = deks.get(&row.id).unwrap();
            blob.read_blob(&row.id, dek, &doc.blob_nonce)
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "[coherence] invariant 3 (blob-decrypt): document {} blob {where_}/blobs/{}.blob did not open/decrypt",
                        row.id, row.id
                    )
                });
        }
    }

    // ── #4 no orphan blobs — every blobs/*.blob maps to a live (incl. trashed) entry ──
    for bid in &blob.list_blobs().await.unwrap() {
        assert!(
            entry_ids.contains(bid),
            "[coherence] invariant 4 (orphan-blob): {where_}/blobs/{bid}.blob has no live entry row"
        );
    }

    // ── #5 every tag decrypts under its own DEK, and NO tag has a NULL dek_wrapped ──
    //    (newly load-bearing since 5.9 retired `decrypt_legacy_tag`: a NULL tag is an unopenable
    //    brick, not a "migrate at unlock". open_tag_row is pub(crate), so inline the primitives.)
    let tag_ids: HashSet<TagId> = tags.iter().map(|t| t.id.clone()).collect();
    for t in &tags {
        let wrapped = t.dek_wrapped.as_ref().unwrap_or_else(|| {
            panic!(
                "[coherence] invariant 5 (tag-decrypt): tag {} has a NULL dek_wrapped (unopenable since 5.9) in {where_}",
                t.id
            )
        });
        let dek = crypto.unwrap_dek(wrapped, &kek).unwrap_or_else(|_| {
            panic!(
                "[coherence] invariant 5 (tag-decrypt): tag {} DEK did not unwrap under the current KEK in {where_}",
                t.id
            )
        });
        let aad = tag_aad(&t.id).unwrap();
        crypto
            .decrypt_entry(&dek, &t.nonce, &t.ciphertext, &aad)
            .unwrap_or_else(|_| {
                panic!(
                    "[coherence] invariant 5 (tag-decrypt): tag {} ciphertext did not decrypt in {where_}",
                    t.id
                )
            });
    }

    // ── #7 recovery_slot: NULL, or unwraps to the CURRENT KEK ──
    //    The slot is AES-KW(KEK_rec, KEK) where KEK_rec derives from the STABLE Recovery Key. The
    //    test has that key only if it enrolled recovery, so the full check is gated on it.
    match (config.recovery_slot, creds.recovery_key) {
        (Some(slot), Some(rk)) => {
            let kek_rec = derive_recovery_kek(&kdf, &config, creds.secret_key, rk);
            let unwrapped = crypto.unwrap_dek(&slot, &kek_rec).unwrap_or_else(|_| {
                panic!("[coherence] invariant 7 (recovery-slot): the slot did not unwrap under the recovery KEK in {where_}")
            });
            assert!(
                *unwrapped == kek,
                "[coherence] invariant 7 (recovery-slot): the slot unwraps to a KEK that is NOT the current vault KEK in {where_}"
            );
        }
        (None, Some(_)) => panic!(
            "[coherence] invariant 7 (recovery-slot): a Recovery Key was supplied but the slot is NULL in {where_}"
        ),
        // slot present + no RK supplied → cannot verify (legal); slot NULL → recovery is off.
        _ => {}
    }

    // ── #8 every snapshot: manifest vault_blake3 matches the file, and it opens + decrypts under
    //    the CURRENT KEK. (Exactly 5.7's latent-since-5.2.1 corruption, as an assertion.) ──
    let snaps = store::list_snapshots(&home.join(SNAPSHOTS_DIR)).unwrap();
    for snap in &snaps {
        // (a) hash the snapshot's vault.vdb — BEFORE any open (open would mutate it via WAL).
        let (_size, disk_hash) = hash_file(&snap.vault_file()).unwrap_or_else(|e| {
            panic!(
                "[coherence] invariant 8 (snapshot): snapshot {} vault.vdb could not be hashed ({e}) in {where_}",
                snap.id()
            )
        });
        assert!(
            disk_hash == snap.manifest.vault_blake3,
            "[coherence] invariant 8 (snapshot): snapshot {} manifest vault_blake3 ({}) != on-disk hash ({disk_hash}) in {where_}",
            snap.id(),
            snap.manifest.vault_blake3
        );
        // (b) open a COPY and decrypt every entry under the current KEK.
        assert_snapshot_opens_under_kek(snap, &crypto, &kek).await;
    }

    // ── #9 a non-empty snapshot store implies last_snapshot_at is set ──
    //    (One-directional: retention can prune a store to empty while a genuine "last taken at"
    //    stamp remains — that is coherent. The re-key case, empty-store-must-be-NULL, is pinned at
    //    the re-key call site, where the context makes it unambiguous.)
    if !snaps.is_empty() {
        assert!(
            config.last_snapshot_at.is_some(),
            "[coherence] invariant 9 (snapshot-stamp): {} snapshots exist but last_snapshot_at is NULL in {where_}",
            snaps.len()
        );
    }

    // ── #10 referential integrity: tag_ids + folder_id resolve (history orphans handled in #2) ──
    let folder_ids: HashSet<EntryId> = decoded
        .iter()
        .filter(|(_, p)| matches!(p, EntryPayload::Folder(_)))
        .map(|(r, _)| r.id.clone())
        .collect();
    for (row, payload) in &decoded {
        let meta = payload.meta();
        for tid in &meta.tag_ids {
            assert!(
                tag_ids.contains(tid),
                "[coherence] invariant 10 (ref-integrity): entry {} references tag {tid} which has no live row in {where_}",
                row.id
            );
        }
        if let Some(fid) = &meta.folder_id {
            assert!(
                folder_ids.contains(fid),
                "[coherence] invariant 10 (ref-integrity): entry {} references folder {fid} which is not a live Folder entry in {where_}",
                row.id
            );
        }
    }

    // ── #11 every audit_log row parses to a known AuditAction ──
    //    `query_audit` counts ALL rows in `total` but drops unparseable ones from `events`, so an
    //    inequality is an exact "some row failed to parse". (The reversed-range summary math — the
    //    "101–100 of 40" class — is a frontend concern, unit-tested in vedge-app.)
    assert_eq!(
        audit.events.len() as u64,
        audit.total,
        "[coherence] invariant 11 (audit-parse): {} of {} audit_log rows failed to parse in {where_}",
        audit.total.saturating_sub(audit.events.len() as u64),
        audit.total
    );

    // ── #12 commit_counter >= keychain rollback baseline, + the SK is stored (OS state) ──
    if let Some(os) = os {
        os.keychain.read_secret_key(os.vault_uuid).unwrap_or_else(|e| {
            panic!(
                "[coherence] invariant 12b (keychain-sk): no Secret Key stored for uuid {} ({e}) in {where_}",
                os.vault_uuid
            )
        });
        if let Some(baseline) = os.keychain.read_commit_baseline(os.vault_uuid).unwrap() {
            assert!(
                config.commit_counter >= baseline,
                "[coherence] invariant 12 (rollback): commit_counter {} < keychain baseline {baseline} for uuid {} in {where_}",
                config.commit_counter,
                os.vault_uuid
            );
        }
    }

    // ── #13 credential-age stamps sane: created_at <= stamp <= now (NULL falls back to created) ──
    let created = config.created_at;
    let now_ts = now();
    for (label, stamp) in [
        ("last_password_change_at", config.last_password_change_at),
        (
            "last_secret_key_rotation_at",
            config.last_secret_key_rotation_at,
        ),
    ] {
        let s = stamp.unwrap_or(created);
        assert!(
            created <= s && s <= now_ts,
            "[coherence] invariant 13 (cred-age): {label}={s} outside [created_at {created}, now {now_ts}] in {where_}"
        );
    }
}

/// Open a COPY of a snapshot's `vault.vdb` (never the original — `open` runs migrations + pins WAL,
/// which would rewrite its bytes and break `vault_blake3`) and assert every entry decrypts under
/// the current KEK. A snapshot left un-rewrapped after a KEK change fails here.
async fn assert_snapshot_opens_under_kek(
    snap: &SnapshotEntry,
    crypto: &XChaCha20CryptoProvider,
    kek: &[u8; KEK_LEN],
) {
    let tmp = tempfile::tempdir().unwrap();
    let copy = tmp.path().join(VAULT_FILE);
    std::fs::copy(snap.vault_file(), &copy).unwrap();

    let db = VaultDbConnection::open(&copy).await.unwrap_or_else(|e| {
        panic!(
            "[coherence] invariant 8 (snapshot): snapshot {} vault.vdb copy did not open ({e})",
            snap.id()
        )
    });
    let repo = SqliteVaultRepository::new(db.handle());
    let entries = repo.all_entries().await.unwrap();
    for row in &entries {
        let dek = crypto.unwrap_dek(&row.dek_wrapped, kek).unwrap_or_else(|_| {
            panic!(
                "[coherence] invariant 8 (snapshot): snapshot {} entry {} DEK did not unwrap under the CURRENT KEK (snapshot not rewrapped after a KEK change)",
                snap.id(),
                row.id
            )
        });
        let aad = entry_aad(&row.id, row.version).unwrap();
        crypto
            .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)
            .unwrap_or_else(|_| {
                panic!(
                    "[coherence] invariant 8 (snapshot): snapshot {} entry {} did not decrypt under the current KEK",
                    snap.id(),
                    row.id
                )
            });
    }
    drop(repo);
    db.close().await.unwrap();
}

/// Re-derive `(KEK, verify_hash)` a password + the vault's Secret Key would produce — reads the
/// salt/params from the freshly-loaded config, so it is self-contained from disk + creds.
fn derive_kek(
    kdf: &Argon2idKdfProvider,
    config: &VaultConfig,
    sk: &[u8; SECRET_KEY_LEN],
    pw: &str,
) -> ([u8; KEK_LEN], [u8; VERIFY_HASH_LEN]) {
    let input = kdf.preprocess_2skd(pw.as_bytes(), sk).unwrap();
    let mk = kdf
        .derive_master_key(&input, &config.vault_salt, &config.kdf_params)
        .unwrap();
    let verify = kdf.derive_verify_hash(&mk).unwrap();
    let kek = kdf.derive_kek(&mk).unwrap();
    (*kek, verify)
}

/// Re-derive `KEK_rec = HKDF-recovery(Argon2id(2SKD(Recovery Key, Secret Key)))` — the derivation
/// `unlock_with_recovery_key` runs to unwrap the recovery slot.
fn derive_recovery_kek(
    kdf: &Argon2idKdfProvider,
    config: &VaultConfig,
    sk: &[u8; SECRET_KEY_LEN],
    rk: &[u8; RECOVERY_KEY_LEN],
) -> [u8; KEK_LEN] {
    let input = kdf.preprocess_2skd(rk.as_slice(), sk).unwrap();
    let mk = kdf
        .derive_master_key(&input, &config.vault_salt, &config.kdf_params)
        .unwrap();
    *kdf.derive_recovery_kek(&mk).unwrap()
}
