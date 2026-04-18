use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use vedge_ui::theme::{ThemeConfig, ThemeState};

struct SidebarGroup {
    label: &'static str,
    items: &'static [SidebarItem],
}

struct SidebarItem {
    name: &'static str,
    path: &'static str,
}

const GROUPS: &[SidebarGroup] = &[
    SidebarGroup {
        label: "Foundation",
        items: &[
            SidebarItem {
                name: "Avatar",
                path: "/playground/avatar",
            },
            SidebarItem {
                name: "Badge",
                path: "/playground/badge",
            },
            SidebarItem {
                name: "Button",
                path: "/playground/button",
            },
            SidebarItem {
                name: "Icon Button",
                path: "/playground/icon-button",
            },
            SidebarItem {
                name: "Kbd",
                path: "/playground/kbd",
            },
            SidebarItem {
                name: "Progress Bar",
                path: "/playground/progress-bar",
            },
            SidebarItem {
                name: "QR Code",
                path: "/playground/qrcode",
            },
            SidebarItem {
                name: "Separator",
                path: "/playground/separator",
            },
            SidebarItem {
                name: "Spinner",
                path: "/playground/spinner",
            },
            SidebarItem {
                name: "Step Indicator",
                path: "/playground/step-indicator",
            },
        ],
    },
    SidebarGroup {
        label: "Form",
        items: &[
            SidebarItem {
                name: "Checkbox",
                path: "/playground/checkbox",
            },
            SidebarItem {
                name: "Color Picker",
                path: "/playground/color-picker",
            },
            SidebarItem {
                name: "Date Picker",
                path: "/playground/date-picker",
            },
            SidebarItem {
                name: "File Upload",
                path: "/playground/file-upload",
            },
            SidebarItem {
                name: "Helper Text",
                path: "/playground/helper-text",
            },
            SidebarItem {
                name: "Input",
                path: "/playground/input",
            },
            SidebarItem {
                name: "Label",
                path: "/playground/label",
            },
            SidebarItem {
                name: "Number Input",
                path: "/playground/number-input",
            },
            SidebarItem {
                name: "Password Strength Meter",
                path: "/playground/password-strength-meter",
            },
            SidebarItem {
                name: "Radio Group",
                path: "/playground/radio-group",
            },
            SidebarItem {
                name: "Segmented Control",
                path: "/playground/segmented-control",
            },
            SidebarItem {
                name: "Select",
                path: "/playground/select",
            },
            SidebarItem {
                name: "Slider",
                path: "/playground/slider",
            },
            SidebarItem {
                name: "Textarea",
                path: "/playground/textarea",
            },
            SidebarItem {
                name: "Time Picker",
                path: "/playground/time-picker",
            },
            SidebarItem {
                name: "Toggle",
                path: "/playground/toggle",
            },
        ],
    },
    SidebarGroup {
        label: "Feedback",
        items: &[
            SidebarItem {
                name: "Dialog",
                path: "/playground/dialog",
            },
            SidebarItem {
                name: "Toast",
                path: "/playground/toast",
            },
            SidebarItem {
                name: "Tooltip",
                path: "/playground/tooltip",
            },
        ],
    },
    SidebarGroup {
        label: "Data Display",
        items: &[
            SidebarItem {
                name: "Data Table",
                path: "/playground/data-table",
            },
            SidebarItem {
                name: "Pagination",
                path: "/playground/pagination",
            },
        ],
    },
];

#[component]
pub fn PlaygroundLayout() -> impl IntoView {
    let location = leptos_router::hooks::use_location();

    let theme_active = move || {
        let p = location.pathname.get();
        p == "/playground" || p == "/playground/"
    };

    let theme_cls = move || {
        if theme_active() {
            "block px-2 py-1 text-sm rounded text-text-primary bg-primary/10 font-medium"
        } else {
            "block px-2 py-1 text-sm rounded text-text-secondary hover:text-text-primary hover:bg-surface-2"
        }
    };

    let theme = expect_context::<ThemeState>();

    let theme_dark = theme.clone();
    let apply_light = move |_: web_sys::MouseEvent| {
        theme.preview(&ThemeConfig {
            background: "#FFFFFF".into(),
            foreground: "#111827".into(),
            primary: "#2563EB".into(),
            danger: Some("#DC2626".into()),
            warning: Some("#D97706".into()),
            success: Some("#16A34A".into()),
        });
    };

    let apply_dark = move |_: web_sys::MouseEvent| {
        theme_dark.preview(&ThemeConfig {
            background: "#09090B".into(),
            foreground: "#FAFAFA".into(),
            primary: "#3B82F6".into(),
            danger: Some("#EF4444".into()),
            warning: Some("#F59E0B".into()),
            success: Some("#22C55E".into()),
        });
    };

    view! {
        <div class="flex h-full">
            // Sidebar
            <aside class="w-[220px] shrink-0 border-r border-border overflow-y-auto flex flex-col">
                <nav class="p-4 space-y-5 flex-1">
                    // Theme — top item
                    <div>
                        <A href="/playground" attr:class=theme_cls>
                            "Theme"
                        </A>
                    </div>

                    // Component groups
                    {GROUPS
                        .iter()
                        .map(|group| {
                            let items = group
                                .items
                                .iter()
                                .map(|item| {
                                    let path = item.path;
                                    let name = item.name;
                                    let is_active = move || {
                                        location.pathname.get().starts_with(path)
                                    };
                                    let cls = move || {
                                        if is_active() {
                                            "block px-2 py-1 text-sm rounded text-text-primary bg-primary/10 font-medium"
                                        } else {
                                            "block px-2 py-1 text-sm rounded text-text-secondary hover:text-text-primary hover:bg-surface-2"
                                        }
                                    };
                                    view! {
                                        <li>
                                            <A href=path attr:class=cls>
                                                {name}
                                            </A>
                                        </li>
                                    }
                                })
                                .collect_view();

                            view! {
                                <div>
                                    <h3 class="text-[11px] font-semibold text-text-tertiary uppercase tracking-wider mb-1.5 px-2">
                                        {group.label}
                                    </h3>
                                    <ul class="space-y-0.5">{items}</ul>
                                </div>
                            }
                        })
                        .collect_view()}
                </nav>
                <div class="p-4 border-t border-border flex gap-2">
                    <button
                        class="flex-1 px-2 py-1.5 text-xs font-medium rounded border border-border bg-background text-text-primary hover:bg-surface-2 transition-colors"
                        on:click=apply_light
                    >
                        "Light"
                    </button>
                    <button
                        class="flex-1 px-2 py-1.5 text-xs font-medium rounded border border-border bg-zinc-900 text-white hover:bg-zinc-800 transition-colors"
                        on:click=apply_dark
                    >
                        "Dark"
                    </button>
                </div>
            </aside>

            // Content
            <div class="flex-1 overflow-auto">
                <Outlet />
            </div>
        </div>
    }
}
