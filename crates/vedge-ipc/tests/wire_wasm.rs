//! Browserless wasm-bindgen tests for the **real** frontend wire codec.
//!
//! Host `serde_json` tests can't catch `serde_wasm_bindgen`-specific bugs
//! (notably `#[serde(flatten)]` + integer coercion, camelCase, tagged enums)
//! because the frontend decodes IPC results with `serde_wasm_bindgen`, not
//! `serde_json`. These tests reproduce the actual path — shell serializes with
//! `serde_json`, the frontend receives a JS object (`JSON.parse`), then decodes
//! with `serde_wasm_bindgen::from_value` — and run under **node** (no browser).
//!
//! Run: `wasm-pack test --node crates/vedge-ipc`
//! (On the host target the whole file is cfg'd out, so `cargo test` is a no-op.)

#![cfg(target_arch = "wasm32")]
#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

use vedge_ipc::*;
use wasm_bindgen_test::wasm_bindgen_test;

/// Reproduce the exact IPC decode: `serde_json` (shell) → `JSON.parse` (the JS
/// value the frontend receives) → `serde_wasm_bindgen::from_value` (the decode
/// that actually runs in the app). Panics (fails the test) if the decode fails.
fn shell_to_frontend<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).unwrap();
    let js = js_sys::JSON::parse(&json).unwrap();
    serde_wasm_bindgen::from_value(js).unwrap()
}

/// The one this suite exists for: `RecentVaultStatusDto` uses `#[serde(flatten)]`,
/// which `serde_wasm_bindgen` is known to mishandle (the flattened `i32`
/// `sort_order` in particular). If this fails, de-flatten the DTO in
/// `vedge-ipc` + update `recent_vault_status_to_dto` in `vedge-tauri`.
#[wasm_bindgen_test]
fn recent_vault_status_flatten_decodes() {
    let dto = RecentVaultStatusDto {
        vault: RecentVaultDto {
            id: "abc-123".into(),
            path: "/home/x/my-vault.vdb".into(),
            display_name: "my-vault".into(),
            last_opened: Some("2026-07-05T12:00:00.000Z".into()),
            sort_order: 7,
        },
        exists: true,
    };
    let back = shell_to_frontend(&dto);
    assert_eq!(back.vault.id, "abc-123");
    assert_eq!(back.vault.path, "/home/x/my-vault.vdb");
    assert_eq!(back.vault.sort_order, 7); // <- the flattened i32
    assert_eq!(
        back.vault.last_opened.as_deref(),
        Some("2026-07-05T12:00:00.000Z")
    );
    assert!(back.exists);
}

/// Also cover `sort_order == 0` and `last_opened: None` — flatten + integer-zero
/// + a `#[serde(default)]` Option are all footgun-prone.
#[wasm_bindgen_test]
fn recent_vault_status_zero_and_none() {
    let dto = RecentVaultStatusDto {
        vault: RecentVaultDto {
            id: "0".into(),
            path: "x.vdb".into(),
            display_name: "x".into(),
            last_opened: None,
            sort_order: 0,
        },
        exists: false,
    };
    let back = shell_to_frontend(&dto);
    assert_eq!(back.vault.sort_order, 0);
    assert!(back.vault.last_opened.is_none());
    assert!(!back.exists);
}

/// `IndexEntryDto` — the read-side projection with an `i32` (`cipher_suite`),
/// several `#[serde(default)]` options, and RFC-3339 timestamps.
#[wasm_bindgen_test]
fn index_entry_decodes() {
    let dto = IndexEntryDto {
        id: "e1".into(),
        name: "GitHub".into(),
        entry_type: EntryTypeDto::Login,
        url: Some("https://github.com".into()),
        favicon_url: None,
        tag_ids: vec!["t1".into(), "t2".into()],
        folder_id: Some("fold-1".into()),
        is_favorite: true,
        color: Some("#4f46e5".into()),
        icon: Some("briefcase".into()),
        is_trashed: false,
        cipher_suite: 1,
        created_at: "2026-07-05T00:00:00.000Z".into(),
        updated_at: "2026-07-05T00:00:00.000Z".into(),
        accessed_at: None,
    };
    let back = shell_to_frontend(&dto);
    assert_eq!(back.id, "e1");
    assert_eq!(back.entry_type, EntryTypeDto::Login);
    assert_eq!(back.cipher_suite, 1);
    assert_eq!(back.tag_ids.len(), 2);
    assert!(back.is_favorite);
    assert_eq!(back.url.as_deref(), Some("https://github.com"));
    assert_eq!(back.color.as_deref(), Some("#4f46e5"));
    assert_eq!(back.icon.as_deref(), Some("briefcase"));
}

/// `EntryTypeDto` — a `#[serde(untagged)]` `Unknown(String)` fallback plus the
/// known unit variants.
#[wasm_bindgen_test]
fn entry_type_known_and_unknown_decode() {
    assert_eq!(shell_to_frontend(&EntryTypeDto::Login), EntryTypeDto::Login);
    assert_eq!(
        shell_to_frontend(&EntryTypeDto::Unknown("Passkey".into())),
        EntryTypeDto::Unknown("Passkey".into())
    );
}

/// `FieldSelectorDto` — internally tagged (`{ "kind": .. , "value": .. }`),
/// mixing unit variants and newtype variants.
#[wasm_bindgen_test]
fn field_selector_tagged_decodes() {
    // Unit variant.
    assert!(matches!(
        shell_to_frontend(&FieldSelectorDto::Password),
        FieldSelectorDto::Password
    ));
    // Newtype variant carrying a String.
    match shell_to_frontend(&FieldSelectorDto::EnvVar("API_URL".into())) {
        FieldSelectorDto::EnvVar(k) => assert_eq!(k, "API_URL"),
        other => panic!("expected EnvVar, got {other:?}"),
    }
}

/// `PayloadDto` — adjacently tagged (`{ "entry_type": "Login", "data": {..} }`)
/// with a nested `CommonMetaDto`. The write-side union the add-entry form sends.
#[wasm_bindgen_test]
fn payload_login_adjacent_decodes() {
    let dto = PayloadDto::Login(LoginPayloadDto {
        meta: CommonMetaDto {
            name: "GitHub".into(),
            entry_type: EntryTypeDto::Login,
            url: Some("https://github.com".into()),
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            notes: None,
            color: None,
            icon: None,
        },
        username: "alice".into(),
        password: "s3cret".into(),
        totp_secret: None,
        recovery_codes: vec![],
    });
    match shell_to_frontend(&dto) {
        PayloadDto::Login(p) => {
            assert_eq!(p.meta.name, "GitHub");
            assert_eq!(p.meta.entry_type, EntryTypeDto::Login);
            assert_eq!(p.username, "alice");
            assert_eq!(p.password, "s3cret");
            assert_eq!(p.meta.url.as_deref(), Some("https://github.com"));
        }
        other => panic!("expected Login, got {other:?}"),
    }
}

/// `ErrorEnvelope` — the `{ kind, message }` shape a rejected `invoke` carries;
/// this is what `ApiError::from_rejection` decodes on the frontend.
#[wasm_bindgen_test]
fn error_envelope_decodes() {
    let env = envelope::ErrorEnvelope {
        kind: envelope::kind::ALREADY_EXISTS.to_string(),
        message: Some("a vault already exists".into()),
    };
    let back = shell_to_frontend(&env);
    assert_eq!(back.kind, "AlreadyExists");
    assert_eq!(back.message.as_deref(), Some("a vault already exists"));
}

/// `CreateVaultInputDto` — the create args, including the `#[serde(default)]`
/// optional `secret_key_b64`, camelCase-free (the shell uses snake_case).
#[wasm_bindgen_test]
fn create_vault_io_decodes() {
    let input = CreateVaultInputDto {
        vault_path: "/x/new.vdb".into(),
        master_password: "hunter2".into(),
        secret_key_b64: None,
    };
    let back = shell_to_frontend(&input);
    assert_eq!(back.vault_path, "/x/new.vdb");
    assert!(back.secret_key_b64.is_none());

    let output = CreateVaultOutputDto {
        secret_key_display: "A3-ABCDE-FGHIJ".into(),
        keychain_stored: true,
    };
    let back = shell_to_frontend(&output);
    assert_eq!(back.secret_key_display, "A3-ABCDE-FGHIJ");
    assert!(back.keychain_stored);
}
