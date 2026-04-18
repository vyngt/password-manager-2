//! End-to-end contract test: password + Secret Key → Argon2id → KEK → wrap DEK →
//! encrypt entry → unwrap DEK → decrypt entry. Exercises the three crypto ports
//! together without any database I/O. This is the contract the future `UnlockVault`
//! use case will consume.

use vedge_core::application::vault::ports::{CryptoProvider, KeyDerivationProvider};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::aad::entry_aad;
use vedge_core::domain::vault::crypto_constants::{SECRET_KEY_LEN, VAULT_SALT_LEN};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::kdf_params::KdfParams;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};

fn fast_params() -> KdfParams {
    // Low-cost Argon2id params so the test doesn't pay the 500ms production cost.
    // Production code path uses `KdfParams::argon2id_default()`.
    KdfParams {
        alg: "argon2id".into(),
        m: 8,
        t: 1,
        p: 1,
        version: 1,
    }
}

#[test]
fn full_unlock_flow_encrypt_then_decrypt_round_trips() {
    let kdf = Argon2idKdfProvider::new();
    let crypto = XChaCha20CryptoProvider::new();

    // --- pretend these came from user input + OS keychain + .vdb row --------
    let master_password = b"correct horse battery staple";
    let secret_key = [0u8; SECRET_KEY_LEN];
    let vault_salt = [0u8; VAULT_SALT_LEN];

    // --- 2SKD + Argon2id + HKDF ------------------------------------------
    let input = *kdf.preprocess_2skd(master_password, &secret_key);
    let master_key = kdf.derive_master_key(&input, &vault_salt, &fast_params()).unwrap();
    let kek = kdf.derive_kek(&master_key);
    let verify_hash = kdf.derive_verify_hash(&master_key);
    assert!(crypto.verify_hash_matches(&verify_hash, &verify_hash));

    // --- encrypt an entry --------------------------------------------------
    let dek = crypto.generate_dek();
    let dek_wrapped = crypto.wrap_dek(&dek, &kek).unwrap();

    let entry_id = EntryId::new();
    let version = 1i64;
    let aad = entry_aad(&entry_id, version).unwrap();
    let payload = br#"{"name":"github","password":"hunter2"}"#;
    let (nonce, ciphertext) = crypto.encrypt_entry(&dek, payload, &aad).unwrap();

    // --- simulate unlock: unwrap DEK using KEK from keychain path ----------
    let dek_read = crypto.unwrap_dek(&dek_wrapped, &kek).unwrap();
    let plaintext = crypto.decrypt_entry(&dek_read, &nonce, &ciphertext, &aad).unwrap();

    assert_eq!(&*plaintext, payload);
}

#[test]
fn wrong_password_produces_different_verify_hash() {
    let kdf = Argon2idKdfProvider::new();
    let crypto = XChaCha20CryptoProvider::new();

    let secret_key = [0u8; SECRET_KEY_LEN];
    let vault_salt = [0u8; VAULT_SALT_LEN];

    // Correct path
    let input_ok = *kdf.preprocess_2skd(b"correct password", &secret_key);
    let mk_ok = kdf.derive_master_key(&input_ok, &vault_salt, &fast_params()).unwrap();
    let vh_ok = kdf.derive_verify_hash(&mk_ok);

    // Wrong password
    let input_bad = *kdf.preprocess_2skd(b"WRONG password", &secret_key);
    let mk_bad = kdf.derive_master_key(&input_bad, &vault_salt, &fast_params()).unwrap();
    let vh_bad = kdf.derive_verify_hash(&mk_bad);

    // Unlock flow fails fast here — no decryption attempted.
    assert!(!crypto.verify_hash_matches(&vh_bad, &vh_ok));
}

#[test]
fn aad_substitution_fails_authentication() {
    let kdf = Argon2idKdfProvider::new();
    let crypto = XChaCha20CryptoProvider::new();

    let input = *kdf.preprocess_2skd(b"pw", &[0u8; SECRET_KEY_LEN]);
    let mk = kdf.derive_master_key(&input, &[0u8; VAULT_SALT_LEN], &fast_params()).unwrap();
    let kek = kdf.derive_kek(&mk);

    let dek = crypto.generate_dek();
    let wrapped = crypto.wrap_dek(&dek, &kek).unwrap();

    // Encrypt under entry A's AAD.
    let id_a = EntryId::new();
    let id_b = EntryId::new();
    let aad_a = entry_aad(&id_a, 1).unwrap();
    let aad_b = entry_aad(&id_b, 1).unwrap();
    assert_ne!(aad_a, aad_b);

    let (nonce, ct) = crypto.encrypt_entry(&dek, b"secret", &aad_a).unwrap();

    // Attacker copies ct into entry B's row — decrypt with B's AAD must fail.
    let dek_read = crypto.unwrap_dek(&wrapped, &kek).unwrap();
    let err = crypto
        .decrypt_entry(&dek_read, &nonce, &ct, &aad_b)
        .unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn version_bump_invalidates_old_ciphertext() {
    let crypto = XChaCha20CryptoProvider::new();
    let dek = crypto.generate_dek();
    let id = EntryId::new();
    let aad_v1 = entry_aad(&id, 1).unwrap();
    let aad_v2 = entry_aad(&id, 2).unwrap();

    let (nonce, ct) = crypto.encrypt_entry(&dek, b"v1", &aad_v1).unwrap();
    let err = crypto.decrypt_entry(&dek, &nonce, &ct, &aad_v2).unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}
