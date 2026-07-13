//! Right pane of the vault picker (2.8.1): the focused unlock panel for the
//! selected vault — name + path header, a master-password field with Unlock,
//! and 2.8's biometric (Windows Hello) button when the vault is enrolled.
//! Shows a placeholder until a vault is chosen from the list.

use crate::features::vault::vault_launch::Selected;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{Size, Variant};

#[component]
pub fn VaultUnlockPanel(
    selected: RwSignal<Option<Selected>>,
    pw: RwSignal<String>,
    unlocking: RwSignal<bool>,
    /// Whether the selected vault has biometric unlock enrolled (2.8).
    bio_enrolled: RwSignal<bool>,
    /// When enrolled, `false` shows the Hello button; the "use master
    /// password" link flips it to reveal the password field.
    show_password: RwSignal<bool>,
    /// Recovery (5.1): whether the "Use Emergency Kit" panel is revealed. The
    /// always-visible CTA toggles it; the orchestrator's `do_unlock` also opens
    /// it on a keychain-missing error so the stuck user lands on it directly.
    recovery_open: RwSignal<bool>,
    /// Recovery (5.1): the `A3-…` Secret Key typed by the user. Passed through
    /// verbatim (the core parser owns normalization + checksum).
    recovery_key: RwSignal<String>,
    on_unlock: Callback<()>,
    on_bio_unlock: Callback<()>,
    on_use_password: Callback<()>,
    /// Recovery (5.1): submit master password + Secret Key via the recovery path.
    on_recover: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex flex-1 flex-col items-center justify-center p-8 text-center">
            {move || match selected.get() {
                None => {
                    Either::Left(
                        view! {
                            <div class="flex flex-col items-center gap-3 text-foreground/40">
                                <span class="flex h-14 w-14 items-center justify-center rounded-2xl bg-surface-subtle text-2xl">
                                    <Icon attr:aria-hidden="true" icon=i::FaFileShieldSolid />
                                </span>
                                <p class="max-w-[220px] text-sm">
                                    {move || t!(i18n, unlock.select_vault)}
                                </p>
                            </div>
                        },
                    )
                }
                Some(sel) => {
                    Either::Right(
                        view! {
                            <span class="mb-3.5 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 text-primary text-2xl">
                                <Icon attr:aria-hidden="true" icon=i::FaFileShieldSolid />
                            </span>
                            <div class="text-lg font-semibold text-text-primary">
                                {sel.display_name.clone()}
                            </div>
                            <div class="mb-5 max-w-full truncate text-xs font-jetbrains-mono text-foreground/40">
                                {sel.path}
                            </div>
                            <div class="w-full max-w-[280px]">
                                {move || {
                                    if bio_enrolled.get() && !show_password.get() {
                                        Either::Left(
                                            view! {
                                                <div class="flex flex-col items-center gap-3">
                                                    // Design-system exception: a bespoke 64px circular tinted
                                                    // biometric tap-target. `IconButton` maxes at 44px (`lg`) with a
                                                    // fixed svg size and has no tinted/outline-primary variant, so a
                                                    // conversion would override every axis. Kept raw (aria-labelled).
                                                    <button
                                                        type="button"
                                                        class="flex h-16 w-16 items-center justify-center rounded-full border-2 border-primary bg-primary/10 text-primary text-3xl transition-colors hover:bg-primary/20 disabled:opacity-50 disabled:cursor-not-allowed"
                                                        aria-label=move || {
                                                            t_string!(i18n, unlock.biometric_unlock_cta).to_owned()
                                                        }
                                                        disabled=move || unlocking.get()
                                                        on:click=move |_: web_sys::MouseEvent| on_bio_unlock.run(())
                                                    >
                                                        <Icon attr:aria-hidden="true" icon=i::FaFingerprintSolid />
                                                    </button>
                                                    <div>
                                                        <div class="text-sm font-medium text-text-primary">
                                                            {move || t!(i18n, unlock.biometric_unlock_cta)}
                                                        </div>
                                                        <div class="text-xs text-foreground/50">
                                                            {move || t!(i18n, unlock.biometric_touch_hint)}
                                                        </div>
                                                    </div>
                                                    <Button
                                                        variant=Variant::Link
                                                        on:click=move |_: web_sys::MouseEvent| {
                                                            on_use_password.run(());
                                                        }
                                                    >
                                                        {move || t!(i18n, unlock.use_master_password)}
                                                    </Button>
                                                </div>
                                            },
                                        )
                                    } else {
                                        Either::Right(
                                            view! {
                                                <div
                                                    class="flex flex-col gap-2.5"
                                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                        if ev.key() == "Enter" {
                                                            on_unlock.run(());
                                                        }
                                                    }
                                                >
                                                    <Input
                                                        id="master-password"
                                                        input_type="password"
                                                        size=Size::Lg
                                                        placeholder=Signal::derive(move || {
                                                            t_string!(i18n, unlock.master_password).to_owned()
                                                        })
                                                        value=Signal::derive(move || pw.get())
                                                        on_input=Callback::new(move |v: String| pw.set(v))
                                                        reveal_label=Signal::derive(move || {
                                                            t_string!(i18n, onboarding.show_password).to_owned()
                                                        })
                                                        hide_label=Signal::derive(move || {
                                                            t_string!(i18n, onboarding.hide_password).to_owned()
                                                        })
                                                    />
                                                    {move || {
                                                        let busy = unlocking.get();
                                                        view! {
                                                            <Button
                                                                variant=Variant::Primary
                                                                size=Size::Lg
                                                                full_width=true
                                                                loading=busy
                                                                attr:data-testid="unlock-submit"
                                                                on:click=move |_: web_sys::MouseEvent| on_unlock.run(())
                                                            >
                                                                {move || t!(i18n, unlock.unlock)}
                                                            </Button>
                                                        }
                                                    }}
                                                    {move || {
                                                        bio_enrolled
                                                            .get()
                                                            .then(|| {
                                                                view! {
                                                                    <Button
                                                                        variant=Variant::Link
                                                                        class="mt-1 gap-1.5"
                                                                        on:click=move |_: web_sys::MouseEvent| on_bio_unlock.run(())
                                                                    >
                                                                        <Icon
                                                                            attr:aria-hidden="true"
                                                                            icon=i::FaFingerprintSolid
                                                                            width="13"
                                                                            height="13"
                                                                        />
                                                                        {move || t!(i18n, unlock.biometric_unlock_cta)}
                                                                    </Button>
                                                                }
                                                            })
                                                    }}
                                                </div>

                                                // ---- Emergency Kit recovery (slice 5.1) --------------
                                                // Always-reachable CTA under the password form; the panel
                                                // also auto-opens on a keychain-missing error (the
                                                // orchestrator's `do_unlock` sets `recovery_open`).
                                                <Button
                                                    variant=Variant::Link
                                                    class="mt-2"
                                                    attr:data-testid="use-kit"
                                                    on:click=move |_: web_sys::MouseEvent| {
                                                        recovery_open.update(|o| *o = !*o);
                                                    }
                                                >
                                                    {move || t!(i18n, unlock.use_kit_cta)}
                                                </Button>
                                                <Show when=move || recovery_open.get()>
                                                    <div
                                                        class="mt-1 flex flex-col gap-2.5"
                                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                            if ev.key() == "Enter" {
                                                                on_recover.run(());
                                                            }
                                                        }
                                                    >
                                                        <p class="text-xs text-foreground/50">
                                                            {move || t!(i18n, unlock.recovery_hint)}
                                                        </p>
                                                        <Input
                                                            id="recovery-key"
                                                            size=Size::Lg
                                                            class="font-jetbrains-mono"
                                                            placeholder=Signal::derive(move || {
                                                                "A3-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX".to_owned()
                                                            })
                                                            aria_label=Signal::derive(move || {
                                                                t_string!(i18n, unlock.recovery_key_label).to_owned()
                                                            })
                                                            value=Signal::derive(move || recovery_key.get())
                                                            on_input=Callback::new(move |v: String| recovery_key.set(v))
                                                        />
                                                        {move || {
                                                            let busy = unlocking.get();
                                                            view! {
                                                                <Button
                                                                    variant=Variant::Primary
                                                                    size=Size::Lg
                                                                    full_width=true
                                                                    loading=busy
                                                                    attr:data-testid="recovery-submit"
                                                                    on:click=move |_: web_sys::MouseEvent| on_recover.run(())
                                                                >
                                                                    {move || t!(i18n, unlock.recovery_submit)}
                                                                </Button>
                                                            }
                                                        }}
                                                    </div>
                                                </Show>
                                            },
                                        )
                                    }
                                }}
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}
