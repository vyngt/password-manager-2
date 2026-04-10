use crate::i18n::*;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use leptos::ev::Targeted;
use leptos::prelude::*;
use reactive_stores::Store;
use ui::components::Input;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn VaultSearch(search_query: RwSignal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    view! {
        <div class="flex-1">
            <Input
                id="vault-search"
                placeholder=Signal::derive(move || t_string!(i18n, vault.search_placeholder).to_string())
                color=Signal::derive(move || color_store.primary().get())
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
