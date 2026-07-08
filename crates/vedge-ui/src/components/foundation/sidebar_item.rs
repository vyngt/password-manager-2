use crate::primitives::text_prop::TextProp;
use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn SidebarItem(
    #[prop(into)] label: TextProp,
    #[prop(optional)] icon: Option<Children>,
    #[prop(optional)] badge: Option<Children>,
    #[prop(into, default = Signal::stored(false))] selected: Signal<bool>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] href: Option<&'static str>,
    #[prop(into, default = None)] on_click: Option<Callback<()>>,
    #[prop(optional, default = 0)] indent: u8,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    /// Extra classes. `TextProp` so consumers can pass a reactive `Signal<String>`
    /// (e.g. a drag-over drop-target highlight) as well as a static literal.
    #[prop(into, default = TextProp::default())]
    class: TextProp,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if href.is_some() && on_click.is_some() {
        web_sys::console::error_1(
            &"SidebarItem: `href` and `on_click` are mutually exclusive. Provide one, not both."
                .into(),
        );
    }

    #[cfg(debug_assertions)]
    if indent > 2 {
        web_sys::console::error_1(
            &"SidebarItem: `indent` must be 0, 1, or 2 — deeper nesting is an IA problem.".into(),
        );
    }

    let indent_cls = match indent {
        1 => "sidebar-item--indent-1",
        2 => "sidebar-item--indent-2",
        _ => "",
    };

    let disabled_cls = if disabled { "sidebar-item--disabled" } else { "" };

    let root_cls = move || {
        let selected_cls = if selected.get() {
            "sidebar-item--selected"
        } else {
            ""
        };
        let extra = class.get();
        ["sidebar-item", indent_cls, selected_cls, disabled_cls, extra.as_str()].join(" ")
    };

    let aria_current = move || selected.get().then_some("page");
    let aria_label_attr = move || {
        let v = aria_label.get();
        if v.is_empty() { None } else { Some(v) }
    };

    if let Some(href_val) = href {
        let effective_href = if disabled { None } else { Some(href_val) };
        let aria_disabled = if disabled { Some("true") } else { None };
        let tabindex = if disabled { Some("-1") } else { None };

        let handle_click = move |ev: web_sys::MouseEvent| {
            if disabled {
                ev.prevent_default();
                return;
            }
            if let Some(cb) = on_click {
                cb.run(());
            }
        };

        Either::Left(view! {
            <a
                class=root_cls
                href=effective_href
                aria-current=aria_current
                aria-disabled=aria_disabled
                tabindex=tabindex
                aria-label=aria_label_attr
                on:click=handle_click
            >
                {icon
                    .map(|i| {
                        view! {
                            <span class="sidebar-item__icon" aria-hidden="true">
                                {i()}
                            </span>
                        }
                    })}
                <span class="sidebar-item__label">{move || label.get()}</span>
                {badge.map(|b| view! { <span class="sidebar-item__badge">{b()}</span> })}
            </a>
        })
    } else {
        let handle_click = move |_: web_sys::MouseEvent| {
            if let Some(cb) = on_click {
                cb.run(());
            }
        };

        Either::Right(view! {
            <button
                type="button"
                class=root_cls
                disabled=disabled
                aria-current=aria_current
                aria-label=aria_label_attr
                on:click=handle_click
            >
                {icon
                    .map(|i| {
                        view! {
                            <span class="sidebar-item__icon" aria-hidden="true">
                                {i()}
                            </span>
                        }
                    })}
                <span class="sidebar-item__label">{move || label.get()}</span>
                {badge.map(|b| view! { <span class="sidebar-item__badge">{b()}</span> })}
            </button>
        })
    }
}
