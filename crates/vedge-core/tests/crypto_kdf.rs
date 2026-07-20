#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::let_underscore_must_use,
    clippy::needless_pass_by_value
)]

use vedge_core::application::vault::ports::KeyDerivationProvider;
use vedge_core::domain::vault::crypto_constants::{MASTER_KEY_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::kdf_params::KdfParams;
use vedge_core::infrastructure::crypto::Argon2idKdfProvider;

const fn provider() -> Argon2idKdfProvider {
    Argon2idKdfProvider::new()
}

const fn password() -> &'static [u8] {
    b"correct horse battery staple"
}

const fn secret_key() -> [u8; SECRET_KEY_LEN] {
    [0u8; SECRET_KEY_LEN]
}

const fn vault_salt() -> [u8; VAULT_SALT_LEN] {
    [0u8; VAULT_SALT_LEN]
}

/// Fast params for tests only — not suitable for production.
/// Any change here also shifts the golden vector below.
fn test_params() -> KdfParams {
    KdfParams {
        alg: "argon2id".into(),
        m: 8,
        t: 1,
        p: 1,
        version: 1,
    }
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[test]
fn preprocess_2skd_is_deterministic() {
    let p = provider();
    let a = p.preprocess_2skd(password(), &secret_key()).unwrap();
    let b = p.preprocess_2skd(password(), &secret_key()).unwrap();
    assert_eq!(*a, *b);
}

#[test]
fn preprocess_2skd_differs_when_password_differs() {
    let p = provider();
    let a = p.preprocess_2skd(b"one", &secret_key()).unwrap();
    let b = p.preprocess_2skd(b"two", &secret_key()).unwrap();
    assert_ne!(*a, *b);
}

#[test]
fn preprocess_2skd_differs_when_secret_key_differs() {
    let p = provider();
    let a = p
        .preprocess_2skd(password(), &[0u8; SECRET_KEY_LEN])
        .unwrap();
    let b = p
        .preprocess_2skd(password(), &[1u8; SECRET_KEY_LEN])
        .unwrap();
    assert_ne!(*a, *b);
}

#[test]
fn derive_master_key_is_deterministic() {
    let p = provider();
    let input = *p.preprocess_2skd(password(), &secret_key()).unwrap();
    let a = p
        .derive_master_key(&input, &vault_salt(), &test_params())
        .unwrap();
    let b = p
        .derive_master_key(&input, &vault_salt(), &test_params())
        .unwrap();
    assert_eq!(*a, *b);
}

#[test]
fn derive_master_key_rejects_unknown_alg() {
    let p = provider();
    let input = [0u8; 32];
    let bad = KdfParams {
        alg: "pbkdf2".into(),
        ..test_params()
    };
    let err = p
        .derive_master_key(&input, &vault_salt(), &bad)
        .unwrap_err();
    assert!(matches!(err, VaultError::KeyDerivationFailed(_)));
}

#[test]
fn derive_kek_verify_and_sync_are_domain_separated() {
    let p = provider();
    let mk = [42u8; MASTER_KEY_LEN];
    let kek = p.derive_kek(&mk).unwrap();
    let vh = p.derive_verify_hash(&mk).unwrap();
    let sa = p.derive_sync_auth(&mk).unwrap();

    assert_ne!(*kek, vh);
    assert_ne!(*kek, *sa);
    assert_ne!(vh, *sa);
}

// Run me first to capture expected bytes:
//   cargo test -p vedge-core --test crypto_kdf -- --ignored reveal_golden --nocapture
#[test]
#[ignore = "reveal-only; prints the current golden vector"]
fn reveal_golden() {
    let p = provider();
    let input = *p.preprocess_2skd(password(), &secret_key()).unwrap();
    let mk = p
        .derive_master_key(&input, &vault_salt(), &test_params())
        .unwrap();
    let vh = p.derive_verify_hash(&mk).unwrap();
    eprintln!("GOLDEN verify_hash = {}", to_hex(&vh));
}

/// Golden vector — tripwire for accidental parameter or info-string drift.
///
/// Inputs:
///   password    = "correct horse battery staple"
///   `secret_key`  = [0; 16]
///   `vault_salt`  = [0; 32]
///   `kdf_params`  = { alg: "argon2id", m: 8, t: 1, p: 1, version: 1 }
///
/// Flow: `preprocess_2skd → derive_master_key → derive_verify_hash`.
/// Captured on 2026-04-18; Argon2id is deterministic, so this reproduces on every
/// platform. If this fails, some parameter, info string, or algorithm has changed —
/// intentionally or accidentally. Re-run `reveal_golden` to refresh after an
/// intentional change.
#[test]
fn golden_vector_verify_hash() {
    const EXPECTED_HEX: &str = "e5b0e4feade8298a7a45180a69f5ffe8fa2e2dae6a48be78a7793d4759b97b3c";

    let p = provider();
    let input = *p.preprocess_2skd(password(), &secret_key()).unwrap();
    let mk = p
        .derive_master_key(&input, &vault_salt(), &test_params())
        .unwrap();
    let vh = p.derive_verify_hash(&mk).unwrap();
    let actual = to_hex(&vh);
    assert!(
        actual == EXPECTED_HEX,
        "golden verify_hash mismatch\n  expected = {EXPECTED_HEX}\n  actual   = {actual}"
    );
}
