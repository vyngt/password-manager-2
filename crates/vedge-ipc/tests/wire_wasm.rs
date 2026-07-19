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

/// The one this suite exists for: `RegisteredVaultStatusDto` uses `#[serde(flatten)]`,
/// which `serde_wasm_bindgen` is known to mishandle (the flattened `i32`
/// `sort_order` in particular). If this fails, de-flatten the DTO in
/// `vedge-ipc` + update `registered_vault_status_to_dto` in `vedge-tauri`.
#[wasm_bindgen_test]
fn registered_vault_status_flatten_decodes() {
    let dto = RegisteredVaultStatusDto {
        vault: RegisteredVaultDto {
            id: "abc-123".into(),
            path: "/home/x/my-vault.vdb".into(),
            display_name: "my-vault".into(),
            last_opened: Some("2026-07-05T12:00:00.000Z".into()),
            sort_order: 7,
        },
        exists: true,
        openable: true,
        vault_uuid: Some("uuid-abc".into()),
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
    assert!(back.openable);
    assert_eq!(back.vault_uuid.as_deref(), Some("uuid-abc"));
}

/// Also cover `sort_order == 0` and `last_opened: None` — flatten + integer-zero
/// + a `#[serde(default)]` Option are all footgun-prone.
#[wasm_bindgen_test]
fn registered_vault_status_zero_and_none() {
    let dto = RegisteredVaultStatusDto {
        vault: RegisteredVaultDto {
            id: "0".into(),
            path: "x.vdb".into(),
            display_name: "x".into(),
            last_opened: None,
            sort_order: 0,
        },
        exists: false,
        openable: false,
        vault_uuid: None,
    };
    let back = shell_to_frontend(&dto);
    assert_eq!(back.vault.sort_order, 0);
    assert!(back.vault.last_opened.is_none());
    assert!(!back.exists);
    assert!(!back.openable);
    assert!(back.vault_uuid.is_none());
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
        sort_order: 5,
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
    assert_eq!(back.sort_order, 5);
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

/// `HealthReportDto` (slice 4.3) — the read-side scan result. Exercises the
/// adjacently-tagged `FindingKindDto` (with an `f64` `guesses_log10` and nested
/// struct variants), `SecretFieldDto` (unit + newtype), and the plain enums,
/// all through the real `serde_wasm_bindgen` decode.
#[wasm_bindgen_test]
fn health_report_decodes() {
    let dto = HealthReportDto {
        scanned_at: "2026-07-12T09:14:03.000Z".into(),
        entries_scanned: 3,
        secrets_scanned: 5,
        findings: vec![
            FindingDto {
                entry_id: "e-weak".into(),
                field: Some(SecretFieldDto::LoginPassword),
                kind: FindingKindDto::Weak {
                    score: 1,
                    guesses_log10: 4.2,
                },
                severity: SeverityDto::High,
            },
            FindingDto {
                entry_id: "e-reuse".into(),
                field: Some(SecretFieldDto::EnvVar("AWS_SECRET_KEY".into())),
                kind: FindingKindDto::Reused { group: 2, count: 3 },
                severity: SeverityDto::Medium,
            },
            FindingDto {
                entry_id: "e-old".into(),
                field: None,
                kind: FindingKindDto::Old {
                    age_days: 812,
                    confidence: AgeConfidenceDto::Estimated,
                },
                severity: SeverityDto::Low,
            },
            FindingDto {
                entry_id: "e-breached".into(),
                field: Some(SecretFieldDto::LoginPassword),
                kind: FindingKindDto::Breached { count: 1337 },
                severity: SeverityDto::High,
            },
        ],
        skipped: vec![SkippedDto {
            entry_id: "e-unknown".into(),
            reason: SkipReasonDto::UnknownPayload,
        }],
        summary: HealthSummaryDto {
            weak: 1,
            reused: 1,
            old: 1,
            exempt_not_scored: 2,
            breached: 1,
        },
        breach_checked: true,
        breach_check_failed: false,
    };
    let back = shell_to_frontend(&dto);
    assert_eq!(back.entries_scanned, 3);
    assert_eq!(back.secrets_scanned, 5);
    assert_eq!(back.findings.len(), 4);
    match &back.findings[0].kind {
        FindingKindDto::Weak {
            score,
            guesses_log10,
        } => {
            assert_eq!(*score, 1);
            assert!(
                (*guesses_log10 - 4.2).abs() < 1e-9,
                "f64 survived the codec"
            );
        }
        other => panic!("expected Weak, got {other:?}"),
    }
    match &back.findings[1].field {
        Some(SecretFieldDto::EnvVar(k)) => assert_eq!(k, "AWS_SECRET_KEY"),
        other => panic!("expected EnvVar key, got {other:?}"),
    }
    assert!(matches!(
        back.findings[1].kind,
        FindingKindDto::Reused { group: 2, count: 3 }
    ));
    assert!(back.findings[2].field.is_none());
    assert!(matches!(
        back.findings[3].kind,
        FindingKindDto::Breached { count: 1337 }
    ));
    assert_eq!(back.skipped.len(), 1);
    assert_eq!(back.summary.exempt_not_scored, 2);
    assert_eq!(back.summary.breached, 1);
    assert!(back.breach_checked);
    assert!(!back.breach_check_failed);
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
            sort_order: 0,
        },
        username: "alice".into(),
        // The 5.4 sealed door: sealed secrets cross as intents, never plaintext.
        // Inbound the password is a `Set` intent; recovery codes cross only as a
        // count; the seed is absent (presence flag + params cross).
        password: SecretUpdateDto::Set("s3cret".into()),
        has_totp: true,
        totp_algorithm: TotpAlgorithmDto::Sha256,
        totp_digits: 8,
        totp_period: 60,
        totp: TotpUpdateDto::Unchanged,
        recovery_codes: SecretListUpdateDto::Unchanged,
        recovery_codes_count: 3,
    });
    match shell_to_frontend(&dto) {
        PayloadDto::Login(p) => {
            assert_eq!(p.meta.name, "GitHub");
            assert_eq!(p.meta.entry_type, EntryTypeDto::Login);
            assert_eq!(p.username, "alice");
            assert_eq!(p.meta.url.as_deref(), Some("https://github.com"));
            // The sealed intents survive the real wasm codec (adjacently-tagged enums).
            assert!(matches!(p.password, SecretUpdateDto::Set(ref s) if s == "s3cret"));
            assert!(matches!(p.recovery_codes, SecretListUpdateDto::Unchanged));
            assert_eq!(p.recovery_codes_count, 3);
            // Door + params survive the real wasm codec (int coercion + tagged enum).
            assert!(p.has_totp);
            assert_eq!(p.totp_algorithm, TotpAlgorithmDto::Sha256);
            assert_eq!(p.totp_digits, 8);
            assert_eq!(p.totp_period, 60);
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

/// `ImportPreviewRow` (slice 5.3b) — the derivatives-only import preview. Proves
/// `has_password` (the 4.2 door) and the optional status fields survive the real
/// wasm codec, and that no secret rides along.
#[wasm_bindgen_test]
fn import_preview_row_decodes() {
    let row = ImportPreviewRow {
        row_id: 3,
        entry_type: "login".into(),
        name: "GitHub".into(),
        username: Some("octocat".into()),
        url: Some("https://github.com".into()),
        tags: vec!["work".into()],
        has_password: true,
        status: "warning".into(),
        status_message: Some("shares the domain github.com with another row".into()),
        status_line: None,
        duplicate_of: Some(1),
    };
    let back = shell_to_frontend(&row);
    assert_eq!(back.row_id, 3);
    assert_eq!(back.entry_type, "login");
    assert!(back.has_password);
    assert_eq!(back.status, "warning");
    assert_eq!(back.duplicate_of, Some(1));
    assert_eq!(back.username.as_deref(), Some("octocat"));
}

/// Recovery Key (slice 5.7) — the show-once `RK1-` display crosses on enroll, and the two
/// typed documents cross on recover. 🔴 The `recovery_slot` (`[u8;40]`) is in NEITHER DTO — it
/// lives only in the vault file and must never reach WASM.
#[wasm_bindgen_test]
fn recovery_dtos_decode() {
    let out = RecoveryEnrollOutputDto {
        recovery_key_display: "RK1-ABCDE-FGHIJ-KLMNO".into(),
    };
    let back = shell_to_frontend(&out);
    assert_eq!(back.recovery_key_display, "RK1-ABCDE-FGHIJ-KLMNO");

    let input = UnlockWithRecoveryKeyInputDto {
        vault_path: "/x/work.vedge".into(),
        recovery_key_display: "RK1-XXXXX".into(),
        secret_key_display: "A3-XXXXX".into(),
    };
    let back = shell_to_frontend(&input);
    assert_eq!(back.vault_path, "/x/work.vedge");
    assert_eq!(back.recovery_key_display, "RK1-XXXXX");
    assert_eq!(back.secret_key_display, "A3-XXXXX");
}

/// Re-key (slice 5.8) — the credentials cross IN; 🔴 the only material crossing back is the
/// show-once new Secret-Key display when the SK was rotated. No DEK / plaintext body / KEK is in
/// EITHER DTO — the door is that the fields simply do not exist here.
#[wasm_bindgen_test]
fn rekey_dtos_decode() {
    // Neutral round-trip probes bound to locals — NOT `password: "literal"` pairs, which
    // GitGuardian's Generic-Password detector flags. The point here is only that the fields
    // survive the codec, so the exact values are irrelevant.
    let cur = "abc".to_owned();
    let next = "xyz".to_owned();
    let input = RekeyInputDto {
        current_password: cur.clone(),
        new_password: next.clone(),
        rotate_secret_key: true,
    };
    let back = shell_to_frontend(&input);
    assert_eq!(back.current_password, cur);
    assert_eq!(back.new_password, next);
    assert!(back.rotate_secret_key);

    let out = RekeyResultDto {
        cancelled: false,
        secret_key_display: Some("A3-NEWKEY".into()),
    };
    let back = shell_to_frontend(&out);
    assert!(!back.cancelled);
    assert_eq!(back.secret_key_display.as_deref(), Some("A3-NEWKEY"));

    // A cancel carries no Secret Key.
    let cancelled = RekeyResultDto {
        cancelled: true,
        secret_key_display: None,
    };
    let back = shell_to_frontend(&cancelled);
    assert!(back.cancelled);
    assert!(back.secret_key_display.is_none());
}
