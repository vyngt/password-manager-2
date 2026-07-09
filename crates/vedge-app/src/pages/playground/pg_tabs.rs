use leptos::prelude::*;
use std::sync::Arc;
use vedge_ui::components::tabs::{Tab, Tabs, TabsActivation, TabsVariant};

use super::common::Section;

fn make_basic_tabs(suffix: &'static str) -> Vec<Tab> {
    vec![
        Tab {
            id: format!("overview-{suffix}"),
            label: Box::new(|| view! { "Overview" }.into_any()),
            panel: Arc::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "Summary of the entry: title, URL, last modified, tags."
                    </p>
                }
                .into_any()
            }),
            disabled: false,
        },
        Tab {
            id: format!("history-{suffix}"),
            label: Box::new(|| view! { "History" }.into_any()),
            panel: Arc::new(|| {
                view! {
                    <ul class="list-disc pl-5 text-sm text-text-secondary space-y-1">
                        <li>"2026-04-19 14:30 — updated password"</li>
                        <li>"2026-04-02 09:12 — renamed entry"</li>
                        <li>"2026-03-15 18:00 — created"</li>
                    </ul>
                }
                .into_any()
            }),
            disabled: false,
        },
        Tab {
            id: format!("security-{suffix}"),
            label: Box::new(|| view! { "Security" }.into_any()),
            panel: Arc::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "Breach check status, 2FA configuration, recovery options."
                    </p>
                }
                .into_any()
            }),
            disabled: false,
        },
    ]
}

#[component]
pub fn TabsPage() -> impl IntoView {
    let controlled_active = RwSignal::new("overview-c".to_string());

    let overflow_tabs: Vec<Tab> = (0_u32..10)
        .map(|i| {
            let id = format!("overflow-{i}");
            let label = format!("Tab {}", i + 1);
            let panel_label = label.clone();
            Tab {
                id,
                label: Box::new(move || view! { {label.clone()} }.into_any()),
                panel: Arc::new(move || {
                    let s = panel_label.clone();
                    view! { <p class="text-sm">{s}" content"</p> }.into_any()
                }),
                disabled: false,
            }
        })
        .collect();

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Tabs"</h1>

            <Section title="Underline variant (default)">
                <Tabs tabs=make_basic_tabs("a") />
            </Section>

            <Section title="Pill variant">
                <Tabs
                    variant=TabsVariant::Pill
                    tabs=vec![
                        Tab {
                            id: "password".into(),
                            label: Box::new(|| view! { "Password" }.into_any()),
                            panel: Arc::new(|| {
                                view! {
                                    <p class="text-sm text-text-secondary">
                                        "Generate a memorable random password."
                                    </p>
                                }
                                    .into_any()
                            }),
                            disabled: false,
                        },
                        Tab {
                            id: "passphrase".into(),
                            label: Box::new(|| view! { "Passphrase" }.into_any()),
                            panel: Arc::new(|| {
                                view! {
                                    <p class="text-sm text-text-secondary">
                                        "Dice-word passphrases (EFF word list)."
                                    </p>
                                }
                                    .into_any()
                            }),
                            disabled: false,
                        },
                        Tab {
                            id: "pin".into(),
                            label: Box::new(|| view! { "PIN" }.into_any()),
                            panel: Arc::new(|| {
                                view! {
                                    <p class="text-sm text-text-secondary">
                                        "Numeric PIN, configurable length."
                                    </p>
                                }
                                    .into_any()
                            }),
                            disabled: false,
                        },
                    ]
                />
            </Section>

            <Section title="Controlled">
                <Tabs
                    active=Signal::derive(move || controlled_active.get())
                    on_change=Callback::new(move |id: String| controlled_active.set(id))
                    tabs=make_basic_tabs("c")
                />
                <div class="mt-2 text-xs text-text-tertiary flex items-center gap-2">
                    "Active: "
                    <code class="text-text-primary">{move || controlled_active.get()}</code>
                    <button
                        type="button"
                        class="btn btn--secondary btn--sm btn--rounded"
                        on:click=move |_| controlled_active.set("overview-c".to_string())
                    >
                        "Reset to Overview"
                    </button>
                </div>
            </Section>

            <Section title="Disabled tab (middle)">
                <Tabs tabs=vec![
                    Tab {
                        id: "t1".into(),
                        label: Box::new(|| view! { "Enabled A" }.into_any()),
                        panel: Arc::new(|| { view! { <p class="text-sm">"A"</p> }.into_any() }),
                        disabled: false,
                    },
                    Tab {
                        id: "t2".into(),
                        label: Box::new(|| view! { "Disabled" }.into_any()),
                        panel: Arc::new(|| {
                            view! { <p class="text-sm">"Unreachable"</p> }.into_any()
                        }),
                        disabled: true,
                    },
                    Tab {
                        id: "t3".into(),
                        label: Box::new(|| view! { "Enabled C" }.into_any()),
                        panel: Arc::new(|| { view! { <p class="text-sm">"C"</p> }.into_any() }),
                        disabled: false,
                    },
                ] />
            </Section>

            <Section title="Lazy vs non-lazy (type in each, then switch and come back)">
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h3 class="text-xs text-text-tertiary mb-1">"lazy = true (default)"</h3>
                        <Tabs tabs=vec![
                            Tab {
                                id: "lazy-a".into(),
                                label: Box::new(|| view! { "A" }.into_any()),
                                panel: Arc::new(|| {
                                    view! {
                                        <input
                                            type="text"
                                            placeholder="Type here (will reset on switch)"
                                            class="w-full rounded border border-border bg-background px-3 py-2 text-sm"
                                        />
                                    }
                                        .into_any()
                                }),
                                disabled: false,
                            },
                            Tab {
                                id: "lazy-b".into(),
                                label: Box::new(|| view! { "B" }.into_any()),
                                panel: Arc::new(|| {
                                    view! { <p class="text-sm">"Panel B"</p> }.into_any()
                                }),
                                disabled: false,
                            },
                        ] />
                    </div>
                    <div>
                        <h3 class="text-xs text-text-tertiary mb-1">"lazy = false"</h3>
                        <Tabs
                            lazy=false
                            tabs=vec![
                                Tab {
                                    id: "keep-a".into(),
                                    label: Box::new(|| view! { "A" }.into_any()),
                                    panel: Arc::new(|| {
                                        view! {
                                            <input
                                                type="text"
                                                placeholder="Type here (preserved)"
                                                class="w-full rounded border border-border bg-background px-3 py-2 text-sm"
                                            />
                                        }
                                            .into_any()
                                    }),
                                    disabled: false,
                                },
                                Tab {
                                    id: "keep-b".into(),
                                    label: Box::new(|| view! { "B" }.into_any()),
                                    panel: Arc::new(|| {
                                        view! { <p class="text-sm">"Panel B"</p> }.into_any()
                                    }),
                                    disabled: false,
                                },
                            ]
                        />
                    </div>
                </div>
            </Section>

            <Section title="Manual activation (arrow moves focus only; Enter activates)">
                <Tabs activation=TabsActivation::Manual tabs=make_basic_tabs("m") />
            </Section>

            <Section title="Overflow — many tabs (horizontal scroll, no wrap)">
                <Tabs tabs=overflow_tabs />
            </Section>

            <div class="text-xs text-text-tertiary">
                "Keyboard: Arrow Left/Right to move focus between tabs (skips disabled, wraps). "
                "Home/End jump to first/last. In automatic activation (default), arrow also activates; "
                "in manual, press Enter or Space to activate the focused tab."
            </div>
        </div>
    }
}
