#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use common::{Harness, build_unlock};
use tokio::io::AsyncWriteExt;

use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    DOCUMENT_SIZE_LIMIT_BYTES, ImportDocumentInput, UnlockVaultInput, export_document,
    hard_delete_entry, import_document,
};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryType};

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn meta(name: &str) -> CommonMeta {
    CommonMeta::new(name, EntryType::Document)
}

#[tokio::test]
async fn round_trip_preserves_bytes() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let payload = (0..10_000u32)
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<u8>>();

    let id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "passport.pdf".into(),
            mime_type: "application/pdf".into(),
            content: payload.clone(),
            meta: meta("passport"),
        },
    )
    .await
    .unwrap();

    let (filename, bytes) = export_document(&session, &id).await.unwrap();
    assert_eq!(filename, "passport.pdf");
    assert_eq!(&*bytes, &payload);

    h.assert_coherent().await;
}

#[tokio::test]
async fn size_cap_rejects_above_limit() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    // Allocate just over the cap — we check the gate before touching disk,
    // so this stays cheap.
    let size = usize::try_from(DOCUMENT_SIZE_LIMIT_BYTES)
        .unwrap()
        .saturating_add(1);
    let payload = vec![0u8; size];
    let err = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "big.bin".into(),
            mime_type: "application/octet-stream".into(),
            content: payload,
            meta: meta("big"),
        },
    )
    .await
    .unwrap_err();
    match err {
        VaultError::DocumentTooLarge { size, limit } => {
            assert_eq!(size, DOCUMENT_SIZE_LIMIT_BYTES + 1);
            assert_eq!(limit, DOCUMENT_SIZE_LIMIT_BYTES);
        }
        other => panic!("expected DocumentTooLarge, got {other:?}"),
    }
}

#[tokio::test]
async fn tampered_blob_fails_decryption() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "doc.txt".into(),
            mime_type: "text/plain".into(),
            content: b"hello world".to_vec(),
            meta: meta("doc"),
        },
    )
    .await
    .unwrap();

    // Flip a byte in the blob file.
    let blob_file = h.blob.root().join(format!("{}.blob", id.as_str()));
    let mut bytes = tokio::fs::read(&blob_file).await.unwrap();
    bytes[30] ^= 0xFF;
    let mut f = tokio::fs::File::create(&blob_file).await.unwrap();
    f.write_all(&bytes).await.unwrap();
    f.flush().await.unwrap();
    drop(f);

    let err = export_document(&session, &id).await.unwrap_err();
    assert!(matches!(err, VaultError::DecryptionFailed));
}

#[tokio::test]
async fn hard_delete_document_removes_blob() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "doc.txt".into(),
            mime_type: "text/plain".into(),
            content: b"hello".to_vec(),
            meta: meta("doc"),
        },
    )
    .await
    .unwrap();

    let blob_file = h.blob.root().join(format!("{}.blob", id.as_str()));
    assert!(blob_file.exists());

    hard_delete_entry(&mut session, &id).await.unwrap();
    assert!(!blob_file.exists());

    // The blob is gone AND leaves no orphan (invariant #4).
    h.assert_coherent().await;
}
