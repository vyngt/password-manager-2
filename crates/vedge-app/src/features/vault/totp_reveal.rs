//! Two-factor reveal block for the Login detail view (slice 4.2).
//!
//! Explicit, user-initiated reveal (fits "user-initiated"): a "Show one-time
//! code" button calls [`api::totp::reveal_totp`] (audited once per session per
//! entry), then a 1-second ticker decrements the countdown locally and
//! re-invokes the reveal at the period boundary (a refresh — audit-silent in
//! core). The **seed never crosses**; only the ephemeral code does. A Login with
//! no seed degrades to a muted "no code" note (the door defers a proper
//! `has_totp` gate to a follow-up — see the 4.2 tracking note). Copy routes
//! through the parent's `on_copy` (the 3.2-hardened backend clipboard).
//!
//! Owns its ticker lifecycle: `on_cleanup` clears the interval on unmount / when
//! the selected entry changes (the drawer re-mounts per entry).

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::FieldSelectorDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::totp_display::TotpDisplay;
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};

use crate::api;
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn TotpReveal(entry_id: String, on_copy: Callback<FieldSelectorDto>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let entry_id = StoredValue::new(entry_id);
    let code = RwSignal::new(String::new());
    let remaining = RwSignal::new(0_u32);
    let period = RwSignal::new(30_u32);
    let shown = RwSignal::new(false);
    let absent = RwSignal::new(false);
    // Boxed teardown (clears the interval) — avoids naming the handle type.
    let ticker = StoredValue::new_local(None::<Box<dyn FnOnce()>>);
    // True while a reveal IPC is in flight — the ticker must not re-fire a slow
    // refresh, and a persistent refresh failure must not storm.
    let refreshing = StoredValue::new(false);
    // Holds the reveal callback so the interval can call it without a
    // construction-time self-reference.
    let reveal_slot = StoredValue::new_local(None::<Callback<()>>);

    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let reveal = Callback::new(move |()| {
        refreshing.set_value(true);
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry_id.get_value();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_reveal_totp).to_owned());
        spawn_local(async move {
            let result = api::totp::reveal_totp(&vault_path, &id).await;
            // Post-await: the detail drawer may have unmounted mid-reveal (entry
            // switch, idle auto-lock, or OS screen lock), disposing these
            // component-scoped values. Use the `try_*` forms so a disposed write is
            // an explicit no-op rather than leaning on `set_value`/`update_value`'s
            // implicit swallow.
            refreshing.try_set_value(false);
            match result {
                Ok(c) => {
                    code.set(c.code);
                    remaining.set(c.seconds_remaining);
                    period.set(c.period);
                    shown.set(true);
                    // Start the 1s ticker once, after the first successful reveal.
                    ticker.try_update_value(|slot| {
                        if slot.is_none() {
                            let tick = move || {
                                // Idle while a refresh is in flight (never re-fire a
                                // slow reveal) or once we've stopped showing.
                                if refreshing.get_value() || !shown.get_untracked() {
                                    return;
                                }
                                let r = remaining.get_untracked();
                                if r <= 1 {
                                    if let Some(cb) = reveal_slot.get_value() {
                                        cb.run(());
                                    }
                                } else {
                                    remaining.set(r.saturating_sub(1));
                                }
                            };
                            if let Ok(iv) = set_interval_with_handle(tick, Duration::from_secs(1)) {
                                *slot = Some(Box::new(move || iv.clear()));
                            }
                        }
                    });
                }
                // A Login with no seed surfaces as `FieldNotApplicable` → the shared
                // `Invalid` kind; only that (on the *first* reveal) is "no code".
                // Real faults (locked session, decrypt, storage) are surfaced, and a
                // refresh failure stops the ticker so it can't storm.
                Err(ApiError::Invalid(_)) if !shown.get_untracked() => absent.set(true),
                Err(e) => {
                    show_error(format!("{err_prefix}{e}"));
                    shown.set(false);
                    ticker.try_update_value(|slot| {
                        if let Some(td) = slot.take() {
                            td();
                        }
                    });
                }
            }
        });
    });
    reveal_slot.set_value(Some(reveal));

    on_cleanup(move || {
        ticker.update_value(|slot| {
            if let Some(td) = slot.take() {
                td();
            }
        });
    });

    view! {
        <Show
            when=move || shown.get()
            fallback=move || {
                view! {
                    <Show
                        when=move || absent.get()
                        fallback=move || {
                            view! {
                                <Button
                                    variant=Variant::Secondary
                                    size=Size::Sm
                                    full_width=true
                                    attr:data-testid="detail-show-totp"
                                    on:click=move |_: web_sys::MouseEvent| reveal.run(())
                                >
                                    {move || t!(i18n, vault.show_totp)}
                                </Button>
                            }
                        }
                    >
                        <p class="text-xs text-text-tertiary text-center py-1">
                            {move || t!(i18n, vault.no_totp)}
                        </p>
                    </Show>
                }
            }
        >
            <div class="flex flex-col items-center gap-2 py-2" data-testid="detail-totp">
                <TotpDisplay
                    code=Signal::derive(move || code.get())
                    seconds_remaining=Signal::derive(move || remaining.get())
                    period=Signal::derive(move || period.get())
                    on_copy=Callback::new(move |()| on_copy.run(FieldSelectorDto::TotpCode))
                    copy_label=Signal::derive(move || t_string!(i18n, vault.copy_totp).to_owned())
                />
            </div>
        </Show>
    }
}
