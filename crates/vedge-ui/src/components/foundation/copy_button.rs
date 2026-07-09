use crate::components::feedback::toast::types::{ToastInput, ToastState};
use crate::components::foundation::tooltip_icon_button::TooltipIconButton;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Shape, Size, ToastVariant, Variant};
use crate::utils::text::text_or;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyState {
    Idle,
    Copied,
}

/// One-click copy-to-clipboard with visual feedback.
///
/// Icon swaps Copy → Check on success, `aria-label` swaps to `copied_label`,
/// and the background pulses `--color-success-muted` for 2s before reverting.
/// Clipboard failures (no permission, insecure context) are silent — the
/// button stays idle and `on_copy(false)` fires so consumers can react.
#[component]
pub fn CopyButton(
    #[prop(into)] value: Signal<String>,
    #[prop(into, default = TextProp::from("Copy"))] label: TextProp,
    #[prop(into, default = TextProp::from("Copied"))] copied_label: TextProp,
    #[prop(into, default = TextProp::from("clears in {n}s"))] countdown_label: TextProp,
    #[prop(into, default = TextProp::from("Dismiss"))] toast_dismiss_label: TextProp,
    #[prop(optional, default = Variant::Ghost)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] shape: Shape,
    #[prop(optional)] show_toast: bool,
    #[prop(optional, default = None)] countdown: Option<u32>,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_copy: Option<Callback<bool>>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if countdown.is_some() && !show_toast {
        web_sys::console::warn_1(
            &"CopyButton: `countdown` has no effect unless `show_toast=true`.".into(),
        );
    }

    let toast_state = use_context::<ToastState>();

    #[cfg(debug_assertions)]
    if show_toast && toast_state.is_none() {
        web_sys::console::warn_1(
            &"CopyButton: `show_toast=true` requires a <ToastProvider> ancestor — toast will be skipped.".into(),
        );
    }

    let state = RwSignal::new(CopyState::Idle);
    let revert_ver = Arc::new(AtomicU32::new(0));

    let effective_label: TextProp = Signal::derive(move || match state.get() {
        CopyState::Idle => text_or(label, "Copy"),
        CopyState::Copied => text_or(copied_label, "Copied"),
    })
    .into();

    let wrapper_cls = move || {
        let state_cls = match state.get() {
            CopyState::Idle => "copy-btn--idle",
            CopyState::Copied => "copy-btn--copied",
        };
        ["copy-btn", state_cls, class].join(" ")
    };

    let revert_ver_click = revert_ver.clone();
    let handle_click = Callback::new(move |_: ()| {
        let Some(window) = web_sys::window() else {
            if let Some(cb) = on_copy {
                cb.run(false);
            }
            return;
        };
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();
        let value_now = value.get_untracked();
        let promise = clipboard.write_text(&value_now);

        let ticket = revert_ver_click.fetch_add(1, Ordering::Relaxed) + 1;
        let ver = revert_ver_click.clone();

        spawn_local(async move {
            match JsFuture::from(promise).await {
                Ok(_) => {
                    state.set(CopyState::Copied);
                    if let Some(cb) = on_copy {
                        cb.run(true);
                    }

                    if show_toast {
                        if let Some(toast) = toast_state {
                            let msg = build_toast_message(copied_label, countdown_label, countdown);
                            toast.show(
                                ToastInput::new(msg)
                                    .variant(ToastVariant::Success)
                                    .dismiss_label(text_or(toast_dismiss_label, "Dismiss")),
                            );
                        }
                    }

                    set_timeout(
                        move || {
                            if ver.load(Ordering::Relaxed) != ticket {
                                return;
                            }
                            state.set(CopyState::Idle);
                        },
                        Duration::from_millis(2000),
                    );
                }
                Err(_) => {
                    if let Some(cb) = on_copy {
                        cb.run(false);
                    }
                }
            }
        });
    });

    view! {
        <span class=wrapper_cls>
            <TooltipIconButton
                label=effective_label
                variant=variant
                size=size
                shape=shape
                disabled=disabled
                on_click=handle_click
            >
                <Show
                    when=move || state.get() == CopyState::Copied
                    fallback=|| view! { <Icon icon=i::FaCopySolid /> }
                >
                    <span class="copy-btn__icon-check">
                        <Icon icon=i::FaCheckSolid />
                    </span>
                </Show>
            </TooltipIconButton>
        </span>
    }
}

fn build_toast_message(
    copied_label: TextProp,
    countdown_label: TextProp,
    countdown: Option<u32>,
) -> String {
    let base = text_or(copied_label, "Copied");
    match countdown {
        Some(n) => {
            let suffix = text_or(countdown_label, "clears in {n}s").replace("{n}", &n.to_string());
            format!("{base} · {suffix}")
        }
        None => base,
    }
}
