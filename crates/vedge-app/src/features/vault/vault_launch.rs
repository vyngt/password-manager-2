//! Returning-user launch screen (`/`) — the vault picker (2.8.1). A two-pane
//! surface: a searchable, recency-sorted, keyboard-navigable list of the
//! user's vaults ([`VaultList`]) on the left, and a focused unlock panel
//! ([`VaultUnlockPanel`]) on the right (master password + 2.8 biometric).
//! Unlocking sets the `ActiveVault` context and lands at `/v/vault`.
//! Single-active-vault model — the Lock control in the `/v` shell returns here.
//!
//! This component is the orchestrator: it owns the recents state + all the
//! signals/closures, deriving the visible list with the pure
//! [`filter_sort_recents`] and threading callbacks to the two panes. Errors
//! surface as `Danger` toasts.

use crate::api;
use crate::api::dialog::OpenDialogOptions;
use crate::api::error::ApiError;
use crate::features::vault::backup_open_dialog::BackupOpenDialog;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::recents_filter::filter_sort_recents;
use crate::features::vault::vault_list::VaultList;
use crate::features::vault::vault_unlock_panel::VaultUnlockPanel;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::time::Duration;
use uuid::Uuid;
use vedge_ipc::{
    RecentVaultDto, RecentVaultStatusDto, UnlockVaultInputDto, UnlockWithRecoveryKeyInputDto,
};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, EmptyState, Spinner};
use vedge_ui::primitives::tokens::{ToastVariant, Variant};
use wasm_bindgen::JsCast;

/// A chosen unlock target: a vault path + display name, plus the recents `id`
/// when it came from the recents list (`None` for a file picked via "Open…").
#[derive(Clone)]
pub struct Selected {
    pub path: String,
    pub id: Option<String>,
    pub display_name: String,
}

/// Derive a human display name from a vault path: the last path segment without its
/// `.vedge` (or legacy `.vdb`) extension.
pub fn display_name_from_path(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".vedge")
        .or_else(|| name.strip_suffix(".vdb"))
        .unwrap_or(name)
        .to_owned()
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

/// Record a just-unlocked vault in recents (touch an existing entry, or add a
/// freshly-picked one). Shared by the password and biometric unlock paths. A
/// plain async fn — no reactive owner, so safe to `.await` inside `spawn_local`.
async fn record_unlock(sel: &Selected) {
    if let Some(id) = &sel.id {
        let _ = api::recent::touch_recent_vault_on_unlock(id).await;
    } else {
        let dto = RecentVaultDto {
            id: Uuid::new_v4().to_string(),
            path: sel.path.clone(),
            display_name: sel.display_name.clone(),
            last_opened: None,
            sort_order: 0,
        };
        let _ = api::recent::add_recent_vault(&dto).await;
    }
}

#[component]
pub fn VaultLaunch() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let recents = RwSignal::new(Vec::<RecentVaultStatusDto>::new());
    let loading = RwSignal::new(true);
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new(Option::<Selected>::None);
    let pw = RwSignal::new(String::new());
    let unlocking = RwSignal::new(false);

    // Recovery (5.1): the "Use Emergency Kit" panel state. `recovery_open` is
    // toggled by the always-visible CTA and force-opened on a keychain error;
    // `recovery_key` holds the `A3-…` Secret Key (passed through verbatim).
    let recovery_open = RwSignal::new(false);
    let recovery_key = RwSignal::new(String::new());

    // Biometric: `bio_available` is device-wide (checked once); `bio_enrolled`
    // is per-selected-vault. When enrolled, the Hello button shows by default;
    // `show_password` reveals the password fallback on demand.
    let bio_available = RwSignal::new(false);
    let bio_enrolled = RwSignal::new(false);
    let show_password = RwSignal::new(true);

    // The visible list: pure filter + recency/missing-last sort over recents.
    let filtered = Signal::derive(move || filter_sort_recents(&recents.get(), &query.get()));

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

    let refresh_recents = move || {
        loading.set(true);
        // Called from an Effect *and* from inside `spawn_local`; `untrack`
        // reads the current locale string safely in both.
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_recents).to_owned());
        spawn_local(async move {
            match api::recent::list_recent_vaults_with_status().await {
                Ok(list) => recents.set(list),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        refresh_recents();
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
            let _ = api::recent::remove_recent_vault(&id).await;
            if clear {
                selected.set(None);
            }
            refresh_recents();
        });
    });

    // Re-point a missing vault: pick the moved file, re-add it (preserving the
    // display name → floats to top with a fresh `last_opened`), drop the stale
    // row. Reuses add + remove; no path-update command needed.
    let on_locate = Callback::new(move |id: String| {
        let existing_name = recents
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
                    let dto = RecentVaultDto {
                        id: Uuid::new_v4().to_string(),
                        path,
                        display_name,
                        last_opened: None,
                        sort_order: 0,
                    };
                    // Re-point only if the chosen file is a real vault; the
                    // stale row stays put (with an error) otherwise.
                    match api::recent::add_recent_vault(&dto).await {
                        Ok(()) => {
                            let _ = api::recent::remove_recent_vault(&id).await;
                            refresh_recents();
                        }
                        Err(e) => show_error(format!("{err_prefix}{e}")),
                    }
                }
                Ok(None) => {}
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // Convert a legacy `.vdb` recents row to a `.vedge/` home (slice 5.2.0), then re-point
    // the row to the new home.
    let on_convert = Callback::new(move |id: String| {
        let row = recents
            .get_untracked()
            .into_iter()
            .find(|r| r.vault.id == id);
        let Some(row) = row else {
            return;
        };
        let legacy_path = row.vault.path;
        let display_name = row.vault.display_name;
        let err_prefix = t_string!(i18n, unlock.err_open).to_owned();
        spawn_local(async move {
            match api::vault::convert(&legacy_path).await {
                Ok(home) => {
                    let dto = RecentVaultDto {
                        id: Uuid::new_v4().to_string(),
                        path: home,
                        display_name,
                        last_opened: None,
                        sort_order: 0,
                    };
                    match api::recent::add_recent_vault(&dto).await {
                        Ok(()) => {
                            let _ = api::recent::remove_recent_vault(&id).await;
                            refresh_recents();
                        }
                        Err(e) => show_error(format!("{err_prefix}{e}")),
                    }
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    let on_rename_commit = Callback::new(move |(id, name): (String, String)| {
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_rename).to_owned());
        spawn_local(async move {
            match api::recent::rename_recent_vault(&id, &name).await {
                Ok(()) => refresh_recents(),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // H0 disaster path: a present-but-corrupt vault (`exists && !openable`) can't be unlocked,
    // so restore it from its NEWEST snapshot. The vault is locked here (launch screen), so the
    // revert runs directly; on success the row becomes openable and the user unlocks normally.
    let on_restore = Callback::new(move |id: String| {
        let Some(row) = recents
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
                    refresh_recents();
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
                    recovery_open.set(true);
                }
                Err(e) => show_error(format!("{msg_failed}{e}")),
            }
            unlocking.set(false);
        });
    };

    // Recover via the Emergency Kit: master password + the `A3-…` Secret Key.
    // Sibling of `do_unlock`; on success the shell inserts the session itself
    // (same as a normal unlock) and reports whether the keychain was restored.
    let do_recover = move || {
        let Some(sel) = selected.get() else {
            return;
        };
        let password = pw.get();
        // Trim ONLY — `parse_secret_key` (core) owns normalization (case, 0/O,
        // 1/I/L) and the checksum. No UI-side re-validation.
        let key = recovery_key.get().trim().to_owned();
        if password.is_empty() || key.is_empty() || unlocking.get() {
            return;
        }
        unlocking.set(true);
        let nav = use_navigate();
        // Hoist EVERY locale string here (owner present); reading inside
        // `spawn_local` trips the reactive-context warning (console must stay clean).
        let msg_invalid = t_string!(i18n, unlock.recovery_invalid_key).to_owned();
        let msg_wrong = t_string!(i18n, unlock.wrong_password).to_owned();
        let msg_failed = t_string!(i18n, unlock.unlock_failed).to_owned();
        let msg_partial = t_string!(i18n, unlock.recovery_partial).to_owned();
        let msg_restored = t_string!(i18n, unlock.recovery_restored).to_owned();
        spawn_local(async move {
            let input = UnlockWithRecoveryKeyInputDto {
                vault_path: sel.path.clone(),
                master_password: password,
                recovery_key_display: key,
            };
            match api::recovery::unlock_with_recovery_key(&input).await {
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
                    recovery_key.set(String::new());
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

    // Open a vault file not in recents → select it into the unlock panel.
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
                    let existing = recents
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
    let on_recover = Callback::new(move |()| do_recover());

    // Slice 5.2.2 — "Open a backup…" (and, behind Advanced, Replace).
    let backup_open = RwSignal::new(false);
    let on_open_backup = Callback::new(move |()| backup_open.set(true));
    let on_backup_done = Callback::new(move |()| refresh_recents());

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
                    // 🔴 A brand-new machine has NO recents — which is exactly the state a user
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
        <div class="flex h-full w-full items-center justify-center p-6">
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
                <Show when=move || !recents.get().is_empty() fallback=empty_state>
                    <div class="flex h-[496px] w-[680px] max-w-full overflow-hidden rounded-xl border border-border bg-surface shadow-lg">
                        <VaultList
                            filtered=filtered
                            query=query
                            selected=selected
                            on_select=on_select
                            on_rename_commit=on_rename_commit
                            on_remove=on_remove
                            on_locate=on_locate
                            on_convert=on_convert
                            on_restore=on_restore
                            on_new=on_new
                            on_open_file=on_open_file
                            on_open_backup=on_open_backup
                        />
                        <VaultUnlockPanel
                            selected=selected
                            pw=pw
                            unlocking=unlocking
                            bio_enrolled=bio_enrolled
                            show_password=show_password
                            recovery_open=recovery_open
                            recovery_key=recovery_key
                            on_unlock=on_unlock
                            on_bio_unlock=on_bio_unlock
                            on_use_password=on_use_password
                            on_recover=on_recover
                        />
                    </div>
                </Show>
            </Show>
            <BackupOpenDialog open=backup_open selected=selected on_done=on_backup_done />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::display_name_from_path;

    #[test]
    fn strips_dir_and_vedge_or_vdb_extension() {
        // A `.vedge` home (slice 5.2.0) and a legacy `.vdb` both reduce to the stem.
        assert_eq!(
            display_name_from_path("C:/Users/me/my-vault.vedge"),
            "my-vault"
        );
        assert_eq!(display_name_from_path("/home/me/vaults/work.vedge"), "work");
        assert_eq!(
            display_name_from_path("/home/me/vaults/legacy.vdb"),
            "legacy"
        );
    }

    #[test]
    fn handles_bare_name_and_no_extension() {
        assert_eq!(display_name_from_path("personal.vdb"), "personal");
        assert_eq!(display_name_from_path("weird"), "weird");
    }

    #[test]
    fn windows_backslash_separator() {
        assert_eq!(
            display_name_from_path("D:\\Vaults\\Home Vault.vdb"),
            "Home Vault"
        );
    }
}
