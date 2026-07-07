//! Idle auto-lock + lock-on-blur.
//!
//! A render-nothing component mounted in the `/v` layout (so it lives exactly as
//! long as a vault is open). It reads [`SecurityPrefsCtx`] reactively and:
//! - arms an idle deadline that user activity (`mousemove`/`keydown`/`click`/
//!   `scroll`) pushes out, polled once a second;
//! - optionally locks when the window loses focus (`blur`).
//!
//! On expiry it runs the same lock path as the sidebar `LockButton`
//! (`api::vault::lock` → clear `ActiveVault` → navigate `/`).
//!
//! Design notes (Leptos owner rules): raw JS callbacks (activity listeners, the
//! interval, the blur listener) only ever *write* signals / `StoredValue`s — no
//! reactive owner required. The actual lock (which calls `use_navigate`) is done
//! from a prev-guarded [`Effect`] observing a bump-ticket, mirroring the
//! `VaultUiState` request-ticket pattern. All listeners + the interval are torn
//! down on config change and on unmount, so re-entering `/v` never stacks them.

use crate::api;
use crate::features::settings::security_prefs::{self, SecurityPrefsCtx};
use crate::features::vault::context::ActiveVault;
use chrono::Utc;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::time::Duration;

/// A boxed one-shot teardown (removes a listener / clears the interval).
type Teardown = Box<dyn FnOnce()>;

/// Wall-clock milliseconds. Not strictly monotonic, but fine for a minute-scale
/// idle timer; avoids pulling in the web-sys `Performance` feature.
fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[component]
pub fn AutoLock() -> impl IntoView {
    let sec = expect_context::<SecurityPrefsCtx>();
    let active = expect_context::<ActiveVault>();

    // Derived so an unrelated pref edit (e.g. the clipboard delay) doesn't churn
    // the timer — only these two facets re-arm it.
    let minutes = Memo::new(move |_| sec.0.get().auto_lock_minutes);
    let lock_on_blur = Memo::new(move |_| sec.0.get().lock_on_blur);

    // Absolute deadline (ms) at which an idle lock fires; `i64::MAX` = disarmed.
    let deadline = StoredValue::new(i64::MAX);
    // Bump-ticket the lock Effect observes; raw callbacks just increment it.
    let lock_trigger = RwSignal::new(0u32);

    // Teardown handles for the *current* config's listeners + interval. `!Send`
    // (they hold JS closures), so a local-storage `StoredValue`.
    let cleanups = StoredValue::new_local(Vec::<Teardown>::new());
    let teardown = move || {
        cleanups.update_value(|v| {
            for c in v.drain(..) {
                c();
            }
        });
    };

    // (Re)arm whenever the relevant prefs change. `teardown()` first so a config
    // change never leaves the previous listeners/interval running.
    Effect::new(move |_| {
        let mins = minutes.get();
        let on_blur = lock_on_blur.get();
        teardown();

        let mut next: Vec<Teardown> = Vec::new();

        if let Some(timeout_ms) =
            security_prefs::idle_ms(mins).and_then(|ms| i64::try_from(ms).ok())
        {
            deadline.set_value(now_ms() + timeout_ms);

            // Any activity pushes the deadline out. Writing a `StoredValue` is
            // non-reactive, so these callbacks need no reactive owner.
            let h = window_event_listener(ev::mousemove, move |_| {
                deadline.set_value(now_ms() + timeout_ms);
            });
            next.push(Box::new(move || h.remove()));
            let h = window_event_listener(ev::keydown, move |_| {
                deadline.set_value(now_ms() + timeout_ms);
            });
            next.push(Box::new(move || h.remove()));
            let h = window_event_listener(ev::click, move |_| {
                deadline.set_value(now_ms() + timeout_ms);
            });
            next.push(Box::new(move || h.remove()));
            let h = window_event_listener(ev::scroll, move |_| {
                deadline.set_value(now_ms() + timeout_ms);
            });
            next.push(Box::new(move || h.remove()));

            // Poll once a second; fire once (disarm) on expiry.
            if let Ok(interval) = set_interval_with_handle(
                move || {
                    if now_ms() >= deadline.get_value() {
                        deadline.set_value(i64::MAX);
                        lock_trigger.update(|n| *n = n.wrapping_add(1));
                    }
                },
                Duration::from_secs(1),
            ) {
                next.push(Box::new(move || interval.clear()));
            }
        } else {
            deadline.set_value(i64::MAX);
        }

        if on_blur {
            let h = window_event_listener(ev::blur, move |_| {
                lock_trigger.update(|n| *n = n.wrapping_add(1));
            });
            next.push(Box::new(move || h.remove()));
        }

        cleanups.set_value(next);
    });

    // Perform the lock when the ticket bumps (prev-guard skips the initial mount).
    // Done in an Effect so `use_navigate` runs in a reactive owner.
    Effect::new(move |prev: Option<u32>| {
        let cur = lock_trigger.get();
        if prev.is_some_and(|p| p != cur) {
            let nav = use_navigate();
            let path = active.path.get_untracked();
            spawn_local(async move {
                if let Some(p) = path {
                    let _ = api::vault::lock(&p).await;
                }
                active.path.set(None);
                nav("/", Default::default());
            });
        }
        cur
    });

    on_cleanup(teardown);
}
