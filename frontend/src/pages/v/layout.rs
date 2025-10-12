use icondata::Icon as IconData;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{A, Outlet};
use ui::components::icon as ui_icon;
use ui::components::{Tooltip, TooltipPosition};

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

    let wrapper_class = move || {
        if is_active() {
            "w-full h-full [&>div]:h-full [&>div]:w-full flex flex-col border-x-3 border-r-transparent border-l-primary"
        } else {
            "w-full h-full [&>div]:h-full [&>div]:w-full flex flex-col border-x-3 border-transparent"
        }
    };

    let item_class = move || {
        if is_active() {
            "p-2 text-white"
        } else {
            "p-2 text-white/60 hover:text-white"
        }
    };

    view! {
        <A href=item.path>
            <div class=wrapper_class>
                <Tooltip
                    position=TooltipPosition::Right
                    class="bg-background text-white whitespace-nowrap"
                    arrow=true
                    content=move || view! { <div class="p-1">{item.name}</div> }
                    trigger=move || view! {
                        <div class=item_class
                        >
                            <Icon icon={item.icon} height="100%" width="100%" />
                        </div>
                    }
                />
            </div>
        </A>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <div class="w-[60px] pt-2 flex flex-col gap-1 bg-primary/10 border-r border-r-secondary/15">
            <For
                each=move || SIDEBAR_ITEMS.into_iter().enumerate()
                key=|(_, record)| record.path
                children=move |(_, record)| {
                    view! {
                        <SidebarItemRow item={record} />
                    }
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
            <div>
                <Outlet/>
            </div>
        </div>
    }
}
