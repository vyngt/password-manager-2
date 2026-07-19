//! Slice 5.9 ① — the automated kill-mid-restore smoke.
//!
//! The journal's crash STATE MACHINE is already proven in-process by
//! `journal.rs`'s crash-matrix `Fixture` (it injects a panic at each checkpoint and runs recovery
//! in the same process). What that can't prove is **fresh-process fsync durability**: that a real
//! hard kill (`TerminateProcess` / SIGKILL) after a journal state becomes durable on disk leaves a
//! home that a brand-new process re-opens consistently.
//!
//! This exercises exactly that, deterministically, using the `VEDGE_RESTORE_PAUSE` seam
//! (`journal::persist`): the post-commit transitions are size-independent microsecond metadata
//! renames, so a kill can never land on one by timing — the seam blocks after each state so the
//! runner can kill precisely there.
//!
//! Shape (the "re-exec self" pattern, so the child reuses the `common::Harness` wiring). The child
//! `kill_harness_child` (ignored, env-gated) builds a throwaway vault + snapshot, prints its HOME to
//! stderr, then reverts — which blocks at `VEDGE_RESTORE_PAUSE`. The runner `kill_mid_restore_smoke`
//! (ignored) spawns that child once per journal state, reads its HOME + the `PAUSED:` marker,
//! hard-kills it, then re-opens the home in THIS process and asserts it still unlocks with its
//! committed data intact. A SIGKILL never runs the child's `TempDir` destructor, so the home
//! survives on disk for the check.
//!
//! Run: `mise kill-smoke` (or `cargo test -p vedge-core --test kill_mid_restore -- --ignored`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;

use zeroize::Zeroizing;

use common::{Harness, build_unlock};
use secrecy::SecretString;
use vedge_core::application::vault::ports::{
    BlobStoreFactory, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepositoryFactory,
};
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, RevertToSnapshotInput, UnlockVault, UnlockVaultInput, create_entry,
    create_snapshot, lock_vault, revert_to_snapshot_in_session,
};
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

const PW: &str = "correct horse battery staple";
/// The `Harness`'s fixed test Secret Key (`common/mod.rs`), so the runner can re-unlock the
/// child's home without a keychain.
const SK: [u8; SECRET_KEY_LEN] = [0x55u8; SECRET_KEY_LEN];
/// Env var the runner sets so the child does its harness work (a no-op otherwise, so a plain
/// `cargo test -- --ignored` doesn't spawn a revert).
const HARNESS_ENV: &str = "VEDGE_KILL_HARNESS";
/// The three durable journal states a revert passes through (mirrors `journal::RestoreState`).
const STATES: [&str; 3] = ["Staged", "HomeAside", "Swapped"];

fn login(name: &str, pw: &str) -> CreateEntryInput {
    CreateEntryInput {
        payload: EntryPayload::Login(LoginPayload {
            meta: CommonMeta::new(name, EntryType::Login),
            username: "user".into(),
            password: SecretString::from(pw),
            totp_secret: None,
            totp_params: vedge_core::TotpParams::default(),
            recovery_codes: vec![],
        }),
    }
}

/// THE CHILD — only runs its body when the runner spawned it (env-gated). Builds a vault, snapshots
/// it, adds a second entry so the live vault is AHEAD of the snapshot, prints `HOME:<path>` to
/// stderr, then reverts. The revert blocks at `VEDGE_RESTORE_PAUSE`; the runner hard-kills it there.
#[tokio::test]
#[ignore = "spawned by kill_mid_restore_smoke; a no-op when run directly"]
async fn kill_harness_child() {
    if std::env::var_os(HARNESS_ENV).is_none() {
        return;
    }

    let h = Harness::fresh().await;
    let unlock = build_unlock(&h);
    let keychain: Arc<dyn KeychainProvider> = Arc::clone(&h.keychain) as _;
    let mut session = unlock
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: Zeroizing::new(PW.to_owned()),
            secret_key: None,
        })
        .await
        .unwrap();

    // Seed a KNOWN entry, snapshot it (so the snapshot carries content), then add a second so the
    // live vault is AHEAD of the snapshot (a real rollback with something to undo). Every consistent
    // recovery — roll-forward to the snapshot (the "first" entry) or roll-back to the ahead state
    // ("first" + "second") — therefore contains at least the "first" entry; a count of 0 would be
    // data loss.
    create_entry(&mut session, login("first", "pw1"))
        .await
        .unwrap();
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    create_entry(&mut session, login("second", "pw2"))
        .await
        .unwrap();

    // 🔴 Drop the Harness's OWN handles on `vault.vdb` before the swap — only the live session may
    // hold the file open when the home is moved aside (the Windows rename constraint; the rekey
    // test does the same). Keep `tempdir` alive so the home survives; the session's own connection
    // is what `close_session_db` releases inside the revert.
    let Harness {
        tempdir,
        home,
        repo,
        blob,
        ..
    } = h;
    drop(repo);
    drop(blob);
    let _tempdir = tempdir;

    // The coordinate the runner needs. stderr (not stdout) so it shares the stream with the seam's
    // `PAUSED:` marker — one pipe, no interleave/deadlock.
    eprintln!("HOME:{}", home.display());

    let factory = SqliteVaultRepositoryFactory::new();
    // Blocks at `VEDGE_RESTORE_PAUSE` inside `journal::persist`; the runner kills us there. If a kill
    // doesn't come (e.g. the state isn't reached), the outcome is logged for diagnosis.
    let outcome = revert_to_snapshot_in_session(
        &session,
        &unlock,
        &factory,
        keychain.as_ref(),
        RevertToSnapshotInput {
            vault: home.clone(),
            snapshot_id: snap.id,
            confirm_rollback: true,
        },
    )
    .await;
    match &outcome {
        Ok(_) => eprintln!("REVERT_DONE:Ok"),
        Err(e) => eprintln!("REVERT_DONE:Err({e:?})"),
    }
}

/// A fresh `UnlockVault` (empty providers) for re-opening the child's home in THIS process — the
/// open runs `recover_if_pending`, so this is the fresh-process recovery under test.
fn fresh_unlock() -> UnlockVault {
    UnlockVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::new(XChaCha20CryptoProvider::new()) as Arc<dyn CryptoProvider>,
        clipboard: Arc::new(MemoryClipboardProvider::new()) as Arc<dyn ClipboardProvider>,
        kdf: Arc::new(Argon2idKdfProvider::new()) as Arc<dyn KeyDerivationProvider>,
        keychain: Arc::new(MemoryKeychainProvider::new()) as Arc<dyn KeychainProvider>,
    }
}

#[tokio::test]
#[ignore = "real process-kill smoke; run via `mise kill-smoke`"]
async fn kill_mid_restore_smoke() {
    let exe = std::env::current_exe().expect("test binary path");

    for state in STATES {
        // Spawn ourselves running only the child test, paused at `state`.
        let mut child = Command::new(&exe)
            .args(["--exact", "kill_harness_child", "--ignored", "--nocapture"])
            .env(HARNESS_ENV, "1")
            .env("VEDGE_RESTORE_PAUSE", state)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn harness child");

        let stderr = child.stderr.take().expect("child stderr");
        let mut home: Option<String> = None;
        let mut paused = false;
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap_or_default();
            if let Some(rest) = line.strip_prefix("HOME:") {
                home = Some(rest.to_owned());
            }
            if line.contains(&format!("PAUSED:{state}")) {
                paused = true;
                break;
            }
        }
        let home = home.unwrap_or_else(|| panic!("child never reported HOME (state {state})"));
        assert!(paused, "child never reached PAUSED:{state}");

        // 🔴 Hard kill — `TerminateProcess` on Windows, SIGKILL elsewhere. No destructors run, so the
        // in-flight journal + the child's TempDir both survive on disk exactly as a crash leaves them.
        child.kill().expect("hard-kill the paused child");
        let _wait = child.wait();

        // Fresh process (this one), fresh providers → open the home → `recover_if_pending` runs.
        // A consistent home unlocks and lands on ONE of the two valid states: rolled forward to the
        // snapshot (1 entry) or rolled back to the ahead state (2 entries). Never torn.
        let session = fresh_unlock()
            .execute(UnlockVaultInput {
                vault_path: home.clone().into(),
                master_password: Zeroizing::new(PW.to_owned()),
                secret_key: Some(Zeroizing::new(SK)),
            })
            .await
            .unwrap_or_else(|e| panic!("home is unopenable after a kill at {state}: {e:?}"));
        // A consistent recovery unlocks (proved above) AND still holds the snapshot's "first"
        // entry — roll-forward → {first}, roll-back → {first, second}. A count of 0 would mean a
        // torn recovery lost committed data.
        let count = session.index().all_active().len();
        assert!(
            count >= 1,
            "home lost its committed data after a kill at {state}: {count} entries"
        );
        lock_vault(session).await.unwrap();

        // Clean up the leaked child home.
        if let Some(parent) = std::path::Path::new(&home).parent() {
            let _rm = std::fs::remove_dir_all(parent);
        }
    }
}
