use icondata::Icon as IconData;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{A, Outlet};
use ui::components::Tooltip;
use ui::components::icon as ui_icon;
use ui::primitives::tokens::Placement;

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
