//! Add-entry form. A type picker selects one of the eight editable entry
//! types; [`EntryForm`] renders that type's fields over a single
//! `RwSignal<EntryFormData>`. On submit it builds the wire `PayloadDto` via
//! `EntryFormData::to_payload` and persists it through the backend
//! (`api::entry::create_entry`), then pings the parent to refresh via
//! `on_created` (a `Callback<()>`).

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_form::{
    EntryForm, EntryFormData, EntryFormError, editable_types, type_from_key, type_to_key,
};
use crate::features::vault::entry_view::type_label_i18n;
use crate::features::vault::folder_move::FolderSelect;
use crate::features::vault::folder_tree::FolderNode;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::EntryTypeDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};

#[component]
pub fn VaultCreateForm(
    show: RwSignal<bool>,
    on_created: Callback<()>,
    /// Folder catalog for the folder picker.
    #[prop(into)]
    folders: Signal<Vec<FolderNode>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let data = RwSignal::new(EntryFormData::new(EntryTypeDto::Login));
    let submitting = RwSignal::new(false);

    let handle_cancel = move |_| {
        data.set(EntryFormData::new(EntryTypeDto::Login));
        show.set(false);
    };

    let handle_save = move |_| {
        if submitting.get() {
            return;
        }
        // Read locale-dependent strings in the handler body (reactive owner
        // present); reading them inside `spawn_local` would warn.
        let payload = match data.get().to_payload() {
            Ok(p) => p,
            Err(EntryFormError::NameRequired) => return,
            Err(EntryFormError::InvalidExpiry) => {
                show_error(t_string!(i18n, vault.err_invalid_expiry).to_string());
                return;
            }
            Err(EntryFormError::UnsupportedType) => {
                show_error(t_string!(i18n, vault.err_save).to_string());
                return;
            }
        };

        submitting.set(true);
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_save).to_string();

        spawn_local(async move {
            match api::entry::create_entry(&vault_path, &payload).await {
                Ok(_id) => {
                    data.set(EntryFormData::new(EntryTypeDto::Login));
                    show.set(false);
                    on_created.run(());
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            submitting.set(false);
        });
    };

    view! {
        <div class="border border-border rounded-lg p-4 bg-primary-muted">
            <h3 class="text-sm font-semibold mb-3 text-text-secondary">
                {move || t!(i18n, vault.create_title)}
            </h3>

            <div class="mb-3">
                // Built inside a reactive closure so the locale read is tracked
                // (no owner-less warning) and the labels relocalize on switch.
                {move || {
                    let options: Vec<SelectItem> = editable_types()
                        .iter()
                        .map(|ty| SelectItem::option(type_to_key(ty), type_label_i18n(i18n, ty)))
                        .collect();
                    view! {
                        <Select
                            options=options
                            value=Signal::derive(move || data.with(|d| type_to_key(&d.entry_type).to_owned()))
                            placeholder=Signal::derive(move || t_string!(i18n, vault.type_picker).to_string())
                            aria_label=Signal::derive(move || t_string!(i18n, vault.type_picker_aria).to_string())
                            on_change=Callback::new(move |key: String| {
                                data.update(|d| *d = d.switch_type(type_from_key(&key)));
                            })
                        />
                    }
                }}
            </div>

            <div class="grid grid-cols-2 gap-3 mt-3">
                <FolderSelect data=data folders=folders />
            </div>

            <EntryForm data=data />

            <div class="flex gap-2 justify-end mt-3">
                <Button variant=Variant::Ghost size=Size::Sm on:click=handle_cancel>
                    {move || t!(i18n, vault.cancel)}
                </Button>
                {move || {
                    let saving = submitting.get();
                    let busy = saving || data.with(|d| d.name.trim().is_empty());
                    view! {
                        <Button
                            variant=Variant::Primary
                            size=Size::Sm
                            disabled=busy
                            loading=saving
                            on:click=handle_save
                        >
                            {move || t!(i18n, vault.save)}
                        </Button>
                    }
                }}
            </div>
        </div>
    }
}
