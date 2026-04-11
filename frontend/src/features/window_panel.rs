use crate::api::tauri::get_current_window;
use crate::i18n::*;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use ui::components::icon::VEdge;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Shape, Size, Variant};
use ui::theme::{ThemeState, compute_primary_foreground};

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
        get_current_window()
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
                        <div class="text-[30px]">
                            <Icon icon=VEdge />
                        </div>
                    </div>
                    <div class="flex flex-col justify-center">
                        <h5>"VEdge"</h5>
                    </div>
                </div>
            </div>
            <div class="flex grow" data-tauri-drag-region=true></div>
            <div class="flex h-full items-center" style=handle_inner_color>
                <button
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
                <IconButton
                    aria_label="Minimize window"
                    size=Size::Lg
                    shape=Shape::Square
                    on:click=move |_ev| {
                        spawn_local(async move {
                            let app_window = get_current_window();
                            app_window.minimize().await;
                        });
                    }
                >
                    <Icon icon=i::FaWindowMinimizeSolid />
                </IconButton>
                <IconButton
                    aria_label="Maximize window"
                    size=Size::Lg
                    shape=Shape::Square
                    on:click=move |_ev| {
                        spawn_local(async move {
                            let app_window = get_current_window();
                            let is_maximized = app_window.is_maximized().await;
                            if is_maximized.as_bool().unwrap_or(false) {
                                app_window.unmaximize().await;
                                set_is_maximized.set(false);
                            } else {
                                app_window.maximize().await;
                                set_is_maximized.set(true);
                            }
                        });
                    }
                >
                    <Show
                        when=move || is_maximized.get()
                        fallback=move || view! { <Icon icon=i::FaWindowMaximizeSolid /> }
                    >
                        <Icon icon=i::FaWindowRestoreSolid />
                    </Show>
                </IconButton>
                <IconButton
                    aria_label="Close window"
                    variant=Variant::Danger
                    size=Size::Lg
                    shape=Shape::Square
                    on:click=move |_ev| {
                        spawn_local(async move {
                            let app_window = get_current_window();
                            app_window.close().await;
                        });
                    }
                >
                    <Icon icon=i::FaXmarkSolid />
                </IconButton>
            </div>
        </header>
    }
}
