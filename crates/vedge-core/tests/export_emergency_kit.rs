#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::sync::Arc;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::{KeychainProvider, VaultRepository};
use vedge_core::application::vault::use_cases::{
    ExportEmergencyKitInput, UnlockVaultInput, export_emergency_kit,
};
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::secret_key::parse_secret_key;

#[tokio::test]
async fn export_round_trips_through_recovery_format() {
    let h = Harness::fresh().await;

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();

    let content = export_emergency_kit(
        &session,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ExportEmergencyKitInput {
            vault_display_name: Some("work vault".into()),
        },
    )
    .await
    .unwrap();

    assert_eq!(content.vault_name, "work vault");
    assert!(
        content.vault_path.ends_with("work.vedge"),
        "got {}",
        content.vault_path
    );
    assert!(content.secret_key_display.starts_with("A3-"));
    assert!(content.kdf_params_summary.contains("argon2id"));

    let parsed = parse_secret_key(&content.secret_key_display).unwrap();
    assert_eq!(*parsed, h.secret_key);
}

#[tokio::test]
async fn export_propagates_keychain_entry_not_found() {
    let h = Harness::fresh().await;

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();

    // Delete the secret key after unlock; export must not silently fabricate
    // one from session state.
    h.keychain.delete_secret_key(&h.vault_uuid).unwrap();

    let err = export_emergency_kit(
        &session,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ExportEmergencyKitInput {
            vault_display_name: None,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, VaultError::KeychainEntryNotFound));
}

#[tokio::test]
async fn export_appends_audit_event() {
    let h = Harness::fresh().await;

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();

    export_emergency_kit(
        &session,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ExportEmergencyKitInput {
            vault_display_name: None,
        },
    )
    .await
    .unwrap();

    // Recent audit has two entries: the Unlocked event from UnlockVault and
    // the Exported event we just appended. Confirm the Exported event is
    // present with no entry_id (vault-level).
    let recent = h.repo.recent_audit(5).await.unwrap();
    let exported = recent
        .iter()
        .find(|e| matches!(e.action, AuditAction::Exported))
        .expect("export audit event present");
    assert!(
        exported.entry_id.is_none(),
        "vault-level export must have no entry_id"
    );
}
