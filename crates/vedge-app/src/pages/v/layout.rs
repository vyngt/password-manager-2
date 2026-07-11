use icondata::Icon as IconData;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use leptos_router::components::Outlet;
use leptos_router::hooks::use_navigate;
use vedge_ui::components::Tooltip;
use vedge_ui::components::icon as ui_icon;
use vedge_ui::primitives::tokens::Placement;

use crate::api;
use crate::features::vault::auto_lock::AutoLock;
use crate::features::vault::command_palette::{CommandPalette, typing_in_field};
use crate::features::vault::context::ActiveVault;
use crate::features::vault::ui_state::VaultUiState;
use crate::i18n::{t_string, use_i18n};

struct SidebarRouteItem {
    path: &'static str,
    icon: IconData,
}

const SIDEBAR_ITEMS: &[SidebarRouteItem] = &[
    SidebarRouteItem {
        path: "/v/vault",
        icon: icondata::FaKeySolid,
    },
    SidebarRouteItem {
        path: "/v/generator",
        icon: icondata::FaWandMagicSparklesSolid,
    },
    SidebarRouteItem {
        path: "/v/settings",
        icon: icondata::FaGearSolid,
    },
];

/// Row layout + active/hover styling for a sidebar nav item. Collapsed items are
/// a fixed square (they sit inside the Tooltip's content-width inline-flex span,
/// so a centering wrapper — not `w-full` — does the horizontal alignment).
fn nav_item_class(active: bool, collapsed: bool) -> String {
    let layout = if collapsed {
        "h-10 w-10 rounded-lg flex items-center justify-center transition-colors"
    } else {
        "w-full h-10 px-2 rounded-lg flex items-center gap-3 text-sm transition-colors"
    };
    let state = if active {
        "bg-primary-muted text-primary font-medium"
    } else {
        "text-text-secondary hover:text-text-primary hover:bg-primary/5"
    };
    format!("{layout} {state}")
}

/// Row layout + hover styling for a non-nav sidebar button (Lock).
fn nav_button_class(collapsed: bool) -> String {
    let layout = if collapsed {
        "h-10 w-10 rounded-lg flex items-center justify-center transition-colors"
    } else {
        "w-full h-10 px-2 rounded-lg flex items-center gap-3 text-sm transition-colors"
    };
    format!("{layout} text-text-secondary hover:text-text-primary hover:bg-primary/5")
}

#[component]
fn SidebarItemRow(item: &'static SidebarRouteItem, collapsed: RwSignal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    let location = leptos_router::hooks::use_location();
    let is_active = move || location.pathname.get().starts_with(item.path);
    let label = Signal::derive(move || match item.path {
        "/v/settings" => t_string!(i18n, nav.settings).to_owned(),
        "/v/generator" => t_string!(i18n, nav.generator).to_owned(),
        _ => t_string!(i18n, nav.vault).to_owned(),
    });
    let go = move |_: web_sys::MouseEvent| use_navigate()(item.path, Default::default());
    // Keyboard activation for the `role="button"` divs (Enter / Space), so the
    // nav isn't mouse-only.
    let go_key = move |ev: web_sys::KeyboardEvent| {
        if matches!(ev.key().as_str(), "Enter" | " ") {
            ev.prevent_default();
            use_navigate()(item.path, Default::default());
        }
    };

    view! {
        {move || {
            let active = is_active();
            if collapsed.get() {
                Either::Left(
                    // Collapsed: icon only, text pops out on hover (Tooltip). The
                    // Tooltip trigger is an inline-flex span (content-width), so a
                    // full-width flex wrapper centers it in the rail. The row carries
                    // its own `aria-label` since the icon is decorative.
                    view! {
                        <div class="flex justify-center">
                            <Tooltip placement=Placement::Right arrow=true content=label>
                                <div
                                    role="button"
                                    tabindex="0"
                                    aria-label=move || label.get()
                                    class=nav_item_class(active, true)
                                    on:click=go
                                    on:keydown=go_key
                                >
                                    <Icon
                                        attr:aria-hidden="true"
                                        icon=item.icon
                                        width="18"
                                        height="18"
                                    />
                                </div>
                            </Tooltip>
                        </div>
                    },
                )
            } else {
                Either::Right(
                    // Expanded: icon + text label (the text is the accessible name).
                    view! {
                        <div
                            role="button"
                            tabindex="0"
                            class=nav_item_class(active, false)
                            on:click=go
                            on:keydown=go_key
                        >
                            <Icon attr:aria-hidden="true" icon=item.icon width="18" height="18" />
                            <span class="truncate">{move || label.get()}</span>
                        </div>
                    },
                )
            }
        }}
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let i18n = use_i18n();
    // Collapsed by default (icon rail); expands to icon + label.
    let collapsed = RwSignal::new(true);
    let toggle_label = Signal::derive(move || t_string!(i18n, nav.toggle_sidebar).to_owned());

    view! {
        <div
            class="flex flex-col gap-0.5 bg-primary/10 border-r border-r-secondary/15 px-2 py-2 transition-[width] duration-200"
            class=("w-[60px]", move || collapsed.get())
            class=("w-44", move || !collapsed.get())
        >
            <div
                class="flex items-center gap-2 h-10 px-1 mb-1 shrink-0"
                class=("justify-center", move || collapsed.get())
            >
                <span class="flex items-center justify-center w-7 h-7 rounded-md bg-primary text-white shrink-0">
                    <Icon attr:aria-hidden="true" icon=ui_icon::Pm width="16" height="16" />
                </span>
                <Show when=move || !collapsed.get()>
                    <span class="text-[15px] font-semibold text-text-primary tracking-tight truncate">
                        "VEdge"
                    </span>
                </Show>
            </div>

            <For
                each=move || SIDEBAR_ITEMS.iter().enumerate()
                key=|(_, record)| record.path
                children=move |(_, record)| {
                    view! { <SidebarItemRow item=record collapsed=collapsed /> }
                }
            />

            <div class="mt-auto flex flex-col gap-0.5">
                <div class="border-t border-secondary/15 pt-1">
                    <LockButton collapsed=collapsed />
                </div>
                // Design-system exception: full-width sidebar collapse bar. `IconButton` is
                // square-only (28/36/44, 6px-max radius); this is w-full / h-8 / rounded-lg with
                // a primary-tint hover. Kept raw (aria-labelled + titled).
                <button
                    type="button"
                    class="w-full h-8 rounded-lg flex items-center justify-center text-text-secondary hover:text-text-primary hover:bg-primary/5 transition-colors"
                    aria-label=move || toggle_label.get()
                    title=move || toggle_label.get()
                    on:click=move |_: web_sys::MouseEvent| collapsed.update(|c| *c = !*c)
                >
                    <Show
                        when=move || collapsed.get()
                        fallback=|| {
                            view! {
                                <Icon
                                    attr:aria-hidden="true"
                                    icon=icondata::FaChevronLeftSolid
                                    width="14"
                                    height="14"
                                />
                            }
                        }
                    >
                        <Icon
                            attr:aria-hidden="true"
                            icon=icondata::FaChevronRightSolid
                            width="14"
                            height="14"
                        />
                    </Show>
                </button>
            </div>
        </div>
    }
}

#[component]
fn LockButton(collapsed: RwSignal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let label = Signal::derive(move || t_string!(i18n, unlock.lock).to_owned());

    let on_lock = move |_: web_sys::MouseEvent| {
        let nav = use_navigate();
        let path = active.path.get();
        spawn_local(async move {
            if let Some(p) = path {
                let _ = api::vault::lock(&p).await;
            }
            active.path.set(None);
            nav("/", Default::default());
        });
    };

    // Design-system exception: the Lock nav rows must stay pixel-identical to their
    // `role="button"` sidebar siblings (`SidebarItemRow`) via the shared
    // `nav_button_class` (40px, 8px radius, primary-tint hover). Converting only Lock
    // to `Button`/`IconButton` would desync the rail; the real fix is moving the whole
    // sidebar onto `SidebarItem` (deferred). Kept raw (aria-labelled).
    view! {
        {move || {
            if collapsed.get() {
                Either::Left(
                    view! {
                        <div class="flex justify-center">
                            <Tooltip placement=Placement::Right arrow=true content=label>
                                <button
                                    type="button"
                                    class=nav_button_class(true)
                                    data-testid="vault-lock"
                                    on:click=on_lock
                                    aria-label=move || label.get()
                                >
                                    <Icon
                                        attr:aria-hidden="true"
                                        icon=icondata::FaLockSolid
                                        width="18"
                                        height="18"
                                    />
                                </button>
                            </Tooltip>
                        </div>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <button
                            type="button"
                            class=nav_button_class(false)
                            data-testid="vault-lock"
                            on:click=on_lock
                            aria-label=move || label.get()
                        >
                            <Icon
                                attr:aria-hidden="true"
                                icon=icondata::FaLockSolid
                                width="18"
                                height="18"
                            />
                            <span class="truncate">{move || label.get()}</span>
                        </button>
                    },
                )
            }
        }}
    }
}

#[component]
pub fn VLayout() -> impl IntoView {
    // Shared UI state for the whole `/v` area (selection, create-toggle, palette,
    // edit-request). Provided here so `VaultPage`, `VaultDetail`, and the palette
    // all read the same signals.
    let ui = VaultUiState::new();
    provide_context(ui);

    // App-wide Ctrl/⌘-K toggles the command palette. Suppressed while typing in a
    // field — unless the palette is already open, in which case the shortcut
    // closes it (covering focus being in the palette's own input). The listener
    // is removed on unmount so re-entering `/v` never stacks handlers.
    let handle = window_event_listener(leptos::ev::keydown, move |ev| {
        if (ev.ctrl_key() || ev.meta_key()) && ev.key().eq_ignore_ascii_case("k") {
            if ui.palette_open.get_untracked() {
                ev.prevent_default();
                ui.palette_open.set(false);
            } else if !typing_in_field() {
                ev.prevent_default();
                ui.palette_open.set(true);
            }
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        <div class="flex flex-row h-full">
            <Sidebar />
            <div class="h-full w-full">
                <Outlet />
            </div>
        </div>
        <CommandPalette />
        <AutoLock />
    }
}
