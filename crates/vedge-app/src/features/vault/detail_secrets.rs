//! Read-drawer secret surface (slice 5.4.1 rework): **copy + reveal-to-view** for
//! every sealed field, shown when you open (`get_entry`) an entry.
//!
//! The edit form is inputs only. *Reading* a stored secret happens here:
//! - **Copy** routes through the secure `copy_field` command (`on_copy`) — the
//!   plaintext goes to the OS clipboard, never to WASM.
//! - **Reveal** fetches the value via the audited `reveal_field` and shows it in a
//!   [`SecretDisplay`] (which also copies). An absent optional secret reveals as
//!   "not set" (mirrors [`TotpReveal`](super::totp_reveal::TotpReveal)).
//!
//! Presence is not known from the list projection (`IndexEntryDto` carries no
//! secrets), so every field of the type is offered and reveal/copy handle absence
//! gracefully — the same shape `TotpReveal` uses for a Login with no seed.

use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ipc::{EntryTypeDto, EnvExportFormatDto, FieldSelectorDto};
use vedge_ui::components::Button;
use vedge_ui::components::feedback::dropdown_menu::{
    DropdownMenu, MenuEntry, MenuItem, MenuItemVariant, MenuSection,
};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::badge::Badge;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};

use super::secret_display::SecretDisplay;
use crate::api;
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

/// One sealed secret field in the read drawer: a label, a secure **Copy** action,
/// and a **Reveal** toggle that shows the value in a [`SecretDisplay`]. An absent
/// optional secret reveals as "not set".
#[component]
fn SecretRow(
    #[prop(into)] label: Signal<String>,
    field: FieldSelectorDto,
    entry_id: StoredValue<String>,
    on_copy: Callback<FieldSelectorDto>,
    /// Per-field `data-testid` on the Reveal button (e.g. `reveal-ssh-key`).
    #[prop(optional, default = "")]
    reveal_testid: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    // Some(value) once revealed; None while hidden. `absent` when the optional
    // secret is not set.
    let revealed = RwSignal::new(Option::<String>::None);
    let absent = RwSignal::new(false);

    let copy_field = field.clone();
    let copy = move |_: web_sys::MouseEvent| on_copy.run(copy_field.clone());

    // Last use of `field` — move it (no clone) into the reveal handler.
    let reveal_field = field;
    let toggle = move |_: web_sys::MouseEvent| {
        if revealed.get_untracked().is_some() || absent.get_untracked() {
            revealed.set(None);
            absent.set(false);
            return;
        }
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        let field = reveal_field.clone();
        if id.is_empty() {
            return;
        }
        spawn_local(async move {
            match api::entry::reveal_field(&vault_path, &id, field).await {
                Ok(value) => revealed.set(Some(value)),
                // `FieldNotApplicable` (an absent optional secret) surfaces as the
                // shared `Invalid` kind → "not set", like `TotpReveal`'s no-code.
                // Caveat: `Invalid` also covers a corrupt/undecodable payload, so a
                // (rare) broken entry would show "not set" for a stored secret; the
                // ciphertext isn't lost (5.3 export still reaches it). A locked /
                // dropped session maps to a different kind → the silent arm below
                // (the app redirects to the lock screen on its own).
                Err(ApiError::Invalid(_)) => absent.set(true),
                Err(_) => {}
            }
        });
    };

    view! {
        <div class="flex flex-col gap-1 py-1">
            <div class="flex items-center justify-between gap-2">
                <span class="text-xs text-foreground/60 truncate">{move || label.get()}</span>
                <div class="flex items-center gap-1">
                    <IconButton
                        variant=Variant::Ghost
                        size=Size::Sm
                        aria_label=Signal::derive(move || {
                            t_string!(i18n, vault.secret_copy).to_owned()
                        })
                        on:click=copy
                    >
                        <span aria-hidden="true">
                            <Icon icon=i::FaCopySolid />
                        </span>
                    </IconButton>
                    <IconButton
                        variant=Variant::Ghost
                        size=Size::Sm
                        attr:data-testid=reveal_testid
                        aria_label=Signal::derive(move || {
                            if revealed.get().is_some() {
                                t_string!(i18n, vault.hide).to_owned()
                            } else {
                                t_string!(i18n, vault.reveal).to_owned()
                            }
                        })
                        on:click=toggle
                    >
                        <Show
                            when=move || revealed.get().is_some()
                            fallback=|| view! { <Icon icon=i::FaEyeSolid /> }
                        >
                            <Icon icon=i::FaEyeSlashSolid />
                        </Show>
                    </IconButton>
                </div>
            </div>
            <Show when=move || revealed.get().is_some()>
                <SecretDisplay value=Signal::derive(move || revealed.get().unwrap_or_default()) />
            </Show>
            <Show when=move || absent.get()>
                <p class="text-xs text-text-tertiary">{move || t!(i18n, vault.secret_not_set)}</p>
            </Show>
        </div>
    }
}

/// Reveal a Login's stored recovery codes as a list (slice 5.4.1 ②). The whole
/// list crosses in one audited call ([`api::entry::reveal_recovery_codes`], one
/// `SecretRevealed` row); each code shows with a 1-based [`Badge`] index and a
/// copyable [`SecretDisplay`]. A Login with none reveals as "not set".
#[component]
fn RecoveryCodesRow(entry_id: StoredValue<String>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    // Some(list) once revealed (may be empty); None while hidden.
    let codes = RwSignal::new(Option::<Vec<String>>::None);

    let toggle = move |_: web_sys::MouseEvent| {
        if codes.get_untracked().is_some() {
            codes.set(None);
            return;
        }
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        if id.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(list) = api::entry::reveal_recovery_codes(&vault_path, &id).await {
                codes.set(Some(list));
            }
        });
    };

    view! {
        <div class="flex flex-col gap-1 py-1">
            <div class="flex items-center justify-between gap-2">
                <span class="text-xs text-foreground/60">
                    {move || t!(i18n, vault.recovery_codes_title)}
                </span>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    attr:data-testid="reveal-recovery-codes"
                    on:click=toggle
                >
                    {move || {
                        if codes.get().is_some() {
                            t_string!(i18n, vault.hide).to_owned()
                        } else {
                            t_string!(i18n, vault.reveal).to_owned()
                        }
                    }}
                </Button>
            </div>
            <Show when=move || codes.get().is_some_and(|l| l.is_empty())>
                <p class="text-xs text-text-tertiary">{move || t!(i18n, vault.secret_not_set)}</p>
            </Show>
            <Show when=move || codes.get().is_some_and(|l| !l.is_empty())>
                <div class="flex flex-col gap-2">
                    <For
                        each=move || {
                            codes
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .enumerate()
                                .collect::<Vec<_>>()
                        }
                        key=|(idx, _)| *idx
                        children=move |(idx, code)| {
                            view! {
                                <div class="flex items-center gap-2">
                                    <Badge>{idx.saturating_add(1).to_string()}</Badge>
                                    <div class="flex-1">
                                        <SecretDisplay value=Signal::derive(move || code.clone()) />
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Copy or reveal an `EnvVars` entry's WHOLE set as `.env` or JSON (slice 5.4.1
/// ⑥), from a `⋯` menu. Copy formats server-side and never crosses the set to
/// WASM; reveal shows it in a [`SecretDisplay`]. A `.env` value with a newline
/// errors — the toast points at JSON.
#[component]
fn EnvVarsSecrets(entry_id: StoredValue<String>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let revealed_set = RwSignal::new(Option::<String>::None);

    let copy_set = Callback::new(move |format: EnvExportFormatDto| {
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        if id.is_empty() {
            return;
        }
        let ok_msg = t_string!(i18n, vault.copied).to_owned();
        let err_prefix = t_string!(i18n, vault.err_copy).to_owned();
        let dismiss = t_string!(i18n, vault.dismiss).to_owned();
        spawn_local(async move {
            let (msg, variant) =
                match api::entry::copy_env_vars(&vault_path, &id, format, None).await {
                    Ok(()) => (ok_msg, ToastVariant::Success),
                    Err(e) => (format!("{err_prefix}{e}"), ToastVariant::Danger),
                };
            toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
        });
    });

    let reveal_set = Callback::new(move |format: EnvExportFormatDto| {
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        if id.is_empty() {
            return;
        }
        let err_prefix = t_string!(i18n, vault.err_reveal).to_owned();
        let dismiss = t_string!(i18n, vault.dismiss).to_owned();
        spawn_local(async move {
            match api::entry::reveal_env_vars(&vault_path, &id, format).await {
                Ok(blob) => revealed_set.set(Some(blob)),
                Err(e) => {
                    toast.show(
                        ToastInput::new(format!("{err_prefix}{e}"))
                            .variant(ToastVariant::Danger)
                            .dismiss_label(dismiss),
                    );
                }
            }
        });
    });

    let menu_item = |id: &str, label: String, on_click: Callback<()>| {
        MenuEntry::Item(MenuItem {
            id: id.to_owned(),
            label,
            variant: MenuItemVariant::Default,
            icon: None,
            shortcut: None,
            on_click: Some(on_click),
            href: None,
        })
    };
    // Built once; `untrack` the label reads (they don't relocalize — fine here).
    let menu_items: Vec<MenuSection> = untrack(|| {
        vec![MenuSection {
            label: None,
            items: vec![
                menu_item(
                    "env-copy-dotenv",
                    t_string!(i18n, vault.env_copy_dotenv).to_owned(),
                    Callback::new(move |()| copy_set.run(EnvExportFormatDto::DotEnv)),
                ),
                menu_item(
                    "env-copy-json",
                    t_string!(i18n, vault.env_copy_json).to_owned(),
                    Callback::new(move |()| copy_set.run(EnvExportFormatDto::Json)),
                ),
                MenuEntry::Separator,
                menu_item(
                    "env-reveal-dotenv",
                    t_string!(i18n, vault.env_reveal_dotenv).to_owned(),
                    Callback::new(move |()| reveal_set.run(EnvExportFormatDto::DotEnv)),
                ),
                menu_item(
                    "env-reveal-json",
                    t_string!(i18n, vault.env_reveal_json).to_owned(),
                    Callback::new(move |()| reveal_set.run(EnvExportFormatDto::Json)),
                ),
            ],
        }]
    });

    view! {
        <div class="flex flex-col gap-1 py-1">
            <div class="flex items-center justify-between gap-2">
                <span class="text-xs text-foreground/60">
                    {move || t!(i18n, vault.type_env_vars)}
                </span>
                <DropdownMenu
                    trigger=Box::new(move || {
                        view! {
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Sm
                                attr:data-testid="env-set-menu"
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, vault.env_set_menu).to_owned()
                                })
                            >
                                <span aria-hidden="true">
                                    <Icon icon=i::FaEllipsisSolid />
                                </span>
                            </IconButton>
                        }
                            .into_any()
                    })
                    items=menu_items
                    aria_label=Signal::derive(move || {
                        t_string!(i18n, vault.env_set_menu).to_owned()
                    })
                />
            </div>
            <Show when=move || revealed_set.get().is_some()>
                <div class="flex flex-col gap-1">
                    <div class="flex items-center justify-between">
                        <span class="text-xs text-foreground/60">
                            {move || t!(i18n, vault.env_revealed_title)}
                        </span>
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| revealed_set.set(None)
                        >
                            {move || t!(i18n, vault.hide)}
                        </Button>
                    </div>
                    <SecretDisplay value=Signal::derive(move || {
                        revealed_set.get().unwrap_or_default()
                    }) />
                </div>
            </Show>
        </div>
    }
}

/// Expand to a [`SecretRow`] whose label is the localized `vault.<key>` string.
/// `t_string!` needs a *literal* key path, so the key is an ident, not a runtime
/// string.
macro_rules! secret_row {
    ($i18n:expr, $eid:expr, $on_copy:expr, $key:ident, $field:expr, $testid:literal) => {
        view! {
            <SecretRow
                label=Signal::derive(move || t_string!($i18n, vault.$key).to_owned())
                field=$field
                entry_id=$eid
                on_copy=$on_copy
                reveal_testid=$testid
            />
        }
    };
}

/// The read-drawer secret block for an entry — copy + reveal-to-view of every
/// secret the type carries. TOTP keeps its own `TotpReveal` widget (the caller
/// renders it); this covers the rest.
#[component]
pub fn DetailSecrets(
    entry_type: EntryTypeDto,
    entry_id: String,
    on_copy: Callback<FieldSelectorDto>,
) -> impl IntoView {
    let i18n = use_i18n();
    let eid = StoredValue::new(entry_id);

    match entry_type {
        EntryTypeDto::Login => view! {
            {secret_row!(i18n, eid, on_copy, form_identifier, FieldSelectorDto::Username, "")}
            {secret_row!(
                i18n, eid, on_copy, form_password, FieldSelectorDto::Password, "reveal-password"
            )}
            <RecoveryCodesRow entry_id=eid />
        }
        .into_any(),
        EntryTypeDto::Card => view! {
            {secret_row!(i18n, eid, on_copy, field_card_number, FieldSelectorDto::CardNumber, "")}
            {secret_row!(i18n, eid, on_copy, field_cvv, FieldSelectorDto::Cvv, "")}
            {secret_row!(i18n, eid, on_copy, field_pin, FieldSelectorDto::Pin, "")}
        }
        .into_any(),
        EntryTypeDto::SshKey => view! {
            {secret_row!(
                i18n, eid, on_copy, field_private_key, FieldSelectorDto::PrivateKey, "reveal-ssh-key"
            )}
            {secret_row!(i18n, eid, on_copy, field_passphrase, FieldSelectorDto::Passphrase, "")}
        }
        .into_any(),
        EntryTypeDto::ApiKey => view! {
            {secret_row!(i18n, eid, on_copy, field_api_key, FieldSelectorDto::ApiKey, "")}
            {secret_row!(i18n, eid, on_copy, field_api_secret, FieldSelectorDto::ApiSecret, "")}
        }
        .into_any(),
        EntryTypeDto::EnvVars => view! { <EnvVarsSecrets entry_id=eid /> }.into_any(),
        EntryTypeDto::Identity => view! { {secret_row!(i18n, eid, on_copy, field_national_id, FieldSelectorDto::NationalId, "")} }
        .into_any(),
        // Bind by value (`_other`, not `_`) so `entry_type` is *moved* into the
        // match — consuming it satisfies `needless_pass_by_value` (a `&` param
        // would force an RPIT lifetime the caller's closure can't hold).
        _other => ().into_any(),
    }
}
