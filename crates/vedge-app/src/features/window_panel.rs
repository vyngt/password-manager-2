use crate::api::window;
use crate::i18n::{I18nLocaleTrait, Locale, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ui::components::icon::VEdge;
use vedge_ui::components::tooltip_icon_button::TooltipIconButton;
use vedge_ui::primitives::tokens::{Placement, Shape, Size, Variant};
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
    }
}
