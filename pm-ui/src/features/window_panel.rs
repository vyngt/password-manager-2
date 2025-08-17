use crate::api::tauri::get_current_window;
use crate::components::icon::vorpal::VorpalIcon;
use crate::components::icon_button::IconButton;
use crate::components::icon_button::variants::{
    Effect as IconButtonEffect, Shape as IconButtonShape, Variant as IconButtonVariant,
};
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use reactive_stores::Store;

#[component]
pub fn WindowPanel() -> impl IntoView {
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
            class="flex h-12 flex-grow-0 justify-between border-b border-secondary/20 bg-primary/20"
        >
            <div
                class="flex flex-col justify-center pl-2 pointer-events-none"
                style=handle_inner_color
            >
                <div class="flex gap-2 justify-center">
                    <div class="flex flex-col justify-center text-foreground">
                        <div class="h-[30px] w-[30px]">
                            <VorpalIcon />
                        </div>
                    </div>
                    <div class="flex flex-col justify-center">
                        <h5 class="text-foreground">Password Manager</h5>
                    </div>
                </div>
            </div>
            <div class="flex flex-grow" data-tauri-drag-region=true></div>
            <div class="flex h-full" style=handle_inner_color>
                <IconButton
                    color=Signal::derive(move || color_store.primary().get())
                    variant=IconButtonVariant::Text
                    effect=IconButtonEffect::Ripple
                    shape=IconButtonShape::Sharp
                    auto_text_color=false
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
                    color=Signal::derive(move || color_store.primary().get())
                    variant=IconButtonVariant::Text
                    effect=IconButtonEffect::Ripple
                    shape=IconButtonShape::Sharp
                    auto_text_color=false
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
                    color=Signal::derive(move || color_store.danger().get())
                    variant=IconButtonVariant::Text
                    effect=IconButtonEffect::Ripple
                    shape=IconButtonShape::Sharp
                    auto_text_color=false
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
