//! Idle auto-lock + lock-on-blur.
//!
//! A render-nothing component mounted in the `/v` layout (so it lives exactly as
//! long as a vault is open). It reads [`SecurityPrefsCtx`] reactively and:
//! - arms an idle deadline that user activity (`mousemove`/`keydown`/`click`/
//!   `scroll`) pushes out, polled once a second;
//! - optionally locks when the window loses focus (`blur`) — *except* while one
//!   of our own native file dialogs is open (which also steals focus); see
//!   [`api::dialog::dialog_in_progress`].
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
use crate::i18n::{t_string, use_i18n};
use chrono::Utc;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::time::Duration;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::ToastVariant;

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
    let i18n = use_i18n();
    let toast = use_toast();

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
                    // Don't lock while our own native dialog is open (the user is
                    // mid file-pick, not idle). It re-fires next tick once closed.
                    if now_ms() >= deadline.get_value() && !api::dialog::dialog_in_progress() {
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
                // A native OS file dialog (Document attach/export, Emergency Kit,
                // vault picker) steals focus and fires `blur` — that's *us*, not
                // the user leaving, so don't lock mid-task. Real alt-tab / minimize
                // still fires `blur` with no dialog open and locks as before.
                if api::dialog::dialog_in_progress() {
                    return;
                }
                lock_trigger.update(|n| *n = n.wrapping_add(1));
            });
            next.push(Box::new(move || h.remove()));
        }

        cleanups.set_value(next);
    });

    // Always-on backend-lock poll (slice 4.5a). The hard session TTL — and, in
    // 4.5b, OS screen-lock — can lock the vault *without us asking*: the backend
    // zeroizes the KEK on a timer. Poll `is_unlocked` once a second and, when the
    // backend has locked, bump the SAME ticket the idle timer uses, so the shared
    // lock Effect below runs the full teardown → `active.path` Some→None, which
    // wipes the generator history (`Zeroizing` plaintext) + health report from
    // WASM. Set up directly in the body (fires once at mount) and independent of
    // the idle timer, so it holds even when idle auto-lock is disabled — that
    // timer only exists when `auto_lock_minutes > 0`.
    // Localized "session expired" copy for the backend-lock toast (backstop C's
    // visible cue). `Memo`s so they relocalize, read `get_untracked` from the
    // owner-less interval callback — the CLAUDE.md-endorsed pattern (never an
    // owner-less `t_string!`).
    let expired_msg = Memo::new(move |_| t_string!(i18n, unlock.session_expired).to_owned());
    let dismiss_msg = Memo::new(move |_| t_string!(i18n, unlock.dismiss).to_owned());
    // `checking` gates overlapping probes (one in flight at a time); `locking`
    // permanently disarms once a backend lock is seen, so the teardown + toast
    // fire exactly once (mirrors the idle path's synchronous deadline disarm).
    let checking = StoredValue::new(false);
    let locking = StoredValue::new(false);

    if let Ok(backend_poll) = set_interval_with_handle(
        move || {
            if locking.get_value() || checking.get_value() {
                return;
            }
            let Some(p) = active.path.get_untracked() else {
                return;
            };
            checking.set_value(true);
            // Read the localized toast copy BEFORE the await, while still mounted:
            // `expired_msg`/`dismiss_msg` are component-local `Memo`s and a read on a
            // disposed `Memo` PANICS. The view can unmount mid-poll (idle lock, nav,
            // OS screen-lock), so capture the strings now and move them into the future.
            let expired = expired_msg.get_untracked();
            let dismiss = dismiss_msg.get_untracked();
            spawn_local(async move {
                // Lock ONLY on a definitive "backend says locked" (`Ok(false)`). An
                // `Err` means "couldn't ask" (a transient IPC blip) — skip this
                // tick rather than eject the user mid-task.
                let locked = matches!(api::vault::is_unlocked(&p).await, Ok(false));
                // Post-await the view may be gone: the `try_*` guard writes are no-ops.
                // Deliberately NOT an early-return — the `toast` (app-level context) and
                // `lock_trigger` (a component `RwSignal`; `.update` no-ops once disposed)
                // still run so a real backend lock propagates. The toast copy was read
                // pre-await above (the `Memo`s would panic if read here after disposal).
                checking.try_set_value(false);
                if locked {
                    locking.try_set_value(true);
                    toast.show(
                        ToastInput::new(expired)
                            .variant(ToastVariant::Warning)
                            .dismiss_label(dismiss),
                    );
                    lock_trigger.update(|n| *n = n.wrapping_add(1));
                }
            });
        },
        Duration::from_secs(1),
    ) {
        on_cleanup(move || backend_poll.clear());
    }

    // Perform the lock when the ticket bumps (prev-guard skips the initial mount).
    // Done in an Effect so `use_navigate` runs in a reactive owner. `api::vault::lock`
    // on an already-backend-locked vault returns `VaultNotOpen` (ignored) — the
    // teardown still drives `active.path` Some→None.
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
