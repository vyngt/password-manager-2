use crate::primitives::tokens::{AvatarSize, AvatarStatus};
use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn Avatar(
    #[prop(optional, default = String::new())] src: String,
    #[prop(optional, default = String::new())] name: String,
    #[prop(optional)] size: AvatarSize,
    #[prop(into, default = None)] status: Option<AvatarStatus>,
    #[prop(optional)] interactive: bool,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_click: Option<Callback<web_sys::MouseEvent>>,
    #[prop(optional, default = String::new())] aria_label: String,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let initials = derive_initials(&name);
    let has_src = !src.trim().is_empty();
    let has_initials = !initials.is_empty();

    let loaded = RwSignal::new(false);
    let errored = RwSignal::new(false);

    let root_cls = ["avatar-root", size.avatar_class(), class].join(" ");

    let fallback_view = if has_initials {
        Either::Left(view! {
            <span class="avatar-initials" aria-hidden="true">
                {initials}
            </span>
        })
    } else {
        Either::Right(view! {
            <svg
                class="avatar-fallback-icon"
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
            >
                <path d="M12 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm0 2c-4.42 0-8 2.69-8 6v2h16v-2c0-3.31-3.58-6-8-6Z" />
            </svg>
        })
    };

    let image_view = if has_src {
        let alt_name = name.clone();
        let img_cls = move || {
            if loaded.get() {
                "avatar-image avatar-image--loaded"
            } else {
                "avatar-image"
            }
        };
        let hidden_attr = move || errored.get().then_some("true");
        Some(view! {
            <img
                class=img_cls
                src=src
                alt=alt_name
                aria-hidden=hidden_attr
                on:load=move |_| loaded.set(true)
                on:error=move |_| errored.set(true)
            />
        })
    } else {
        None
    };

    let status_view = status.map(|s| {
        view! {
            <span class="avatar-status-wrap" role="img" aria-label=s.aria_label()>
                <span class=format!("avatar-status {}", s.avatar_class()) aria-hidden="true"></span>
            </span>
        }
    });

    let media = view! { <span class="avatar-media">{fallback_view} {image_view}</span> };

    if interactive {
        let label = if !aria_label.is_empty() {
            aria_label
        } else if !name.is_empty() {
            name
        } else {
            String::from("User avatar")
        };
        Either::Left(view! {
            <button
                type="button"
                class=root_cls
                disabled=disabled
                aria-label=label
                on:click=move |ev: web_sys::MouseEvent| {
                    if let Some(cb) = on_click {
                        cb.run(ev);
                    }
                }
            >
                {media}
                {status_view}
            </button>
        })
    } else {
        let label = if !name.is_empty() {
            name
        } else {
            String::from("User avatar")
        };
        Either::Right(view! {
            <span class=root_cls role="img" aria-label=label>
                {media}
                {status_view}
            </span>
        })
    }
}

fn derive_initials(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let first_ch = words.first().and_then(|w| w.chars().next());
    let last_ch = if words.len() >= 2 {
        words.last().and_then(|w| w.chars().next())
    } else {
        None
    };
    let mut out = String::new();
    if let Some(c) = first_ch {
        out.extend(c.to_uppercase());
    }
    if let Some(c) = last_ch {
        out.extend(c.to_uppercase());
    }
    out.chars().take(2).collect()
}
