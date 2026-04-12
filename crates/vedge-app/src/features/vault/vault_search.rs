use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::Input;
use vedge_ui::primitives::tokens::Size;

#[component]
pub fn VaultSearch(search_query: RwSignal<String>) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex-1">
            <Input
                id="vault-search"
                placeholder=Signal::derive(move || {
                    t_string!(i18n, vault.search_placeholder).to_string()
                })
                input_type="search"
                size=Size::Lg
                value=Signal::derive(move || search_query.get())
                on_input=Callback::new(move |v: String| search_query.set(v))
                clear_label=Signal::derive(move || t_string!(i18n, playground.clear).to_string())
            />
        </div>
    }
}
