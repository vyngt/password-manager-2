#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use std::sync::Arc;

use tempfile::tempdir;
use tokio::io::AsyncWriteExt;

use vedge_core::application::vault::ports::{BlobStore, CryptoProvider};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::crypto_constants::DEK_LEN;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::infrastructure::blob::FilesystemBlobStore;
use vedge_core::infrastructure::crypto::XChaCha20CryptoProvider;

const fn dek() -> [u8; DEK_LEN] {
    [0x11; DEK_LEN]
}

fn blob_file(store: &FilesystemBlobStore, id: &EntryId) -> std::path::PathBuf {
    let mut p = store.root().join(id.as_str());
    p.set_extension("blob");
    p
}

fn make_store() -> (tempfile::TempDir, FilesystemBlobStore) {
    let dir = tempdir().unwrap();
    // A vault home with its `blobs/` provisioned (slice 5.2.0 — the store is fail-closed).
    let home = dir.path().join("work.vedge");
    std::fs::create_dir_all(home.join("blobs")).unwrap();
    let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
    let store = FilesystemBlobStore::new(&home, crypto).unwrap();
    (dir, store)
}

#[tokio::test]
async fn write_then_read_round_trips() {
    let (_dir, store) = make_store();
    let id = EntryId::new();
    let payload = b"the rain in spain falls mainly on the plain".repeat(100);
    let nonce = store.write_blob(&id, &dek(), &payload).await.unwrap();
    let read = store.read_blob(&id, &dek(), &nonce).await.unwrap();
    assert_eq!(&*read, &payload);
}

#[tokio::test]
async fn delete_missing_is_ok() {
    let (_dir, store) = make_store();
    store
        .delete_blob(&EntryId::from_raw("01HV0000000000000000000001"))
        .await
        .unwrap();
}

#[tokio::test]
async fn delete_then_read_is_io_error() {
    let (_dir, store) = make_store();
    let id = EntryId::new();
    let nonce = store.write_blob(&id, &dek(), b"data").await.unwrap();
    store.delete_blob(&id).await.unwrap();
    let err = store.read_blob(&id, &dek(), &nonce).await.unwrap_err();
    assert!(matches!(err, VaultError::Storage(_)));
}

#[tokio::test]
async fn list_returns_all_written_ids() {
    let (_dir, store) = make_store();
    let ids: Vec<EntryId> = (0..3).map(|_| EntryId::new()).collect();
    for id in &ids {
        store.write_blob(id, &dek(), b"x").await.unwrap();
    }
    let mut listed: Vec<String> = store
        .list_blobs()
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.as_str().to_owned())
        .collect();
    listed.sort();
    let mut expected: Vec<String> = ids.iter().map(|e| e.as_str().to_owned()).collect();
    expected.sort();
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn reading_with_wrong_entry_id_fails_authentication() {
    let (_dir, store) = make_store();
    let a = EntryId::new();
    let b = EntryId::new();
    let nonce = store.write_blob(&a, &dek(), b"secret").await.unwrap();

    // Copy the blob under b's filename — AAD differs (blob_aad embeds entry
    // ID) so authentication must fail.
    tokio::fs::copy(blob_file(&store, &a), blob_file(&store, &b))
        .await
        .unwrap();
    let err = store.read_blob(&b, &dek(), &nonce).await.unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[tokio::test]
async fn tampering_ciphertext_fails_authentication() {
    let (_dir, store) = make_store();
    let id = EntryId::new();
    let nonce = store
        .write_blob(&id, &dek(), b"a reasonable secret")
        .await
        .unwrap();

    // Flip a byte past the 24-byte nonce header.
    let path = blob_file(&store, &id);
    let mut bytes = tokio::fs::read(&path).await.unwrap();
    bytes[30] ^= 0xFF;
    let mut f = tokio::fs::File::create(&path).await.unwrap();
    f.write_all(&bytes).await.unwrap();
    f.flush().await.unwrap();
    drop(f);

    let err = store.read_blob(&id, &dek(), &nonce).await.unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[tokio::test]
async fn wrong_nonce_claim_is_rejected() {
    let (_dir, store) = make_store();
    let id = EntryId::new();
    let real_nonce = store.write_blob(&id, &dek(), b"x").await.unwrap();
    let mut fake_nonce = real_nonce;
    fake_nonce[0] ^= 0xFF;
    let err = store.read_blob(&id, &dek(), &fake_nonce).await.unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}
