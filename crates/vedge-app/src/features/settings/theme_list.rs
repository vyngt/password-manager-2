//! Theme manager — the Settings **Appearance** section.
//!
//! Themes are DB records rendered as a scrollable list of rows grouped Built-in
//! (read-only) / Your themes (custom). Clicking a row applies it live
//! (`set_active_theme` persists the choice; `ThemeState::commit` re-derives +
//! injects the `--color-*` cascade so the whole app recolors). New / Duplicate /
//! Edit open the [`ThemeEditor`]; Delete (custom, confirmed) removes it and falls
//! back to the light built-in if it was active.
//!
//! Adapted from the proven `pages/playground/theme.rs` flow.

use super::theme_editor::{EditorTarget, ThemeEditor};
use super::theme_util::{is_active, theme_config_from_dto, theme_swatches};
use crate::api;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ipc::ThemeDto;
use vedge_ui::components::Badge;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogFooter, DialogHeader, DialogTitle};
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{BadgeSize, BadgeVariant, DialogSize, Size, Variant};
use vedge_ui::theme::{ThemeConfig, use_theme};

/// The light built-in id used as the fallback when the active theme is deleted.
const BUILTIN_LIGHT: &str = "builtin-light";

/// App-default light colors — the New-theme seed when there's no active theme,
/// and the recolor target after deleting the active theme.
fn default_config() -> ThemeConfig {
    ThemeConfig {
        background: "#FFFFFF".to_owned(),
        foreground: "#111827".to_owned(),
        primary: "#2563EB".to_owned(),
        danger: None,
        warning: None,
        success: None,
    }
}

/// Reload the theme list + the active id (fire-and-forget).
fn refresh(themes: RwSignal<Vec<ThemeDto>>, active_id: RwSignal<Option<String>>) {
    spawn_local(async move {
        if let Ok(rows) = api::settings::list_themes().await {
            themes.set(rows);
        }
        if let Ok(active) = api::settings::get_active_theme().await {
            active_id.set(Some(active.id));
        }
    });
}

#[component]
pub fn ThemeList() -> impl IntoView {
    let i18n = use_i18n();
    let theme = use_theme();
    let theme_apply = theme.clone();
    let theme_delete = theme;

    let themes = RwSignal::new(Vec::<ThemeDto>::new());
    let active_id = RwSignal::new(Option::<String>::None);
    let editor = RwSignal::new(Option::<EditorTarget>::None);
    let delete_target = RwSignal::new(Option::<ThemeDto>::None);

    Effect::new(move |_| refresh(themes, active_id));

    // Apply an existing theme: persist the active id, then recolor via ThemeState.
    let on_apply = Callback::new(move |dto: ThemeDto| {
        let theme = theme_apply.clone();
        let id = dto.id.clone();
        let cfg = theme_config_from_dto(&dto);
        spawn_local(async move {
            if api::settings::set_active_theme(&id).await.is_ok() {
                active_id.set(Some(id));
                theme.commit(&cfg);
            }
        });
    });

    let on_new = Callback::new(move |()| {
        let seed = themes
            .get_untracked()
            .iter()
            .find(|t| is_active(t, active_id.get_untracked().as_deref()))
            .map_or_else(default_config, theme_config_from_dto);
        editor.set(Some(EditorTarget {
            source: None,
            editing: false,
            seed,
            seed_name: String::new(),
        }));
    });

    let on_edit = Callback::new(move |dto: ThemeDto| {
        editor.set(Some(EditorTarget {
            seed: theme_config_from_dto(&dto),
            seed_name: dto.name.clone(),
            source: Some(dto),
            editing: true,
        }));
    });

    let on_duplicate = Callback::new(move |dto: ThemeDto| {
        editor.set(Some(EditorTarget {
            seed: theme_config_from_dto(&dto),
            seed_name: format!("{} copy", dto.name),
            source: Some(dto),
            editing: false,
        }));
    });

    let on_delete_request = Callback::new(move |dto: ThemeDto| delete_target.set(Some(dto)));

    // A `Callback` (Copy) — capturing the non-Copy `ThemeState` in a bare closure
    // would make the Dialog's `Fn` children `FnOnce`.
    let confirm_delete = Callback::new(move |()| {
        let Some(dto) = delete_target.get_untracked() else {
            return;
        };
        let theme = theme_delete.clone();
        let id = dto.id;
        let was_active = active_id.get_untracked().as_deref() == Some(id.as_str());
        spawn_local(async move {
            if api::settings::delete_custom_theme(&id).await.is_ok() {
                if was_active {
                    let _ = api::settings::set_active_theme(BUILTIN_LIGHT).await;
                    theme.commit(&default_config());
                }
                refresh(themes, active_id);
            }
        });
        delete_target.set(None);
    });

    let close_delete = Callback::new(move |()| delete_target.set(None));
    let on_saved = Callback::new(move |()| refresh(themes, active_id));

    let builtin = Signal::derive(move || {
        themes
            .get()
            .into_iter()
            .filter(|t| t.is_built_in)
            .collect::<Vec<_>>()
    });
    let custom = Signal::derive(move || {
        themes
            .get()
            .into_iter()
            .filter(|t| !t.is_built_in)
            .collect::<Vec<_>>()
    });

    view! {
        <div class="h-full flex flex-col min-h-0">
            <div class="flex items-center justify-between mb-3 shrink-0">
                <div>
                    <div class="text-base font-semibold text-text-primary">
                        {move || t!(i18n, settings.themes)}
                    </div>
                    <div class="text-xs text-text-secondary">
                        {move || t!(i18n, settings.themes_desc)}
                    </div>
                </div>
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    on:click=move |_: web_sys::MouseEvent| on_new.run(())
                >
                    <span class="inline-flex items-center gap-1.5">
                        <Icon attr:aria-hidden="true" icon=i::FaPlusSolid />
                        {move || t!(i18n, settings.theme_new)}
                    </span>
                </Button>
            </div>

            <div class="flex-1 overflow-auto min-h-0 border border-border rounded-lg">
                <GroupHeader label=Signal::derive(move || {
                    t_string!(i18n, settings.theme_group_builtin).to_owned()
                }) />
                <For
                    each=move || builtin.get()
                    key=|t| (t.id.clone(), t.updated_at.clone())
                    children=move |dto| {
                        view! {
                            <ThemeRow
                                dto=dto
                                active_id=active_id
                                on_apply=on_apply
                                on_edit=on_edit
                                on_duplicate=on_duplicate
                                on_delete=on_delete_request
                            />
                        }
                    }
                />
                <GroupHeader label=Signal::derive(move || {
                    t_string!(i18n, settings.theme_group_custom).to_owned()
                }) />
                <For
                    each=move || custom.get()
                    key=|t| (t.id.clone(), t.updated_at.clone())
                    children=move |dto| {
                        view! {
                            <ThemeRow
                                dto=dto
                                active_id=active_id
                                on_apply=on_apply
                                on_edit=on_edit
                                on_duplicate=on_duplicate
                                on_delete=on_delete_request
                            />
                        }
                    }
                />
            </div>
        </div>

        <ThemeEditor target=editor on_saved=on_saved />

        // Delete confirm.
        <Dialog
            open=Signal::derive(move || delete_target.get().is_some())
            on_close=close_delete
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.theme_delete)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    <p class="text-sm text-text-secondary">
                        {move || t!(i18n, settings.theme_delete_confirm)}
                    </p>
                </div>
            </DialogBody>
            <DialogFooter>
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    on:click=move |_: web_sys::MouseEvent| close_delete.run(())
                >
                    {move || t!(i18n, settings.cancel)}
                </Button>
                <Button
                    variant=Variant::Danger
                    size=Size::Sm
                    on:click=move |_: web_sys::MouseEvent| confirm_delete.run(())
                >
                    {move || t!(i18n, settings.theme_delete)}
                </Button>
            </DialogFooter>
        </Dialog>
    }
}

#[component]
fn GroupHeader(#[prop(into)] label: Signal<String>) -> impl IntoView {
    view! {
        <div class="text-[11px] uppercase tracking-wider text-text-tertiary px-3.5 pt-3 pb-1 border-t border-border first:border-t-0">
            {move || label.get()}
        </div>
    }
}

#[component]
fn ThemeRow(
    dto: ThemeDto,
    active_id: RwSignal<Option<String>>,
    on_apply: Callback<ThemeDto>,
    on_edit: Callback<ThemeDto>,
    on_duplicate: Callback<ThemeDto>,
    on_delete: Callback<ThemeDto>,
) -> impl IntoView {
    let i18n = use_i18n();
    let built_in = dto.is_built_in;
    let swatches = theme_swatches(&dto);
    let name = dto.name.clone();
    let id = dto.id.clone();
    let is_on = Memo::new(move |_| active_id.get().as_deref() == Some(id.as_str()));

    let d_apply = dto.clone();
    let d_edit = dto.clone();
    let d_dup = dto.clone();
    let d_del = dto;

    let actions = if built_in {
        view! {
            <IconButton
                aria_label=Signal::derive(move || {
                    t_string!(i18n, settings.theme_duplicate).to_owned()
                })
                size=Size::Sm
                on:click=move |_: web_sys::MouseEvent| on_duplicate.run(d_dup.clone())
            >
                <Icon attr:aria-hidden="true" icon=i::FaCopySolid />
            </IconButton>
        }
        .into_any()
    } else {
        view! {
            <IconButton
                aria_label=Signal::derive(move || t_string!(i18n, settings.theme_edit).to_owned())
                size=Size::Sm
                on:click=move |_: web_sys::MouseEvent| on_edit.run(d_edit.clone())
            >
                <Icon attr:aria-hidden="true" icon=i::FaPenSolid />
            </IconButton>
            <IconButton
                aria_label=Signal::derive(move || {
                    t_string!(i18n, settings.theme_duplicate).to_owned()
                })
                size=Size::Sm
                on:click=move |_: web_sys::MouseEvent| on_duplicate.run(d_dup.clone())
            >
                <Icon attr:aria-hidden="true" icon=i::FaCopySolid />
            </IconButton>
            <IconButton
                aria_label=Signal::derive(move || {
                    t_string!(i18n, settings.theme_delete).to_owned()
                })
                size=Size::Sm
                on:click=move |_: web_sys::MouseEvent| on_delete.run(d_del.clone())
            >
                <Icon attr:aria-hidden="true" icon=i::FaTrashSolid />
            </IconButton>
        }
        .into_any()
    };

    view! {
        <div
            class="flex items-center gap-3 px-3.5 py-2.5 border-b border-border cursor-pointer hover:bg-surface-1"
            class=("bg-primary-muted", move || is_on.get())
            on:click=move |_: web_sys::MouseEvent| on_apply.run(d_apply.clone())
        >
            <span
                class="w-4 h-4 rounded-full border-2 shrink-0 flex items-center justify-center"
                class=("border-primary", move || is_on.get())
                class=("border-border-strong", move || !is_on.get())
            >
                <Show when=move || is_on.get()>
                    <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
                </Show>
            </span>

            <span class="inline-flex gap-1 shrink-0">
                {swatches
                    .into_iter()
                    .map(|c| {
                        view! {
                            <span
                                class="w-3.5 h-3.5 rounded-sm border border-border"
                                style=format!("background-color:{c}")
                            ></span>
                        }
                    })
                    .collect_view()}
            </span>

            <span class="flex-1 text-sm font-medium text-text-primary truncate">{name}</span>

            <Badge
                size=BadgeSize::Sm
                variant=if built_in { BadgeVariant::Default } else { BadgeVariant::Info }
            >
                {move || {
                    if built_in {
                        t_string!(i18n, settings.theme_builtin).to_owned()
                    } else {
                        t_string!(i18n, settings.theme_custom).to_owned()
                    }
                }}
            </Badge>

            // Actions — stop propagation so clicking them doesn't apply the row.
            <span
                class="flex gap-1 shrink-0"
                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            >
                {actions}
            </span>
        </div>
    }
}
