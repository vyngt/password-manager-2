use leptos::prelude::*;
use vedge_ui::components::popover::{Popover, PopoverPlacement};

use super::common::Section;

#[component]
pub fn PopoverPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-5xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Popover"</h1>

            <Section title="Basic — bottom-start (default)">
                <BasicDemo />
            </Section>

            <Section title="Placements — 12 anchors">
                <PlacementsDemo />
            </Section>

            <Section title="match_trigger_width">
                <MatchWidthDemo />
            </Section>

            <Section title="close_on_scroll">
                <CloseOnScrollDemo />
            </Section>

            <div class="text-xs text-text-tertiary">
                "Keyboard: Tab into the panel, Tab past last focusable to auto-close, "
                "Escape to close, click outside to close. Focus returns to the trigger."
            </div>
        </div>
    }
}

#[component]
fn BasicDemo() -> impl IntoView {
    let (open, set_open) = signal(false);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    view! {
        <div class="flex items-center gap-4">
            <button
                node_ref=trigger_ref
                type="button"
                class="btn btn--primary btn--md btn--rounded"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                "Open menu"
            </button>
            <Popover
                open=open
                on_close=Callback::new(move |_: ()| set_open.set(false))
                anchor=anchor
            >
                <div class="p-2 w-56 space-y-1 text-sm">
                    <button
                        class="w-full text-left px-2 py-1.5 rounded hover:bg-surface-2 text-text-primary"
                        on:click=move |_| set_open.set(false)
                    >
                        "Edit"
                    </button>
                    <button
                        class="w-full text-left px-2 py-1.5 rounded hover:bg-surface-2 text-text-primary"
                        on:click=move |_| set_open.set(false)
                    >
                        "Duplicate"
                    </button>
                    <button
                        class="w-full text-left px-2 py-1.5 rounded hover:bg-surface-2 text-text-primary"
                        on:click=move |_| set_open.set(false)
                    >
                        "Archive"
                    </button>
                    <div class="h-px bg-border my-1" />
                    <button
                        class="w-full text-left px-2 py-1.5 rounded hover:bg-surface-2 text-danger-text"
                        on:click=move |_| set_open.set(false)
                    >
                        "Delete"
                    </button>
                </div>
            </Popover>
        </div>
    }
}

#[component]
fn PlacementsDemo() -> impl IntoView {
    let placements = [
        ("top-start", PopoverPlacement::TopStart),
        ("top", PopoverPlacement::Top),
        ("top-end", PopoverPlacement::TopEnd),
        ("right-start", PopoverPlacement::RightStart),
        ("right", PopoverPlacement::Right),
        ("right-end", PopoverPlacement::RightEnd),
        ("bottom-start", PopoverPlacement::BottomStart),
        ("bottom", PopoverPlacement::Bottom),
        ("bottom-end", PopoverPlacement::BottomEnd),
        ("left-start", PopoverPlacement::LeftStart),
        ("left", PopoverPlacement::Left),
        ("left-end", PopoverPlacement::LeftEnd),
    ];

    view! {
        <div class="grid grid-cols-3 gap-4">
            {placements
                .into_iter()
                .map(|(label, placement)| {
                    view! { <PlacementButton label=label placement=placement /> }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn PlacementButton(label: &'static str, placement: PopoverPlacement) -> impl IntoView {
    let (open, set_open) = signal(false);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    view! {
        <div>
            <button
                node_ref=trigger_ref
                type="button"
                class="btn btn--secondary btn--md btn--rounded w-full"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                {label}
            </button>
            <Popover
                open=open
                on_close=Callback::new(move |_: ()| set_open.set(false))
                anchor=anchor
                placement=placement
            >
                <div class="p-3 text-xs text-text-primary min-w-[140px]">
                    "Placement: " <code>{label}</code>
                </div>
            </Popover>
        </div>
    }
}

#[component]
fn MatchWidthDemo() -> impl IntoView {
    let (open, set_open) = signal(false);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    view! {
        <div class="w-[320px]">
            <button
                node_ref=trigger_ref
                type="button"
                class="btn btn--secondary btn--md btn--rounded w-full"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                "Wide trigger (320px)"
            </button>
            <Popover
                open=open
                on_close=Callback::new(move |_: ()| set_open.set(false))
                anchor=anchor
                match_trigger_width=true
            >
                <div class="p-2 text-sm text-text-primary">
                    "Panel min-width matches trigger width."
                </div>
            </Popover>
        </div>
    }
}

#[component]
fn CloseOnScrollDemo() -> impl IntoView {
    let (open, set_open) = signal(false);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    view! {
        <div class="space-y-2">
            <div class="text-xs text-text-secondary">
                "Scroll the outer page — the popover closes instead of repositioning."
            </div>
            <button
                node_ref=trigger_ref
                type="button"
                class="btn btn--secondary btn--md btn--rounded"
                on:click=move |_| set_open.update(|v| *v = !*v)
            >
                "Open (close_on_scroll)"
            </button>
            <Popover
                open=open
                on_close=Callback::new(move |_: ()| set_open.set(false))
                anchor=anchor
                close_on_scroll=true
            >
                <div class="p-3 text-sm text-text-primary">
                    "I will close on scroll."
                </div>
            </Popover>
        </div>
    }
}

