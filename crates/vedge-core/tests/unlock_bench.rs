//! Unlock benchmark (slice 5.3c, hard gate 2) — measure the O(n) unlock loop at
//! 100 / 1k / 10k entries to decide slice 5.5 (the Encrypted `VaultIndex` blob).
//!
//! `#[ignore]`d so it stays out of the default `mise test`; run it explicitly:
//!
//! ```text
//! cargo test -p vedge-core --test unlock_bench -- --ignored --nocapture   # or: mise bench
//! ```
//!
//! It prints a table; the numbers are recorded in the 5.3 tracking retro and drive
//! the 5.5 go/defer call (a deferral without numbers is a guess wearing a decision's
//! clothes).
//!
//! **Method.** 5.3b's import IS the harness — entries are seeded through the REAL
//! write path (`begin_import` → `commit_import`, i.e. `create_entry`), not a bespoke
//! fixture generator. The timed re-open is `unlock_with_kek`, which takes the
//! pre-derived KEK and **skips Argon2id entirely**, so the measurement isolates the
//! per-entry decrypt loop (AES-KW unwrap + `XChaCha20` open + JSON parse) the `VaultIndex`
//! blob would eliminate — the exact cost 5.5 targets. It also prints the fixed 256 MiB
//! Argon2id-default cost the loop competes against, so the decision rests on a real
//! comparison rather than the harness's 8 KiB test KDF.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss
)]

mod common;

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use common::Harness;
use vedge_core::application::vault::ports::KeyDerivationProvider;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::unlock_vault::UnlockVaultInput;
use vedge_core::domain::vault::crypto_constants::VAULT_SALT_LEN;
use vedge_core::domain::vault::kdf_params::KdfParams;
use vedge_core::{ImportAction, ImportSource, RowAction, begin_import, commit_import};

/// Password-unlock the (initially empty) vault to get a session for seeding.
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

/// Seed `n` synthetic logins through the REAL write path — the 5.3b import
/// (`begin_import` CSV → `commit_import` = `create_entry`). No URL, so the advisory
/// dedup is a no-op (it is not what we are measuring).
async fn seed_via_import(session: &mut VaultSession, n: u32) {
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
    assert_eq!(report.imported, u64::from(n), "every seeded row imported");
    assert!(report.failed.is_empty(), "no import failures while seeding");
}

/// Seed `n` entries through the real write path, then time the keyless re-open (which
/// skips Argon2id) — i.e. the O(n) index-build loop.
async fn bench_one(n: u32) -> Duration {
    let h = Harness::fresh().await;
    {
        let mut session = unlock_pw(&h).await;
        seed_via_import(&mut session, n).await;
        // Drop the seeding session; its committed rows are visible to the next
        // connection via WAL (build_unlock opens a fresh one).
        drop(session);
    }
    let unlock = common::build_unlock(&h);
    let kek = Zeroizing::new(h.kek);
    let t0 = Instant::now();
    let _session = unlock.unlock_with_kek(h.home.clone(), kek).await.unwrap();
    t0.elapsed()
}

/// The fixed KDF cost the O(n) loop competes against: one 256 MiB Argon2id-default
/// derivation on this machine (the harness itself uses 8 KiB to stay fast).
fn argon2id_default_cost(h: &Harness) -> Duration {
    let params = KdfParams::argon2id_default();
    let salt = [0x22u8; VAULT_SALT_LEN];
    let input = h
        .kdf
        .preprocess_2skd(h.master_password.as_bytes(), &h.secret_key)
        .unwrap();
    let t0 = Instant::now();
    let _mk = h.kdf.derive_master_key(&input, &salt, &params).unwrap();
    t0.elapsed()
}

#[tokio::test]
#[ignore = "benchmark — run explicitly via `mise bench` (--ignored --nocapture)"]
async fn unlock_scaling_100_1k_10k() {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;

    // The fixed KDF baseline, measured once on this machine.
    let h0 = Harness::fresh().await;
    let kdf = argon2id_default_cost(&h0);
    drop(h0);

    println!("\n=== VEdge unlock benchmark (slice 5.3c · hard gate 2) ===");
    println!(
        "Fixed KDF baseline (256 MiB Argon2id-default, one derivation): {:.1} ms",
        ms(kdf)
    );
    println!(
        "{:>8} | {:>14} | {:>14}",
        "entries", "unlock loop", "per entry"
    );
    println!("{:->9}+{:->16}+{:->16}", "", "", "");
    for n in [100_u32, 1_000, 10_000] {
        let elapsed = bench_one(n).await;
        let per_us = elapsed.as_nanos() as f64 / f64::from(n) / 1000.0;
        println!("{n:>8} | {:>11.1} ms | {per_us:>10.2} µs", ms(elapsed));
    }
    println!("=========================================================\n");
}
