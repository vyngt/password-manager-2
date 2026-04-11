use leptos::prelude::*;
use leptos_router::components::{A, Outlet};

struct PlaygroundTab {
    name: &'static str,
    path: &'static str,
}

const TABS: &[PlaygroundTab] = &[
    PlaygroundTab {
        name: "Theme",
        path: "/playground",
    },
    PlaygroundTab {
        name: "Buttons",
        path: "/playground/buttons",
    },
    PlaygroundTab {
        name: "Forms",
        path: "/playground/forms",
    },
    PlaygroundTab {
        name: "Controls",
        path: "/playground/controls",
    },
    PlaygroundTab {
        name: "Atoms",
        path: "/playground/atoms",
    },
    PlaygroundTab {
        name: "ColorPicker",
        path: "/playground/color-picker",
    },
];

#[component]
pub fn PlaygroundLayout() -> impl IntoView {
    let location = leptos_router::hooks::use_location();

    view! {
        <div class="flex flex-col h-full">
            <nav class="flex gap-1 px-6 pt-4 border-b border-border">
                {TABS
                    .iter()
                    .map(|tab| {
                        let path = tab.path;
                        let name = tab.name;
                        let is_active = move || {
                            let pathname = location.pathname.get();
                            if path == "/playground" {
                                pathname == "/playground" || pathname == "/playground/"
                            } else {
                                pathname.starts_with(path)
                            }
                        };
                        let cls = move || {
                            if is_active() {
                                "px-3 py-2 text-sm font-medium text-text-primary border-b-2 border-primary -mb-px"
                            } else {
                                "px-3 py-2 text-sm font-medium text-text-tertiary hover:text-text-secondary -mb-px border-b-2 border-transparent"
                            }
                        };

                        view! {
                            <A href=path attr:class=cls>
                                {name}
                            </A>
                        }
                    })
                    .collect_view()}
            </nav>
            <div class="flex-1 overflow-auto">
                <Outlet />
            </div>
        </div>
    }
}
