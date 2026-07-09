use leptos::prelude::*;
use vedge_ui::components::data_display::pagination::{Pagination, PaginationModel};

use super::common::Section;

#[component]
pub fn PaginationPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-5xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Pagination"</h1>

            <OffsetBasicSection />
            <OffsetFullSection />
            <OffsetSmallTotalSection />
            <OffsetCompactSection />
            <CursorSection />
            <DisabledSection />
            <PageWindowSection />
        </div>
    }
}

#[component]
fn OffsetBasicSection() -> impl IntoView {
    let page = RwSignal::new(1u32);
    let total_pages = Signal::stored(25u32);
    view! {
        <Section title="Offset — Prev + page numbers + Next">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=total_pages
                on_page_change=Callback::new(move |p: u32| page.set(p))
            />
            <div class="text-xs text-text-tertiary mt-2">"current page: " {move || page.get()}</div>
        </Section>
    }
}

#[component]
fn OffsetFullSection() -> impl IntoView {
    let page = RwSignal::new(1u32);
    let page_size = RwSignal::new(10u32);
    let page_size_sig: Signal<u32> = page_size.into();
    let total_items = Signal::stored(247u32);
    let total_pages = Signal::derive(move || {
        let items = total_items.get();
        let size = page_size.get().max(1);
        items.div_ceil(size)
    });

    view! {
        <Section title="Offset — full bar (size select + summary + controls)">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=total_pages
                total_items=total_items
                page_size=page_size_sig
                page_size_options=vec![10, 25, 50, 100]
                on_page_change=Callback::new(move |p: u32| page.set(p))
                on_page_size_change=Callback::new(move |s: u32| {
                    page_size.set(s);
                    page.set(1);
                })
            />
            <div class="text-xs text-text-tertiary mt-2">
                "page " {move || page.get()} " · size " {move || page_size.get()}
                " · derived total_pages " {move || total_pages.get()}
            </div>
        </Section>
    }
}

#[component]
fn OffsetSmallTotalSection() -> impl IntoView {
    let page = RwSignal::new(2u32);
    view! {
        <Section title="Offset — total_pages ≤ 7 (all pages, no ellipsis)">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=Signal::stored(5u32)
                on_page_change=Callback::new(move |p: u32| page.set(p))
            />
        </Section>
    }
}

#[component]
fn OffsetCompactSection() -> impl IntoView {
    let page = RwSignal::new(3u32);
    view! {
        <Section title="Offset — compact (Prev / \"Page X of Y\" / Next)">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=Signal::stored(25u32)
                compact=true
                on_page_change=Callback::new(move |p: u32| page.set(p))
            />
        </Section>
    }
}

#[component]
fn CursorSection() -> impl IntoView {
    let step = RwSignal::new(1i32);
    let has_prev = Signal::derive(move || step.get() > 0);
    let has_next = Signal::derive(move || step.get() < 4);

    view! {
        <Section title="Cursor — only Prev / Next">
            <Pagination
                model=PaginationModel::Cursor
                has_prev_page=has_prev
                has_next_page=has_next
                on_prev=Callback::new(move |_: ()| step.update(|s| *s -= 1))
                on_next=Callback::new(move |_: ()| step.update(|s| *s += 1))
            />
            <div class="text-xs text-text-tertiary mt-2">
                "cursor step: " {move || step.get()} " (has_prev="
                {move || has_prev.get().to_string()} ", has_next="
                {move || has_next.get().to_string()} ")"
            </div>
        </Section>
    }
}

#[component]
fn DisabledSection() -> impl IntoView {
    let page = RwSignal::new(3u32);
    view! {
        <Section title="Disabled (loading state)">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=Signal::stored(25u32)
                page_size=Signal::stored(10u32)
                page_size_options=vec![10, 25, 50]
                total_items=Signal::stored(247u32)
                disabled=true
                on_page_change=Callback::new(move |p: u32| page.set(p))
            />
        </Section>
    }
}

#[component]
fn PageWindowSection() -> impl IntoView {
    let page = RwSignal::new(1u32);
    view! {
        <Section title="Page window — jump around to test edge cases">
            <Pagination
                model=PaginationModel::Offset
                page=page
                total_pages=Signal::stored(25u32)
                on_page_change=Callback::new(move |p: u32| page.set(p))
            />
            <div class="flex gap-2 mt-3">
                {[1u32, 2, 3, 5, 10, 13, 20, 23, 24, 25]
                    .into_iter()
                    .map(|target| {
                        view! {
                            <button
                                class="px-2 py-1 text-xs rounded border border-border bg-background text-text-primary hover:bg-surface-2"
                                on:click=move |_| page.set(target)
                            >
                                {format!("jump to {target}")}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </Section>
    }
}
