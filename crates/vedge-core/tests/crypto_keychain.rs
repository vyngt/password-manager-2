#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use vedge_core::application::vault::ports::KeychainProvider;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;

// The keychain is keyed on the intrinsic `vault_uuid` (slice 5.2.0), so these are
// plain uuid strings — never file paths.
const fn sample_key() -> [u8; SECRET_KEY_LEN] {
    [0x7Fu8; SECRET_KEY_LEN]
}

#[test]
fn memory_store_read_delete_round_trip() {
    let kc = MemoryKeychainProvider::new();
    let id = "01JVAULTWORK0000000000000";
    let key = sample_key();

    kc.store_secret_key(id, &key).unwrap();
    let got = kc.read_secret_key(id).unwrap();
    assert_eq!(*got, key);

    kc.delete_secret_key(id).unwrap();
    let err = kc.read_secret_key(id).unwrap_err();
    assert!(matches!(err, VaultError::KeychainEntryNotFound));
}

#[test]
fn memory_read_missing_returns_not_found() {
    let kc = MemoryKeychainProvider::new();
    let err = kc.read_secret_key("no-such-uuid").unwrap_err();
    assert!(matches!(err, VaultError::KeychainEntryNotFound));
}

#[test]
fn memory_delete_missing_returns_not_found() {
    let kc = MemoryKeychainProvider::new();
    let err = kc.delete_secret_key("no-such-uuid").unwrap_err();
    assert!(matches!(err, VaultError::KeychainEntryNotFound));
}

#[test]
fn memory_overwrites_on_duplicate_store() {
    let kc = MemoryKeychainProvider::new();
    let id = "01JVAULTWORK0000000000000";
    kc.store_secret_key(id, &[1u8; SECRET_KEY_LEN]).unwrap();
    kc.store_secret_key(id, &[2u8; SECRET_KEY_LEN]).unwrap();
    let got = kc.read_secret_key(id).unwrap();
    assert_eq!(*got, [2u8; SECRET_KEY_LEN]);
}

#[test]
fn memory_multiple_vaults_are_independent() {
    let kc = MemoryKeychainProvider::new();
    let a = "01JVAULTA0000000000000000";
    let b = "01JVAULTB0000000000000000";
    kc.store_secret_key(a, &[1u8; SECRET_KEY_LEN]).unwrap();
    kc.store_secret_key(b, &[2u8; SECRET_KEY_LEN]).unwrap();
    assert_eq!(*kc.read_secret_key(a).unwrap(), [1u8; SECRET_KEY_LEN]);
    assert_eq!(*kc.read_secret_key(b).unwrap(), [2u8; SECRET_KEY_LEN]);
}

// The real OS keychain is tested manually. Running this touches the host's real
// credential store, so it's gated behind --ignored. Test service name is
// namespaced so dev machines don't collide with production vedge vaults.
#[test]
#[ignore = "touches real OS keychain; run with `cargo test -- --ignored`"]
fn os_keychain_round_trip_manual() {
    use vedge_core::infrastructure::keychain::OsKeychainProvider;

    let service = format!("vedge-test-{}", ulid::Ulid::new());
    let kc = OsKeychainProvider::with_service(&service);
    let id = "01JVAULTTEST0000000000000";
    let key = sample_key();

    kc.store_secret_key(id, &key).expect("store");
    let got = kc.read_secret_key(id).expect("read");
    assert_eq!(*got, key);
    kc.delete_secret_key(id).expect("delete");
    assert!(matches!(
        kc.read_secret_key(id).unwrap_err(),
        VaultError::KeychainEntryNotFound
    ));
}
