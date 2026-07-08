//! Left pane of the vault picker (2.8.1): a searchable, keyboard-navigable,
//! recency-sorted list of the user's vaults ("Your vaults"). Rows show
//! icon + name + path + last-opened date; missing files are dimmed with a
//! `missing` tag plus Remove/Locate. Each existing row has an inline rename
//! (hover pencil). Selecting a row feeds the focused unlock panel via
//! `on_select`. Keyboard model mirrors the 2.3 command palette.

use crate::features::vault::vault_launch::Selected;
use crate::i18n::*;
use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::time::Duration;
use vedge_ipc::RecentVaultStatusDto;
use vedge_ui::components::Button;
use vedge_ui::primitives::tokens::{Size, Variant};
use wasm_bindgen::JsCast;

/// Short display of an RFC3339 timestamp: just the `YYYY-MM-DD` date part.
fn short_date(rfc3339: &str) -> String {
    rfc3339.get(..10).unwrap_or(rfc3339).to_string()
}

/// Scroll the option at `idx` into view within the list (keyboard nav).
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
pub fn VaultList(
    filtered: Signal<Vec<RecentVaultStatusDto>>,
    query: RwSignal<String>,
    selected: RwSignal<Option<Selected>>,
    on_select: Callback<Selected>,
    /// `(id, new_display_name)` — commit an inline rename.
    on_rename_commit: Callback<(String, String)>,
    on_remove: Callback<String>,
    on_locate: Callback<String>,
    on_new: Callback<()>,
    on_open_file: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let highlighted = RwSignal::new(0usize);
    let editing_id = RwSignal::new(Option::<String>::None);
    let rename_draft = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let list_ref = NodeRef::<leptos::html::Div>::new();
    let rename_ref = NodeRef::<leptos::html::Input>::new();

    // Select the (existing) row at `idx`. Missing rows are not unlockable.
    let select_at = move |idx: usize| {
        let rows = filtered.get();
        if let Some(rec) = rows.get(idx)
            && rec.exists
        {
            on_select.run(Selected {
                path: rec.vault.path.clone(),
                id: Some(rec.vault.id.clone()),
                display_name: rec.vault.display_name.clone(),
            });
        }
    };

    let on_key = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "ArrowDown" => {
            ev.prevent_default();
            let len = filtered.get().len();
            if len > 0 {
                highlighted.update(|h| *h = (*h + 1) % len);
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "ArrowUp" => {
            ev.prevent_default();
            let len = filtered.get().len();
            if len > 0 {
                highlighted.update(|h| *h = if *h == 0 { len - 1 } else { *h - 1 });
                scroll_into_view(list_ref, highlighted.get_untracked());
            }
        }
        "Enter" => {
            ev.prevent_default();
            let len = filtered.get().len();
            if len > 0 {
                select_at(highlighted.get().min(len - 1));
            }
        }
        _ => {}
    };

    view! {
        <div class="flex w-72 shrink-0 flex-col border-r border-border bg-surface-subtle">
            // Header + search.
            <div class="px-3.5 pb-2.5 pt-3.5">
                <div class="mb-2.5 flex items-center gap-2">
                    <span class="flex h-5.5 w-5.5 items-center justify-center rounded-md bg-primary text-white">
                        <Icon icon=i::FaShieldHalvedSolid width="13" height="13" />
                    </span>
                    <span class="text-sm font-semibold text-text-primary">
                        {move || t!(i18n, unlock.your_vaults)}
                    </span>
                </div>
                <div class="flex h-8 items-center gap-2 rounded-lg border border-border bg-surface px-2.5">
                    <span class="flex shrink-0 text-foreground/40">
                        <Icon icon=i::FaMagnifyingGlassSolid width="13" height="13" />
                    </span>
                    <input
                        node_ref=input_ref
                        class="flex-1 bg-transparent text-sm text-text-primary outline-none placeholder:text-foreground/40"
                        type="text"
                        autocomplete="off"
                        spellcheck="false"
                        aria-label=move || t_string!(i18n, unlock.search_vaults).to_string()
                        placeholder=move || t_string!(i18n, unlock.search_vaults).to_string()
                        prop:value=move || query.get()
                        on:input:target=move |ev| {
                            query.set(ev.target().value());
                            highlighted.set(0);
                        }
                        on:keydown=on_key
                    />
                </div>
            </div>

            // Scrollable list.
            <div node_ref=list_ref role="listbox" class="flex-1 overflow-auto px-2 pb-2">
                {move || {
                    let rows = filtered.get();
                    if rows.is_empty() {
                        return Either::Left(view! {
                            <div class="px-3 py-8 text-center text-sm text-foreground/40">
                                {move || t!(i18n, unlock.no_match)}
                            </div>
                        });
                    }
                    let views = rows
                        .into_iter()
                        .enumerate()
                        .map(|(idx, rec)| {
                            let exists = rec.exists;
                            let id = rec.vault.id.clone();
                            let path = rec.vault.path.clone();
                            let name = rec.vault.display_name.clone();
                            let recency = rec.vault.last_opened.as_deref().map(short_date);
                            let row_icon = if exists {
                                i::FaFileShieldSolid
                            } else {
                                i::FaFileCircleXmarkSolid
                            };

                            let is_hl = move || highlighted.get() == idx;
                            let sel_id = id.clone();
                            // `Copy` Memo so the flag can be read in several
                            // view positions without moving a captured String.
                            let is_selected = Memo::new(move |_| {
                                selected.get().and_then(|s| s.id).as_deref()
                                    == Some(sel_id.as_str())
                            });

                            // Clones for the various closures on this row.
                            let click_path = path.clone();
                            let click_id = id.clone();
                            let click_name = name.clone();
                            let editing_check_id = id.clone();
                            let disp_name = name.clone();
                            let disp_path = path.clone();
                            let start_id = id.clone();
                            let start_name = name.clone();
                            let commit_id_kd = id.clone();
                            let commit_id_blur = id.clone();
                            let loc_id = id.clone();
                            let rem_id = id.clone();

                            view! {
                                <div
                                    role="option"
                                    aria-selected=move || is_hl().then_some("true")
                                    class="group relative mb-0.5 flex items-center gap-2.5 rounded-lg px-2.5 py-2"
                                    class=("bg-primary-muted", is_hl)
                                    class=("opacity-55", move || !exists)
                                    class=("cursor-pointer", move || exists)
                                    on:mouseenter=move |_: web_sys::MouseEvent| highlighted.set(idx)
                                    on:click=move |_: web_sys::MouseEvent| {
                                        if exists && editing_id.get().is_none() {
                                            on_select.run(Selected {
                                                path: click_path.clone(),
                                                id: Some(click_id.clone()),
                                                display_name: click_name.clone(),
                                            });
                                        }
                                    }
                                >
                                    {move || {
                                        is_selected.get().then(|| view! {
                                            <span class="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded bg-primary"></span>
                                        })
                                    }}
                                    <span
                                        class="flex shrink-0 text-lg"
                                        class=("text-primary", move || is_selected.get())
                                        class=("text-foreground/40", move || !is_selected.get())
                                    >
                                        <Icon icon=row_icon />
                                    </span>
                                    <div class="min-w-0 flex-1">
                                        {move || {
                                            // Clone the captured ids/strings per re-run so this
                                            // reactive closure stays `FnMut` (the inner handlers
                                            // move their clones, not the originals).
                                            let kd_id = commit_id_kd.clone();
                                            let blur_id = commit_id_blur.clone();
                                            let name_txt = disp_name.clone();
                                            let path_txt = disp_path.clone();
                                            if editing_id.get().as_deref()
                                                == Some(editing_check_id.as_str())
                                            {
                                                Either::Left(view! {
                                                    <input
                                                        node_ref=rename_ref
                                                        class="w-full rounded border border-primary bg-surface px-1 py-0.5 text-[13px] text-text-primary outline-none"
                                                        type="text"
                                                        prop:value=move || rename_draft.get()
                                                        aria-label=move || {
                                                            t_string!(i18n, unlock.vault_rename).to_string()
                                                        }
                                                        placeholder=move || {
                                                            t_string!(i18n, unlock.vault_rename_placeholder)
                                                                .to_string()
                                                        }
                                                        on:click=move |ev: web_sys::MouseEvent| {
                                                            ev.stop_propagation()
                                                        }
                                                        on:input:target=move |ev| {
                                                            rename_draft.set(ev.target().value())
                                                        }
                                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                            match ev.key().as_str() {
                                                                "Enter" => {
                                                                    ev.prevent_default();
                                                                    editing_id.set(None);
                                                                    let t = rename_draft
                                                                        .get()
                                                                        .trim()
                                                                        .to_string();
                                                                    if !t.is_empty() {
                                                                        on_rename_commit
                                                                            .run((kd_id.clone(), t));
                                                                    }
                                                                }
                                                                "Escape" => {
                                                                    ev.prevent_default();
                                                                    editing_id.set(None);
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                        on:blur=move |_: web_sys::FocusEvent| {
                                                            // Guard against the unmount-blur double-fire
                                                            // after Enter/Escape already cleared editing.
                                                            if editing_id.get().as_deref()
                                                                == Some(blur_id.as_str())
                                                            {
                                                                editing_id.set(None);
                                                                let t = rename_draft
                                                                    .get()
                                                                    .trim()
                                                                    .to_string();
                                                                if !t.is_empty() {
                                                                    on_rename_commit
                                                                        .run((blur_id.clone(), t));
                                                                }
                                                            }
                                                        }
                                                    />
                                                })
                                            } else {
                                                Either::Right(view! {
                                                    <div
                                                        class="truncate text-[13px] font-medium"
                                                        class=("text-primary", move || is_selected.get())
                                                        class=("text-text-primary", move || !is_selected.get())
                                                    >
                                                        {name_txt}
                                                    </div>
                                                    <div class="truncate text-[11px] font-jetbrains-mono text-foreground/50">
                                                        {path_txt}
                                                    </div>
                                                })
                                            }
                                        }}
                                    </div>
                                    {if exists {
                                        let recency_view = recency
                                            .clone()
                                            .map(|d| view! {
                                                <span class="text-[11px] text-foreground/40">{d}</span>
                                            });
                                        Either::Left(view! {
                                            <div class="flex shrink-0 items-center gap-1.5">
                                                {recency_view}
                                                <button
                                                    class="text-foreground/40 opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
                                                    aria-label=move || {
                                                        t_string!(i18n, unlock.vault_rename).to_string()
                                                    }
                                                    on:click=move |ev: web_sys::MouseEvent| {
                                                        ev.stop_propagation();
                                                        editing_id.set(Some(start_id.clone()));
                                                        rename_draft.set(start_name.clone());
                                                        set_timeout(
                                                            move || {
                                                                if let Some(el) = rename_ref.get_untracked() {
                                                                    let _ = el.focus();
                                                                    el.select();
                                                                }
                                                            },
                                                            Duration::from_millis(20),
                                                        );
                                                    }
                                                >
                                                    <Icon icon=i::FaPenSolid width="12" height="12" />
                                                </button>
                                            </div>
                                        })
                                    } else {
                                        Either::Right(view! {
                                            <div class="flex shrink-0 items-center gap-1.5">
                                                <span
                                                    class="rounded px-1.5 py-0.5 text-[10px]"
                                                    style="color:var(--color-danger-text);background:var(--color-danger-muted)"
                                                >
                                                    {move || t!(i18n, unlock.vault_missing)}
                                                </span>
                                                <Button
                                                    variant=Variant::Ghost
                                                    size=Size::Sm
                                                    on:click=move |ev: web_sys::MouseEvent| {
                                                        ev.stop_propagation();
                                                        on_locate.run(loc_id.clone());
                                                    }
                                                >
                                                    {move || t!(i18n, unlock.vault_locate)}
                                                </Button>
                                                <Button
                                                    variant=Variant::Ghost
                                                    size=Size::Sm
                                                    on:click=move |ev: web_sys::MouseEvent| {
                                                        ev.stop_propagation();
                                                        on_remove.run(rem_id.clone());
                                                    }
                                                >
                                                    {move || t!(i18n, unlock.vault_remove)}
                                                </Button>
                                            </div>
                                        })
                                    }}
                                </div>
                            }
                        })
                        .collect_view();
                    Either::Right(views)
                }}
            </div>

            // Footer: New + Open.
            <div class="flex gap-2 border-t border-border px-3 py-2.5">
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_new.run(())
                >
                    {move || t!(i18n, unlock.new_vault)}
                </Button>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    full_width=true
                    on:click=move |_: web_sys::MouseEvent| on_open_file.run(())
                >
                    {move || t!(i18n, unlock.open_file)}
                </Button>
            </div>
        </div>
    }
}
