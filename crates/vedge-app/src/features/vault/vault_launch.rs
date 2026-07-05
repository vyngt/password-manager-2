//! Returning-user launch screen (`/`). Lists recent vaults with on-disk
//! status, unlocks the selected one (Secret Key resolved from the OS
//! keychain by the core), sets the `ActiveVault` context, and lands at
//! `/v/vault`. Single-active-vault model — the Lock control in the `/v`
//! shell returns here.

use crate::api;
use crate::api::dialog::{DialogFilter, OpenDialogOptions};
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;
use vedge_ipc::{RecentVaultDto, RecentVaultStatusDto, UnlockVaultInputDto};
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::components::icon::Decrypt;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use leptos_icons::Icon;

/// A chosen unlock target: a vault path, plus the recents `id` when it came
/// from the recents list (`None` for a file picked via "Open other…").
#[derive(Clone)]
struct Selected {
    path: String,
    id: Option<String>,
}

/// Derive a human display name from a vault path: the file stem without its
/// directory or `.vdb` extension.
pub fn display_name_from_path(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".vdb").unwrap_or(name).to_string()
}

#[component]
pub fn VaultLaunch() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let recents = RwSignal::new(Vec::<RecentVaultStatusDto>::new());
    let loading = RwSignal::new(true);
    let selected = RwSignal::new(Option::<Selected>::None);
    let pw = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let unlocking = RwSignal::new(false);

    let refresh_recents = move || {
        loading.set(true);
        spawn_local(async move {
            match api::recent::list_recent_vaults_with_status().await {
                Ok(list) => recents.set(list),
                Err(e) => error.set(Some(format!("{e}"))),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        refresh_recents();
    });

    let on_select = Callback::new(move |sel: Selected| {
        selected.set(Some(sel));
        error.set(None);
    });

    let on_remove = Callback::new(move |id: String| {
        spawn_local(async move {
            let _ = api::recent::remove_recent_vault(&id).await;
            if selected.get().and_then(|s| s.id) == Some(id) {
                selected.set(None);
            }
            refresh_recents();
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
        error.set(None);
        unlocking.set(true);
        let nav = use_navigate();
        // Read locale-dependent strings in the handler (which has a reactive
        // owner); reading them inside `spawn_local` is outside any owner and
        // trips Leptos's "accessed outside a reactive tracking context" warning.
        let msg_wrong = t_string!(i18n, unlock.wrong_password).to_string();
        let msg_keychain = t_string!(i18n, unlock.keychain_missing).to_string();
        let msg_failed = t_string!(i18n, unlock.unlock_failed).to_string();
        spawn_local(async move {
            let input = UnlockVaultInputDto {
                vault_path: sel.path.clone(),
                master_password: password,
                secret_key_b64: None,
            };
            match api::vault::unlock(&input).await {
                Ok(()) => {
                    match &sel.id {
                        Some(id) => {
                            let _ = api::recent::touch_recent_vault_on_unlock(id).await;
                        }
                        None => {
                            let dto = RecentVaultDto {
                                id: Uuid::new_v4().to_string(),
                                path: sel.path.clone(),
                                display_name: display_name_from_path(&sel.path),
                                last_opened: None,
                                sort_order: 0,
                            };
                            let _ = api::recent::add_recent_vault(&dto).await;
                        }
                    }
                    active.path.set(Some(sel.path.clone()));
                    nav("/v/vault", Default::default());
                }
                Err(ApiError::WrongCredentials) => error.set(Some(msg_wrong)),
                Err(ApiError::Keychain(_)) => error.set(Some(msg_keychain)),
                Err(e) => error.set(Some(format!("{msg_failed}{e}"))),
            }
            unlocking.set(false);
        });
    };

    let open_other = move || {
        error.set(None);
        let dialog_title = t_string!(i18n, unlock.open_other).to_string();
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(dialog_title),
                filters: vec![DialogFilter {
                    name: "VEdge Vault".to_string(),
                    extensions: vec!["vdb".to_string()],
                }],
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    let existing_id = recents
                        .get_untracked()
                        .into_iter()
                        .find(|r| r.vault.path == path)
                        .map(|r| r.vault.id);
                    selected.set(Some(Selected {
                        path,
                        id: existing_id,
                    }));
                }
                Ok(None) => {}
                Err(e) => error.set(Some(format!("{e}"))),
            }
        });
    };

    let go_onboarding = move |_: web_sys::MouseEvent| {
        let nav = use_navigate();
        nav("/onboarding", Default::default());
    };

    let empty_state = move || {
        view! {
            <div class="flex flex-col items-center gap-3 text-center">
                <p class="text-sm text-foreground/50">{move || t!(i18n, unlock.no_recents)}</p>
                <Button variant=Variant::Primary on:click=go_onboarding>
                    {move || t!(i18n, unlock.create_first)}
                </Button>
            </div>
        }
    };

    view! {
        <div class="mx-auto flex h-full w-full max-w-xl flex-col justify-center gap-4 p-8">
            <h1 class="text-center text-lg font-semibold text-text-primary">
                {move || t!(i18n, unlock.select_vault)}
            </h1>

            <Show
                when=move || !loading.get()
                fallback=|| {
                    view! {
                        <div class="text-center text-sm text-foreground/40">"Loading…"</div>
                    }
                }
            >
                <Show when=move || !recents.get().is_empty() fallback=empty_state>
                    <div class="flex flex-col gap-2">
                        <For
                            each=move || recents.get()
                            key=|r| r.vault.id.clone()
                            children=move |rec| {
                                view! {
                                    <VaultCard
                                        rec=rec
                                        on_select=on_select
                                        on_remove=on_remove
                                        selected=selected
                                    />
                                }
                            }
                        />
                    </div>
                </Show>
            </Show>

            {move || selected.get().map(|_| view! {
                <div
                    class="flex w-full"
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            do_unlock();
                        }
                    }
                >
                    <Input
                        id="master-password"
                        placeholder=Signal::derive(move || {
                            t_string!(i18n, unlock.master_password).to_string()
                        })
                        size=Size::Lg
                        input_type="password"
                        value=Signal::derive(move || pw.get())
                        on_input=Callback::new(move |v: String| pw.set(v))
                        class="flex-1 rounded-r-none border-r-0"
                    />
                    {move || {
                        let busy = unlocking.get();
                        view! {
                            <IconButton
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, unlock.unlock).to_string()
                                })
                                variant=Variant::Primary
                                size=Size::Lg
                                loading=busy
                                class="rounded-l-none"
                                on:click=move |_: web_sys::MouseEvent| do_unlock()
                            >
                                <Icon icon=Decrypt />
                            </IconButton>
                        }
                    }}
                </div>
            })}

            {move || error.get().map(|e| view! {
                <p class="text-center text-sm" style="color:var(--color-danger-text)">{e}</p>
            })}

            <div class="flex justify-center gap-2">
                <Button variant=Variant::Ghost on:click=go_onboarding>
                    {move || t!(i18n, unlock.create_vault)}
                </Button>
                <Button
                    variant=Variant::Ghost
                    on:click=move |_: web_sys::MouseEvent| open_other()
                >
                    {move || t!(i18n, unlock.open_other)}
                </Button>
            </div>
        </div>
    }
}

#[component]
fn VaultCard(
    rec: RecentVaultStatusDto,
    on_select: Callback<Selected>,
    on_remove: Callback<String>,
    selected: RwSignal<Option<Selected>>,
) -> impl IntoView {
    let i18n = use_i18n();

    let exists = rec.exists;
    let path = rec.vault.path.clone();
    let id = rec.vault.id.clone();
    let display_name = rec.vault.display_name.clone();
    let last_opened = rec.vault.last_opened.clone();

    let sel_path = path.clone();
    let is_selected = move || selected.get().map(|s| s.path) == Some(sel_path.clone());

    let base = if exists {
        "flex items-center justify-between gap-3 rounded-lg border p-3 transition-colors cursor-pointer hover:bg-primary/5"
    } else {
        "flex items-center justify-between gap-3 rounded-lg border p-3 transition-colors opacity-60"
    };
    let cls = move || {
        let edge = if is_selected() {
            "border-primary"
        } else {
            "border-border"
        };
        format!("{base} {edge}")
    };

    let click_path = path.clone();
    let click_id = id.clone();
    let remove_id = id;

    view! {
        <div
            class=cls
            on:click=move |_: web_sys::MouseEvent| {
                if exists {
                    on_select.run(Selected {
                        path: click_path.clone(),
                        id: Some(click_id.clone()),
                    });
                }
            }
        >
            <div class="min-w-0">
                <div class="truncate text-sm text-text-primary">{display_name}</div>
                <div class="truncate text-xs font-jetbrains-mono text-foreground/50">{path}</div>
                {last_opened.map(|lo| view! {
                    <div class="text-xs text-foreground/40">
                        {move || t!(i18n, unlock.last_opened)}
                        " "
                        {lo}
                    </div>
                })}
            </div>
            {(!exists).then(|| view! {
                <div class="flex shrink-0 items-center gap-2">
                    <span class="text-xs" style="color:var(--color-danger-text)">
                        {move || t!(i18n, unlock.file_missing)}
                    </span>
                    <Button
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            on_remove.run(remove_id.clone());
                        }
                    >
                        {move || t!(i18n, unlock.remove)}
                    </Button>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::display_name_from_path;

    #[test]
    fn strips_dir_and_vdb_extension() {
        assert_eq!(
            display_name_from_path("C:/Users/me/my-vault.vdb"),
            "my-vault"
        );
        assert_eq!(display_name_from_path("/home/me/vaults/work.vdb"), "work");
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
