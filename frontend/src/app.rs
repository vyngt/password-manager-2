use crate::i18n::*;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};

use leptos::prelude::*;

use crate::features::window_panel::WindowPanel;
use crate::routes::AppRoutes;
use reactive_stores::Store;
use ui::components::feedback::toast::provider::ToastProvider;

#[component]
pub fn App() -> impl IntoView {
    let color_store = Store::new(ColorStore::new());
    provide_context(color_store);

    view! {
        <I18nContextProvider>
            <div
                class="h-full flex flex-col bg-background text-foreground"
                style=move || {
                    let primary_color = color_store.primary().get().to_rgb_string();
                    let secondary_color = color_store.secondary().get().to_rgb_string();
                    let success_color = color_store.success().get().to_rgb_string();
                    let danger_color = color_store.danger().get().to_rgb_string();
                    let warning_color = color_store.warning().get().to_rgb_string();
                    let bg_color = color_store.background().get().to_rgb_string();
                    let fg_color = color_store.foreground().get().to_rgb_string();
                    let colors = vec![
                        format!("--color-primary: {}", primary_color),
                        format!("--color-secondary: {}", secondary_color),
                        format!("--color-success: {}", success_color),
                        format!("--color-danger: {}", danger_color),
                        format!("--color-warning: {}", warning_color),
                        format!("--color-background: {}", bg_color),
                        format!("--color-foreground: {}", fg_color),
                    ];
                    colors.join(";")
                }
            >
                <ToastProvider>
                    <WindowPanel />
                    <main class="h-[calc(100%-48px)] overflow-y-auto app-scrollbar">
                        <AppRoutes />
                    </main>
                </ToastProvider>
            </div>
        </I18nContextProvider>
    }
}
