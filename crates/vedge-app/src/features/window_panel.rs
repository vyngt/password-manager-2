use crate::api::about;
use crate::api::window;
use crate::features::settings::about_panel::UpdateStatusCtx;
use crate::i18n::{I18nLocaleTrait, Locale, t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ipc::UpdateStatus;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::icon::VEdge;
use vedge_ui::components::tooltip_icon_button::TooltipIconButton;
use vedge_ui::primitives::tokens::{DialogSize, Placement, Shape, Size, Variant};
use vedge_ui::theme::{ThemeState, compute_primary_foreground};

#[component]
pub fn WindowPanel() -> impl IntoView {
    let i18n = use_i18n();
    let theme = expect_context::<ThemeState>();

    let (is_maximized, set_is_maximized) = signal(false);

    let tokens_signal = theme.tokens();
    let handle_inner_color = move || {
        let tokens = tokens_signal.get();
        let fg = compute_primary_foreground(&tokens.color_background).unwrap_or("#FAFAFA");
        format!("color: {fg};")
    };

    let maximized_state = LocalResource::new(async move || {
        window::current()
            .is_maximized()
            .await
            .as_bool()
            .unwrap_or(false)
    });

    Effect::new(move || {
        let state = maximized_state.get().unwrap_or(false);
        set_is_maximized.set(state);
    });

    // About surface (PG.4): the window panel is app-level chrome rendered OUTSIDE
    // `/v`, so this is reachable PRE-UNLOCK — the launch screen's answer to "what
    // version am I on?" without unlocking into Settings. Version comes from the
    // cheap, offline `app_version` command (no vault, no network → no capability).
    let about_open = RwSignal::new(false);
    let version = RwSignal::new(String::new());
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(v) = about::app_version().await {
                version.set(v);
            }
        });
    });
    // Update-available dot: reads the shared launch auto-check result (PG.3). Only
    // populated when the user opted in, so it stays hidden otherwise.
    let update_status = expect_context::<UpdateStatusCtx>();
    let update_available = move || {
        update_status
            .0
            .get()
            .is_some_and(|d| d.status == UpdateStatus::UpdateAvailable)
    };

    view! {
        <header
            data-tauri-drag-region=true
            class="flex grow-0 justify-between border-b border-border bg-primary/20"
        >
            <div
                class="flex flex-col justify-center pl-2 pointer-events-none"
                style=handle_inner_color
            >
                <div class="flex gap-2 justify-center">
                    <div class="flex flex-col justify-center">
                        <div class="text-[30px]" aria-hidden="true">
                            <Icon icon=VEdge />
                        </div>
                    </div>
                    <div class="flex flex-col justify-center">
                        <h5>"VEdge"</h5>
                    </div>
                </div>
            </div>
            <div class="flex grow" data-tauri-drag-region=true></div>
            // About (PG.4) — left of the locale toggle, OUTSIDE the pointer-events-none
            // brand block so it stays clickable. `Size::Lg`/`Shape::Square` matches the
            // window controls. A dot appears when an opted-in update check found a newer
            // release.
            <div class="relative flex" style=handle_inner_color data-testid="window-about">
                <TooltipIconButton
                    label=Signal::derive(move || t_string!(i18n, window.about).to_owned())
                    size=Size::Lg
                    shape=Shape::Square
                    tooltip_placement=Placement::Bottom
                    on_click=Callback::new(move |_: ()| about_open.set(true))
                >
                    <span aria-hidden="true">
                        <Icon icon=i::FaCircleInfoSolid />
                    </span>
                </TooltipIconButton>
                <Show when=update_available>
                    <span
                        class="absolute top-1 right-1 w-2 h-2 rounded-full bg-primary pointer-events-none"
                        aria-hidden="true"
                    ></span>
                </Show>
            </div>
            <div class="flex flex-col justify-center" style=handle_inner_color>
                // Design-system exception (OS titlebar chrome): the locale toggle's color
                // inherits the computed titlebar foreground (`handle_inner_color`). Ghost
                // `Button` forces a fixed height + text-secondary and `Link` forces
                // --color-primary, neither able to take the dynamic chrome color. Kept raw.
                <button
                    type="button"
                    class="px-2 text-xs font-semibold opacity-60 hover:opacity-100 cursor-pointer uppercase tracking-wide"
                    on:click=move |_| {
                        let new_locale = match i18n.get_locale() {
                            Locale::en => Locale::vi,
                            Locale::vi => Locale::en,
                        };
                        i18n.set_locale(new_locale);
                    }
                >
                    {move || i18n.get_locale().as_str()}
                </button>
            </div>
            <div class="flex h-full" style=handle_inner_color>
                <TooltipIconButton
                    label=Signal::derive(move || t_string!(i18n, window.minimize).to_owned())
                    size=Size::Lg
                    shape=Shape::Square
                    tooltip_placement=Placement::Bottom
                    on_click=Callback::new(move |_: ()| {
                        spawn_local(async move {
                            let app_window = window::current();
                            window::minimize(&app_window).await;
                        });
                    })
                >
                    // aria-hidden on a wrapping <span>, not the <Icon>: `attr:` on an
                    // <Icon> passed through TooltipIconButton's boxed children trips a
                    // Leptos RPIT capture bound. The button's tooltip/label names it.
                    <span aria-hidden="true">
                        <Icon icon=i::FaWindowMinimizeSolid />
                    </span>
                </TooltipIconButton>
                <TooltipIconButton
                    label=Signal::derive(move || t_string!(i18n, window.maximize).to_owned())
                    size=Size::Lg
                    shape=Shape::Square
                    tooltip_placement=Placement::Left
                    on_click=Callback::new(move |_: ()| {
                        spawn_local(async move {
                            let app_window = window::current();
                            let is_maximized = window::is_maximized(&app_window).await;
                            if is_maximized {
                                window::unmaximize(&app_window).await;
                                set_is_maximized.set(false);
                            } else {
                                window::maximize(&app_window).await;
                                set_is_maximized.set(true);
                            }
                        });
                    })
                >
                    <span aria-hidden="true">
                        <Show
                            when=move || is_maximized.get()
                            fallback=move || view! { <Icon icon=i::FaWindowMaximizeSolid /> }
                        >
                            <Icon icon=i::FaWindowRestoreSolid />
                        </Show>
                    </span>
                </TooltipIconButton>
                <TooltipIconButton
                    label=Signal::derive(move || t_string!(i18n, window.close).to_owned())
                    variant=Variant::Danger
                    size=Size::Lg
                    shape=Shape::Square
                    tooltip_placement=Placement::Left
                    on_click=Callback::new(move |_: ()| {
                        spawn_local(async move {
                            let app_window = window::current();
                            window::close(&app_window).await;
                        });
                    })
                >
                    <span aria-hidden="true">
                        <Icon icon=i::FaXmarkSolid />
                    </span>
                </TooltipIconButton>
            </div>
        </header>
        <Dialog
            open=Signal::derive(move || about_open.get())
            on_close=Callback::new(move |()| about_open.set(false))
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, window.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, window.about)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-3 min-w-[18rem]">
                    <div class="flex items-center gap-2">
                        <div class="text-[28px]" aria-hidden="true">
                            <Icon icon=VEdge />
                        </div>
                        <div>
                            <div class="text-sm font-semibold text-text-primary">"VEdge"</div>
                            <div
                                class="text-xs text-text-secondary font-jetbrains-mono"
                                data-testid="window-about-version"
                            >
                                {move || version.get()}
                            </div>
                        </div>
                    </div>
                    <div class="text-xs text-text-secondary">
                        <div>{move || t!(i18n, settings.about_licence_body)}</div>
                        <div class="mt-0.5">{move || t!(i18n, settings.about_copyright)}</div>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
