//! Operation benchmark (PG.1 ⑦ — Gate 1.0 §G). Times the mutating operations a user waits
//! through, at 100 / 1k / 10k entries, plus the worst case: a re-key of a large-blob vault.
//!
//! 🔴 `rekey_vault` is `O(total plaintext bytes)` — it mints a fresh DEK per entry and re-encrypts
//! EVERY surface (entries, every history version, document blobs, tags). It is the only operation
//! whose cost users will physically wait through, and the big-blob case is its worst case by far.
//!
//! `#[ignore]`d — run explicitly (the big-blob case writes ~384 MB to a temp dir):
//!
//! ```text
//! cargo test -p vedge-core --test operation_bench -- --ignored --nocapture   # or: mise bench-ops
//! ```
//!
//! The seeding path is the REAL write path (5.3b import = `create_entry`; `import_document` for
//! blobs), never a bespoke fixture — the numbers reflect production code. The re-key harness drops
//! the harness's own `vault.vdb` handles before the swap (Windows will not rename an open home),
//! the same discipline the re-key tests use.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value
)]

mod common;

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use secrecy::SecretString;
use zeroize::Zeroizing;

use common::Harness;
use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::unlock_vault::UnlockVaultInput;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, ImportDocumentInput, RekeyVaultInput, UpdateEntryInput, create_entry,
    create_snapshot, import_document, rekey_vault, update_entry,
};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::{ImportAction, ImportSource, RowAction, begin_import, commit_import};

const NEW_PW: &str = "bench-new-password-9";

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn login(name: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "user".into(),
        password: SecretString::from("pw"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn unlock_pw(h: &Harness) -> VaultSession {
    common::build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

/// Seed `n` logins through the real write path (5.3b CSV import = `create_entry`).
async fn seed_logins(session: &mut VaultSession, n: u32) {
    let mut csv = String::from("name,username,password,url,notes,tags\n");
    for i in 0..n {
        writeln!(csv, "entry-{i},user-{i},pw-{i},,,").unwrap();
    }
    begin_import(session, ImportSource::Csv { text: csv }).unwrap();
    let actions: Vec<ImportAction> = (0..n)
        .map(|row_id| ImportAction {
            row_id,
            action: RowAction::Import,
        })
        .collect();
    let report = commit_import(session, &actions).await.unwrap();
    assert_eq!(report.imported, u64::from(n));
}

/// Seed `count` documents of `mb` MiB each (each under the 50 MiB cap).
async fn seed_docs(session: &mut VaultSession, count: usize, mb: usize) {
    let bytes = vec![0x5Au8; mb * 1024 * 1024];
    for i in 0..count {
        import_document(
            session,
            ImportDocumentInput {
                filename: format!("doc-{i}.bin"),
                mime_type: "application/octet-stream".into(),
                content: bytes.clone(),
                meta: CommonMeta::new(format!("doc-{i}"), EntryType::Document),
            },
        )
        .await
        .unwrap();
    }
}

/// Time `create_entry`, `update_entry`, and `create_snapshot` over an `n`-entry vault (none swaps
/// the home, so the harness stays whole). Returns (create µs, update µs, snapshot).
async fn bench_row_ops(n: u32) -> (f64, f64, Duration) {
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h).await;
    seed_logins(&mut session, n).await;

    let t = Instant::now();
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("extra"),
        },
    )
    .await
    .unwrap();
    let create = t.elapsed();

    let id = session.index().entries.keys().next().cloned().unwrap();
    let t = Instant::now();
    update_entry(&mut session, UpdateEntryInput::full(id, login("updated")))
        .await
        .unwrap();
    let update = t.elapsed();

    let t = Instant::now();
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    let snapshot = t.elapsed();

    (
        create.as_micros() as f64,
        update.as_micros() as f64,
        snapshot,
    )
}

/// The re-key harness: seed, capture the ports, drop the harness's OWN `vault.vdb` handles (the
/// swap needs no other open handle), and hand back everything `rekey_vault` needs. `tempdir` is
/// returned so the vault files outlive the call.
struct RekeyLive {
    _tempdir: tempfile::TempDir,
    session: VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
}

async fn build_rekey_live_entries(n: u32) -> RekeyLive {
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h).await;
    seed_logins(&mut session, n).await;
    finish_rekey_live(h, session)
}

async fn build_rekey_live_docs(count: usize, mb: usize) -> RekeyLive {
    let h = Harness::fresh().await;
    let mut session = unlock_pw(&h).await;
    seed_docs(&mut session, count, mb).await;
    finish_rekey_live(h, session)
}

fn finish_rekey_live(h: Harness, session: VaultSession) -> RekeyLive {
    let kdf = Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>;
    let keychain = Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>;
    let biometric = Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>;
    let Harness {
        tempdir,
        repo,
        blob,
        ..
    } = h;
    drop(repo);
    drop(blob);
    RekeyLive {
        _tempdir: tempdir,
        session,
        kdf,
        keychain,
        biometric,
    }
}

/// Time one `rekey_vault` over the built vault.
async fn bench_rekey(live: RekeyLive) -> Duration {
    let cancel = AtomicBool::new(false);
    let noop = |_done: u64, _total: u64| {};
    let t0 = Instant::now();
    rekey_vault(
        &live.session,
        live.kdf,
        live.keychain,
        live.biometric,
        RekeyVaultInput {
            new_password: Zeroizing::new(NEW_PW.to_owned()),
            new_secret_key: None,
        },
        &noop,
        &cancel,
    )
    .await
    .unwrap();
    t0.elapsed()
}

#[tokio::test]
#[ignore = "benchmark — run explicitly via `mise bench-ops` (--ignored --nocapture)"]
async fn operation_scaling_100_1k_10k() {
    println!("\n=== VEdge operation benchmark (PG.1 ⑦ · Gate 1.0 §G) ===");
    println!(
        "{:>8} | {:>13} | {:>13} | {:>15} | {:>13}",
        "entries", "create_entry", "update_entry", "create_snapshot", "rekey_vault"
    );
    println!("{:->9}+{:->15}+{:->15}+{:->17}+{:->15}", "", "", "", "", "");
    for n in [100_u32, 1_000, 10_000] {
        let (create_us, update_us, snap) = bench_row_ops(n).await;
        let rekey = bench_rekey(build_rekey_live_entries(n).await).await;
        println!(
            "{n:>8} | {create_us:>10.1} µs | {update_us:>10.1} µs | {:>12.1} ms | {:>10.1} ms",
            ms(snap),
            ms(rekey)
        );
    }
    println!("=========================================================\n");
}

#[tokio::test]
#[ignore = "benchmark (writes ~384 MB) — run explicitly via `mise bench-ops`"]
async fn rekey_big_blob_vault() {
    const COUNT: usize = 8;
    const MB: usize = 48; // under the 50 MiB document cap
    let total = COUNT * MB;

    let live = build_rekey_live_docs(COUNT, MB).await;
    let rekey = bench_rekey(live).await;

    println!("\n=== VEdge big-blob re-key (PG.1 ⑦ · Gate 1.0 §G) ===");
    println!(
        "re-key of a ~{total} MB vault ({COUNT} × {MB} MiB document blobs): {:.1} ms ({:.1} MB/s)",
        ms(rekey),
        total as f64 / rekey.as_secs_f64()
    );
    println!("===================================================\n");
}
