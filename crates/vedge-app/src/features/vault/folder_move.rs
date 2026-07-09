//! Move-to-folder UI: a page-level [`FolderMove`] dialog (driven by a `target`
//! signal set from the row action, the detail button, or the palette command)
//! and a [`FolderSelect`] field bound to `EntryFormData.folder_id` for the
//! create/edit form. Both offer only valid targets — a folder can't be moved into
//! itself or a descendant ([`available_move_targets`]).

use super::entry_form::EntryFormData;
use super::folder_tree::{FlatFolder, FolderNode, available_move_targets, flatten_tree};
use crate::i18n::*;
use leptos::prelude::*;
use std::collections::HashSet;
use vedge_ipc::IndexEntryDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

/// Indent a flattened folder's name for a `Select` option label (options can't
/// carry padding, so pad the text with figure spaces).
fn indent_label(f: &FlatFolder) -> String {
    let pad = "\u{2007}".repeat(f.depth.saturating_mul(2));
    format!("{pad}{}", f.name)
}

/// The move dialog. Shown while `target` is `Some`; picks a destination folder
/// (or "No folder" = root) then fires `on_move((entry_id, destination))`.
#[component]
pub fn FolderMove(
    target: RwSignal<Option<IndexEntryDto>>,
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    on_move: Callback<(String, Option<String>)>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chosen = RwSignal::new(Option::<String>::None);

    // Seed the picker with the entry's current folder each time it opens.
    Effect::new(move |_| {
        if let Some(e) = target.get() {
            chosen.set(e.folder_id.clone());
        }
    });

    let close = Callback::new(move |()| target.set(None));
    let confirm = move |_: web_sys::MouseEvent| {
        if let Some(e) = target.get_untracked() {
            on_move.run((e.id, chosen.get_untracked()));
        }
        target.set(None);
    };

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.folder_move_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-6 min-w-[20rem]">
                    {move || {
                        target
                            .get()
                            .map(|e| {
                                view! {
                                    <p class="text-sm font-medium text-text-primary truncate">
                                        {e.name}
                                    </p>
                                }
                            })
                    }}
                    {move || {
                        let moving_id = target.get().map(|e| e.id).unwrap_or_default();
                        let mut options = vec![
                            SelectItem::option("", t_string!(i18n, vault.folder_none).to_string()),
                        ];
                        options
                            .extend(
                                available_move_targets(&folders.get(), &moving_id)
                                    .iter()
                                    .map(|f| SelectItem::option(f.id.clone(), indent_label(f))),
                            );
                        view! {
                            <Select
                                options=options
                                value=Signal::derive(move || chosen.get().unwrap_or_default())
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, vault.folder_move_dest_aria).to_string()
                                })
                                on_change=Callback::new(move |v: String| {
                                    chosen.set(if v.is_empty() { None } else { Some(v) });
                                })
                            />
                        }
                    }} <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        <Button variant=Variant::Primary size=Size::Sm on:click=confirm>
                            {move || t!(i18n, vault.folder_move_here)}
                        </Button>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// A folder picker bound to `data.folder_id`, mounted beside `EntryForm` in the
/// create form and the detail edit branch. `exclude_id` (the entry being edited,
/// when it's a folder) drops that folder and its descendants so an edit can't
/// build a cycle; `None` (create) offers every folder.
#[component]
pub fn FolderSelect(
    data: RwSignal<EntryFormData>,
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    #[prop(into, default = None)] exclude_id: Option<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let exclude = StoredValue::new(exclude_id);

    view! {
        <div class="col-span-2 flex flex-col gap-1">
            <span class="text-foreground/50 text-xs uppercase tracking-wider">
                {move || t!(i18n, vault.folder_label)}
            </span>
            {move || {
                let nodes = folders.get();
                let targets = match exclude.get_value() {
                    Some(id) => available_move_targets(&nodes, &id),
                    None => flatten_tree(&nodes, &HashSet::new()),
                };
                let mut options = vec![
                    SelectItem::option("", t_string!(i18n, vault.folder_none).to_string()),
                ];
                options
                    .extend(
                        targets.iter().map(|f| SelectItem::option(f.id.clone(), indent_label(f))),
                    );
                view! {
                    <Select
                        options=options
                        value=Signal::derive(move || {
                            data.with(|d| d.folder_id.clone().unwrap_or_default())
                        })
                        aria_label=Signal::derive(move || {
                            t_string!(i18n, vault.folder_label).to_string()
                        })
                        on_change=Callback::new(move |v: String| {
                            data.update(|d| {
                                d.folder_id = if v.is_empty() { None } else { Some(v) };
                            });
                        })
                    />
                }
            }}
        </div>
    }
}
