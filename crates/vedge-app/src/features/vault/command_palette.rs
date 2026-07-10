//! `Ctrl/⌘-K` command palette — the vault's keyboard entry point.
//!
//! A `Dialog`-based modal (Escape + focus-trap + overlay come from the shared
//! [`Dialog`](vedge_ui::components::feedback::Dialog)) with a search header, a
//! keyboard-navigable results list, and a hint footer. Results are two groups
//! filtered by the query: **actions** (New / Lock, plus Edit / Copy when an entry
//! is selected) and **entries** (quick-open, filtered with 2.2's
//! [`query_matches`]). It drives the shared [`VaultUiState`] (open flag,
//! selection, create-toggle, edit-request) — the first app-wide shortcut surface
//! later slices hang commands on.
//!
//! Rows are modelled as a flat [`PaletteRow`] enum dispatched through a single
//! `Copy` `run_row` closure (rather than a `Callback` per command) to keep the
//! list cheap to rebuild on each keystroke. Locale/signal reads all happen in
//! reactive closures or event-handler bodies (owner rule).

use crate::api;
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_view::{type_icon, type_label_i18n};
use crate::features::vault::ui_state::VaultUiState;
use crate::features::vault::vault_filters::query_matches;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use leptos_router::hooks::use_navigate;
use std::collections::HashMap;
use std::time::Duration;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::empty_state::EmptyState;
use vedge_ui::components::feedback::Dialog;
use vedge_ui::primitives::tokens::DialogSize;
use wasm_bindgen::JsCast;

/// Upper bound on quick-open rows so a large vault doesn't blow up the DOM.
const PALETTE_ENTRY_CAP: usize = 50;

/// The selectable actions (entries are handled separately).
#[derive(Clone, Copy, PartialEq)]
enum ActionId {
    NewEntry,
    Lock,
    GeneratePassword,
    Edit,
    MoveToFolder,
    CopyPassword,
    CopyUsername,
}

/// One rendered/selectable palette row. Entries carry only what the palette
/// needs (id to select, name + type to show) — not the whole `IndexEntryDto`.
/// `PartialEq` so the row list can live in a `Memo` (skips downstream re-renders
/// when the recomputed rows are identical).
#[derive(Clone, PartialEq)]
enum PaletteRow {
    Action {
        id: ActionId,
        label: String,
    },
    Entry {
        id: String,
        name: String,
        entry_type: EntryTypeDto,
    },
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

/// Icon token for an action row.
fn action_icon(id: ActionId) -> icondata::Icon {
    match id {
        ActionId::NewEntry => i::FaPlusSolid,
        ActionId::Lock => i::FaLockSolid,
        ActionId::GeneratePassword => i::FaWandMagicSparklesSolid,
        ActionId::Edit => i::FaPenSolid,
        ActionId::MoveToFolder => i::FaFolderOpenSolid,
        ActionId::CopyPassword | ActionId::CopyUsername => i::FaCopySolid,
    }
}

/// Copy a field of the selected entry through the backend clipboard, wiping it
/// after the user-configured `secs` delay.
fn copy_selected(active: ActiveVault, ui: VaultUiState, secs: u32, field: FieldSelectorDto) {
    let vault_path = active.path.get_untracked().unwrap_or_default();
    let Some(id) = ui.selected_id.get_untracked() else {
        return;
    };
    spawn_local(async move {
        let _ = api::entry::copy_field(&vault_path, &id, &field, Some(secs)).await;
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
    let sec = expect_context::<SecurityPrefsCtx>();

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

    // Tag `id -> name` lookup, rebuilt only when the loaded `tags` change (not on
    // every keystroke) so `query_matches` can match a tag's display name.
    let tag_map = Memo::new(move |_| {
        tags.get()
            .into_iter()
            .map(|t| (t.id, t.name))
            .collect::<HashMap<String, String>>()
    });

    // The flat, ordered (actions-first) row list. A `Memo` so it recomputes only
    // when an input signal (query / entries / selection / tags / locale) actually
    // changes — not on every reactive read (the view alone reads it ~3× per render,
    // plus once per arrow key). `Copy`, so both the view and the keydown handler
    // read it. Locale reads via `t_string!` are owner-safe inside a `Memo` and make
    // the rows relocalize on a language switch.
    let rows = Memo::new(move |_| -> Vec<PaletteRow> {
        let q = query.get();
        let all = entries.get();
        let sel = ui.selected_id.get();
        let sel_entry = sel
            .as_ref()
            .and_then(|id| all.iter().find(|e| &e.id == id).cloned());

        let mut actions: Vec<(ActionId, String)> = vec![
            (
                ActionId::NewEntry,
                t_string!(i18n, vault.cmd_new_entry).to_owned(),
            ),
            (ActionId::Lock, t_string!(i18n, vault.cmd_lock).to_owned()),
            (
                ActionId::GeneratePassword,
                t_string!(i18n, vault.cmd_generate_password).to_owned(),
            ),
        ];
        if selection_actions_enabled(sel.as_deref())
            && let Some(e) = &sel_entry
        {
            if is_editable(&e.entry_type) {
                actions.push((ActionId::Edit, t_string!(i18n, vault.cmd_edit).to_owned()));
            }
            // Move works for every type (Document / Folder included).
            actions.push((
                ActionId::MoveToFolder,
                t_string!(i18n, vault.cmd_move_folder).to_owned(),
            ));
            if e.entry_type == EntryTypeDto::Login {
                actions.push((
                    ActionId::CopyPassword,
                    t_string!(i18n, vault.cmd_copy_password).to_owned(),
                ));
                actions.push((
                    ActionId::CopyUsername,
                    t_string!(i18n, vault.cmd_copy_username).to_owned(),
                ));
            }
        }

        let mut out: Vec<PaletteRow> = actions
            .into_iter()
            .filter(|(_, label)| matches_command(label, &q))
            .map(|(id, label)| PaletteRow::Action { id, label })
            .collect();

        let ql = q.trim().to_lowercase();
        tag_map.with(|tm| {
            out.extend(
                all.iter()
                    .filter(|e| query_matches(e, &ql, tm))
                    .take(PALETTE_ENTRY_CAP)
                    .map(|e| PaletteRow::Entry {
                        id: e.id.clone(),
                        name: e.name.clone(),
                        entry_type: e.entry_type.clone(),
                    }),
            );
        });
        out
    });

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
                ActionId::GeneratePassword => {
                    use_navigate()("/v/generator", Default::default());
                }
                ActionId::Edit => {
                    // `edit_request` is observed by `VaultDetail`'s Effect, which
                    // lives only on the vault route. Navigate there first, then
                    // defer the pulse one macrotask: a synchronous bump would land
                    // before the (re)mounted Effect captures its `prev` baseline
                    // and be swallowed — so Edit no-ops from `/v/generator` etc.
                    // The `set_timeout` runs after the microtask flush that mounts
                    // the route (same ordering rationale as the focus timeout).
                    use_navigate()("/v/vault", Default::default());
                    set_timeout(
                        move || ui.edit_request.update(|n| *n = n.wrapping_add(1)),
                        Duration::from_millis(0),
                    );
                }
                ActionId::MoveToFolder => {
                    ui.move_request.update(|n| *n = n.wrapping_add(1));
                }
                ActionId::CopyPassword => copy_selected(
                    active,
                    ui,
                    sec.0.get_untracked().clipboard_clear_seconds,
                    FieldSelectorDto::Password,
                ),
                ActionId::CopyUsername => copy_selected(
                    active,
                    ui,
                    sec.0.get_untracked().clipboard_clear_seconds,
                    FieldSelectorDto::Username,
                ),
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
            let len = rows.with(Vec::len);
            if len > 0 {
                highlighted.update(|h| *h = (*h + 1) % len);
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "ArrowUp" => {
            ev.prevent_default();
            let len = rows.with(Vec::len);
            if len > 0 {
                highlighted.update(|h| *h = if *h == 0 { len - 1 } else { *h - 1 });
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "Enter" => {
            ev.prevent_default();
            let current = rows.get();
            if current.is_empty() {
                return;
            }
            let idx = highlighted.get().min(current.len() - 1);
            if let Some(row) = current.get(idx) {
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
            // Search header — search icon + input + esc badge.
            <div class="flex items-center gap-3 px-4 py-3 border-b border-border shrink-0">
                <span class="flex shrink-0 text-foreground/40">
                    <Icon
                        attr:aria-hidden="true"
                        icon=i::FaMagnifyingGlassSolid
                        width="16"
                        height="16"
                    />
                </span>
                <input
                    node_ref=input_ref
                    class="flex-1 bg-transparent text-text-primary text-[15px] outline-none placeholder:text-foreground/40"
                    type="text"
                    autocomplete="off"
                    spellcheck="false"
                    role="combobox"
                    aria-controls="cmdk-listbox"
                    aria-expanded=move || rows.with(|r| (!r.is_empty()).to_string())
                    aria-activedescendant=move || {
                        rows.with(|r| {
                            (!r.is_empty()).then(|| format!("cmdk-opt-{}", highlighted.get()))
                        })
                    }
                    aria-label=move || t_string!(i18n, vault.palette_placeholder).to_owned()
                    placeholder=move || t_string!(i18n, vault.palette_placeholder).to_owned()
                    prop:value=move || query.get()
                    on:input:target=move |ev| {
                        query.set(ev.target().value());
                        highlighted.set(0);
                    }
                    on:keydown=on_key
                />
                <kbd class="shrink-0 text-[11px] text-foreground/40 border border-border rounded px-1.5 py-0.5 font-jetbrains-mono">
                    "esc"
                </kbd>
            </div>

            // Results.
            <div
                node_ref=list_ref
                id="cmdk-listbox"
                role="listbox"
                class="overflow-auto px-2 py-2 max-h-[360px]"
            >
                {move || {
                    let rows = rows.get();
                    if rows.is_empty() {
                        return Either::Left(
                            view! {
                                <EmptyState
                                    icon=i::FaMagnifyingGlassSolid
                                    title=Signal::derive(move || {
                                        t_string!(i18n, vault.palette_empty).to_owned()
                                    })
                                />
                            },
                        );
                    }
                    let n_actions = rows
                        .iter()
                        .take_while(|r| matches!(r, PaletteRow::Action { .. }))
                        .count();
                    let views = rows
                        .into_iter()
                        .enumerate()
                        .map(|(idx, row)| {
                            let show_actions_heading = idx == 0
                                && matches!(row, PaletteRow::Action { .. });
                            let show_entries_heading = idx == n_actions
                                && matches!(row, PaletteRow::Entry { .. });
                            let (icon, label, type_label) = match &row {
                                PaletteRow::Action { id, label } => {
                                    (action_icon(*id), label.clone(), None::<String>)
                                }
                                PaletteRow::Entry { name, entry_type, .. } => {
                                    (
                                        type_icon(entry_type),
                                        name.clone(),
                                        Some(type_label_i18n(i18n, entry_type)),
                                    )
                                }
                            };
                            let is_action = matches!(row, PaletteRow::Action { .. });
                            let is_hl = move || highlighted.get() == idx;
                            let row_for_click = row.clone();
                            view! {
                                {show_actions_heading
                                    .then(|| {
                                        view! {
                                            <div class="px-3 pt-2 pb-1 text-[11px] uppercase tracking-wider text-foreground/40">
                                                {move || t!(i18n, vault.palette_actions)}
                                            </div>
                                        }
                                    })}
                                {show_entries_heading
                                    .then(|| {
                                        view! {
                                            <div class="px-3 pt-3 pb-1 text-[11px] uppercase tracking-wider text-foreground/40">
                                                {move || t!(i18n, vault.palette_entries)}
                                            </div>
                                        }
                                    })}
                                <div
                                    id=format!("cmdk-opt-{idx}")
                                    role="option"
                                    aria-selected=move || is_hl().then_some("true")
                                    class="relative flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer text-sm"
                                    class=("bg-primary-muted", is_hl)
                                    on:mouseenter=move |_: web_sys::MouseEvent| highlighted.set(idx)
                                    on:click=move |_: web_sys::MouseEvent| run_row(
                                        row_for_click.clone(),
                                    )
                                >
                                    {move || {
                                        is_hl()
                                            .then(|| {
                                                view! {
                                                    <span class="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded bg-primary"></span>
                                                }
                                            })
                                    }}
                                    <span
                                        class="flex shrink-0"
                                        class=("text-primary", is_hl)
                                        class=("text-foreground/50", move || !is_hl())
                                    >
                                        <Icon
                                            attr:aria-hidden="true"
                                            icon=icon
                                            width="16"
                                            height="16"
                                        />
                                    </span>
                                    <span
                                        class="flex-1 truncate"
                                        class=("text-primary", is_hl)
                                        class=("text-text-primary", move || !is_hl())
                                    >
                                        {label}
                                    </span>
                                    {type_label
                                        .map(|t| {
                                            view! {
                                                <span class="shrink-0 text-xs text-foreground/40">{t}</span>
                                            }
                                        })}
                                    {move || {
                                        (is_action && is_hl())
                                            .then(|| {
                                                view! {
                                                    <span class="shrink-0 text-xs text-primary font-jetbrains-mono">
                                                        "⏎"
                                                    </span>
                                                }
                                            })
                                    }}
                                </div>
                            }
                        })
                        .collect_view();
                    Either::Right(views)
                }}
            </div>

            // Hint footer.
            <div class="flex items-center gap-4 px-4 py-2 border-t border-border text-[11px] text-foreground/40 font-jetbrains-mono shrink-0">
                <span>"↑↓ "{move || t!(i18n, vault.palette_hint_navigate)}</span>
                <span>"⏎ "{move || t!(i18n, vault.palette_hint_run)}</span>
                <span>"esc "{move || t!(i18n, vault.palette_hint_close)}</span>
                <span class="ml-auto">"⌘K"</span>
            </div>
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
        // The unconditional "Generate password" action (slice 3.3) filters by label.
        assert!(matches_command("Generate password", "generate"));
        assert!(matches_command("Generate password", "PASSWORD"));
        assert!(!matches_command("Generate password", "lock"));
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
