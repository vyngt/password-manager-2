//! `Ctrl/⌘-K` command palette — the vault's keyboard entry point.
//!
//! A `Dialog`-based modal (Escape + focus-trap + overlay come from the shared
//! [`Dialog`](vedge_ui::components::feedback::Dialog)) with an autofocused search
//! box and a keyboard-navigable results list. Results are two groups filtered by
//! the query: **actions** (New / Lock, plus Edit / Copy when an entry is
//! selected) and **entries** (quick-open, filtered with 2.2's
//! [`query_matches`]). It drives the shared [`VaultUiState`] (open flag,
//! selection, create-toggle, edit-request) — the first app-wide shortcut surface
//! later slices hang commands on.
//!
//! Rows are modelled as a flat [`PaletteRow`] enum dispatched through a single
//! `Copy` `run_row` closure (rather than a `Callback` per command) to keep the
//! list cheap to rebuild on each keystroke. Locale/signal reads all happen in
//! reactive closures or event-handler bodies (owner rule).

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::ui_state::VaultUiState;
use crate::features::vault::vault_filters::query_matches;
use crate::i18n::*;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::collections::HashMap;
use std::time::Duration;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::feedback::{Dialog, DialogBody};
use vedge_ui::primitives::tokens::DialogSize;
use wasm_bindgen::JsCast;

/// Upper bound on quick-open rows so a large vault doesn't blow up the DOM.
const PALETTE_ENTRY_CAP: usize = 50;

/// The selectable actions (entries are handled separately).
#[derive(Clone, Copy, PartialEq)]
enum ActionId {
    NewEntry,
    Lock,
    Edit,
    CopyPassword,
    CopyUsername,
}

/// One rendered/selectable palette row. Entries carry only what the palette
/// needs (id to select, name to show) — not the whole `IndexEntryDto`.
#[derive(Clone)]
enum PaletteRow {
    Action { id: ActionId, label: String },
    Entry { id: String, name: String },
}

// --- pure helpers (host-tested) ---------------------------------------------

/// Case-insensitive substring match of a command `label` against `query`.
/// An empty/whitespace query matches everything.
#[must_use]
pub fn matches_command(label: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || label.to_lowercase().contains(&q)
}

/// Whether the selection-dependent actions (Edit / Copy) are available.
#[must_use]
pub fn selection_actions_enabled(selected: Option<&str>) -> bool {
    selected.is_some()
}

/// Whether focus being in an element of tag `tag` (or `content_editable`) should
/// suppress the global shortcut. `tag` may be any case (DOM `tagName` is upper).
#[must_use]
pub fn is_text_entry(tag: &str, content_editable: bool) -> bool {
    content_editable
        || matches!(
            tag.to_ascii_lowercase().as_str(),
            "input" | "textarea" | "select"
        )
}

/// Runtime check used by the global `Ctrl/⌘-K` listener: is the user currently
/// typing in a form field? Reads the active element from the DOM.
#[must_use]
pub fn typing_in_field() -> bool {
    let Some(el) = document().active_element() else {
        return false;
    };
    let content_editable = el
        .dyn_ref::<web_sys::HtmlElement>()
        .is_some_and(web_sys::HtmlElement::is_content_editable);
    is_text_entry(&el.tag_name(), content_editable)
}

/// Non-Document/Unknown entries can be edited (mirrors `vault_detail`).
fn is_editable(ty: &EntryTypeDto) -> bool {
    !matches!(ty, EntryTypeDto::Document | EntryTypeDto::Unknown(_))
}

/// Copy a field of the selected entry through the backend clipboard.
fn copy_selected(active: ActiveVault, ui: VaultUiState, field: FieldSelectorDto) {
    let vault_path = active.path.get_untracked().unwrap_or_default();
    let Some(id) = ui.selected_id.get_untracked() else {
        return;
    };
    spawn_local(async move {
        let _ = api::entry::copy_field(&vault_path, &id, &field, Some(30)).await;
    });
}

/// Scroll the option at `idx` into view within the results list.
fn scroll_into_view(list_ref: NodeRef<leptos::html::Div>, idx: usize) {
    let Some(list) = list_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::HtmlElement = &list;
    if let Ok(nodes) = el.query_selector_all("[role=\"option\"]") {
        if let Some(node) = nodes.item(idx as u32) {
            if let Ok(opt) = node.dyn_into::<web_sys::HtmlElement>() {
                opt.scroll_into_view_with_bool(false);
            }
        }
    }
}

#[component]
pub fn CommandPalette() -> impl IntoView {
    let i18n = use_i18n();
    let ui = expect_context::<VaultUiState>();
    let active = expect_context::<ActiveVault>();

    let query = RwSignal::new(String::new());
    let entries = RwSignal::new(Vec::<IndexEntryDto>::new());
    let tags = RwSignal::new(Vec::<TagMetaDto>::new());
    let highlighted = RwSignal::new(0usize);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let list_ref = NodeRef::<leptos::html::Div>::new();

    // On open: reset, fetch the index + tags for filtering, focus the input.
    Effect::new(move |_| {
        if !ui.palette_open.get() {
            return;
        }
        query.set(String::new());
        highlighted.set(0);
        let vault_path = untrack(|| active.path.get()).unwrap_or_default();
        if !vault_path.is_empty() {
            let tags_path = vault_path.clone();
            spawn_local(async move {
                if let Ok(list) = api::vault::list_tags(&tags_path).await {
                    tags.set(list);
                }
                if let Ok(list) = api::vault::list_entries(&vault_path).await {
                    entries.set(list);
                }
            });
        }
        // Belt-and-suspenders focus (the Dialog also autofocuses its first
        // focusable child, which is this input).
        set_timeout(
            move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.focus();
                }
            },
            Duration::from_millis(20),
        );
    });

    // Build the flat, ordered (actions-first) row list from the current query,
    // loaded entries/tags, and selection. `Copy` so both the view and the
    // keydown handler can call it.
    let build_rows = move || -> Vec<PaletteRow> {
        let q = query.get();
        let all = entries.get();
        let sel = ui.selected_id.get();
        let sel_entry = sel
            .as_ref()
            .and_then(|id| all.iter().find(|e| &e.id == id).cloned());

        let mut actions: Vec<(ActionId, String)> = vec![
            (
                ActionId::NewEntry,
                t_string!(i18n, vault.cmd_new_entry).to_string(),
            ),
            (ActionId::Lock, t_string!(i18n, vault.cmd_lock).to_string()),
        ];
        if selection_actions_enabled(sel.as_deref())
            && let Some(e) = &sel_entry
        {
            if is_editable(&e.entry_type) {
                actions.push((ActionId::Edit, t_string!(i18n, vault.cmd_edit).to_string()));
            }
            if e.entry_type == EntryTypeDto::Login {
                actions.push((
                    ActionId::CopyPassword,
                    t_string!(i18n, vault.cmd_copy_password).to_string(),
                ));
                actions.push((
                    ActionId::CopyUsername,
                    t_string!(i18n, vault.cmd_copy_username).to_string(),
                ));
            }
        }

        let mut rows: Vec<PaletteRow> = actions
            .into_iter()
            .filter(|(_, label)| matches_command(label, &q))
            .map(|(id, label)| PaletteRow::Action { id, label })
            .collect();

        let ql = q.trim().to_lowercase();
        let tag_map: HashMap<String, String> =
            tags.get().into_iter().map(|t| (t.id, t.name)).collect();
        rows.extend(
            all.iter()
                .filter(|e| query_matches(e, &ql, &tag_map))
                .take(PALETTE_ENTRY_CAP)
                .map(|e| PaletteRow::Entry {
                    id: e.id.clone(),
                    name: e.name.clone(),
                }),
        );
        rows
    };

    // Run the selected row, then close the palette. `Copy` (captures only Copy
    // signals; `use_navigate()` is called inline, like `LockButton`).
    let run_row = move |row: PaletteRow| {
        match row {
            PaletteRow::Action { id, .. } => match id {
                ActionId::NewEntry => {
                    ui.show_create.set(true);
                    use_navigate()("/v/vault", Default::default());
                }
                ActionId::Lock => {
                    let path = active.path.get_untracked();
                    let nav = use_navigate();
                    spawn_local(async move {
                        if let Some(p) = path {
                            let _ = api::vault::lock(&p).await;
                        }
                        active.path.set(None);
                        nav("/", Default::default());
                    });
                }
                ActionId::Edit => {
                    ui.edit_request.update(|n| *n = n.wrapping_add(1));
                }
                ActionId::CopyPassword => copy_selected(active, ui, FieldSelectorDto::Password),
                ActionId::CopyUsername => copy_selected(active, ui, FieldSelectorDto::Username),
            },
            PaletteRow::Entry { id, .. } => {
                ui.selected_id.set(Some(id));
                use_navigate()("/v/vault", Default::default());
            }
        }
        ui.palette_open.set(false);
    };

    let on_key = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "ArrowDown" => {
            ev.prevent_default();
            let len = build_rows().len();
            if len > 0 {
                highlighted.update(|h| *h = (*h + 1) % len);
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "ArrowUp" => {
            ev.prevent_default();
            let len = build_rows().len();
            if len > 0 {
                highlighted.update(|h| *h = if *h == 0 { len - 1 } else { *h - 1 });
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "Enter" => {
            ev.prevent_default();
            let rows = build_rows();
            if rows.is_empty() {
                return;
            }
            let idx = highlighted.get().min(rows.len() - 1);
            if let Some(row) = rows.get(idx) {
                run_row(row.clone());
            }
        }
        _ => {}
    };

    view! {
        <Dialog
            open=Signal::derive(move || ui.palette_open.get())
            on_close=Callback::new(move |()| ui.palette_open.set(false))
            size=DialogSize::Sm
        >
            <DialogBody>
                <input
                    node_ref=input_ref
                    class="w-full bg-transparent text-text-primary text-base px-1 py-2 outline-none border-b border-border"
                    type="text"
                    autocomplete="off"
                    spellcheck="false"
                    aria-label=move || t_string!(i18n, vault.palette_placeholder).to_string()
                    placeholder=move || t_string!(i18n, vault.palette_placeholder).to_string()
                    prop:value=move || query.get()
                    on:input:target=move |ev| {
                        query.set(ev.target().value());
                        highlighted.set(0);
                    }
                    on:keydown=on_key
                />
                <div node_ref=list_ref role="listbox" class="mt-2 max-h-80 overflow-auto flex flex-col">
                    {move || {
                        let rows = build_rows();
                        if rows.is_empty() {
                            return Either::Left(view! {
                                <div class="px-3 py-6 text-center text-sm text-foreground/40">
                                    {move || t!(i18n, vault.palette_empty)}
                                </div>
                            });
                        }
                        let n_actions = rows
                            .iter()
                            .take_while(|r| matches!(r, PaletteRow::Action { .. }))
                            .count();
                        let views = rows
                            .into_iter()
                            .enumerate()
                            .map(|(idx, row)| {
                                let show_actions_heading =
                                    idx == 0 && matches!(row, PaletteRow::Action { .. });
                                let show_entries_heading =
                                    idx == n_actions && matches!(row, PaletteRow::Entry { .. });
                                let label = match &row {
                                    PaletteRow::Action { label, .. } => label.clone(),
                                    PaletteRow::Entry { name, .. } => name.clone(),
                                };
                                let is_hl = move || highlighted.get() == idx;
                                let row_for_click = row.clone();
                                view! {
                                    {show_actions_heading.then(|| view! {
                                        <div class="px-1 pt-2 pb-1 text-xs uppercase tracking-wider text-foreground/40">
                                            {move || t!(i18n, vault.palette_actions)}
                                        </div>
                                    })}
                                    {show_entries_heading.then(|| view! {
                                        <div class="px-1 pt-3 pb-1 text-xs uppercase tracking-wider text-foreground/40">
                                            {move || t!(i18n, vault.palette_entries)}
                                        </div>
                                    })}
                                    <div
                                        role="option"
                                        aria-selected=move || is_hl().then_some("true")
                                        class="px-3 py-2 rounded cursor-pointer text-sm text-text-primary truncate"
                                        class=("bg-primary/10", is_hl)
                                        on:mouseenter=move |_: web_sys::MouseEvent| highlighted.set(idx)
                                        on:click=move |_: web_sys::MouseEvent| run_row(row_for_click.clone())
                                    >
                                        {label}
                                    </div>
                                }
                            })
                            .collect_view();
                        Either::Right(views)
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::{is_text_entry, matches_command, selection_actions_enabled};

    #[test]
    fn palette_filters_actions_by_query() {
        assert!(matches_command("New entry", "")); // empty ⇒ all
        assert!(matches_command("New entry", "  ")); // whitespace ⇒ all
        assert!(matches_command("Lock vault", "lock")); // case-insensitive substring
        assert!(matches_command("Lock vault", "VAULT"));
        assert!(!matches_command("Lock vault", "copy"));
    }

    #[test]
    fn edit_action_disabled_without_selection() {
        assert!(!selection_actions_enabled(None));
        assert!(selection_actions_enabled(Some("id-1")));
    }

    #[test]
    fn typing_in_field_classification() {
        assert!(is_text_entry("input", false));
        assert!(is_text_entry("textarea", false));
        assert!(is_text_entry("select", false));
        assert!(is_text_entry("INPUT", false)); // DOM tagName is uppercase
        assert!(is_text_entry("div", true)); // contenteditable
        assert!(!is_text_entry("div", false));
        assert!(!is_text_entry("button", false));
    }
}
