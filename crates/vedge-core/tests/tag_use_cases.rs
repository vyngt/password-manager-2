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
use secrecy::SecretString;

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, UnlockVaultInput, create_entry, create_tag, delete_tag, normalize_tag_name,
    rename_tag,
};
use vedge_core::domain::shared::TagId;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login_with_tags(name: &str, tag_ids: Vec<TagId>) -> EntryPayload {
    let mut meta = CommonMeta::new(name, EntryType::Login);
    meta.tag_ids = tag_ids;
    EntryPayload::Login(LoginPayload {
        meta,
        username: "alice".into(),
        password: SecretString::from("pw"),
        totp_secret: None,
        recovery_codes: vec![],
    })
}

#[test]
fn normalize_lowercases_trims_and_collapses() {
    assert_eq!(normalize_tag_name("GitHub"), "github");
    assert_eq!(normalize_tag_name("  AWS  "), "aws");
    assert_eq!(normalize_tag_name("API  Key"), "api key");
    assert_eq!(normalize_tag_name("api\tkey"), "api key");
}

#[tokio::test]
async fn create_tag_stores_normalized_name() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let id = create_tag(&mut session, "GitHub", None).await.unwrap();
    let meta = session.index().tags.get(&id).unwrap();
    assert_eq!(meta.name, "github");
}

#[tokio::test]
async fn create_tag_is_idempotent_on_normalized_name() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let a = create_tag(&mut session, "GitHub", None).await.unwrap();
    let b = create_tag(&mut session, "github", None).await.unwrap();
    assert_eq!(a, b);
    assert_eq!(session.index().tags.len(), 1);
}

#[tokio::test]
async fn rename_tag_is_o1_on_entries() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let tag_id = create_tag(&mut session, "github", None).await.unwrap();
    let entry_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_with_tags("gh", vec![tag_id.clone()]),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let v_before = h.repo.get_entry(&entry_id).await.unwrap().version;

    rename_tag(&mut session, &tag_id, "git").await.unwrap();

    let v_after = h.repo.get_entry(&entry_id).await.unwrap().version;
    assert_eq!(
        v_before, v_after,
        "entries must not be re-encrypted on rename"
    );
    assert_eq!(session.index().tags.get(&tag_id).unwrap().name, "git");
}

#[tokio::test]
async fn delete_tag_removes_from_referencing_entries() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let tag_id = create_tag(&mut session, "github", None).await.unwrap();
    let e1 = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_with_tags("a", vec![tag_id.clone()]),
        },
    )
    .await
    .unwrap()
    .entry_id;
    let e2 = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_with_tags("b", vec![tag_id.clone()]),
        },
    )
    .await
    .unwrap()
    .entry_id;

    let v1_before = h.repo.get_entry(&e1).await.unwrap().version;

    delete_tag(&mut session, &tag_id).await.unwrap();

    // Tag gone from index + db.
    assert!(!session.index().tags.contains_key(&tag_id));
    let err = h.repo.get_tag(&tag_id).await.unwrap_err();
    assert!(matches!(err, VaultError::TagNotFound(_)));

    // Both entries had their version bumped (re-encrypted).
    let v1_after = h.repo.get_entry(&e1).await.unwrap().version;
    let v2_after = h.repo.get_entry(&e2).await.unwrap().version;
    assert_eq!(v1_after, v1_before + 1);
    assert_eq!(v2_after, v1_before + 1);

    // Index entries no longer carry the tag.
    assert!(
        !session
            .index()
            .entries
            .get(&e1)
            .unwrap()
            .tag_ids
            .contains(&tag_id)
    );
    assert!(
        !session
            .index()
            .entries
            .get(&e2)
            .unwrap()
            .tag_ids
            .contains(&tag_id)
    );
}

#[tokio::test]
async fn delete_nonexistent_tag_errors() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    let err = delete_tag(&mut session, &TagId::new()).await.unwrap_err();
    assert!(matches!(err, VaultError::TagNotFound(_)));
}
