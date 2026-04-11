use crate::i18n::*;
use leptos::ev::Targeted;
use leptos::prelude::*;
use ui::components::Input;
use ui::primitives::color::RgbColor;
use ui::theme::ThemeState;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn VaultSearch(search_query: RwSignal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let theme = expect_context::<ThemeState>();

    let primary_color = Signal::derive(move || {
        RgbColor::from_hex(&theme.tokens().get().color_primary)
    });

    view! {
        <div class="flex-1">
            <Input
                id="vault-search"
                placeholder=Signal::derive(move || t_string!(i18n, vault.search_placeholder).to_string())
                color=primary_color
                input_type="text"
                value=Signal::derive(move || search_query.get())
                class="h-[42px]"
                label_class="text-[14px] peer-focus:text-[10px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[10px]"
                on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                    search_query.set(ev.target().value());
                })
            />
        </div>
    }
}
