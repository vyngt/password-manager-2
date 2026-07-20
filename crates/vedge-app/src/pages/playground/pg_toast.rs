use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::ToastVariant;

use super::common::Section;

#[component]
pub fn ToastPage() -> impl IntoView {
    let i18n = use_i18n();
    let toast = use_toast();

    let show_default = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Settings saved successfully").dismiss_label(untrack(|| {
                t_string!(i18n, playground.dismiss_notification).to_string()
            })),
        );
    };
    let show_success = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Entry created")
                .variant(ToastVariant::Success)
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };
    let show_warning = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Exported — some fields were skipped")
                .variant(ToastVariant::Warning)
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };
    let show_danger = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Sync error — please try again")
                .variant(ToastVariant::Danger)
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };

    let show_with_action = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Item deleted")
                .variant(ToastVariant::Default)
                .action(
                    "Undo",
                    Callback::new(move |()| {
                        toast.show(
                            ToastInput::new("Undo successful")
                                .variant(ToastVariant::Success)
                                .dismiss_label(untrack(|| {
                                    t_string!(i18n, playground.dismiss_notification).to_string()
                                })),
                        );
                    }),
                )
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };

    let show_persistent = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Connection lost — waiting for reconnect")
                .variant(ToastVariant::Danger)
                .duration(0)
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };

    let show_short = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Copied to clipboard")
                .duration(2000)
                .dismiss_label(untrack(|| {
                    t_string!(i18n, playground.dismiss_notification).to_string()
                })),
        );
    };

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Toast"</h1>

            <Section title="Variants">
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_default
                    >
                        "Default"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_success
                    >
                        "Success"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_warning
                    >
                        "Warning"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_danger
                    >
                        "Danger"
                    </button>
                </div>
            </Section>

            <Section title="Action Button">
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_with_action
                    >
                        "With Undo Action"
                    </button>
                </div>
                <p class="text-xs text-text-tertiary mt-2">
                    "Click the action link inside the toast to trigger a callback."
                </p>
            </Section>

            <Section title="Duration">
                <div class="flex flex-wrap gap-3">
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_short
                    >
                        "Short (2s)"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary"
                        on:click=show_persistent
                    >
                        "Persistent (manual dismiss)"
                    </button>
                </div>
                <p class="text-xs text-text-tertiary mt-2">
                    "Hover over a toast to pause its auto-dismiss timer."
                </p>
            </Section>
        </div>
    }
}
