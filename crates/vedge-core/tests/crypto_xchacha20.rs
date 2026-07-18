#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use vedge_core::application::vault::ports::CryptoProvider;
use vedge_core::domain::vault::crypto_constants::{DEK_LEN, DEK_WRAPPED_LEN, KEK_LEN, NONCE_LEN};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::infrastructure::crypto::XChaCha20CryptoProvider;

const fn provider() -> XChaCha20CryptoProvider {
    XChaCha20CryptoProvider::new()
}

const fn dek_a() -> [u8; DEK_LEN] {
    [0x11; DEK_LEN]
}
const fn dek_b() -> [u8; DEK_LEN] {
    [0x22; DEK_LEN]
}
const fn kek_a() -> [u8; KEK_LEN] {
    [0xAA; KEK_LEN]
}
const fn kek_b() -> [u8; KEK_LEN] {
    [0xBB; KEK_LEN]
}

#[test]
fn entry_round_trip_returns_plaintext() {
    let p = provider();
    let payload = b"{\"name\":\"github\",\"password\":\"hunter2\"}";
    let aad = b"entry-aad-1";
    let (nonce, ct) = p.encrypt_entry(&dek_a(), payload, aad).unwrap();
    assert_eq!(nonce.len(), NONCE_LEN);
    let pt = p.decrypt_entry(&dek_a(), &nonce, &ct, aad).unwrap();
    assert_eq!(&*pt, payload);
}

#[test]
fn decrypt_with_wrong_dek_fails() {
    let p = provider();
    let (nonce, ct) = p.encrypt_entry(&dek_a(), b"secret", b"aad").unwrap();
    let err = p.decrypt_entry(&dek_b(), &nonce, &ct, b"aad").unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn decrypt_with_wrong_aad_fails() {
    let p = provider();
    let (nonce, ct) = p.encrypt_entry(&dek_a(), b"secret", b"aad-a").unwrap();
    let err = p
        .decrypt_entry(&dek_a(), &nonce, &ct, b"aad-b")
        .unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn decrypt_with_tampered_ciphertext_fails() {
    let p = provider();
    let (nonce, mut ct) = p.encrypt_entry(&dek_a(), b"secret", b"aad").unwrap();
    ct[0] ^= 0xFF;
    let err = p.decrypt_entry(&dek_a(), &nonce, &ct, b"aad").unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn legacy_tag_decrypts_under_kek() {
    // A pre-5.6.0 tag was sealed directly under the KEK. `decrypt_legacy_tag` reads that
    // shape; `encrypt_entry(kek, …)` reproduces the byte-identical legacy ciphertext (the
    // retired `encrypt_tag` was the same AEAD helper).
    let p = provider();
    let (nonce, ct) = p
        .encrypt_entry(&kek_a(), b"{\"name\":\"aws\"}", b"tag-aad")
        .unwrap();
    let pt = p
        .decrypt_legacy_tag(&kek_a(), &nonce, &ct, b"tag-aad")
        .unwrap();
    assert_eq!(&*pt, b"{\"name\":\"aws\"}");
}

#[test]
fn wrap_unwrap_dek_round_trips() {
    let p = provider();
    let dek = dek_a();
    let kek = kek_a();
    let wrapped = p.wrap_dek(&dek, &kek).unwrap();
    assert_eq!(wrapped.len(), DEK_WRAPPED_LEN);
    let unwrapped = p.unwrap_dek(&wrapped, &kek).unwrap();
    assert_eq!(*unwrapped, dek);
}

#[test]
fn unwrap_dek_with_wrong_kek_fails() {
    let p = provider();
    let wrapped = p.wrap_dek(&dek_a(), &kek_a()).unwrap();
    let err = p.unwrap_dek(&wrapped, &kek_b()).unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn unwrap_dek_with_tampered_blob_fails() {
    let p = provider();
    let mut wrapped = p.wrap_dek(&dek_a(), &kek_a()).unwrap();
    wrapped[5] ^= 0xFF;
    let err = p.unwrap_dek(&wrapped, &kek_a()).unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[test]
fn generate_dek_produces_distinct_keys() {
    let p = provider();
    let a = p.generate_dek();
    let b = p.generate_dek();
    assert_ne!(*a, *b);
}

#[test]
fn generate_nonce_produces_distinct_nonces() {
    let p = provider();
    let a = p.generate_nonce();
    let b = p.generate_nonce();
    assert_ne!(a, b);
}

#[test]
fn verify_hash_matches_equal_and_unequal() {
    let p = provider();
    let a = [7u8; 32];
    let b = [7u8; 32];
    let c = [8u8; 32];
    assert!(p.verify_hash_matches(&a, &b));
    assert!(!p.verify_hash_matches(&a, &c));
}
