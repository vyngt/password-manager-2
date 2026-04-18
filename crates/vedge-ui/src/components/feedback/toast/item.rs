use super::types::{ToastData, ToastState};

use crate::primitives::tokens::ToastVariant;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::time::Duration;


#[component]
pub fn ToastItem(toast: ToastData) -> impl IntoView {
    let state = expect_context::<ToastState>();
    let toast_id = toast.id;
    let variant = toast.variant;
    let duration = toast.duration;
    let dismiss_label = toast.dismiss_label.clone();

    // --- Timer with hover pause/resume (generation-based) ---
    let remaining = RwSignal::new(duration as f64);
    let timer_start = RwSignal::new(0.0_f64);
    let generation = RwSignal::new(0u32);

    let start_timer = move || {
        if duration == 0 {
            return;
        }
        let rem = remaining.get_untracked();
        if rem <= 0.0 {
            return;
        }

        let next_gen = generation.get_untracked() + 1;
        generation.set(next_gen);
        timer_start.set(js_sys::Date::now());

        set_timeout(
            move || {
                // Only fire if this is still the active generation
                if generation.try_get_untracked() == Some(next_gen) {
                    state.dismiss(toast_id);
                }
            },
            Duration::from_millis(rem as u64),
        );
    };

    let pause_timer = move || {
        // Bump generation to invalidate the running timeout
        generation.update(|g| *g += 1);
        let elapsed = js_sys::Date::now() - timer_start.get_untracked();
        remaining.update(|r| *r = (*r - elapsed).max(0.0));
    };

    // Start auto-dismiss timer
    start_timer();

    // --- Event handlers ---
    let on_mouseenter = move |_: web_sys::MouseEvent| {
        pause_timer();
    };
    let on_mouseleave = move |_: web_sys::MouseEvent| {
        start_timer();
    };
    let handle_dismiss = move |_: web_sys::MouseEvent| {
        // Invalidate any pending auto-dismiss
        generation.update(|g| *g += 1);
        state.dismiss(toast_id);
    };

    // --- data-state: driven by the dismissing set ---
    let data_state = move || {
        if state.dismissing.get().contains(&toast_id) {
            "leaving"
        } else {
            "entering"
        }
    };

    // --- CSS class ---
    let root_cls = format!("toast {}", variant.toast_class());

    // --- ARIA ---
    let (role, aria_live) = match variant {
        ToastVariant::Default | ToastVariant::Success => ("status", "polite"),
        ToastVariant::Warning | ToastVariant::Danger => ("alert", "assertive"),
    };

    // --- Icon ---
    let icon = match variant {
        ToastVariant::Default => i::FaCircleInfoSolid,
        ToastVariant::Success => i::FaCircleCheckSolid,
        ToastVariant::Warning => i::FaTriangleExclamationSolid,
        ToastVariant::Danger => i::FaCircleExclamationSolid,
    };

    // --- Action button ---
    let action_label = toast.action_label.clone();
    let on_action = toast.on_action;

    view! {
        <div
            class=root_cls
            role=role
            aria-live=aria_live
            data-state=data_state
            on:mouseenter=on_mouseenter
            on:mouseleave=on_mouseleave
        >
            <span class="toast__icon">
                <Icon icon=icon />
            </span>
            <div class="toast__body">
                <span class="toast__message">{toast.message.clone()}</span>
                {action_label
                    .map(|label| {
                        view! {
                            <button
                                class="toast__action"
                                on:click=move |_| {
                                    if let Some(cb) = on_action {
                                        cb.run(());
                                    }
                                }
                            >
                                {label}
                            </button>
                        }
                    })}
            </div>
            <button class="toast__dismiss" aria-label=dismiss_label.clone() on:click=handle_dismiss>
                <Icon icon=i::FaXmarkSolid />
            </button>
        </div>
    }
}
