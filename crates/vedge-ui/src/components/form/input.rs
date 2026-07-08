mod variants;

use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Size, Status};
use icondata as i;
use leptos::ev::Targeted;
use leptos::prelude::*;
use leptos_icons::Icon;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn Input(
    id: &'static str,
    #[prop(optional)] size: Size,
    #[prop(optional)] status: Status,
    #[prop(optional, default = "text")] input_type: &'static str,
    #[prop(into, default = None)] value: Option<Signal<String>>,
    #[prop(into, default = None)] placeholder: Option<Signal<String>>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] read_only: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] leading_icon: Option<Children>,
    #[prop(optional)] trailing_icon: Option<Children>,
    #[prop(optional, default = "")] prefix: &'static str,
    #[prop(optional, default = "")] suffix: &'static str,
    #[prop(into, default = None)] on_input: Option<Callback<String>>,
    #[prop(optional)] required: bool,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(into, default = TextProp::default())] clear_label: TextProp,
    #[prop(into, default = TextProp::default())] reveal_label: TextProp,
    #[prop(into, default = TextProp::default())] hide_label: TextProp,
    /// Optional external ref to the native `<input>`. When supplied, callers can
    /// imperatively `focus()`/`select()` the element (e.g. inline rename fields).
    /// Reconciled with the internal ref so the search clear-button still refocuses.
    #[prop(into, default = None)]
    input_ref: Option<NodeRef<leptos::html::Input>>,
    /// Focus the field on mount (HTML `autofocus`). Useful for fields revealed by
    /// a `<Show>`/`Either` (new-folder, inline rename) that should grab focus.
    #[prop(optional)]
    autofocus: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    // Dev assertions
    #[cfg(debug_assertions)]
    variants::assert_supported_type(input_type);

    #[cfg(debug_assertions)]
    if leading_icon.is_some() && !prefix.is_empty() {
        web_sys::console::error_1(
            &"Input: `leading_icon` and `prefix` are mutually exclusive.".into(),
        );
    }

    #[cfg(debug_assertions)]
    if input_type == "search" && clear_label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"Input: type=\"search\" requires `clear_label` for i18n.".into(),
        );
    }

    #[cfg(debug_assertions)]
    if input_type == "password"
        && (reveal_label.get_untracked().is_empty() || hide_label.get_untracked().is_empty())
    {
        web_sys::console::error_1(
            &"Input: type=\"password\" requires `reveal_label` and `hide_label` for i18n.".into(),
        );
    }

    // Internal state
    let (revealed, set_revealed) = signal(false);
    // Use the caller's ref when provided; otherwise create our own. Either way the
    // component binds this ref to the native input, so `handle_clear` (search) and
    // any external `focus()`/`select()` operate on the same element.
    let input_ref = input_ref.unwrap_or_else(NodeRef::<leptos::html::Input>::new);

    let is_password = input_type == "password";
    let is_search = input_type == "search";

    // Derived values
    let effective_type = move || {
        if is_password && revealed.get() {
            "text"
        } else {
            input_type
        }
    };

    let has_value = move || value.map(|s| !s.get().is_empty()).unwrap_or(false);

    // Root class
    let root_cls = [
        "input-root",
        size.input_root_class(),
        status.input_root_class(),
        if disabled { "input-root--disabled" } else { "" },
        if read_only {
            "input-root--readonly"
        } else {
            ""
        },
        if loading { "input-root--loading" } else { "" },
        class,
    ]
    .join(" ");

    // Event handlers
    let handle_input = move |ev: Targeted<Event, HtmlInputElement>| {
        if let Some(cb) = on_input {
            cb.run(ev.target().value());
        }
    };

    let handle_clear = move |_: web_sys::MouseEvent| {
        if let Some(cb) = on_input {
            cb.run(String::new());
        }
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    // ARIA
    let aria_invalid = if status == Status::Error {
        Some("true")
    } else {
        None
    };
    let aria_describedby_attr = if aria_describedby.is_empty() {
        None
    } else {
        Some(aria_describedby)
    };
    let aria_required_attr = if required { Some("true") } else { None };

    view! {
        <div class=root_cls>
            // Leading area
            {leading_icon.map(|icon| view! { <span class="input-icon">{icon()}</span> })}
            {(!prefix.is_empty()).then(|| view! { <span class="input-affix">{prefix}</span> })}
            // Native input
            <input
                node_ref=input_ref
                class="input-native"
                id=id
                type=effective_type
                autofocus=autofocus
                disabled=disabled
                readonly=read_only
                prop:value=move || value.map(|s| s.get()).unwrap_or_default()
                placeholder=move || placeholder.map(|s| s.get()).unwrap_or_default()
                aria-invalid=aria_invalid
                aria-describedby=aria_describedby_attr
                aria-required=aria_required_attr
                on:input:target=handle_input
            /> // Suffix
            {(!suffix.is_empty()).then(|| view! { <span class="input-affix">{suffix}</span> })}
            // Trailing area — priority resolution
            {if loading {
                Some(view! { <span class="input-spinner"></span> }.into_any())
            } else if status != Status::Default {
                let icon_data = match status {
                    Status::Error => i::FaCircleExclamationSolid,
                    Status::Success => i::FaCircleCheckSolid,
                    Status::Warning => i::FaTriangleExclamationSolid,
                    Status::Default => unreachable!(),
                };
                Some(
                    view! {
                        <span class="input-status-icon">
                            <Icon icon=icon_data />
                        </span>
                    }
                        .into_any(),
                )
            } else if is_search {
                Some(
                    view! {
                        <Show when=has_value>
                            <button
                                type="button"
                                class="input-trailing-btn"
                                aria-label=move || clear_label.get()
                                on:click=handle_clear
                            >
                                <Icon icon=i::FaXmarkSolid />
                            </button>
                        </Show>
                    }
                        .into_any(),
                )
            } else if is_password {
                Some(
                    view! {
                        <button
                            type="button"
                            class="input-trailing-btn"
                            aria-label=move || {
                                if revealed.get() { hide_label.get() } else { reveal_label.get() }
                            }
                            on:click=move |_| set_revealed.update(|r| *r = !*r)
                        >
                            <Show
                                when=move || revealed.get()
                                fallback=|| view! { <Icon icon=i::FaEyeSolid /> }
                            >
                                <Icon icon=i::FaEyeSlashSolid />
                            </Show>
                        </button>
                    }
                        .into_any(),
                )
            } else if let Some(icon) = trailing_icon {
                Some(view! { <span class="input-icon">{icon()}</span> }.into_any())
            } else {
                None
            }}
        </div>
    }
}
