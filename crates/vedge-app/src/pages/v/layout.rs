use icondata::Icon as IconData;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_navigate;
use vedge_ui::components::Tooltip;
use vedge_ui::components::icon as ui_icon;
use vedge_ui::primitives::tokens::Placement;

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;

struct SidebarRouteItem {
    name: &'static str,
    path: &'static str,
    icon: IconData,
}

const SIDEBAR_ITEMS: &[SidebarRouteItem] = &[
    SidebarRouteItem {
        name: "Vault",
        path: "/v/vault",
        icon: ui_icon::Pm,
    },
    SidebarRouteItem {
        name: "Settings",
        path: "/v/settings",
        icon: ui_icon::Pm,
    },
];

#[component]
fn SidebarItemRow(item: &'static SidebarRouteItem) -> impl IntoView {
    let location = leptos_router::hooks::use_location();

    let is_active = move || location.pathname.get().starts_with(item.path);

    let item_class = move || {
        if is_active() {
            "p-2 text-white"
        } else {
            "p-2 text-white/60 hover:text-white"
        }
    };

    view! {
        <A href=item.path>
            <Tooltip placement=Placement::Right arrow=true content=item.name>
                <div class=item_class>
                    <Icon icon=item.icon height="100%" width="100%" />
                </div>
            </Tooltip>
        </A>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <div class="w-[60px] flex flex-col gap-1 bg-primary/10 border-r border-r-secondary/15">
            <For
                each=move || SIDEBAR_ITEMS.into_iter().enumerate()
                key=|(_, record)| record.path
                children=move |(_, record)| {
                    view! { <SidebarItemRow item=record /> }
                }
            />
            <LockButton />
        </div>
    }
}

#[component]
fn LockButton() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

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

    view! {
        <div class="mt-auto">
            <Tooltip
                placement=Placement::Right
                arrow=true
                content=Signal::derive(move || t_string!(i18n, unlock.lock).to_string())
            >
                <button
                    class="p-2 text-white/60 hover:text-white"
                    aria-label=move || t_string!(i18n, unlock.lock).to_string()
                    on:click=on_lock
                >
                    <Icon icon=icondata::FaLockSolid height="100%" width="100%" />
                </button>
            </Tooltip>
        </div>
    }
}

#[component]
pub fn VLayout() -> impl IntoView {
    view! {
        <div class="flex flex-row h-full">
            <Sidebar />
            <div class="h-full w-full">
                <Outlet />
            </div>
        </div>
    }
}
