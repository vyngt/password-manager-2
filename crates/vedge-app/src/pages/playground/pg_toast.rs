use leptos::prelude::*;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::ToastVariant;

use super::common::Section;

#[component]
pub fn ToastPage() -> impl IntoView {
    let toast = use_toast();

    let show_default = move |_: web_sys::MouseEvent| {
        toast.show(ToastInput::new("Settings saved successfully"));
    };
    let show_success = move |_: web_sys::MouseEvent| {
        toast.show(ToastInput::new("Entry created").variant(ToastVariant::Success));
    };
    let show_warning = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Exported — some fields were skipped")
                .variant(ToastVariant::Warning),
        );
    };
    let show_danger = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Sync error — please try again")
                .variant(ToastVariant::Danger),
        );
    };

    let show_with_action = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Item deleted")
                .variant(ToastVariant::Default)
                .action("Undo", Callback::new(move |()| {
                    toast.show(ToastInput::new("Undo successful").variant(ToastVariant::Success));
                })),
        );
    };

    let show_persistent = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Connection lost — waiting for reconnect")
                .variant(ToastVariant::Danger)
                .duration(0),
        );
    };

    let show_short = move |_: web_sys::MouseEvent| {
        toast.show(
            ToastInput::new("Copied to clipboard")
                .duration(2000),
        );
    };

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Toast"</h1>

            <Section title="Variants">
                <div class="flex flex-wrap gap-3">
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_default>"Default"</button>
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_success>"Success"</button>
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_warning>"Warning"</button>
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_danger>"Danger"</button>
                </div>
            </Section>

            <Section title="Action Button">
                <div class="flex flex-wrap gap-3">
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_with_action>"With Undo Action"</button>
                </div>
                <p class="text-xs text-text-tertiary mt-2">"Click the action link inside the toast to trigger a callback."</p>
            </Section>

            <Section title="Duration">
                <div class="flex flex-wrap gap-3">
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_short>"Short (2s)"</button>
                    <button class="px-3 py-1.5 text-sm rounded border border-border bg-surface-1 hover:bg-surface-2 text-text-primary" on:click=show_persistent>"Persistent (manual dismiss)"</button>
                </div>
                <p class="text-xs text-text-tertiary mt-2">"Hover over a toast to pause its auto-dismiss timer."</p>
            </Section>
        </div>
    }
}
