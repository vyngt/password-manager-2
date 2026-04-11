use leptos::prelude::*;
use ui::components::button::Button;
use ui::components::form::color_picker::ColorPicker;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Size, Variant};
use ui::theme::{Severity, ThemeConfig, ThemeState, ThemeValidation, validate_theme_config};

use icondata as i;
use leptos_icons::Icon;

// ---------------------------------------------------------------------------
// Color picker row
// ---------------------------------------------------------------------------

#[component]
fn ColorField(
    label: &'static str,
    value: Signal<String>,
    on_change: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <span class="text-xs text-text-tertiary">{label}</span>
            <ColorPicker value=value on_change_end=on_change size=Size::Sm />
        </div>
    }
}

// ---------------------------------------------------------------------------
// Validation display
// ---------------------------------------------------------------------------

#[component]
fn ValidationPanel(validation: Signal<Option<ThemeValidation>>) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4">
            <h3 class="text-sm font-semibold text-text-primary mb-3">"Contrast Validation"</h3>
            {move || {
                let Some(v) = validation.get() else {
                    return view! { <p class="text-text-tertiary text-xs">"No validation data"</p> }
                        .into_any();
                };
                let status = if v.is_valid { "Valid" } else { "Invalid \u{2014} blocked" };
                view! {
                    <div>
                        <p
                            class="text-xs mb-2 font-semibold"
                            class:text-success=v.is_valid
                            class:text-danger=!v.is_valid
                        >
                            {status}
                        </p>
                        <div class="space-y-1">
                            {v
                                .checks
                                .into_iter()
                                .map(|c| {
                                    let icon = if c.pass {
                                        "\u{2713}"
                                    } else if c.severity == Severity::Block {
                                        "\u{2717}"
                                    } else {
                                        "!"
                                    };
                                    let color_class = if c.pass {
                                        "text-success"
                                    } else if c.severity == Severity::Block {
                                        "text-danger"
                                    } else {
                                        "text-warning"
                                    };
                                    view! {
                                        <div class="flex items-center gap-2 text-xs">
                                            <span class=color_class>{icon}</span>
                                            <span class="text-text-secondary font-mono">
                                                {format!("{:.1}:1", c.ratio)}
                                            </span>
                                            <span class="text-text-tertiary">{c.pair_label}</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}

// ---------------------------------------------------------------------------
// Token swatch
// ---------------------------------------------------------------------------

#[component]
fn Swatch(label: &'static str, color: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <div
                class="w-6 h-6 rounded border border-border"
                style=move || format!("background: {}", color.get())
            />
            <div class="flex flex-col">
                <span class="text-[10px] text-text-tertiary">{label}</span>
                <span class="text-[10px] font-mono text-text-secondary">{color}</span>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Quick component preview
// ---------------------------------------------------------------------------

#[component]
fn ComponentPreview() -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-4">
            <h3 class="text-sm font-semibold text-text-primary">"Quick Preview"</h3>
            <div class="flex flex-wrap gap-2">
                <Button variant=Variant::Primary size=Size::Sm>
                    "Primary"
                </Button>
                <Button variant=Variant::Secondary size=Size::Sm>
                    "Secondary"
                </Button>
                <Button variant=Variant::Ghost size=Size::Sm>
                    "Ghost"
                </Button>
                <Button variant=Variant::Danger size=Size::Sm>
                    "Danger"
                </Button>
                <Button variant=Variant::Warning size=Size::Sm>
                    "Warning"
                </Button>
            </div>
            <div class="flex flex-wrap gap-2">
                <IconButton aria_label="Primary" variant=Variant::Primary>
                    <Icon icon=i::FaStarSolid />
                </IconButton>
                <IconButton aria_label="Secondary" variant=Variant::Secondary>
                    <Icon icon=i::FaGearSolid />
                </IconButton>
                <IconButton aria_label="Ghost">
                    <Icon icon=i::FaMagnifyingGlassSolid />
                </IconButton>
                <IconButton aria_label="Danger" variant=Variant::Danger>
                    <Icon icon=i::FaTrashSolid />
                </IconButton>
            </div>
            <div class="space-y-1">
                <p class="text-text-primary text-sm">"Primary text \u{2014} body content"</p>
                <p class="text-text-secondary text-sm">"Secondary text \u{2014} descriptions"</p>
                <p class="text-text-tertiary text-sm">"Tertiary text \u{2014} captions"</p>
            </div>
            <div class="flex gap-1">
                <div class="flex-1 h-10 rounded bg-background border border-border flex items-center justify-center text-[10px] text-text-tertiary">
                    "bg"
                </div>
                <div class="flex-1 h-10 rounded bg-surface-1 border border-border flex items-center justify-center text-[10px] text-text-tertiary">
                    "1"
                </div>
                <div class="flex-1 h-10 rounded bg-surface-2 border border-border flex items-center justify-center text-[10px] text-text-tertiary">
                    "2"
                </div>
                <div class="flex-1 h-10 rounded bg-surface-3 border border-border flex items-center justify-center text-[10px] text-text-tertiary">
                    "3"
                </div>
                <div class="flex-1 h-10 rounded bg-surface-4 border border-border flex items-center justify-center text-[10px] text-text-tertiary">
                    "4"
                </div>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Theme Page
// ---------------------------------------------------------------------------

#[component]
pub fn ThemePage() -> impl IntoView {
    let theme = expect_context::<ThemeState>();

    let (background, set_background) = signal("#FFFFFF".to_string());
    let (foreground, set_foreground) = signal("#111827".to_string());
    let (primary, set_primary) = signal("#2563EB".to_string());
    let (danger, set_danger) = signal("#DC2626".to_string());
    let (warning, set_warning) = signal("#D97706".to_string());
    let (success, set_success) = signal("#16A34A".to_string());

    let validation: RwSignal<Option<ThemeValidation>> = RwSignal::new(None);
    let tokens_signal = theme.tokens();

    Effect::new(move || {
        let config = ThemeConfig {
            background: background.get(),
            foreground: foreground.get(),
            primary: primary.get(),
            danger: Some(danger.get()),
            warning: Some(warning.get()),
            success: Some(success.get()),
        };
        if let Ok(v) = validate_theme_config(&config) {
            validation.set(Some(v));
        }
        theme.preview(&config);
    });

    let apply_light = move |_| {
        set_background.set("#FFFFFF".into());
        set_foreground.set("#111827".into());
        set_primary.set("#2563EB".into());
        set_danger.set("#DC2626".into());
        set_warning.set("#D97706".into());
        set_success.set("#16A34A".into());
    };

    let apply_dark = move |_| {
        set_background.set("#09090B".into());
        set_foreground.set("#FAFAFA".into());
        set_primary.set("#3B82F6".into());
        set_danger.set("#EF4444".into());
        set_warning.set("#F59E0B".into());
        set_success.set("#22C55E".into());
    };

    view! {
        <div class="p-6 max-w-6xl mx-auto space-y-6">
            <div class="flex items-center justify-between">
                <h1 class="text-xl font-semibold text-text-primary">"Theme Playground"</h1>
                <div class="flex gap-2">
                    <Button variant=Variant::Secondary size=Size::Sm on:click=apply_light>
                        "Light Preset"
                    </Button>
                    <Button variant=Variant::Secondary size=Size::Sm on:click=apply_dark>
                        "Dark Preset"
                    </Button>
                </div>
            </div>

            <div class="grid grid-cols-[320px_1fr] gap-6">
                // Left panel: controls
                <div class="space-y-4">
                    <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
                        <h3 class="text-sm font-semibold text-text-primary">"Color Inputs"</h3>
                        <ColorField
                            label="Background"
                            value=Signal::derive(move || background.get())
                            on_change=Callback::new(move |v| set_background.set(v))
                        />
                        <ColorField
                            label="Foreground"
                            value=Signal::derive(move || foreground.get())
                            on_change=Callback::new(move |v| set_foreground.set(v))
                        />
                        <ColorField
                            label="Primary"
                            value=Signal::derive(move || primary.get())
                            on_change=Callback::new(move |v| set_primary.set(v))
                        />
                        <hr class="border-border" />
                        <ColorField
                            label="Danger"
                            value=Signal::derive(move || danger.get())
                            on_change=Callback::new(move |v| set_danger.set(v))
                        />
                        <ColorField
                            label="Warning"
                            value=Signal::derive(move || warning.get())
                            on_change=Callback::new(move |v| set_warning.set(v))
                        />
                        <ColorField
                            label="Success"
                            value=Signal::derive(move || success.get())
                            on_change=Callback::new(move |v| set_success.set(v))
                        />
                    </div>
                    <ValidationPanel validation=Signal::derive(move || validation.get()) />
                </div>

                // Right panel: preview
                <div class="space-y-4">
                    <ComponentPreview />
                    <div class="rounded-lg border border-border bg-surface-1 p-4">
                        <h3 class="text-sm font-semibold text-text-primary mb-3">
                            "Derived Tokens"
                        </h3>
                        <div class="grid grid-cols-3 gap-2">
                            <Swatch
                                label="background"
                                color=Signal::derive(move || tokens_signal.get().color_background)
                            />
                            <Swatch
                                label="surface-1"
                                color=Signal::derive(move || tokens_signal.get().color_surface_1)
                            />
                            <Swatch
                                label="surface-2"
                                color=Signal::derive(move || tokens_signal.get().color_surface_2)
                            />
                            <Swatch
                                label="surface-3"
                                color=Signal::derive(move || tokens_signal.get().color_surface_3)
                            />
                            <Swatch
                                label="surface-4"
                                color=Signal::derive(move || tokens_signal.get().color_surface_4)
                            />
                            <Swatch
                                label="border"
                                color=Signal::derive(move || tokens_signal.get().color_border)
                            />
                            <Swatch
                                label="border-strong"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_border_strong
                                })
                            />
                            <Swatch
                                label="text-primary"
                                color=Signal::derive(move || tokens_signal.get().color_text_primary)
                            />
                            <Swatch
                                label="text-secondary"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_text_secondary
                                })
                            />
                            <Swatch
                                label="text-tertiary"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_text_tertiary
                                })
                            />
                            <Swatch
                                label="primary"
                                color=Signal::derive(move || tokens_signal.get().color_primary)
                            />
                            <Swatch
                                label="primary-hover"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_primary_hover
                                })
                            />
                            <Swatch
                                label="primary-muted"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_primary_muted
                                })
                            />
                            <Swatch
                                label="primary-fg"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_primary_foreground
                                })
                            />
                            <Swatch
                                label="primary-text"
                                color=Signal::derive(move || tokens_signal.get().color_primary_text)
                            />
                            <Swatch
                                label="focus-ring"
                                color=Signal::derive(move || tokens_signal.get().color_focus_ring)
                            />
                            <Swatch
                                label="danger"
                                color=Signal::derive(move || tokens_signal.get().color_danger)
                            />
                            <Swatch
                                label="danger-hover"
                                color=Signal::derive(move || tokens_signal.get().color_danger_hover)
                            />
                            <Swatch
                                label="danger-muted"
                                color=Signal::derive(move || tokens_signal.get().color_danger_muted)
                            />
                            <Swatch
                                label="warning"
                                color=Signal::derive(move || tokens_signal.get().color_warning)
                            />
                            <Swatch
                                label="warning-hover"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_warning_hover
                                })
                            />
                            <Swatch
                                label="warning-muted"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_warning_muted
                                })
                            />
                            <Swatch
                                label="success"
                                color=Signal::derive(move || tokens_signal.get().color_success)
                            />
                            <Swatch
                                label="success-hover"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_success_hover
                                })
                            />
                            <Swatch
                                label="success-muted"
                                color=Signal::derive(move || {
                                    tokens_signal.get().color_success_muted
                                })
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
