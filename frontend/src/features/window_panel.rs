use crate::api::tauri::get_current_window;
use crate::i18n::*;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use reactive_stores::Store;
use ui::components::icon::VEdge;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::Variant;

#[component]
pub fn WindowPanel() -> impl IntoView {
    let i18n = use_i18n();
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    let (is_maximized, set_is_maximized) = signal(false);

    let handle_inner_color = move || {
        let text_color = color_store
            .background()
            .get()
            .calculate_white_black_text_color(None);

        format!("--color-foreground: {};", text_color.to_rgb_string())
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
            class="flex h-[48px] grow-0 justify-between border-b border-secondary/20 bg-primary/20"
        >
            <div
                class="flex flex-col justify-center pl-2 pointer-events-none"
                style=handle_inner_color
            >
                <div class="flex gap-2 justify-center">
                    <div class="flex flex-col justify-center text-foreground">
                        <div class="text-[30px]">
                            <Icon icon=VEdge />
                        </div>
                    </div>
                    <div class="flex flex-col justify-center">
                        <h5 class="text-foreground">"VEdge"</h5>
                    </div>
                </div>
            </div>
            <div class="flex grow" data-tauri-drag-region=true></div>
            <div class="flex h-full items-center" style=handle_inner_color>
                <button
                    class="px-2 text-xs font-semibold text-foreground/60 hover:text-foreground cursor-pointer uppercase tracking-wide"
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
                    class="w-12"
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
                    class="w-12"
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
                    class="w-12"
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
