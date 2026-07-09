//! Add-entry form. Hosts the shared [`EntryFormBody`] (type picker + fields +
//! folder + footer) inline; on Save it builds the wire `PayloadDto` via
//! `EntryFormData::to_payload` and persists it through the backend
//! (`api::entry::create_entry`), then pings the parent to refresh via
//! `on_created` (a `Callback<()>`). Width-capped so the 2-col form doesn't
//! stretch full-monitor-wide.

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_form::{EntryFormData, EntryFormError};
use crate::features::vault::entry_form_body::EntryFormBody;
use crate::features::vault::folder_tree::FolderNode;
use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::EntryTypeDto;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::ToastVariant;

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
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let data = RwSignal::new(EntryFormData::new(EntryTypeDto::Login));
    let submitting = RwSignal::new(false);

    let do_cancel = Callback::new(move |()| {
        data.set(EntryFormData::new(EntryTypeDto::Login));
        show.set(false);
    });

    let do_save = Callback::new(move |()| {
        if submitting.get() {
            return;
        }
        // Read locale-dependent strings in the callback body (reactive owner
        // present); reading them inside `spawn_local` would warn.
        let payload = match data.get().to_payload() {
            Ok(p) => p,
            Err(EntryFormError::NameRequired) => return,
            Err(EntryFormError::InvalidExpiry) => {
                show_error(t_string!(i18n, vault.err_invalid_expiry).to_owned());
                return;
            }
            Err(EntryFormError::UnsupportedType) => {
                show_error(t_string!(i18n, vault.err_save).to_owned());
                return;
            }
        };

        submitting.set(true);
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_save).to_owned();

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
    });

    view! {
        <div class="border border-border rounded-lg p-4 bg-primary-muted max-w-2xl">
            <h3 class="text-sm font-semibold mb-3 text-text-secondary">
                {move || t!(i18n, vault.create_title)}
            </h3>

            <EntryFormBody
                data=data
                folders=folders
                show_type_picker=true
                saving=submitting
                on_save=do_save
                on_cancel=do_cancel
            />
        </div>
    }
}
