//! Theme color editor — a `Dialog` exposing the six `ThemeConfig` inputs
//! (background / text / primary required + danger / warning / success optional),
//! a **scoped** live preview, and headline contrast badges. Save persists via
//! `create_custom_theme` (New / Duplicate) or `update_custom_theme` (Edit).
//!
//! The preview is scoped: derived `--color-*` values are set as inline custom
//! properties on a preview container, so only that box renders in the draft
//! theme (custom properties inherit to its children) — the rest of the app is
//! untouched, so no commit/revert dance is needed (cf. the playground, which
//! previews globally via `ThemeState::preview`). Contrast comes from
//! `validate_tokens` on the same derived set.

use super::theme_util::{
    config_from_fields, create_input_from, headline_contrast, update_input_from,
};
use crate::api;
use crate::i18n::*;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ipc::ThemeDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::form::color_picker::ColorPicker;
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};
use vedge_ui::theme::{ThemeConfig, derive_tokens, validate_tokens};

/// What the editor is opened for. `editing = true` → update the `source` custom
/// theme; otherwise create a new theme from the draft (New / Duplicate).
#[derive(Clone)]
pub struct EditorTarget {
    pub source: Option<ThemeDto>,
    pub editing: bool,
    pub seed: ThemeConfig,
    pub seed_name: String,
}

#[component]
pub fn ThemeEditor(
    /// `Some` while the dialog is open; the list sets this to open/close it.
    target: RwSignal<Option<EditorTarget>>,
    /// Fired after a successful create/update so the list refreshes.
    on_saved: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    let name = RwSignal::new(String::new());
    let background = RwSignal::new(String::new());
    let foreground = RwSignal::new(String::new());
    let primary = RwSignal::new(String::new());
    let danger = RwSignal::new(String::new());
    let warning = RwSignal::new(String::new());
    let success = RwSignal::new(String::new());
    let err = RwSignal::new(Option::<String>::None);

    // Seed the draft each time the dialog opens.
    Effect::new(move |_| {
        if let Some(t) = target.get() {
            name.set(t.seed_name.clone());
            background.set(t.seed.background.clone());
            foreground.set(t.seed.foreground.clone());
            primary.set(t.seed.primary.clone());
            danger.set(t.seed.danger.clone().unwrap_or_default());
            warning.set(t.seed.warning.clone().unwrap_or_default());
            success.set(t.seed.success.clone().unwrap_or_default());
            err.set(None);
        }
    });

    let draft = move || {
        config_from_fields(
            background.get(),
            foreground.get(),
            primary.get(),
            danger.get(),
            warning.get(),
            success.get(),
        )
    };

    // One derive per change; drives both the scoped preview and the contrast.
    let derived = Memo::new(move |_| derive_tokens(&draft()).ok());
    let preview_style = move || {
        derived.get().map_or_else(String::new, |t| {
            format!(
                "--color-background:{};--color-surface-1:{};--color-border:{};\
                 --color-primary:{};--color-primary-foreground:{};\
                 --color-text-primary:{};--color-text-secondary:{};\
                 --color-danger:{};--color-warning:{};--color-success:{};",
                t.color_background,
                t.color_surface_1,
                t.color_border,
                t.color_primary,
                t.color_primary_foreground,
                t.color_text_primary,
                t.color_text_secondary,
                t.color_danger,
                t.color_warning,
                t.color_success,
            )
        })
    };
    // `Option<(bool, bool)>` is `PartialEq` (unlike `ThemeValidation`), so a Memo works.
    let contrast = Memo::new(move |_| {
        derived
            .get()
            .and_then(|t| validate_tokens(&t).ok())
            .map(|v| headline_contrast(&v))
    });

    let close = Callback::new(move |()| target.set(None));

    let save = move |_: web_sys::MouseEvent| {
        let Some(t) = target.get_untracked() else {
            return;
        };
        let name_val = name.get_untracked().trim().to_owned();
        if name_val.is_empty() {
            err.set(Some(
                t_string!(i18n, settings.theme_name_required).to_string(),
            ));
            return;
        }
        let cfg = config_from_fields(
            background.get_untracked(),
            foreground.get_untracked(),
            primary.get_untracked(),
            danger.get_untracked(),
            warning.get_untracked(),
            success.get_untracked(),
        );
        let err_prefix = t_string!(i18n, settings.err_theme_save).to_string();
        let editing = t.editing;
        let source_id = t.source.as_ref().map(|d| d.id.clone());
        spawn_local(async move {
            let result = if editing {
                let id = source_id.unwrap_or_default();
                api::settings::update_custom_theme(&update_input_from(id, name_val, &cfg)).await
            } else {
                api::settings::create_custom_theme(&create_input_from(name_val, &cfg))
                    .await
                    .map(|_id| ())
            };
            match result {
                Ok(()) => {
                    target.set(None);
                    on_saved.run(());
                }
                Err(e) => err.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    };

    let title = move || {
        if target.get().is_some_and(|t| t.editing) {
            t_string!(i18n, settings.theme_edit).to_string()
        } else {
            t_string!(i18n, settings.theme_new).to_string()
        }
    };
    let save_label = move || {
        if target.get().is_some_and(|t| t.editing) {
            t_string!(i18n, settings.theme_save).to_string()
        } else {
            t_string!(i18n, settings.theme_create).to_string()
        }
    };

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Md
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_string())
        >
            <DialogHeader>
                <DialogTitle>{title}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-6 min-w-[26rem]">
                    // Name.
                    <div>
                        <div class="text-xs text-text-tertiary mb-1">
                            {move || t!(i18n, settings.theme_name)}
                        </div>
                        <input
                            class="w-full h-9 px-3 rounded-md border border-border bg-background text-sm text-text-primary"
                            prop:value=move || name.get()
                            on:input:target=move |ev| name.set(ev.target().value())
                        />
                    </div>

                    // Colors — 3 required + 3 optional.
                    <div>
                        <div class="text-xs font-medium text-text-secondary mb-2">
                            {move || t!(i18n, settings.theme_colors)}
                        </div>
                        <div class="grid grid-cols-2 gap-3">
                            <ColorField
                                label=Signal::derive(move || {
                                    t_string!(i18n, settings.color_background).to_string()
                                })
                                value=Signal::derive(move || background.get())
                                on_change=Callback::new(move |v: String| background.set(v))
                            />
                            <ColorField
                                label=Signal::derive(move || {
                                    t_string!(i18n, settings.color_foreground).to_string()
                                })
                                value=Signal::derive(move || foreground.get())
                                on_change=Callback::new(move |v: String| foreground.set(v))
                            />
                            <ColorField
                                label=Signal::derive(move || {
                                    t_string!(i18n, settings.color_primary).to_string()
                                })
                                value=Signal::derive(move || primary.get())
                                on_change=Callback::new(move |v: String| primary.set(v))
                            />
                            <ColorField
                                label=Signal::derive(move || {
                                    format!(
                                        "{} · {}",
                                        t_string!(i18n, settings.color_danger),
                                        t_string!(i18n, settings.theme_optional),
                                    )
                                })
                                value=Signal::derive(move || danger.get())
                                on_change=Callback::new(move |v: String| danger.set(v))
                            />
                            <ColorField
                                label=Signal::derive(move || {
                                    format!(
                                        "{} · {}",
                                        t_string!(i18n, settings.color_warning),
                                        t_string!(i18n, settings.theme_optional),
                                    )
                                })
                                value=Signal::derive(move || warning.get())
                                on_change=Callback::new(move |v: String| warning.set(v))
                            />
                            <ColorField
                                label=Signal::derive(move || {
                                    format!(
                                        "{} · {}",
                                        t_string!(i18n, settings.color_success),
                                        t_string!(i18n, settings.theme_optional),
                                    )
                                })
                                value=Signal::derive(move || success.get())
                                on_change=Callback::new(move |v: String| success.set(v))
                            />
                        </div>
                    </div>

                    // Scoped live preview.
                    <div>
                        <div class="text-xs font-medium text-text-secondary mb-2">
                            {move || t!(i18n, settings.theme_preview)}
                        </div>
                        <div
                            class="rounded-lg border border-border bg-background p-3 flex items-center gap-3"
                            style=preview_style
                        >
                            <span class="inline-flex items-center h-7 px-3 rounded-md bg-primary text-primary-foreground text-xs font-medium">
                                "Save"
                            </span>
                            <span class="flex-1 inline-flex items-center h-7 px-2 rounded-md border border-primary text-text-secondary text-xs">
                                "Input"
                            </span>
                            <span class="inline-flex items-center gap-1.5 text-text-primary text-xs">
                                <span class="text-primary">
                                    <Icon icon=i::FaGlobeSolid />
                                </span>
                                "Web"
                            </span>
                            <span class="flex gap-1">
                                <span class="w-2 h-2 rounded-full bg-danger"></span>
                                <span class="w-2 h-2 rounded-full bg-warning"></span>
                                <span class="w-2 h-2 rounded-full bg-success"></span>
                            </span>
                        </div>
                    </div>

                    // Headline contrast badges.
                    {move || {
                        contrast
                            .get()
                            .map(|(text_ok, primary_ok)| {
                                let fg = t_string!(i18n, settings.color_foreground).to_string();
                                let pr = t_string!(i18n, settings.color_primary).to_string();
                                let ok_word = t_string!(i18n, settings.theme_contrast_ok).to_string();
                                let low_word = t_string!(i18n, settings.theme_contrast_low)
                                    .to_string();
                                let mk = move |ok: bool, label: String| {
                                    let (icon, color) = if ok {
                                        (i::FaCircleCheckSolid, "text-success-text")
                                    } else {
                                        (i::FaTriangleExclamationSolid, "text-warning-text")
                                    };
                                    let word = if ok { ok_word.clone() } else { low_word.clone() };
                                    view! {
                                        <span class=format!(
                                            "inline-flex items-center gap-1.5 text-xs {color}",
                                        )>
                                            <Icon icon=icon />
                                            <span>{format!("{label} · {word}")}</span>
                                        </span>
                                    }
                                };
                                view! {
                                    <div class="flex gap-3">
                                        {mk(text_ok, fg)}
                                        {mk(primary_ok, pr)}
                                    </div>
                                }
                            })
                    }}

                    {move || {
                        err.get()
                            .map(|e| view! { <p class="text-sm text-danger-text">{e}</p> })
                    }}

                    <div class="flex justify-end gap-2 border-t border-border pt-3">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, settings.cancel)}
                        </Button>
                        <Button variant=Variant::Primary size=Size::Sm on:click=save>
                            {save_label}
                        </Button>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

#[component]
fn ColorField(
    #[prop(into)] label: Signal<String>,
    value: Signal<String>,
    on_change: Callback<String>,
) -> impl IntoView {
    view! {
        <div>
            <div class="text-xs text-text-tertiary mb-1">{move || label.get()}</div>
            <ColorPicker value=value on_change_end=on_change size=Size::Sm />
        </div>
    }
}
