//! Entry detail panel. In **read** mode it shows metadata (`IndexEntryDto`
//! carries no secrets) plus type-appropriate copy actions — copying routes
//! through the backend `copy_field` command (`on_copy`), so plaintext lands on
//! the OS clipboard, never in the renderer. In **edit** mode it reveals the
//! full payload via `api::entry::get_entry` (the sanctioned secret-reveal
//! path), seeds the shared [`EntryForm`], and saves through
//! `api::entry::update_entry`; `on_saved` pings the parent to refresh.

use super::entry_form::{EntryFormData, EntryFormError};
use super::entry_form_body::EntryFormBody;
use super::entry_view::{human_size, short_date, type_label_i18n};
use super::folder_tree::{FolderNode, folder_display_name, folder_icon_from_key, folder_style};
use super::tag_assign::TagAssign;
use crate::api;
use crate::api::dialog::SaveDialogOptions;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::ui_state::VaultUiState;
use crate::i18n::*;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::I18nContext;
use vedge_ipc::{
    DocumentPayloadDto, EntryTypeDto, FieldSelectorDto, IndexEntryDto, PayloadDto, TagMetaDto,
};
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn VaultDetail(
    entry: IndexEntryDto,
    /// Ordered tag catalog (chip resolution + the tag widget's "add" list).
    #[prop(into)]
    tags: Signal<Vec<TagMetaDto>>,
    on_copy: Callback<FieldSelectorDto>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
    /// `(entry_id, next_state)` — flip this entry's favorite flag.
    on_favorite: Callback<(String, bool)>,
    /// `(entry_id, new_tag_ids)` — persist this entry's tag assignments.
    on_tags: Callback<(String, Vec<String>)>,
    /// Reload the tag catalog (after an inline tag create).
    on_catalog: Callback<()>,
    /// Folder catalog for the "Folder" row + the edit-mode folder picker.
    #[prop(into)]
    folders: Signal<Vec<FolderNode>>,
    /// Open the move-to-folder picker for this entry.
    on_move_request: Callback<IndexEntryDto>,
    /// Open the version-history panel for this entry.
    on_history_request: Callback<IndexEntryDto>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let ui = expect_context::<VaultUiState>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let entry_id = StoredValue::new(entry.id.clone());
    let fav_id = entry.id.clone();
    let is_fav = entry.is_favorite;
    let entry_for_move = entry.clone();
    let folder_of = entry.folder_id.clone();
    // Exclude this folder (+ its descendants) from its own parent picker so an
    // edit can't build a cycle; non-folder entries get every folder as a target.
    // `StoredValue` so the edit `Dialog`'s (re-callable) body can clone it.
    let move_exclude = StoredValue::new(
        (entry.entry_type == EntryTypeDto::Folder).then(|| entry.id.clone()),
    );
    let tag_entry_id = entry.id.clone();
    let tag_ids = entry.tag_ids.clone();
    let name = entry.name.clone();
    let url = entry.url.clone();
    let updated = short_date(&entry.updated_at);
    let created = short_date(&entry.created_at);
    let copy_type = entry.entry_type.clone();
    let label_type = entry.entry_type.clone();
    let is_editable = !matches!(
        entry.entry_type,
        EntryTypeDto::Document | EntryTypeDto::Unknown(_)
    );
    // Version history is the ✅-Full set from the spec: the field-editable,
    // secret-bearing types. Folder / Unknown are excluded (trivial / not
    // editable); Document is too — it never routes through the `update_entry`
    // use-case (non-editable), so it accumulates no snapshots, and restoring old
    // metadata would carry a stale blob nonce.
    let has_history = !matches!(
        entry.entry_type,
        EntryTypeDto::Folder | EntryTypeDto::Document | EntryTypeDto::Unknown(_)
    );
    let entry_for_history = entry.clone();
    let type_lbl = Signal::derive(move || type_label_i18n(i18n, &label_type));

    let editing = RwSignal::new(false);
    let saving = RwSignal::new(false);
    let form = RwSignal::new(EntryFormData::new(EntryTypeDto::Login));

    // Document detail: reveal metadata (filename / mime / size) once on open so
    // the read view can show it and the export dialog can default to the real
    // filename. Metadata only — the blob bytes stay in the sidecar, so no
    // plaintext file bytes reach WASM. (`get_entry` audits this as `Viewed`.)
    let is_document = matches!(entry.entry_type, EntryTypeDto::Document);
    let doc_meta = RwSignal::new(Option::<DocumentPayloadDto>::None);
    let doc_name_default = StoredValue::new(entry.name.clone());
    if is_document {
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        spawn_local(async move {
            match api::entry::get_entry(&vault_path, &id).await {
                Ok(PayloadDto::Document(p)) => doc_meta.set(Some(p)),
                Ok(_) => {}
                Err(e) => web_sys::console::error_1(
                    &format!("document metadata reveal failed: {e:?}").into(),
                ),
            }
        });
    }

    // Reveal the full payload (the sanctioned secret-reveal path) then enter edit
    // mode. Called from the Edit button *and* from the command palette via
    // `ui.edit_request` (the effect below), so reads are `untrack`ed — inside the
    // effect a tracked read of `active.path`/locale would wrongly re-subscribe it.
    let reveal_and_edit = move || {
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_reveal).to_string());
        spawn_local(async move {
            match api::entry::get_entry(&vault_path, &id).await {
                Ok(payload) => {
                    form.set(EntryFormData::from_payload(&payload));
                    editing.set(true);
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    };
    let start_edit = move |_: web_sys::MouseEvent| reveal_and_edit();

    // The command palette's "Edit" bumps `ui.edit_request`; enter edit mode when
    // it changes for an editable entry. The `prev`-value guard skips the initial
    // mount (and any stale bump captured when a new entry is selected).
    Effect::new(move |prev: Option<u32>| {
        let cur = ui.edit_request.get();
        if let Some(p) = prev {
            if cur != p && is_editable {
                reveal_and_edit();
            }
        }
        cur
    });

    // Cancel/Save are `Callback<()>` so the shared `EntryFormBody` footer (and the
    // dialog's scrim/Escape close, via `on_close`) can drive them.
    let do_cancel = Callback::new(move |()| editing.set(false));

    let do_save = Callback::new(move |()| {
        if saving.get() {
            return;
        }
        let payload = match form.get().to_payload() {
            Ok(p) => p,
            Err(EntryFormError::NameRequired) => return,
            Err(EntryFormError::InvalidExpiry) => {
                show_error(t_string!(i18n, vault.err_invalid_expiry).to_string());
                return;
            }
            Err(EntryFormError::UnsupportedType) => {
                show_error(t_string!(i18n, vault.err_update).to_string());
                return;
            }
        };
        saving.set(true);
        let vault_path = active.path.get().unwrap_or_default();
        let id = entry_id.get_value();
        let err_prefix = t_string!(i18n, vault.err_update).to_string();
        spawn_local(async move {
            match api::entry::update_entry(&vault_path, &id, &payload).await {
                Ok(()) => {
                    editing.set(false);
                    on_saved.run(());
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            saving.set(false);
        });
    });

    let export_doc = move |_: web_sys::MouseEvent| {
        // Read signals + locale strings in the handler body (owner present);
        // reading them inside `spawn_local` would warn.
        let vault_path = active.path.get().unwrap_or_default();
        let id = entry_id.get_value();
        let default_name = doc_meta
            .get()
            .map(|p| p.filename)
            .unwrap_or_else(|| doc_name_default.get_value());
        let dialog_title = t_string!(i18n, vault.export).to_string();
        let err_prefix = t_string!(i18n, vault.err_export).to_string();
        spawn_local(async move {
            let opts = SaveDialogOptions {
                title: Some(dialog_title),
                default_path: Some(default_name),
                filters: vec![],
            };
            match api::dialog::save(&opts).await {
                Ok(Some(dest)) => {
                    if let Err(e) =
                        api::document::export_document_to_path(&vault_path, &id, &dest).await
                    {
                        show_error(format!("{err_prefix}{e}"));
                    }
                }
                Ok(None) => {}
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    };

    view! {
        <aside class="w-80 min-w-0 border border-border rounded-lg p-4 bg-primary-muted overflow-auto">
            <div class="flex items-center justify-between mb-3">
                <h3 class="text-sm font-semibold text-text-secondary">
                    {move || t!(i18n, vault.detail_title)}
                </h3>
                <div class="flex items-center gap-1">
                    <IconButton
                        aria_label=Signal::derive(move || {
                            if is_fav {
                                t_string!(i18n, vault.unfavorite).to_string()
                            } else {
                                t_string!(i18n, vault.favorite).to_string()
                            }
                        })
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |_: web_sys::MouseEvent| {
                            on_favorite.run((fav_id.clone(), !is_fav));
                        }
                    >
                        {if is_fav {
                            Either::Left(view! { <Icon attr:aria-hidden="true" icon=i::FaStarSolid /> })
                        } else {
                            Either::Right(view! { <Icon attr:aria-hidden="true" icon=i::FaStarRegular /> })
                        }}
                    </IconButton>
                    {has_history.then(|| view! {
                        <IconButton
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, vault.version_history).to_string()
                            })
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| {
                                on_history_request.run(entry_for_history.clone());
                            }
                        >
                            <Icon attr:aria-hidden="true" icon=i::FaClockRotateLeftSolid />
                        </IconButton>
                    })}
                    <IconButton
                        aria_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |_: web_sys::MouseEvent| on_close.run(())
                    >
                        <span aria-hidden="true">"✕"</span>
                    </IconButton>
                </div>
            </div>

            {move || {
                // Edit now happens in the roomy `<Dialog>` below; while editing,
                // the compact read aside collapses to just its header.
                if editing.get() {
                    Either::Left(view! {})
                } else {
                    let name = name.clone();
                    let url = url.clone();
                    let updated = updated.clone();
                    let created = created.clone();
                    let copy_type = copy_type.clone();
                    let tag_ids = tag_ids.clone();
                    let tag_entry_id = tag_entry_id.clone();
                    let folder_of = folder_of.clone();
                    let entry_for_move = entry_for_move.clone();
                    Either::Right(view! {
                        <dl class="flex flex-col gap-2 text-sm">
                            <div>
                                <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                    {move || t!(i18n, vault.col_name)}
                                </dt>
                                <dd class="text-text-primary">{name}</dd>
                            </div>
                            <div>
                                <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                    {move || t!(i18n, vault.col_type)}
                                </dt>
                                <dd class="text-text-primary">{type_lbl}</dd>
                            </div>
                            {url.map(|u| {
                                let href = u.clone();
                                view! {
                                    <div>
                                        <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                            {move || t!(i18n, vault.col_url)}
                                        </dt>
                                        <dd>
                                            <a
                                                class="text-primary hover:underline font-jetbrains-mono break-all"
                                                href=href
                                                target="_blank"
                                                rel="noreferrer"
                                            >
                                                {u}
                                            </a>
                                        </dd>
                                    </div>
                                }
                            })}
                            <div>
                                <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                    {move || t!(i18n, vault.col_updated)}
                                </dt>
                                <dd class="text-foreground/70">{updated}</dd>
                            </div>
                            <div>
                                <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                    {move || t!(i18n, vault.col_created)}
                                </dt>
                                <dd class="text-foreground/70">{created}</dd>
                            </div>
                            <div>
                                <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                    {move || t!(i18n, vault.folder_label)}
                                </dt>
                                <dd class="text-foreground/70">
                                    {move || match folder_of.as_deref() {
                                        Some(id) => {
                                            let nodes = folders.get();
                                            let name = folder_display_name(&nodes, id)
                                                .unwrap_or_else(|| {
                                                    t_string!(i18n, vault.folder_none).to_string()
                                                });
                                            let (color, icon) = folder_style(&nodes, id);
                                            let icon_data = folder_icon_from_key(
                                                icon.as_deref().unwrap_or_default(),
                                            );
                                            Either::Left(
                                                view! {
                                                    <span class="flex items-center gap-1.5">
                                                        <span
                                                            class="flex shrink-0 text-foreground/50"
                                                            style:color=color.unwrap_or_default()
                                                        >
                                                            <Icon attr:aria-hidden="true" icon=icon_data width="12" height="12" />
                                                        </span>
                                                        {name}
                                                    </span>
                                                },
                                            )
                                        }
                                        None => {
                                            Either::Right(
                                                view! {
                                                    <span>{t_string!(i18n, vault.folder_none).to_string()}</span>
                                                },
                                            )
                                        }
                                    }}
                                </dd>
                            </div>
                            {move || {
                                is_document
                                    .then(|| doc_meta.get())
                                    .flatten()
                                    .map(|p| view! {
                                        <div>
                                            <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                                {move || t!(i18n, vault.document_filename)}
                                            </dt>
                                            <dd class="text-text-primary font-jetbrains-mono break-all">
                                                {p.filename.clone()}
                                            </dd>
                                        </div>
                                        <div>
                                            <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                                {move || t!(i18n, vault.document_type)}
                                            </dt>
                                            <dd class="text-foreground/70 font-jetbrains-mono break-all">
                                                {p.mime_type.clone()}
                                            </dd>
                                        </div>
                                        <div>
                                            <dt class="text-foreground/50 text-xs uppercase tracking-wider">
                                                {move || t!(i18n, vault.document_size)}
                                            </dt>
                                            <dd class="text-foreground/70">{human_size(p.size_bytes)}</dd>
                                        </div>
                                    })
                            }}
                        </dl>

                        <div class="flex flex-col gap-2 mt-4">
                            {copy_buttons(i18n, copy_type, on_copy)}
                            <Button
                                variant=Variant::Secondary
                                size=Size::Sm
                                full_width=true
                                on:click=move |_: web_sys::MouseEvent| {
                                    on_move_request.run(entry_for_move.clone());
                                }
                            >
                                {move || t!(i18n, vault.folder_move)}
                            </Button>
                            {is_editable.then(|| view! {
                                <Button
                                    variant=Variant::Secondary
                                    size=Size::Sm
                                    full_width=true
                                    on:click=start_edit
                                >
                                    {move || t!(i18n, vault.edit)}
                                </Button>
                            })}
                            {is_document.then(|| view! {
                                <Button
                                    variant=Variant::Primary
                                    size=Size::Sm
                                    full_width=true
                                    on:click=export_doc
                                >
                                    {move || t!(i18n, vault.export)}
                                </Button>
                            })}
                        </div>

                        // Tagging lives here (read view) so it works for any
                        // type — Document included — without entering edit mode.
                        <div class="mt-4 pt-4 border-t border-secondary/15">
                            <TagAssign
                                entry_id=tag_entry_id
                                initial=tag_ids
                                catalog=tags
                                on_tags=on_tags
                                on_catalog=on_catalog
                            />
                        </div>
                    })
                }
            }}
        </aside>

        // Roomy shared edit surface — the same fields as the create form, hosted
        // in a large dialog so multi-field types (Identity/SSH/Card) aren't
        // squeezed into the 320px read rail. Scrim/Escape close = cancel.
        <Dialog
            open=Signal::derive(move || editing.get())
            on_close=do_cancel
            size=DialogSize::Lg
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>
                    {move || t!(i18n, vault.edit)}
                    <span class="ml-2 inline-flex items-center text-xs font-medium text-primary bg-primary/10 px-2 py-0.5 rounded">
                        {move || type_lbl.get()}
                    </span>
                </DialogTitle>
            </DialogHeader>
            <DialogBody>
                <EntryFormBody
                    data=form
                    folders=folders
                    exclude_id=move_exclude.get_value()
                    saving=saving
                    on_save=do_save
                    on_cancel=do_cancel
                />
            </DialogBody>
        </Dialog>
    }
}

/// Type-appropriate copy buttons for the read view. Only fields with a
/// `FieldSelectorDto` variant + backend `copy_field` support are offered.
fn copy_buttons(
    i18n: I18nContext<Locale>,
    entry_type: EntryTypeDto,
    on_copy: Callback<FieldSelectorDto>,
) -> impl IntoView {
    match entry_type {
        EntryTypeDto::Login => Some(
            view! {
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::Username)
                >
                    {move || t!(i18n, vault.copy_username)}
                </Button>
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::Password)
                >
                    {move || t!(i18n, vault.copy_password)}
                </Button>
            }
            .into_any(),
        ),
        EntryTypeDto::Card => Some(
            view! {
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::CardNumber)
                >
                    {move || t!(i18n, vault.copy_card_number)}
                </Button>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::Cvv)
                >
                    {move || t!(i18n, vault.copy_cvv)}
                </Button>
            }
            .into_any(),
        ),
        EntryTypeDto::ApiKey => Some(
            view! {
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_copy.run(FieldSelectorDto::ApiKey)
                >
                    {move || t!(i18n, vault.copy_api_key)}
                </Button>
            }
            .into_any(),
        ),
        _ => None,
    }
}
