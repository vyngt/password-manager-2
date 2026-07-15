//! The launch-screen vault-management dialogs (slice 5.2.4, Decision ⑥d/⑤).
//!
//! The row's `⋯` opens [`VaultDetailsDialog`] — the relocated rename, the location, best-effort
//! stats, the **copyable Vault ID** (`mise keychain-audit` keys on uuids), and a danger zone of
//! prose. "Delete vault…" opens a SECOND dialog, [`DeleteVaultDialog`], which **types-to-confirm
//! the vault name** (an unrecoverable act needs two steps) and states plainly that `.vbk` backups
//! saved elsewhere survive.
//!
//! Both are always-mounted and self-gate on a `target` signal (the `trash_confirm` idiom). The
//! parent owns the destructive `on_*` callbacks; these components only gather intent.

use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use vedge_ipc::{RegisteredVaultStatusDto, VaultDetailsDto};
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::foundation::copy_button::CopyButton;
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

/// A coarse human byte size (`14 MB`). Best-effort display only — integer units, since the
/// workspace denies `float_arithmetic` (and every arithmetic operator); `div_euclid` is a method.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1_048_576;
    const GB: u64 = 1_073_741_824;
    if bytes >= GB {
        format!("{} GB", bytes.div_euclid(GB))
    } else if bytes >= MB {
        format!("{} MB", bytes.div_euclid(MB))
    } else if bytes >= KB {
        format!("{} KB", bytes.div_euclid(KB))
    } else {
        format!("{bytes} B")
    }
}

/// The `⋯` vault-details dialog: rename · location · stats · copyable Vault ID · danger zone.
#[component]
pub fn VaultDetailsDialog(
    /// The row whose details are shown. `None` ⇒ closed.
    target: RwSignal<Option<RegisteredVaultStatusDto>>,
    /// Best-effort stats loaded by the parent after the dialog opens.
    details: RwSignal<Option<VaultDetailsDto>>,
    on_rename: Callback<(String, String)>,
    on_remove: Callback<String>,
    /// Open the delete-confirm dialog for this row.
    on_delete_request: Callback<RegisteredVaultStatusDto>,
) -> impl IntoView {
    let i18n = use_i18n();
    let name_draft = RwSignal::new(String::new());
    let close = Callback::new(move |()| target.set(None));

    // Seed the rename draft when the dialog opens.
    Effect::new(move |_| {
        if let Some(row) = target.get() {
            name_draft.set(row.vault.display_name);
        }
    });

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Md
            close_label=Signal::derive(move || t_string!(i18n, unlock.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, unlock.vault_details_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-4 min-w-[22rem]" data-testid="vault-details">
                    {move || {
                        target
                            .get()
                            .map(|row| {
                                let rename_id = row.vault.id.clone();
                                let remove_id = row.vault.id.clone();
                                let path = row.vault.path.clone();
                                let uuid = row.vault_uuid.clone();
                                let del_row = row;
                                view! {
                                    // Display name (the relocated rename).
                                    <div class="flex flex-col gap-1.5">
                                        <label class="text-xs font-medium text-foreground/60">
                                            {move || t!(i18n, unlock.vault_display_name)}
                                        </label>
                                        <div class="flex gap-2">
                                            <Input
                                                id="vault-rename"
                                                class="flex-1"
                                                value=Signal::derive(move || name_draft.get())
                                                on_input=Callback::new(move |v: String| {
                                                    name_draft.set(v);
                                                })
                                            />
                                            <Button
                                                variant=Variant::Secondary
                                                size=Size::Sm
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    let t = name_draft.get().trim().to_owned();
                                                    if !t.is_empty() {
                                                        on_rename.run((rename_id.clone(), t));
                                                        close.run(());
                                                    }
                                                }
                                            >
                                                {move || t!(i18n, unlock.vault_rename_save)}
                                            </Button>
                                        </div>
                                    </div>

                                    // Location.
                                    <div class="flex flex-col gap-1">
                                        <span class="text-xs font-medium text-foreground/60">
                                            {move || t!(i18n, unlock.vault_location)}
                                        </span>
                                        <span class="break-all text-[11px] font-jetbrains-mono text-foreground/70">
                                            {path}
                                        </span>
                                    </div>

                                    // Best-effort stats.
                                    <div class="flex gap-5 text-xs text-foreground/70">
                                        {move || {
                                            details
                                                .get()
                                                .map(|d| {
                                                    let size = format_bytes(d.on_disk_bytes);
                                                    view! {
                                                        <span>
                                                            {d.entry_count.to_string()} " "
                                                            {move || t!(i18n, unlock.stat_entries)}
                                                        </span>
                                                        <span>
                                                            {d.snapshot_count.to_string()} " "
                                                            {move || t!(i18n, unlock.stat_snapshots)}
                                                        </span>
                                                        <span>{size}</span>
                                                    }
                                                })
                                        }}
                                    </div>

                                    // Copyable Vault ID — makes `mise keychain-audit` usable.
                                    {uuid
                                        .map(|u| {
                                            let u_copy = u.clone();
                                            view! {
                                                <div class="flex items-center gap-2">
                                                    <span class="text-xs font-medium text-foreground/60">
                                                        {move || t!(i18n, unlock.vault_id)}
                                                    </span>
                                                    <span class="min-w-0 flex-1 truncate text-[11px] font-jetbrains-mono text-foreground/70">
                                                        {u}
                                                    </span>
                                                    <CopyButton
                                                        value=Signal::derive(move || u_copy.clone())
                                                        size=Size::Xs
                                                        label=Signal::derive(move || {
                                                            t_string!(i18n, unlock.vault_id_copy).to_owned()
                                                        })
                                                    />
                                                </div>
                                            }
                                        })}

                                    // Danger zone (prose, not a wall of buttons).
                                    <div class="flex flex-col gap-3 rounded-lg border border-danger/30 p-3">
                                        <p class="text-sm text-foreground/70">
                                            <span class="font-medium text-text-primary">
                                                {move || t!(i18n, unlock.vault_remove)}
                                            </span>
                                            " — "
                                            {move || t!(i18n, unlock.detach_explain)}
                                        </p>
                                        <div>
                                            <Button
                                                variant=Variant::Secondary
                                                size=Size::Sm
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    on_remove.run(remove_id.clone());
                                                    close.run(());
                                                }
                                            >
                                                {move || t!(i18n, unlock.vault_remove)}
                                            </Button>
                                        </div>
                                        <p class="text-sm text-foreground/70">
                                            <span class="font-medium text-danger-text">
                                                {move || t!(i18n, unlock.delete_vault)}
                                            </span>
                                            " — "
                                            {move || t!(i18n, unlock.delete_explain)}
                                        </p>
                                        <div>
                                            <Button
                                                variant=Variant::Danger
                                                size=Size::Sm
                                                attr:data-testid="vault-delete-open"
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    on_delete_request.run(del_row.clone());
                                                }
                                            >
                                                {move || t!(i18n, unlock.delete_vault)}
                                            </Button>
                                        </div>
                                    </div>
                                }
                            })
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// The type-to-confirm delete dialog (Decision ⑤). Unrecoverable — two steps, and it says out
/// loud that `.vbk` backups saved elsewhere survive.
#[component]
pub fn DeleteVaultDialog(
    target: RwSignal<Option<RegisteredVaultStatusDto>>,
    /// The typed-name confirmation buffer (owned by the parent, cleared on close).
    typed: RwSignal<String>,
    details: RwSignal<Option<VaultDetailsDto>>,
    on_confirm: Callback<RegisteredVaultStatusDto>,
) -> impl IntoView {
    let i18n = use_i18n();
    let close = Callback::new(move |()| {
        target.set(None);
        typed.set(String::new());
    });

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, unlock.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, unlock.delete_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div
                    class="flex flex-col gap-4 pb-4 min-w-[22rem]"
                    data-testid="vault-delete-confirm"
                >
                    {move || {
                        target
                            .get()
                            .map(|row| {
                                let name = row.vault.display_name.clone();
                                let gate_name = row.vault.display_name.clone();
                                let confirm_row = row;
                                view! {
                                    <p class="truncate text-sm font-medium text-text-primary">
                                        {name}
                                    </p>
                                    <p class="text-sm text-foreground/70">
                                        {move || {
                                            details.get().map_or(0, |d| d.snapshot_count).to_string()
                                        }} " " {move || t!(i18n, unlock.delete_body)}
                                    </p>
                                    <p class="text-sm text-warning-text">
                                        {move || t!(i18n, unlock.delete_backups_survive)}
                                    </p>
                                    <div class="flex flex-col gap-1.5">
                                        <label
                                            class="text-xs text-foreground/60"
                                            for="vault-delete-input"
                                        >
                                            {move || t!(i18n, unlock.delete_type_hint)}
                                        </label>
                                        <Input
                                            id="vault-delete-input"
                                            value=Signal::derive(move || typed.get())
                                            on_input=Callback::new(move |v: String| typed.set(v))
                                        />
                                    </div>
                                    <div class="flex justify-end gap-2">
                                        <Button
                                            variant=Variant::Ghost
                                            size=Size::Sm
                                            on:click=move |_: web_sys::MouseEvent| close.run(())
                                        >
                                            {move || t!(i18n, unlock.delete_cancel)}
                                        </Button>
                                        {move || {
                                            let disabled = typed.get().trim() != gate_name;
                                            let confirm_row = confirm_row.clone();
                                            view! {
                                                <Button
                                                    variant=Variant::Danger
                                                    size=Size::Sm
                                                    disabled=disabled
                                                    attr:data-testid="vault-delete"
                                                    on:click=move |_: web_sys::MouseEvent| {
                                                        on_confirm.run(confirm_row.clone());
                                                    }
                                                >
                                                    {move || t!(i18n, unlock.delete_vault)}
                                                </Button>
                                            }
                                        }}
                                    </div>
                                }
                            })
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}
