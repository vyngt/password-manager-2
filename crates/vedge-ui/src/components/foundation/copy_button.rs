use crate::components::feedback::toast::types::{ToastInput, ToastState};
use crate::components::foundation::tooltip_icon_button::TooltipIconButton;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Shape, Size, ToastVariant, Variant};
use crate::utils::text::text_or;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyState {
    Idle,
    Copied,
}

/// The future a [`CopyButton`] `copy_with` strategy returns: resolves to `true`
/// when the write succeeded, `false` otherwise.
pub type CopyFuture = Pin<Box<dyn Future<Output = bool>>>;

/// Async clipboard-write strategy for [`CopyButton`]'s `copy_with` prop — given
/// the current value, it performs the write and reports success.
pub type CopyWriter = Callback<String, CopyFuture>;

/// One-click copy-to-clipboard with visual feedback.
///
/// Icon swaps Copy → Check on success, `aria-label` swaps to `copied_label`,
/// and the background pulses `--color-success-muted` for 2s before reverting.
/// Clipboard failures (no permission, insecure context) are silent — the
/// button stays idle and `on_copy(false)` fires so consumers can react.
///
/// By default the write goes through the browser Clipboard API. Pass
/// [`copy_with`] to instead route the value through a caller-supplied strategy
/// (e.g. a hardened native command) — see the prop docs.
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
    /// Optional write strategy. When `Some`, the copy is performed by this
    /// callback (given the current value) instead of the browser Clipboard API
    /// — used to route secrets through the hardened native `copy_text` command
    /// (slice 3.2) so they carry the OS no-history / no-cloud hints. The callback
    /// returns a future resolving to `true` on success; the button swaps to the
    /// check-mark only when that future reports success, so the indicator can't
    /// disagree with the real outcome.
    #[prop(into, default = None)]
    copy_with: Option<CopyWriter>,
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

    let revert_ver_click = revert_ver;
    let handle_click = Callback::new(move |(): ()| {
        // Success transition shared by both write strategies: icon → check,
        // `on_copy(true)`, optional toast, and the 2 s revert (ticket-guarded so
        // a later copy doesn't get reverted by an earlier one's timer).
        let ticket = revert_ver_click.fetch_add(1, Ordering::Relaxed) + 1;
        let ver = Arc::clone(&revert_ver_click);
        let mark_copied = move || {
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
                Duration::from_secs(2),
            );
        };

        // Native strategy (slice 3.2): hand the value to the caller's writer
        // (the hardened `copy_text` command). It reports success via the future
        // it returns, so the check-mark reflects the real outcome instead of
        // firing optimistically.
        if let Some(cw) = copy_with {
            let writing = cw.run(value.get_untracked());
            spawn_local(async move {
                if writing.await {
                    mark_copied();
                } else if let Some(cb) = on_copy {
                    cb.run(false);
                }
            });
            return;
        }

        // Default strategy: the browser Clipboard API.
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

        spawn_local(async move {
            match JsFuture::from(promise).await {
                Ok(_) => mark_copied(),
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
