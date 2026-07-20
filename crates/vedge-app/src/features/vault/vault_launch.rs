//! Returning-user launch screen (`/`) — the vault picker (2.8.1). A two-pane
//! surface: a searchable, recency-sorted, keyboard-navigable list of the
//! user's vaults ([`VaultList`]) on the left, and a focused unlock panel
//! ([`VaultUnlockPanel`]) on the right (master password + 2.8 biometric).
//! Unlocking sets the `ActiveVault` context and lands at `/v/vault`.
//! Single-active-vault model — the Lock control in the `/v` shell returns here.
//!
//! This component is the orchestrator: it owns the registry state + all the
//! signals/closures, deriving the visible list with the pure
//! [`filter_sort_registry`] and threading callbacks to the two panes. Errors
//! surface as `Danger` toasts.

use crate::api;
use crate::api::dialog::OpenDialogOptions;
use crate::api::error::ApiError;
use crate::features::vault::backup_open_dialog::BackupOpenDialog;
use crate::features::vault::context::{ActiveVault, NewSecretKit};
use crate::features::vault::recovery_reset_dialog::RecoveryResetDialog;
use crate::features::vault::registry_filter::filter_sort_registry;
use crate::features::vault::secret_display::SecretDisplay;
use crate::features::vault::vault_list::VaultList;
use crate::features::vault::vault_manage_dialogs::{DeleteVaultDialog, VaultDetailsDialog};
use crate::features::vault::vault_unlock_panel::VaultUnlockPanel;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::time::Duration;
use uuid::Uuid;
use vedge_ipc::{
    RegisteredVaultDto, RegisteredVaultStatusDto, UnlockVaultInputDto,
    UnlockWithRecoveryKeyInputDto, UnlockWithSecretKeyInputDto, VaultDetailsDto,
};
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, EmptyState, Spinner};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};
use wasm_bindgen::JsCast;

/// A chosen unlock target: a vault path + display name, plus the registry `id`
/// when it came from the registry list (`None` for a file picked via "Open…").
#[derive(Clone)]
pub struct Selected {
    pub path: String,
    pub id: Option<String>,
    pub display_name: String,
}

/// Derive a human display name from a vault path: the last path segment without its
/// `.vedge` extension.
pub fn display_name_from_path(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".vedge").unwrap_or(name).to_owned()
}

/// Focus the master-password field after a vault is selected. A no-op if the
/// biometric button is shown instead, or the panel isn't mounted yet.
fn focus_password() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(el) = doc.get_element_by_id("master-password")
        && let Some(html) = el.dyn_ref::<web_sys::HtmlElement>()
    {
        let _ = html.focus();
    }
}

/// Record a just-unlocked vault in registry (touch an existing entry, or add a
/// freshly-picked one). Shared by the password and biometric unlock paths. A
/// plain async fn — no reactive owner, so safe to `.await` inside `spawn_local`.
async fn record_unlock(sel: &Selected) {
    if let Some(id) = &sel.id {
        let _ = api::registry::touch_registered_vault_on_unlock(id).await;
    } else {
        let dto = RegisteredVaultDto {
            id: Uuid::new_v4().to_string(),
            path: sel.path.clone(),
            display_name: sel.display_name.clone(),
            last_opened: None,
            sort_order: 0,
        };
        let _ = api::registry::register_vault(&dto).await;
    }
}

#[component]
pub fn VaultLaunch() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    // The show-once new Secret Key handed off by a re-key that rotated it (5.8): re-key LOCKS and
    // sends the user here, so this is where it's safe to show (no auto-lock to eject it).
    let new_kit = expect_context::<NewSecretKit>();
    let on_new_kit_close = Callback::new(move |()| new_kit.display.set(None));
    let toast = use_toast();

    let registry = RwSignal::new(Vec::<RegisteredVaultStatusDto>::new());
    let loading = RwSignal::new(true);
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new(Option::<Selected>::None);
    let pw = RwSignal::new(String::new());
    let unlocking = RwSignal::new(false);

    // Secret-Key unlock (5.1): the "Use Emergency Kit" panel state. `secret_key_open` is
    // toggled by the always-visible CTA and force-opened on a keychain error;
    // `secret_key_input` holds the `A3-…` Secret Key (passed through verbatim).
    let secret_key_open = RwSignal::new(false);
    let secret_key_input = RwSignal::new(String::new());

    // Recovery Key (5.7): the forgot-password flow. `recovery_open` reveals the two-document
    // panel; the two inputs hold the `RK1-` + `A3-` documents. A successful recovery unlock
    // opens the forced set-new-password dialog (`recovery_reset_open`) before entering the
    // vault; `recovery_target` carries the vault through that second step.
    let recovery_open = RwSignal::new(false);
    let recovery_key_input = RwSignal::new(String::new());
    let recovery_secret_key_input = RwSignal::new(String::new());
    let recovery_reset_open = RwSignal::new(false);
    let recovery_new_password = RwSignal::new(String::new());
    let recovery_confirm_password = RwSignal::new(String::new());
    let recovery_error = RwSignal::new(Option::<String>::None);
    let recovery_target = RwSignal::new(Option::<Selected>::None);

    // Biometric: `bio_available` is device-wide (checked once); `bio_enrolled`
    // is per-selected-vault. When enrolled, the Hello button shows by default;
    // `show_password` reveals the password fallback on demand.
    let bio_available = RwSignal::new(false);
    let bio_enrolled = RwSignal::new(false);
    let show_password = RwSignal::new(true);

    // 5.2.4 launch-screen management (Decision ⑥): the `⋯` details dialog + the type-to-confirm
    // delete dialog. `details_stats` is loaded best-effort after the details dialog opens.
    let details_target = RwSignal::new(Option::<RegisteredVaultStatusDto>::None);
    let details_stats = RwSignal::new(Option::<VaultDetailsDto>::None);
    let delete_target = RwSignal::new(Option::<RegisteredVaultStatusDto>::None);
    let delete_typed = RwSignal::new(String::new());

    // The visible list: pure filter + recency/missing-last sort over registry.
    let filtered = Signal::derive(move || filter_sort_registry(&registry.get(), &query.get()));

    // Danger-toast helper. Called from event handlers *and* `spawn_local`
    // futures, so it reads its dismiss label via `untrack` (owner-less async).
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, unlock.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    // Success/Warning siblings of `show_error`, added for the recovery flow
    // (5.1): a full keychain restore is a Success, a partial one a Warning.
    // Same owner-less-async discipline — read the dismiss label via `untrack`.
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, unlock.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };
    let show_warning = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, unlock.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Warning)
                .dismiss_label(dismiss),
        );
    };

    let refresh_registry = move || {
        loading.set(true);
        // Called from an Effect *and* from inside `spawn_local`; `untrack`
        // reads the current locale string safely in both.
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_vaults).to_owned());
        spawn_local(async move {
            match api::registry::list_registered_vaults_with_status().await {
                Ok(list) => registry.set(list),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        refresh_registry();
    });

    // Biometric availability once on mount (device-wide, vault-independent).
    Effect::new(move |_| {
        spawn_local(async move {
            let avail = api::biometric::available().await.unwrap_or(false);
            bio_available.set(avail);
        });
    });

    // Re-check enrollment whenever the selection (or availability) changes.
    // When enrolled, default to the biometric button (hide the password row).
    Effect::new(move |_| {
        let sel = selected.get();
        let avail = bio_available.get();
        match sel {
            Some(sel) if avail => {
                spawn_local(async move {
                    let enrolled = api::biometric::is_enrolled(&sel.path)
                        .await
                        .unwrap_or(false);
                    bio_enrolled.set(enrolled);
                    show_password.set(!enrolled);
                });
            }
            _ => {
                bio_enrolled.set(false);
                show_password.set(true);
            }
        }
    });

    let on_select = Callback::new(move |sel: Selected| {
        selected.set(Some(sel));
        set_timeout(focus_password, Duration::from_millis(30));
    });

    let on_remove = Callback::new(move |id: String| {
        // Read `selected` in the handler body (owner-less inside `spawn_local`).
        let clear = selected.get().and_then(|s| s.id).as_deref() == Some(id.as_str());
        spawn_local(async move {
            let _ = api::registry::deregister_vault(&id).await;
            if clear {
                selected.set(None);
            }
            refresh_registry();
        });
    });

    // Re-point a missing vault: pick the moved file, re-add it (preserving the
    // display name → floats to top with a fresh `last_opened`), drop the stale
    // row. Reuses add + remove; no path-update command needed.
    let on_locate = Callback::new(move |id: String| {
        let existing_name = registry
            .get_untracked()
            .into_iter()
            .find(|r| r.vault.id == id)
            .map(|r| r.vault.display_name);
        let dialog_title = t_string!(i18n, unlock.open_file).to_owned();
        let err_prefix = t_string!(i18n, unlock.err_open).to_owned();
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(dialog_title),
                filters: vec![],
                // A vault is a `.vedge/` folder (slice 5.2.0) → pick a directory.
                directory: true,
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    let display_name =
                        existing_name.unwrap_or_else(|| display_name_from_path(&path));
                    let dto = RegisteredVaultDto {
                        id: Uuid::new_v4().to_string(),
                        path,
                        display_name,
                        last_opened: None,
                        sort_order: 0,
                    };
                    // Re-point only if the chosen file is a real vault; the
                    // stale row stays put (with an error) otherwise.
                    match api::registry::register_vault(&dto).await {
                        Ok(()) => {
                            let _ = api::registry::deregister_vault(&id).await;
                            refresh_registry();
                        }
                        Err(e) => show_error(format!("{err_prefix}{e}")),
                    }
                }
                Ok(None) => {}
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    let on_rename_commit = Callback::new(move |(id, name): (String, String)| {
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_rename).to_owned());
        spawn_local(async move {
            match api::registry::rename_registered_vault(&id, &name).await {
                Ok(()) => refresh_registry(),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // ⋯ → the vault-details dialog (5.2.4). Snapshot the row, then load best-effort stats.
    let on_menu = Callback::new(move |id: String| {
        let Some(row) = registry
            .get_untracked()
            .into_iter()
            .find(|r| r.vault.id == id)
        else {
            return;
        };
        let path = row.vault.path.clone();
        details_stats.set(None);
        details_target.set(Some(row));
        spawn_local(async move {
            if let Ok(d) = api::vault::details(&path).await {
                // Ignore a stale load if the user has since opened a different vault's details.
                if details_target
                    .get_untracked()
                    .is_some_and(|r| r.vault.id == id)
                {
                    details_stats.set(Some(d));
                }
            }
        });
    });

    // Delete a vault (from the confirm dialog): files + three credentials + registry row. The
    // launch-screen target is always locked, so no lock step. Refresh + toast; a
    // `credentials_cleaned == false` is a Warning naming `mise keychain-audit`.
    let on_delete = Callback::new(move |row: RegisteredVaultStatusDto| {
        let id = row.vault.id.clone();
        let path = row.vault.path;
        let clear = selected.get().and_then(|s| s.id).as_deref() == Some(id.as_str());
        let ok_msg = untrack(|| t_string!(i18n, unlock.delete_done).to_owned());
        let warn_msg = untrack(|| t_string!(i18n, unlock.delete_creds_left).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_open).to_owned());
        spawn_local(async move {
            match api::vault::delete(&path, Some(&id)).await {
                Ok(report) => {
                    delete_target.set(None);
                    delete_typed.set(String::new());
                    details_target.set(None);
                    if clear {
                        selected.set(None);
                    }
                    refresh_registry();
                    if report.credentials_cleaned {
                        show_success(ok_msg);
                    } else {
                        show_warning(warn_msg);
                    }
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // "Delete vault…" inside the details dialog → open the type-to-confirm dialog (close details).
    let on_delete_request = Callback::new(move |row: RegisteredVaultStatusDto| {
        details_target.set(None);
        delete_typed.set(String::new());
        delete_target.set(Some(row));
    });

    // H0 disaster path: a present-but-corrupt vault (`exists && !openable`) can't be unlocked,
    // so restore it from its NEWEST snapshot. The vault is locked here (launch screen), so the
    // revert runs directly; on success the row becomes openable and the user unlocks normally.
    let on_restore = Callback::new(move |id: String| {
        let Some(row) = registry
            .get_untracked()
            .into_iter()
            .find(|r| r.vault.id == id)
        else {
            return;
        };
        let path = row.vault.path;
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_restore).to_owned());
        let none_msg = untrack(|| t_string!(i18n, unlock.restore_none).to_owned());
        let done = untrack(|| t_string!(i18n, unlock.restore_done).to_owned());
        spawn_local(async move {
            let snaps = match api::snapshot::list(&path).await {
                Ok(s) => s,
                Err(e) => {
                    show_error(format!("{err_prefix}{e}"));
                    return;
                }
            };
            let Some(newest) = snaps.first() else {
                show_error(none_msg);
                return;
            };
            match api::snapshot::revert(&path, &newest.id, true).await {
                Ok(_) => {
                    show_success(done);
                    refresh_registry();
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    let do_unlock = move || {
        let Some(sel) = selected.get() else {
            return;
        };
        let password = pw.get();
        if password.is_empty() || unlocking.get() {
            return;
        }
        unlocking.set(true);
        let nav = use_navigate();
        // Read locale-dependent strings in the handler (owner present); reading
        // them inside `spawn_local` trips the reactive-context warning.
        let msg_wrong = t_string!(i18n, unlock.wrong_password).to_owned();
        let msg_keychain = t_string!(i18n, unlock.keychain_missing).to_owned();
        let msg_failed = t_string!(i18n, unlock.unlock_failed).to_owned();
        let msg_rollback = t_string!(i18n, unlock.rollback_warning).to_owned();
        spawn_local(async move {
            let input = UnlockVaultInputDto {
                vault_path: sel.path.clone(),
                master_password: password,
                secret_key_b64: None,
            };
            match api::vault::unlock(&input).await {
                Ok(result) => {
                    // Advisory rollback warning (5.2c): the vault was behind this
                    // device's last-seen state. Still unlocks; surface a warning.
                    if result.rollback_detected {
                        show_warning(msg_rollback);
                    }
                    record_unlock(&sel).await;
                    active.path.set(Some(sel.path.clone()));
                    nav("/v/vault", Default::default());
                }
                Err(ApiError::WrongCredentials) => show_error(msg_wrong),
                Err(ApiError::Keychain(_)) => {
                    // The keychain entry is gone — this is the recovery dead-end.
                    // Toast the pointer *and* open the Emergency Kit panel so the
                    // user lands directly on where to type their Secret Key.
                    show_error(msg_keychain);
                    secret_key_open.set(true);
                }
                Err(e) => show_error(format!("{msg_failed}{e}")),
            }
            unlocking.set(false);
        });
    };

    // Recover via the Emergency Kit: master password + the `A3-…` Secret Key.
    // Sibling of `do_unlock`; on success the shell inserts the session itself
    // (same as a normal unlock) and reports whether the keychain was restored.
    let do_secret_key_unlock = move || {
        let Some(sel) = selected.get() else {
            return;
        };
        let password = pw.get();
        // Trim ONLY — `parse_secret_key` (core) owns normalization (case, 0/O,
        // 1/I/L) and the checksum. No UI-side re-validation.
        let key = secret_key_input.get().trim().to_owned();
        if password.is_empty() || key.is_empty() || unlocking.get() {
            return;
        }
        unlocking.set(true);
        let nav = use_navigate();
        // Hoist EVERY locale string here (owner present); reading inside
        // `spawn_local` trips the reactive-context warning (console must stay clean).
        let msg_invalid = t_string!(i18n, unlock.secret_key_invalid).to_owned();
        let msg_wrong = t_string!(i18n, unlock.wrong_password).to_owned();
        let msg_failed = t_string!(i18n, unlock.unlock_failed).to_owned();
        let msg_partial = t_string!(i18n, unlock.secret_key_partial).to_owned();
        let msg_restored = t_string!(i18n, unlock.secret_key_restored).to_owned();
        spawn_local(async move {
            let input = UnlockWithSecretKeyInputDto {
                vault_path: sel.path.clone(),
                master_password: password,
                secret_key_display: key,
            };
            match api::secret_key::unlock_with_secret_key(&input).await {
                Ok(outcome) => {
                    if outcome.keychain_restored {
                        show_success(msg_restored);
                    } else {
                        show_warning(format!(
                            "{msg_partial}{}",
                            outcome.keychain_error.unwrap_or_default()
                        ));
                    }
                    pw.set(String::new());
                    secret_key_input.set(String::new());
                    record_unlock(&sel).await;
                    active.path.set(Some(sel.path.clone()));
                    nav("/v/vault", Default::default());
                }
                // Checksum/format failure — the parser never echoes the input.
                Err(ApiError::Invalid(_)) => show_error(msg_invalid),
                Err(ApiError::WrongCredentials) => show_error(msg_wrong),
                Err(e) => show_error(format!("{msg_failed}{e}")),
            }
            unlocking.set(false);
        });
    };

    // Recover a forgotten password (5.7): the `RK1-` Recovery Key + the `A3-` Secret Key,
    // no master password. On success the session is unlocked but pending — open the forced
    // set-new-password dialog (we enter the vault only after it completes).
    let do_recovery_unlock = move || {
        let Some(sel) = selected.get() else {
            return;
        };
        // Trim ONLY — the core parsers own normalization + checksum for both documents.
        let rk = recovery_key_input.get().trim().to_owned();
        let sk = recovery_secret_key_input.get().trim().to_owned();
        if rk.is_empty() || sk.is_empty() || unlocking.get() {
            return;
        }
        unlocking.set(true);
        let msg_wrong = t_string!(i18n, unlock.recover_wrong).to_owned();
        let msg_invalid = t_string!(i18n, unlock.recover_invalid).to_owned();
        let msg_failed = t_string!(i18n, unlock.unlock_failed).to_owned();
        spawn_local(async move {
            let input = UnlockWithRecoveryKeyInputDto {
                vault_path: sel.path.clone(),
                recovery_key_display: rk,
                secret_key_display: sk,
            };
            match api::recovery::unlock_with_recovery_key(&input).await {
                Ok(()) => {
                    recovery_target.set(Some(sel));
                    recovery_new_password.set(String::new());
                    recovery_confirm_password.set(String::new());
                    recovery_error.set(None);
                    recovery_reset_open.set(true);
                }
                // A wrong Recovery Key OR Secret Key (the 2SKD binding) fails the unwrap.
                Err(ApiError::WrongCredentials) => show_error(msg_wrong),
                // A malformed kit, or a vault with no Recovery Key set up.
                Err(ApiError::Invalid(_)) => show_error(msg_invalid),
                Err(e) => show_error(format!("{msg_failed}{e}")),
            }
            unlocking.set(false);
        });
    };

    // The forced new master password after a recovery unlock (5.7). No old-password reauth —
    // the core gate ensures this only runs on the just-recovered session, once. On success we
    // finally enter the vault; recovery is now off (the change nulled the slot).
    let do_recovery_reset = move || {
        let Some(sel) = recovery_target.get() else {
            return;
        };
        let new_pw = recovery_new_password.get();
        if new_pw.is_empty() || new_pw != recovery_confirm_password.get() || unlocking.get() {
            return;
        }
        unlocking.set(true);
        recovery_error.set(None);
        let nav = use_navigate();
        let msg_err = t_string!(i18n, unlock.recover_reset_err).to_owned();
        let msg_done = t_string!(i18n, unlock.recover_reset_done).to_owned();
        spawn_local(async move {
            match api::recovery::change_password_after_recovery(&sel.path, &new_pw).await {
                Ok(()) => {
                    recovery_reset_open.set(false);
                    recovery_key_input.set(String::new());
                    recovery_secret_key_input.set(String::new());
                    recovery_new_password.set(String::new());
                    recovery_confirm_password.set(String::new());
                    show_success(msg_done);
                    record_unlock(&sel).await;
                    active.path.set(Some(sel.path.clone()));
                    nav("/v/vault", Default::default());
                }
                Err(e) => recovery_error.set(Some(format!("{msg_err} {e}"))),
            }
            unlocking.set(false);
        });
    };

    // Unlock via the biometric gate (Windows Hello). No master password; any
    // failure reveals the password fallback with an error toast.
    let do_bio_unlock = move || {
        let Some(sel) = selected.get() else {
            return;
        };
        if unlocking.get() {
            return;
        }
        unlocking.set(true);
        let nav = use_navigate();
        let msg_failed = t_string!(i18n, unlock.err_biometric).to_owned();
        let msg_rollback = t_string!(i18n, unlock.rollback_warning).to_owned();
        spawn_local(async move {
            if let Ok(result) = api::biometric::unlock(&sel.path).await {
                if result.rollback_detected {
                    show_warning(msg_rollback);
                }
                record_unlock(&sel).await;
                active.path.set(Some(sel.path.clone()));
                nav("/v/vault", Default::default());
            } else {
                show_password.set(true);
                show_error(msg_failed);
            }
            unlocking.set(false);
        });
    };

    // Open a vault file not in registry → select it into the unlock panel.
    let on_open_file = Callback::new(move |()| {
        let dialog_title = t_string!(i18n, unlock.open_file).to_owned();
        let err_prefix = t_string!(i18n, unlock.err_open).to_owned();
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(dialog_title),
                filters: vec![],
                // A vault is a `.vedge/` folder (slice 5.2.0) → pick a directory.
                directory: true,
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    let existing = registry
                        .get_untracked()
                        .into_iter()
                        .find(|r| r.vault.path == path);
                    let (id, display_name) = match existing {
                        Some(r) => (Some(r.vault.id), r.vault.display_name),
                        None => (None, display_name_from_path(&path)),
                    };
                    selected.set(Some(Selected {
                        path,
                        id,
                        display_name,
                    }));
                    set_timeout(focus_password, Duration::from_millis(30));
                }
                Ok(None) => {}
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    let on_new = Callback::new(move |()| {
        use_navigate()("/onboarding", Default::default());
    });
    let on_unlock = Callback::new(move |()| do_unlock());
    let on_bio_unlock = Callback::new(move |()| do_bio_unlock());
    let on_use_password = Callback::new(move |()| show_password.set(true));
    let on_secret_key_unlock = Callback::new(move |()| do_secret_key_unlock());
    let on_recovery_unlock = Callback::new(move |()| do_recovery_unlock());
    let on_recovery_reset = Callback::new(move |()| do_recovery_reset());
    // Dismissing the forced dialog enters the just-recovered (live) session rather than
    // stranding it on the launch screen. The recovery slot is untouched until the password is
    // actually changed, so recovery still works on the next lock — no brick, just a nudge
    // deferred (Settings will still show recovery as on until they change the password).
    let on_recovery_reset_close = Callback::new(move |()| {
        recovery_reset_open.set(false);
        if let Some(sel) = recovery_target.get() {
            active.path.set(Some(sel.path));
            use_navigate()("/v/vault", Default::default());
        }
    });

    // Slice 5.2.2 — "Open a backup…" (and, behind Advanced, Replace).
    let backup_open = RwSignal::new(false);
    let on_open_backup = Callback::new(move |()| backup_open.set(true));
    let on_backup_done = Callback::new(move |()| refresh_registry());

    let empty_state = move || {
        view! {
            <div class="w-[420px] max-w-full rounded-xl border border-border bg-surface p-6 shadow-lg">
                <EmptyState
                    icon=i::FaFileShieldSolid
                    title=Signal::derive(move || {
                        t_string!(i18n, unlock.no_vaults_title).to_owned()
                    })
                    description=Signal::derive(move || {
                        t_string!(i18n, unlock.no_vaults_body).to_owned()
                    })
                >
                    <div class="flex gap-2">
                        <Button
                            variant=Variant::Primary
                            attr:data-testid="launch-new-vault"
                            on:click=move |_: web_sys::MouseEvent| on_new.run(())
                        >
                            {move || t!(i18n, unlock.new_vault)}
                        </Button>
                        <Button
                            variant=Variant::Secondary
                            on:click=move |_: web_sys::MouseEvent| on_open_file.run(())
                        >
                            {move || t!(i18n, unlock.open_file)}
                        </Button>
                    </div>
                    // 🔴 A brand-new machine has NO registry — which is exactly the state a user
                    // arrives in holding a `.vbk` and an Emergency Kit. If "Open a backup…" only
                    // existed in the populated picker's footer, the new-machine flow (the whole
                    // reason this slice exists) would have no door at all.
                    <div class="mt-3">
                        <Button
                            variant=Variant::Ghost
                            attr:data-testid="open-backup"
                            on:click=move |_: web_sys::MouseEvent| on_open_backup.run(())
                        >
                            {move || t!(i18n, unlock.ob_cta)}
                        </Button>
                    </div>
                </EmptyState>
            </div>
        }
    };

    view! {
        <div class="flex h-full w-full items-center justify-center p-2">
            <Show
                when=move || !loading.get()
                fallback=move || {
                    view! {
                        <div class="flex items-center justify-center">
                            <Spinner label=Signal::derive(move || {
                                t_string!(i18n, unlock.loading).to_owned()
                            }) />
                        </div>
                    }
                }
            >
                <Show when=move || !registry.get().is_empty() fallback=empty_state>
                    <div class="flex h-full w-full gap-2">
                        <VaultList
                            filtered=filtered
                            query=query
                            selected=selected
                            on_select=on_select
                            on_menu=on_menu
                            on_locate=on_locate
                            on_restore=on_restore
                            on_new=on_new
                            on_open_file=on_open_file
                            on_open_backup=on_open_backup
                        />
                        <div class="flex flex-1 overflow-hidden rounded-lg border border-border bg-surface">
                            <VaultUnlockPanel
                                selected=selected
                                pw=pw
                                unlocking=unlocking
                                bio_enrolled=bio_enrolled
                                show_password=show_password
                                secret_key_open=secret_key_open
                                secret_key_input=secret_key_input
                                recovery_open=recovery_open
                                recovery_key_input=recovery_key_input
                                recovery_secret_key_input=recovery_secret_key_input
                                on_unlock=on_unlock
                                on_bio_unlock=on_bio_unlock
                                on_use_password=on_use_password
                                on_secret_key_unlock=on_secret_key_unlock
                                on_recovery_unlock=on_recovery_unlock
                            />
                        </div>
                    </div>
                </Show>
            </Show>
            <BackupOpenDialog open=backup_open selected=selected on_done=on_backup_done />
            <RecoveryResetDialog
                open=recovery_reset_open
                new_password=recovery_new_password
                confirm_password=recovery_confirm_password
                busy=unlocking
                error=recovery_error
                on_submit=on_recovery_reset
                on_close=on_recovery_reset_close
            />
            <VaultDetailsDialog
                target=details_target
                details=details_stats
                on_rename=on_rename_commit
                on_remove=on_remove
                on_delete_request=on_delete_request
            />
            <DeleteVaultDialog
                target=delete_target
                typed=delete_typed
                details=details_stats
                on_confirm=on_delete
            />
            // The show-once new Secret Key after a re-key rotated it (5.8). Shown HERE (not on the
            // settings dialog) because re-key locks the vault and the auto-lock would eject the
            // dialog before the user could copy it — the launch screen has no auto-lock.
            <Dialog
                open=Signal::derive(move || new_kit.display.get().is_some())
                on_close=on_new_kit_close
                size=DialogSize::Sm
                close_label=Signal::derive(move || t_string!(i18n, unlock.dismiss).to_owned())
            >
                <DialogHeader>
                    <DialogTitle>{move || t!(i18n, unlock.new_kit_title)}</DialogTitle>
                </DialogHeader>
                <DialogBody>
                    <div class="flex flex-col gap-4 min-w-[22rem]">
                        <p class="text-sm text-text-primary">
                            {move || t!(i18n, settings.rekey_new_kit_intro)}
                        </p>
                        <SecretDisplay value=Signal::derive(move || {
                            new_kit.display.get().unwrap_or_default()
                        }) />
                        <div class="flex justify-end">
                            <Button
                                variant=Variant::Primary
                                size=Size::Sm
                                attr:data-testid="rekey-new-kit-done"
                                on:click=move |_: web_sys::MouseEvent| on_new_kit_close.run(())
                            >
                                {move || t!(i18n, settings.rekey_continue)}
                            </Button>
                        </div>
                    </div>
                </DialogBody>
            </Dialog>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::display_name_from_path;

    #[test]
    fn strips_dir_and_vedge_extension() {
        // A `.vedge` home (slice 5.2.0) reduces to its stem.
        assert_eq!(
            display_name_from_path("C:/Users/me/my-vault.vedge"),
            "my-vault"
        );
        assert_eq!(display_name_from_path("/home/me/vaults/work.vedge"), "work");
    }

    #[test]
    fn handles_bare_name_and_no_extension() {
        assert_eq!(display_name_from_path("personal.vedge"), "personal");
        assert_eq!(display_name_from_path("weird"), "weird");
    }

    #[test]
    fn windows_backslash_separator() {
        assert_eq!(
            display_name_from_path("D:\\Vaults\\Home Vault.vedge"),
            "Home Vault"
        );
    }
}
