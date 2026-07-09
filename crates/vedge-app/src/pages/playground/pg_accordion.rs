use leptos::prelude::*;
use vedge_ui::components::accordion::{Accordion, AccordionItem, AccordionMode};
use vedge_ui::primitives::tokens::Size;

use super::common::Section;

fn make_items(suffix: &str) -> Vec<AccordionItem> {
    let s = suffix.to_string();
    vec![
        AccordionItem {
            id: format!("general-{s}"),
            trigger: Box::new(|| view! { "General" }.into_any()),
            content: Box::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "General settings control the overall behavior of the vault: display name, default view, and startup actions."
                    </p>
                }
                    .into_any()
            }),
            disabled: false,
        },
        AccordionItem {
            id: format!("security-{s}"),
            trigger: Box::new(|| view! { "Security" }.into_any()),
            content: Box::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "Master password strength, auto-lock timers, and clipboard clearing preferences live here."
                    </p>
                }
                    .into_any()
            }),
            disabled: false,
        },
        AccordionItem {
            id: format!("sync-{s}"),
            trigger: Box::new(|| view! { "Sync" }.into_any()),
            content: Box::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "LAN sync pairing, peer trust, and conflict resolution policies."
                    </p>
                }
                .into_any()
            }),
            disabled: false,
        },
        AccordionItem {
            id: format!("advanced-{s}"),
            trigger: Box::new(|| view! { "Advanced" }.into_any()),
            content: Box::new(|| {
                view! {
                    <p class="text-sm text-text-secondary">
                        "Cryptographic backend, indexing granularity, and experimental flags. Change with care."
                    </p>
                }
                    .into_any()
            }),
            disabled: false,
        },
    ]
}

#[component]
pub fn AccordionPage() -> impl IntoView {
    let (open_multi, set_open_multi) = signal(Vec::<String>::new());

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Accordion"</h1>

            <Section title="Single mode (default) — lg triggers, General open">
                <Accordion
                    mode=AccordionMode::Single
                    trigger_size=Size::Lg
                    default_open=vec!["general-a".to_string()]
                    items=make_items("a")
                />
            </Section>

            <Section title="Multiple mode — md triggers, independent toggles">
                <div class="space-y-2">
                    <Accordion
                        mode=AccordionMode::Multiple
                        default_open=vec!["general-b".to_string(), "sync-b".to_string()]
                        on_change=Callback::new(move |ids: Vec<String>| set_open_multi.set(ids))
                        items=make_items("b")
                    />
                    <div class="text-xs text-text-tertiary">
                        "Open IDs: "
                        <code class="text-text-primary">
                            {move || format!("{:?}", open_multi.get())}
                        </code>
                    </div>
                </div>
            </Section>

            <Section title="Disabled item (Security)">
                <Accordion
                    mode=AccordionMode::Single
                    items=vec![
                        AccordionItem {
                            id: "general-c".into(),
                            trigger: Box::new(|| view! { "General" }.into_any()),
                            content: Box::new(|| {
                                view! { <p class="text-sm">"General content"</p> }.into_any()
                            }),
                            disabled: false,
                        },
                        AccordionItem {
                            id: "security-c".into(),
                            trigger: Box::new(|| view! { "Security (disabled)" }.into_any()),
                            content: Box::new(|| {
                                view! { <p class="text-sm">"Unreachable"</p> }.into_any()
                            }),
                            disabled: true,
                        },
                        AccordionItem {
                            id: "advanced-c".into(),
                            trigger: Box::new(|| view! { "Advanced" }.into_any()),
                            content: Box::new(|| {
                                view! { <p class="text-sm">"Advanced content"</p> }.into_any()
                            }),
                            disabled: false,
                        },
                    ]
                />
            </Section>

            <Section title="Rich content (form fields inside)">
                <Accordion
                    mode=AccordionMode::Single
                    items=vec![
                        AccordionItem {
                            id: "form-section".into(),
                            trigger: Box::new(|| view! { "Expand to edit" }.into_any()),
                            content: Box::new(|| {
                                view! {
                                    <div class="space-y-3">
                                        <label class="block text-sm">
                                            <span class="text-text-secondary">"Name"</span>
                                            <input
                                                type="text"
                                                class="mt-1 block w-full rounded border border-border bg-background px-3 py-2 text-sm"
                                            />
                                        </label>
                                        <label class="block text-sm">
                                            <span class="text-text-secondary">"Notes"</span>
                                            <textarea class="mt-1 block w-full rounded border border-border bg-background px-3 py-2 text-sm" />
                                        </label>
                                    </div>
                                }
                                    .into_any()
                            }),
                            disabled: false,
                        },
                    ]
                />
            </Section>

            <div class="text-xs text-text-tertiary">
                "Keyboard: Tab to focus triggers. Arrow Up/Down to move between triggers (wraps). Home/End to first/last. Enter/Space to toggle."
            </div>
        </div>
    }
}
