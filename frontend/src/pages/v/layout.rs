use icondata::Icon as IconData;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{A, Outlet};
use reactive_stores::Store;
use ui::components::icon as ui_icon;
use ui::components::icon_button::IconButton;
use ui::components::{Tooltip, TooltipPosition};
use ui::primitives::tokens::Variant;

use crate::stores::color::{ColorStore, ColorStoreStoreFields};

struct SidebarRouteItem {
    name: &'static str,
    path: &'static str,
    icon: IconData,
}

const SIDEBAR_ITEMS: &[SidebarRouteItem] = &[
    SidebarRouteItem {
        name: "Vault",
        path: "/v",
        icon: ui_icon::Pm,
    },
    SidebarRouteItem {
        name: "Playground",
        path: "/playground",
        icon: ui_icon::Pm,
    },
    SidebarRouteItem {
        name: "Settings",
        path: "/settings",
        icon: ui_icon::Pm,
    },
];

#[component]
fn SidebarItemRow(item: &'static SidebarRouteItem) -> impl IntoView {
    view! {
        <A href=item.path>
            <div class="w-full h-full [&>div]:h-full [&>div]:w-full flex flex-col">
                <Tooltip
                    position=TooltipPosition::Right
                    class="bg-background text-white whitespace-nowrap"
                    arrow=true
                    content=move || view! { <div class="p-1">{item.name}</div> }
                    trigger=move || view! {
                        <div class="p-2">
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
                key=|(_, record)| record.name
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
