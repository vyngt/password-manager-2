use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::ThemeDto;
use vedge_ui::components::Badge;
use vedge_ui::components::Input;
use vedge_ui::components::button::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogFooter, DialogHeader, DialogTitle};
use vedge_ui::components::form::color_picker::ColorPicker;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{BadgeSize, BadgeVariant, Size, Variant};
use vedge_ui::theme::{Severity, ThemeConfig, ThemeState, ThemeValidation, validate_theme_config};

use icondata as i;
use leptos_icons::Icon;

use crate::api;
use crate::pages::playground::theme_api::{
    create_input_from, theme_config_from_dto, update_input_from,
};

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
// Themes panel
// ---------------------------------------------------------------------------

/// Refresh both the theme list and the active-theme id. Called on mount
/// and after every mutation.
fn refresh_themes(
    themes: RwSignal<Vec<ThemeDto>>,
    active_id: RwSignal<Option<String>>,
    list_error: RwSignal<Option<String>>,
) {
    spawn_local(async move {
        match api::settings::list_themes().await {
            Ok(rows) => {
                list_error.set(None);
                themes.set(rows);
            }
            Err(e) => {
                list_error.set(Some(format!("list failed: {e}")));
            }
        }
        if let Ok(active) = api::settings::get_active_theme().await {
            active_id.set(Some(active.id));
        }
    });
}

#[component]
#[allow(clippy::too_many_arguments)]
fn ThemeRow(
    theme: ThemeDto,
    active_id: RwSignal<Option<String>>,
    selected_id: RwSignal<Option<String>>,
    on_preview: Callback<ThemeDto>,
    on_update: Callback<ThemeDto>,
    on_delete: Callback<ThemeDto>,
    on_duplicate: Callback<ThemeDto>,
    on_set_active: Callback<ThemeDto>,
) -> impl IntoView {
    let id_for_active = theme.id.clone();
    let id_for_selected = theme.id.clone();
    let is_built_in = theme.is_built_in;
    let display_name = theme.name.clone();
    let swatch_bg = theme.root_background.clone();
    let swatch_primary = theme.root_primary.clone();

    let t_preview = theme.clone();
    let t_update = theme.clone();
    let t_delete = theme.clone();
    let t_duplicate = theme.clone();
    let t_set_active = theme.clone();
    let update_disabled_reason = !is_built_in;

    view! {
        <div class="flex items-center gap-2 rounded border border-border bg-surface-1 p-2">
            <button
                type="button"
                class="flex items-center gap-2 flex-1 text-left"
                on:click=move |_| on_preview.run(t_preview.clone())
            >
                <span
                    class="w-4 h-4 rounded border border-border flex-shrink-0"
                    style=format!("background: {swatch_bg}")
                />
                <span
                    class="w-4 h-4 rounded border border-border flex-shrink-0"
                    style=format!("background: {swatch_primary}")
                />
                <span class="text-sm text-text-primary flex-1 truncate">{display_name}</span>
                <Show when=move || is_built_in>
                    <Badge variant=BadgeVariant::Info size=BadgeSize::Sm>
                        "Built-in"
                    </Badge>
                </Show>
                <Show when=move || {
                    active_id.get().as_deref() == Some(id_for_active.as_str())
                }>
                    <Badge variant=BadgeVariant::Success size=BadgeSize::Sm>
                        "Active"
                    </Badge>
                </Show>
                <Show when=move || {
                    selected_id.get().as_deref() == Some(id_for_selected.as_str())
                }>
                    <span class="text-xs text-text-tertiary">"(editing)"</span>
                </Show>
            </button>

            <IconButton
                aria_label="Duplicate"
                size=Size::Sm
                on:click=move |_| on_duplicate.run(t_duplicate.clone())
            >
                <Icon icon=i::FiCopy />
            </IconButton>

            <Show when=move || update_disabled_reason>
                {
                    let t_u = t_update.clone();
                    let t_d = t_delete.clone();
                    view! {
                        <IconButton
                            aria_label="Save current editor state to this theme"
                            size=Size::Sm
                            variant=Variant::Secondary
                            on:click=move |_| on_update.run(t_u.clone())
                        >
                            <Icon icon=i::ChFloppyDisk />
                        </IconButton>
                        <IconButton
                            aria_label="Delete"
                            size=Size::Sm
                            variant=Variant::Danger
                            on:click=move |_| on_delete.run(t_d.clone())
                        >
                            <Icon icon=i::FaTrashSolid />
                        </IconButton>
                    }
                }
            </Show>

            <Button
                size=Size::Sm
                variant=Variant::Primary
                on:click=move |_| on_set_active.run(t_set_active.clone())
            >
                "Use"
            </Button>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Theme Page
// ---------------------------------------------------------------------------

#[component]
pub fn ThemePage() -> impl IntoView {
    let theme_state = expect_context::<ThemeState>();

    let (background, set_background) = signal("#FFFFFF".to_string());
    let (foreground, set_foreground) = signal("#111827".to_string());
    let (primary, set_primary) = signal("#2563EB".to_string());
    let (danger, set_danger) = signal("#DC2626".to_string());
    let (warning, set_warning) = signal("#D97706".to_string());
    let (success, set_success) = signal("#16A34A".to_string());

    let validation: RwSignal<Option<ThemeValidation>> = RwSignal::new(None);
    let tokens_signal = theme_state.tokens();

    // ---- Themes panel state --------------------------------------------------
    let themes: RwSignal<Vec<ThemeDto>> = RwSignal::new(Vec::new());
    let active_id: RwSignal<Option<String>> = RwSignal::new(None);
    let selected_id: RwSignal<Option<String>> = RwSignal::new(None);
    let list_error: RwSignal<Option<String>> = RwSignal::new(None);
    let save_error: RwSignal<Option<String>> = RwSignal::new(None);
    let row_error: RwSignal<Option<String>> = RwSignal::new(None);

    // Dialog state: create new theme.
    let save_dialog_open = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());

    // Dialog state: confirm delete.
    let delete_target: RwSignal<Option<ThemeDto>> = RwSignal::new(None);

    Effect::new(move |_| {
        refresh_themes(themes, active_id, list_error);
    });

    // Live-preview from the editor.
    let theme_for_preview = theme_state.clone();
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
        theme_for_preview.preview(&config);
    });

    // Snapshot the editor state imperatively at click time — `current_config`
    // is only called from non-reactive click handlers, so we use
    // `get_untracked()` to signal that we don't want a subscription (the
    // live-preview Effect above owns the reactive reads).
    let current_config = move || ThemeConfig {
        background: background.get_untracked(),
        foreground: foreground.get_untracked(),
        primary: primary.get_untracked(),
        danger: Some(danger.get_untracked()),
        warning: Some(warning.get_untracked()),
        success: Some(success.get_untracked()),
    };

    let load_into_editor = move |cfg: &ThemeConfig| {
        set_background.set(cfg.background.clone());
        set_foreground.set(cfg.foreground.clone());
        set_primary.set(cfg.primary.clone());
        set_danger.set(cfg.danger.clone().unwrap_or_default());
        set_warning.set(cfg.warning.clone().unwrap_or_default());
        set_success.set(cfg.success.clone().unwrap_or_default());
    };

    // ---- Row callbacks -------------------------------------------------------
    let on_preview = Callback::new(move |dto: ThemeDto| {
        row_error.set(None);
        let cfg = theme_config_from_dto(&dto);
        load_into_editor(&cfg);
        selected_id.set(Some(dto.id.clone()));
    });

    let theme_for_commit = theme_state.clone();
    let on_set_active = Callback::new(move |dto: ThemeDto| {
        let id = dto.id.clone();
        let cfg = theme_config_from_dto(&dto);
        let theme_for_this_call = theme_for_commit.clone();
        spawn_local(async move {
            match api::settings::set_active_theme(&id).await {
                Ok(()) => {
                    active_id.set(Some(id));
                    row_error.set(None);
                    theme_for_this_call.commit(&cfg);
                }
                Err(e) => {
                    row_error.set(Some(format!("set-active failed: {e}")));
                }
            }
        });
    });

    let on_duplicate = Callback::new(move |dto: ThemeDto| {
        let source_id = dto.id;
        spawn_local(async move {
            match api::settings::duplicate_theme(&source_id, None).await {
                Ok(_new_id) => {
                    row_error.set(None);
                    refresh_themes(themes, active_id, list_error);
                }
                Err(e) => {
                    row_error.set(Some(format!("duplicate failed: {e}")));
                }
            }
        });
    });

    let on_update = Callback::new(move |dto: ThemeDto| {
        let input = update_input_from(dto.id.clone(), dto.name.clone(), &current_config());
        spawn_local(async move {
            match api::settings::update_custom_theme(&input).await {
                Ok(()) => {
                    row_error.set(None);
                    refresh_themes(themes, active_id, list_error);
                }
                Err(e) => {
                    row_error.set(Some(format!("update failed: {e}")));
                }
            }
        });
    });

    let on_delete_request = Callback::new(move |dto: ThemeDto| {
        delete_target.set(Some(dto));
    });

    let confirm_delete = move |_| {
        let Some(dto) = delete_target.get_untracked() else {
            return;
        };
        let id = dto.id;
        spawn_local(async move {
            match api::settings::delete_custom_theme(&id).await {
                Ok(()) => {
                    row_error.set(None);
                    // If the deleted theme was loaded in the editor, clear the marker.
                    if selected_id.get_untracked().as_deref() == Some(id.as_str()) {
                        selected_id.set(None);
                    }
                    refresh_themes(themes, active_id, list_error);
                }
                Err(e) => {
                    row_error.set(Some(format!("delete failed: {e}")));
                }
            }
        });
        delete_target.set(None);
    };

    let cancel_delete = move |_| {
        delete_target.set(None);
    };

    // ---- Save-as-new flow ----------------------------------------------------
    let open_save_dialog = move |_| {
        new_name.set(String::new());
        save_error.set(None);
        save_dialog_open.set(true);
    };

    let confirm_save = move |_| {
        let name = new_name.get_untracked();
        if name.trim().is_empty() {
            save_error.set(Some("name required".into()));
            return;
        }
        let input = create_input_from(name, &current_config());
        spawn_local(async move {
            match api::settings::create_custom_theme(&input).await {
                Ok(new_id) => {
                    save_dialog_open.set(false);
                    selected_id.set(Some(new_id));
                    refresh_themes(themes, active_id, list_error);
                }
                Err(e) => {
                    save_error.set(Some(format!("save failed: {e}")));
                }
            }
        });
    };

    let apply_light = move |_| {
        set_background.set("#FFFFFF".into());
        set_foreground.set("#111827".into());
        set_primary.set("#2563EB".into());
        set_danger.set("#DC2626".into());
        set_warning.set("#D97706".into());
        set_success.set("#16A34A".into());
        selected_id.set(None);
    };

    let apply_dark = move |_| {
        set_background.set("#09090B".into());
        set_foreground.set("#FAFAFA".into());
        set_primary.set("#3B82F6".into());
        set_danger.set("#EF4444".into());
        set_warning.set("#F59E0B".into());
        set_success.set("#22C55E".into());
        selected_id.set(None);
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
                    <Button variant=Variant::Primary size=Size::Sm on:click=open_save_dialog>
                        "Save as new\u{2026}"
                    </Button>
                </div>
            </div>

            // ---- Themes from app.db --------------------------------------
            <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
                <div class="flex items-center justify-between">
                    <h3 class="text-sm font-semibold text-text-primary">"Themes (app.db)"</h3>
                    {move || {
                        list_error
                            .get()
                            .map(|e| view! { <span class="text-xs text-danger">{e}</span> })
                    }}
                </div>
                {move || {
                    row_error.get().map(|e| view! { <div class="text-xs text-danger">{e}</div> })
                }}
                <div class="space-y-2">
                    <For
                        each=move || themes.get()
                        key=|t| t.id.clone()
                        children=move |t| {
                            view! {
                                <ThemeRow
                                    theme=t
                                    active_id=active_id
                                    selected_id=selected_id
                                    on_preview=on_preview
                                    on_update=on_update
                                    on_delete=on_delete_request
                                    on_duplicate=on_duplicate
                                    on_set_active=on_set_active
                                />
                            }
                        }
                    />
                    <Show when=move || themes.get().is_empty() && list_error.get().is_none()>
                        <p class="text-xs text-text-tertiary">
                            "No themes yet \u{2014} built-ins seed on first app.db open."
                        </p>
                    </Show>
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

            // ---- Save-as-new dialog --------------------------------------
            <Dialog
                open=Signal::derive(move || save_dialog_open.get())
                on_close=Callback::new(move |()| save_dialog_open.set(false))
            >
                <DialogHeader>
                    <DialogTitle>"Save theme"</DialogTitle>
                </DialogHeader>
                <DialogBody>
                    <div class="space-y-3">
                        <Input
                            id="theme-new-name"
                            placeholder=Signal::derive(|| "Theme name".to_string())
                            value=Signal::derive(move || new_name.get())
                            on_input=Callback::new(move |v| new_name.set(v))
                        />
                        {move || {
                            save_error
                                .get()
                                .map(|e| view! { <p class="text-xs text-danger">{e}</p> })
                        }}
                    </div>
                </DialogBody>
                <DialogFooter>
                    <Button
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |_| save_dialog_open.set(false)
                    >
                        "Cancel"
                    </Button>
                    <Button variant=Variant::Primary size=Size::Sm on:click=confirm_save>
                        "Save"
                    </Button>
                </DialogFooter>
            </Dialog>

            // ---- Confirm-delete dialog -----------------------------------
            <Dialog
                open=Signal::derive(move || delete_target.get().is_some())
                on_close=Callback::new(move |()| delete_target.set(None))
            >
                <DialogHeader>
                    <DialogTitle>"Delete theme?"</DialogTitle>
                </DialogHeader>
                <DialogBody>
                    <p class="text-sm text-text-secondary">
                        {move || {
                            delete_target
                                .get()
                                .map_or_else(String::new, |t| format!("Delete \"{}\"?", t.name))
                        }}
                    </p>
                </DialogBody>
                <DialogFooter>
                    <Button variant=Variant::Ghost size=Size::Sm on:click=cancel_delete>
                        "Cancel"
                    </Button>
                    <Button variant=Variant::Danger size=Size::Sm on:click=confirm_delete>
                        "Delete"
                    </Button>
                </DialogFooter>
            </Dialog>
        </div>
    }
}
