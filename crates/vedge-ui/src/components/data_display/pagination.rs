use crate::components::form::select::{Select, SelectItem};
use crate::components::foundation::button::Button;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Size, Variant};
use crate::utils::text::text_or;
use icondata as i;
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos_icons::Icon;
use std::collections::BTreeSet;

/// Which pagination model the consumer is driving.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaginationModel {
    /// Total page count is known — renders page numbers, summary label,
    /// and items-per-page select.
    #[default]
    Offset,
    /// Only next/prev cursors are known — renders only Prev/Next.
    Cursor,
}

#[component]
pub fn Pagination(
    #[prop(optional)] model: PaginationModel,

    // ---- Offset model ----
    #[prop(into, default = Signal::stored(1))] page: Signal<u32>,
    #[prop(into, default = Signal::stored(1))] total_pages: Signal<u32>,
    #[prop(into, default = None)] total_items: Option<Signal<u32>>,
    #[prop(into, default = None)] on_page_change: Option<Callback<u32>>,

    // ---- Cursor model ----
    #[prop(into, default = Signal::stored(false))] has_prev_page: Signal<bool>,
    #[prop(into, default = Signal::stored(false))] has_next_page: Signal<bool>,
    #[prop(into, default = None)] on_prev: Option<Callback<()>>,
    #[prop(into, default = None)] on_next: Option<Callback<()>>,

    // ---- Items-per-page (both models) ----
    #[prop(into, default = None)] page_size: Option<Signal<u32>>,
    #[prop(optional, default = Vec::new())] page_size_options: Vec<u32>,
    #[prop(into, default = None)] on_page_size_change: Option<Callback<u32>>,

    #[prop(optional)] compact: bool,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = TextProp::from("Previous page"))] prev_label: TextProp,
    #[prop(into, default = TextProp::from("Next page"))] next_label: TextProp,
    #[prop(into, default = TextProp::from("Rows per page"))] page_size_label: TextProp,
    #[prop(into, default = TextProp::from("Pagination"))] nav_label: TextProp,
    /// Localized summary text, already interpolated by the consumer (e.g.
    /// "Showing 1–20 of 57"). Empty → the built-in English "start–end of total".
    /// Rendered in the Offset model when `total_items` + `page_size` are supplied.
    #[prop(into, default = TextProp::default())]
    summary: TextProp,
    /// Localized zero-state summary. Empty → the built-in English "0 of 0".
    #[prop(into, default = TextProp::default())]
    summary_empty: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let root_cls = ["pagination", class].join(" ");

    // ---- Left zone: items-per-page select ----
    // `page_size` is `Some` and there are ≥2 options → render the control (and move
    // `page_size_options` into the closure, consuming it).
    let has_size_options = page_size_options.len() >= 2;
    let size_control = page_size.filter(|_| has_size_options).map(move |ps| {
        let value_sig: Signal<String> = Signal::derive(move || ps.get().to_string());
        let select_items: Vec<SelectItem> = page_size_options
            .into_iter()
            .map(|n| SelectItem::option(n.to_string(), n.to_string()))
            .collect();
        let on_select_change = on_page_size_change.map(|cb| {
            Callback::new(move |s: String| {
                if let Ok(n) = s.parse::<u32>() {
                    cb.run(n);
                }
            })
        });
        let select_id = "pagination-size-select";
        view! {
            <div class="pagination__size-control">
                <label for=select_id>{move || page_size_label.get()}</label>
                <Select
                    options=select_items
                    value=value_sig
                    on_change=on_select_change
                    size=Size::Sm
                    disabled=disabled
                    class="pagination__size-select"
                />
            </div>
        }
    });

    // ---- Center zone: summary label (offset only, when total_items provided) ----
    // Consumer-provided localized `summary`/`summary_empty` win; the built-in
    // English "start–end of total" is the standalone fallback.
    let summary_view = (model == PaginationModel::Offset)
        .then(|| {
            total_items.zip(page_size).map(|(items_sig, size_sig)| {
                let p = page;
                view! {
                    <span class="pagination__summary" aria-live="polite">
                        {move || {
                            let total = items_sig.get();
                            if total == 0 {
                                return text_or(summary_empty, "0 of 0");
                            }
                            let localized = summary.get();
                            if !localized.is_empty() {
                                return localized;
                            }
                            let size = size_sig.get().max(1);
                            let current = p.get().max(1);
                            let start = (current - 1) * size + 1;
                            let end = (current * size).min(total);
                            format!("{start}\u{2013}{end} of {total}")
                        }}
                    </span>
                }
            })
        })
        .flatten();

    // ---- Right zone: page controls ----
    let controls = match model {
        PaginationModel::Offset => {
            if compact {
                EitherOf3::A(view! {
                    <div class="pagination__controls">
                        <PrevButton
                            label=prev_label
                            disabled=Signal::derive(move || disabled || page.get() <= 1)
                            on_click=Callback::new(move |(): ()| {
                                let p = page.get_untracked();
                                if p > 1 {
                                    if let Some(cb) = on_page_change {
                                        cb.run(p - 1);
                                    }
                                }
                            })
                        />
                        <span class="pagination__compact-label">
                            {move || format!("Page {} of {}", page.get(), total_pages.get())}
                        </span>
                        <NextButton
                            label=next_label
                            disabled=Signal::derive(move || {
                                disabled || page.get() >= total_pages.get()
                            })
                            on_click=Callback::new(move |(): ()| {
                                let p = page.get_untracked();
                                let tp = total_pages.get_untracked();
                                if p < tp {
                                    if let Some(cb) = on_page_change {
                                        cb.run(p + 1);
                                    }
                                }
                            })
                        />
                    </div>
                })
            } else {
                EitherOf3::B(view! {
                    <div class="pagination__controls">
                        <PrevButton
                            label=prev_label
                            disabled=Signal::derive(move || disabled || page.get() <= 1)
                            on_click=Callback::new(move |(): ()| {
                                let p = page.get_untracked();
                                if p > 1 {
                                    if let Some(cb) = on_page_change {
                                        cb.run(p - 1);
                                    }
                                }
                            })
                        />
                        {move || {
                            let current = page.get();
                            let total = total_pages.get();
                            build_page_window(current, total)
                                .into_iter()
                                .enumerate()
                                .map(|(idx, item)| match item {
                                    PageItem::Page(n) => {
                                        let is_current = n == current;
                                        let variant = if is_current {
                                            Variant::Primary
                                        } else {
                                            Variant::Ghost
                                        };
                                        let aria_current = if is_current {
                                            Some("page")
                                        } else {
                                            None
                                        };
                                        Either::Left(
                                            view! {
                                                <Button
                                                    variant=variant
                                                    size=Size::Sm
                                                    disabled=disabled
                                                    class="pagination__page-btn"
                                                    aria_label=format!("Page {}", n)
                                                    on:click=move |_| {
                                                        if !is_current {
                                                            if let Some(cb) = on_page_change {
                                                                cb.run(n);
                                                            }
                                                        }
                                                    }
                                                    attr:aria-current=aria_current
                                                >
                                                    {n.to_string()}
                                                </Button>
                                            },
                                        )
                                    }
                                    PageItem::Ellipsis => {
                                        Either::Right(
                                            view! {
                                                <span
                                                    class="pagination__ellipsis"
                                                    aria-hidden="true"
                                                    data-idx=idx
                                                >
                                                    "\u{2026}"
                                                </span>
                                            },
                                        )
                                    }
                                })
                                .collect_view()
                        }}
                        <NextButton
                            label=next_label
                            disabled=Signal::derive(move || {
                                disabled || page.get() >= total_pages.get()
                            })
                            on_click=Callback::new(move |(): ()| {
                                let p = page.get_untracked();
                                let tp = total_pages.get_untracked();
                                if p < tp {
                                    if let Some(cb) = on_page_change {
                                        cb.run(p + 1);
                                    }
                                }
                            })
                        />
                    </div>
                })
            }
        }
        PaginationModel::Cursor => EitherOf3::C(view! {
            <div class="pagination__controls">
                <PrevButton
                    label=prev_label
                    disabled=Signal::derive(move || disabled || !has_prev_page.get())
                    on_click=Callback::new(move |(): ()| {
                        if has_prev_page.get_untracked() {
                            if let Some(cb) = on_prev {
                                cb.run(());
                            }
                        }
                    })
                />
                <NextButton
                    label=next_label
                    disabled=Signal::derive(move || disabled || !has_next_page.get())
                    on_click=Callback::new(move |(): ()| {
                        if has_next_page.get_untracked() {
                            if let Some(cb) = on_next {
                                cb.run(());
                            }
                        }
                    })
                />
            </div>
        }),
    };

    view! {
        <nav aria-label=move || nav_label.get() class=root_cls>
            {size_control}
            {summary_view}
            {controls}
        </nav>
    }
}

// ---- Internal button wrappers ----
// Button's `disabled` prop is taken by value, so we wrap the ghost prev/next
// buttons in a reactive closure when the disabled state needs to track a signal.

#[component]
fn PrevButton(label: TextProp, disabled: Signal<bool>, on_click: Callback<()>) -> impl IntoView {
    view! {
        {move || {
            let is_disabled = disabled.get();
            view! {
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    disabled=is_disabled
                    aria_label=label
                    on:click=move |_| {
                        if !is_disabled {
                            on_click.run(());
                        }
                    }
                >
                    <Icon icon=i::FaChevronLeftSolid />
                </Button>
            }
        }}
    }
}

#[component]
fn NextButton(label: TextProp, disabled: Signal<bool>, on_click: Callback<()>) -> impl IntoView {
    view! {
        {move || {
            let is_disabled = disabled.get();
            view! {
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    disabled=is_disabled
                    aria_label=label
                    on:click=move |_| {
                        if !is_disabled {
                            on_click.run(());
                        }
                    }
                >
                    <Icon icon=i::FaChevronRightSolid />
                </Button>
            }
        }}
    }
}

// ---- Page window algorithm ----

#[derive(Debug, Clone, Copy, PartialEq)]
enum PageItem {
    Page(u32),
    Ellipsis,
}

fn build_page_window(current: u32, total: u32) -> Vec<PageItem> {
    if total == 0 {
        return Vec::new();
    }
    if total <= 7 {
        return (1..=total).map(PageItem::Page).collect();
    }

    let current = current.clamp(1, total);

    let mut pages: BTreeSet<u32> = BTreeSet::new();
    pages.insert(1);
    pages.insert(total);

    for delta in [-1i32, 0, 1] {
        let p = current as i32 + delta;
        if p >= 1 && p <= total as i32 {
            pages.insert(p as u32);
        }
    }

    // Near the start: extend window forward so there's no [1]···[3] gap.
    if current <= 3 {
        for p in 1..=(current + 2).min(total) {
            pages.insert(p);
        }
    }

    // Near the end: include the final 3 pages.
    if current + 2 >= total {
        let start = total.saturating_sub(2).max(1);
        for p in start..=total {
            pages.insert(p);
        }
    }

    let mut items = Vec::new();
    let mut prev: Option<u32> = None;
    for p in pages {
        if let Some(pv) = prev {
            if p > pv + 1 {
                items.push(PageItem::Ellipsis);
            }
        }
        items.push(PageItem::Page(p));
        prev = Some(p);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(items: &[PageItem]) -> Vec<i32> {
        items
            .iter()
            .map(|it| match it {
                PageItem::Page(n) => *n as i32,
                PageItem::Ellipsis => -1,
            })
            .collect()
    }

    #[test]
    fn total_le_7_shows_all() {
        assert_eq!(pages(&build_page_window(1, 5)), vec![1, 2, 3, 4, 5]);
        assert_eq!(pages(&build_page_window(4, 7)), vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn start_edge() {
        // current = 1
        assert_eq!(pages(&build_page_window(1, 25)), vec![1, 2, 3, -1, 25]);
        // current = 2
        assert_eq!(pages(&build_page_window(2, 25)), vec![1, 2, 3, 4, -1, 25]);
        // current = 3
        assert_eq!(
            pages(&build_page_window(3, 25)),
            vec![1, 2, 3, 4, 5, -1, 25]
        );
    }

    #[test]
    fn middle() {
        assert_eq!(
            pages(&build_page_window(10, 25)),
            vec![1, -1, 9, 10, 11, -1, 25]
        );
    }

    #[test]
    fn end_edge() {
        assert_eq!(pages(&build_page_window(24, 25)), vec![1, -1, 23, 24, 25]);
        assert_eq!(pages(&build_page_window(25, 25)), vec![1, -1, 23, 24, 25]);
    }
}
