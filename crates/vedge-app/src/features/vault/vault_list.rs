//! Left pane of the vault picker (2.8.1; redesigned 5.2.4): a searchable,
//! keyboard-navigable, recency-sorted list of the user's vaults ("Your vaults").
//! Rows show icon + name + middle-truncated path + last-opened date.
//!
//! 🔴 **The row gave up its buttons (Decision ⑥).** A 288px row could not host a
//! fifth action, so every row now carries ONE `⋯` (opening the vault-details
//! dialog) in the same place in every state — including the **missing** row, which
//! is the resume surface for a crashed delete. Broken rows keep exactly one inline
//! action (corrupt → Restore, missing → Convert/Locate); everything else — rename,
//! Remove, Delete, the copyable Vault ID — moved into the modal. The inline
//! hover-pencil rename is gone (invisible until hover, unreachable by touch).

use crate::features::vault::home_path::middle_truncate;
use crate::features::vault::vault_launch::Selected;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ipc::RegisteredVaultStatusDto;
use vedge_ui::components::{Button, IconButton, Input};
use vedge_ui::primitives::tokens::{Size, Variant};
use wasm_bindgen::JsCast;

/// Short display of an RFC3339 timestamp: just the `YYYY-MM-DD` date part.
fn short_date(rfc3339: &str) -> String {
    rfc3339.get(..10).unwrap_or(rfc3339).to_owned()
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
    filtered: Signal<Vec<RegisteredVaultStatusDto>>,
    query: RwSignal<String>,
    selected: RwSignal<Option<Selected>>,
    on_select: Callback<Selected>,
    /// Open the vault-details dialog for a row id (`⋯`) — hosts rename, the copyable Vault ID,
    /// and the danger zone (Remove / Delete). Slice 5.2.4.
    on_menu: Callback<String>,
    on_locate: Callback<String>,
    /// Convert a legacy `.vdb` row to a `.vedge/` home (slice 5.2.0).
    on_convert: Callback<String>,
    /// Restore a present-but-corrupt vault (`exists && !openable`) from its newest snapshot
    /// (slice 5.2.1 — the H0 disaster path). Passes the vault id.
    on_restore: Callback<String>,
    on_new: Callback<()>,
    on_open_file: Callback<()>,
    /// Open the "Open a backup…" dialog (slice 5.2.2), which also hosts Advanced ▸ Replace.
    on_open_backup: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let highlighted = RwSignal::new(0usize);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let list_ref = NodeRef::<leptos::html::Div>::new();

    // Select the (existing) row at `idx`. Missing rows are not unlockable.
    let select_at = move |idx: usize| {
        let rows = filtered.get();
        if let Some(rec) = rows.get(idx)
            && rec.exists
            && rec.openable
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
        <div class="flex w-80 shrink-0 flex-col overflow-hidden rounded-lg border border-border bg-surface">
            // Header + search.
            <div class="px-3.5 pb-2.5 pt-3.5">
                <div class="mb-2.5 flex items-center gap-2">
                    <span class="flex h-5.5 w-5.5 items-center justify-center rounded-md bg-primary text-white">
                        <Icon
                            attr:aria-hidden="true"
                            icon=i::FaShieldHalvedSolid
                            width="13"
                            height="13"
                        />
                    </span>
                    <span class="text-sm font-semibold text-text-primary">
                        {move || t!(i18n, unlock.your_vaults)}
                    </span>
                </div>
                <Input
                    id="vault-search"
                    input_ref=input_ref
                    autocomplete="off"
                    spellcheck="false"
                    leading_icon=Box::new(|| {
                        view! { <Icon icon=i::FaMagnifyingGlassSolid width="13" height="13" /> }
                            .into_any()
                    })
                    aria_label=Signal::derive(move || {
                        t_string!(i18n, unlock.search_vaults).to_owned()
                    })
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, unlock.search_vaults).to_owned()
                    })
                    value=Signal::derive(move || query.get())
                    on_input=Callback::new(move |v: String| {
                        query.set(v);
                        highlighted.set(0);
                    })
                    on:keydown=on_key
                />
            </div>

            // Scrollable list.
            <div node_ref=list_ref role="listbox" class="flex-1 overflow-auto px-2 pb-2">
                {move || {
                    let rows = filtered.get();
                    if rows.is_empty() {
                        return Either::Left(
                            view! {
                                <div class="px-3 py-8 text-center text-sm text-foreground/40">
                                    {move || t!(i18n, unlock.no_match)}
                                </div>
                            },
                        );
                    }
                    let views = rows
                        .into_iter()
                        .enumerate()
                        .map(|(idx, rec)| {
                            let exists = rec.exists;
                            let openable = rec.openable;
                            let id = rec.vault.id.clone();
                            let path = rec.vault.path.clone();
                            let is_old_layout = std::path::Path::new(path.as_str())
                                .extension()
                                .is_some_and(|e| e.eq_ignore_ascii_case("vdb"));
                            let name = rec.vault.display_name.clone();
                            let recency = rec.vault.last_opened.as_deref().map(short_date);
                            let row_icon = if exists {
                                i::FaFileShieldSolid
                            } else {
                                i::FaFileCircleXmarkSolid
                            };
                            let is_hl = move || highlighted.get() == idx;
                            let sel_id = id.clone();
                            let is_selected = Memo::new(move |_| {
                                selected.get().and_then(|s| s.id).as_deref()
                                    == Some(sel_id.as_str())
                            });
                            let click_path = path.clone();
                            let click_id = id.clone();
                            let click_name = name.clone();
                            let disp_name = name.clone();
                            let title_name = name;
                            let disp_path = middle_truncate(&path, 40);
                            let menu_id = id.clone();
                            let loc_id = id.clone();
                            let conv_id = id.clone();
                            let restore_id = id;

                            view! {
                                <div
                                    role="option"
                                    aria-selected=move || is_hl().then_some("true")
                                    class="group relative mb-0.5 flex items-center gap-2.5 rounded-lg px-2.5 py-2"
                                    class=("bg-primary-muted", is_hl)
                                    class=("opacity-55", move || !exists)
                                    class=("cursor-pointer", move || exists && openable)
                                    on:mouseenter=move |_: web_sys::MouseEvent| highlighted.set(idx)
                                    on:click=move |_: web_sys::MouseEvent| {
                                        if exists && openable {
                                            on_select
                                                .run(Selected {
                                                    path: click_path.clone(),
                                                    id: Some(click_id.clone()),
                                                    display_name: click_name.clone(),
                                                });
                                        }
                                    }
                                >
                                    {move || {
                                        is_selected
                                            .get()
                                            .then(|| {
                                                view! {
                                                    <span class="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded bg-primary"></span>
                                                }
                                            })
                                    }}
                                    <span
                                        class="flex shrink-0 text-lg"
                                        class=("text-primary", move || is_selected.get())
                                        class=("text-foreground/40", move || !is_selected.get())
                                    >
                                        <Icon attr:aria-hidden="true" icon=row_icon />
                                    </span>
                                    <div class="min-w-0 flex-1">
                                        <div
                                            class="truncate text-[13px] font-medium"
                                            class=("text-primary", move || is_selected.get())
                                            class=("text-text-primary", move || !is_selected.get())
                                            title=title_name
                                        >
                                            {disp_name}
                                        </div>
                                        <div class="truncate text-[11px] font-jetbrains-mono text-foreground/50">
                                            {disp_path}
                                        </div>
                                    </div>
                                    <div class="flex shrink-0 items-center gap-1.5">
                                        {if exists && openable {
                                            EitherOf3::A(
                                                recency
                                                    .map(|d| {
                                                        view! {
                                                            <span class="text-[11px] text-foreground/40">{d}</span>
                                                        }
                                                    }),
                                            )
                                        } else if exists {
                                            EitherOf3::B(
                                                // Present-but-corrupt: the disaster path (H0). Offer
                                                // Restore from the vault's newest snapshot.
                                                view! {
                                                    <span
                                                        class="rounded px-1.5 py-0.5 text-[10px]"
                                                        style="color:var(--color-danger-text);background:var(--color-danger-muted)"
                                                    >
                                                        {move || t!(i18n, unlock.vault_corrupt)}
                                                    </span>
                                                    <Button
                                                        variant=Variant::Primary
                                                        size=Size::Sm
                                                        attr:data-testid="vault-restore"
                                                        on:click=move |ev: web_sys::MouseEvent| {
                                                            ev.stop_propagation();
                                                            on_restore.run(restore_id.clone());
                                                        }
                                                    >
                                                        {move || t!(i18n, unlock.vault_restore)}
                                                    </Button>
                                                },
                                            )
                                        } else {
                                            EitherOf3::C(
                                                // Missing: the ONE inline action is Convert (legacy
                                                // `.vdb`, un-pickable by a folder dialog) or Locate.
                                                view! {
                                                    <span
                                                        class="rounded px-1.5 py-0.5 text-[10px]"
                                                        style="color:var(--color-danger-text);background:var(--color-danger-muted)"
                                                    >
                                                        {move || t!(i18n, unlock.vault_missing)}
                                                    </span>
                                                    {if is_old_layout {
                                                        Either::Left(
                                                            view! {
                                                                <Button
                                                                    variant=Variant::Ghost
                                                                    size=Size::Sm
                                                                    attr:data-testid="vault-convert"
                                                                    on:click=move |ev: web_sys::MouseEvent| {
                                                                        ev.stop_propagation();
                                                                        on_convert.run(conv_id.clone());
                                                                    }
                                                                >
                                                                    {move || t!(i18n, unlock.vault_convert)}
                                                                </Button>
                                                            },
                                                        )
                                                    } else {
                                                        Either::Right(
                                                            view! {
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
                                                            },
                                                        )
                                                    }}
                                                },
                                            )
                                        }}
                                        // ⋯ — same place, every state. Opens the details dialog.
                                        <IconButton
                                            variant=Variant::Ghost
                                            size=Size::Xs
                                            attr:data-testid="vault-menu"
                                            aria_label=Signal::derive(move || {
                                                t_string!(i18n, unlock.vault_menu).to_owned()
                                            })
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                ev.stop_propagation();
                                                on_menu.run(menu_id.clone());
                                            }
                                        >
                                            <Icon
                                                attr:aria-hidden="true"
                                                icon=i::FaEllipsisSolid
                                                width="14"
                                                height="14"
                                            />
                                        </IconButton>
                                    </div>
                                </div>
                            }
                        })
                        .collect_view();
                    Either::Right(views)
                }}
            </div>

            // Footer: New + Open + Open a backup.
            <div class="border-t border-border px-3 py-2.5 space-y-2">
                <div class="flex gap-2">
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
                // Slice 5.2.2 — "Open a backup…" belongs HERE, on the closed-vault screen. It is
                // the new-machine flow's front door, and it is also where the Advanced ▸ Replace
                // escape hatch hides.
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    full_width=true
                    attr:data-testid="open-backup"
                    on:click=move |_: web_sys::MouseEvent| on_open_backup.run(())
                >
                    {move || t!(i18n, unlock.ob_cta)}
                </Button>
            </div>
        </div>
    }
}
