//! The shared create/edit form surface. Wraps the per-type [`EntryForm`] field
//! grid with an optional type picker (create only — an entry's type is fixed
//! once created), the [`FolderSelect`] placement picker, and a Cancel/Save
//! footer. The **create** flow renders it inline (`VaultCreateForm`); the
//! **edit** flow hosts it in a roomy `Dialog` (`VaultDetail`) so multi-field
//! types (Identity, SSH, Card) aren't squeezed into the 320px detail rail.
//!
//! It owns no persistence — the host passes `on_save`/`on_cancel` callbacks and
//! a `saving` flag; the footer disables + spins Save while a save is in flight
//! or the name is blank.

use crate::features::vault::entry_form::{
    EntryForm, EntryFormData, editable_types, type_from_key, type_to_key,
};
use crate::features::vault::entry_view::type_label_i18n;
use crate::features::vault::folder_move::FolderSelect;
use crate::features::vault::folder_tree::FolderNode;
use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use vedge_ui::components::Button;
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::{Size, Variant};

#[component]
pub fn EntryFormBody(
    /// The single form model, shared with [`EntryForm`] and [`FolderSelect`].
    data: RwSignal<EntryFormData>,
    /// Folder catalog for the placement picker.
    #[prop(into)]
    folders: Signal<Vec<FolderNode>>,
    /// Exclude this folder (+ descendants) from its own picker so an edit can't
    /// build a cycle. `None` (create) offers every folder.
    #[prop(into, default = None)]
    exclude_id: Option<String>,
    /// Show the entry-type picker (create only — type is immutable after create).
    #[prop(optional)]
    show_type_picker: bool,
    /// Save-in-flight flag; drives the footer spinner + disabled state.
    saving: RwSignal<bool>,
    /// Fired when the user confirms Save (host persists + closes).
    on_save: Callback<()>,
    /// Fired when the user cancels (host resets + closes).
    on_cancel: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        {show_type_picker
            .then(|| {
                view! {
                    <div class="mb-3">
                        // Built inside a reactive closure so the locale read is tracked
                        // (no owner-less warning) and the labels relocalize on switch.
                        {move || {
                            let options: Vec<SelectItem> = editable_types()
                                .iter()
                                .map(|ty| {
                                    SelectItem::option(type_to_key(ty), type_label_i18n(i18n, ty))
                                })
                                .collect();
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        data.with(|d| type_to_key(&d.entry_type).to_owned())
                                    })
                                    placeholder=Signal::derive(move || {
                                        t_string!(i18n, vault.type_picker).to_owned()
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.type_picker_aria).to_owned()
                                    })
                                    on_change=Callback::new(move |key: String| {
                                        data.update(|d| *d = d.switch_type(type_from_key(&key)));
                                    })
                                />
                            }
                        }}
                    </div>
                }
            })}

        <EntryForm data=data />

        <div class="grid grid-cols-2 gap-3 mt-3">
            <FolderSelect data=data folders=folders exclude_id=exclude_id />
        </div>

        <div class="flex gap-2 justify-end mt-4">
            <Button
                variant=Variant::Ghost
                size=Size::Sm
                attr:data-testid="entry-cancel"
                on:click=move |_: web_sys::MouseEvent| on_cancel.run(())
            >
                {move || t!(i18n, vault.cancel)}
            </Button>
            {move || {
                let is_saving = saving.get();
                let busy = is_saving || data.with(|d| d.name.trim().is_empty());
                view! {
                    <Button
                        variant=Variant::Primary
                        size=Size::Sm
                        disabled=busy
                        loading=is_saving
                        attr:data-testid="entry-save"
                        on:click=move |_: web_sys::MouseEvent| on_save.run(())
                    >
                        {move || t!(i18n, vault.save)}
                    </Button>
                }
            }}
        </div>
    }
}
