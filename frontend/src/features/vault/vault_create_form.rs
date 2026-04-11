use super::types::VaultItem;
use crate::api::tauri;
use crate::i18n::*;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use leptos::ev::Targeted;
use leptos::prelude::*;
use leptos::task::spawn_local;
use reactive_stores::Store;
use serde_json::json;
use serde_wasm_bindgen::{from_value, to_value as to_js_value};
use ui::components::Button;
use ui::components::Input;
use ui::primitives::tokens::{Size, Variant};
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn VaultCreateForm(
    show: RwSignal<bool>,
    on_created: Callback<VaultItem>,
) -> impl IntoView {
    let i18n = use_i18n();
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    let form_title = RwSignal::new(String::new());
    let form_identifier = RwSignal::new(String::new());
    let form_password = RwSignal::new(String::new());
    let form_url = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);

    let reset_form = move || {
        form_title.set(String::new());
        form_identifier.set(String::new());
        form_password.set(String::new());
        form_url.set(String::new());
    };

    let handle_cancel = move |_| {
        reset_form();
        show.set(false);
    };

    let handle_save = move |_| {
        if submitting.get() {
            return;
        }

        let title = form_title.get();
        if title.is_empty() {
            return;
        }

        submitting.set(true);

        let identifier = form_identifier.get();
        let password = form_password.get();
        let url = form_url.get();

        spawn_local(async move {
            let Some(args) = to_js_value(&json!({
                "request": {
                    "title": title,
                    "kind": "Credential",
                    "data": {
                        "kind": "Credential",
                        "identifier": identifier,
                        "password": password,
                        "url": url
                    }
                }
            }))
            .ok() else {
                submitting.set(false);
                return;
            };

            let res = tauri::invoke("create_vault_item", args).await;

            match from_value::<VaultItem>(res) {
                Ok(new_item) => {
                    on_created.run(new_item);
                    reset_form();
                    show.set(false);
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to create vault item: {:?}", e).into(),
                    );
                }
            }

            submitting.set(false);
        });
    };

    view! {
        <div class="border border-secondary/20 rounded-lg p-4 bg-primary/5">
            <h3 class="text-sm font-semibold mb-3 text-foreground/80">
                {move || t!(i18n, vault.create_title)}
            </h3>
            <div class="grid grid-cols-2 gap-3">
                <Input
                    id="vault-form-title"
                    placeholder=Signal::derive(move || t_string!(i18n, vault.form_title).to_string())
                    color=Signal::derive(move || color_store.primary().get())
                    input_type="text"
                    value=Signal::derive(move || form_title.get())
                    class="h-[40px]"
                    label_class="text-[13px] peer-focus:text-[10px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[10px]"
                    on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                        form_title.set(ev.target().value());
                    })
                />
                <Input
                    id="vault-form-identifier"
                    placeholder=Signal::derive(move || t_string!(i18n, vault.form_identifier).to_string())
                    color=Signal::derive(move || color_store.primary().get())
                    input_type="text"
                    value=Signal::derive(move || form_identifier.get())
                    class="h-[40px]"
                    label_class="text-[13px] peer-focus:text-[10px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[10px]"
                    on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                        form_identifier.set(ev.target().value());
                    })
                />
                <Input
                    id="vault-form-password"
                    placeholder=Signal::derive(move || t_string!(i18n, vault.form_password).to_string())
                    color=Signal::derive(move || color_store.primary().get())
                    input_type="password"
                    value=Signal::derive(move || form_password.get())
                    class="h-[40px]"
                    label_class="text-[13px] peer-focus:text-[10px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[10px]"
                    on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                        form_password.set(ev.target().value());
                    })
                />
                <Input
                    id="vault-form-url"
                    placeholder=Signal::derive(move || t_string!(i18n, vault.form_url).to_string())
                    color=Signal::derive(move || color_store.primary().get())
                    input_type="text"
                    value=Signal::derive(move || form_url.get())
                    class="h-[40px]"
                    label_class="text-[13px] peer-focus:text-[10px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[10px]"
                    on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                        form_url.set(ev.target().value());
                    })
                />
            </div>
            <div class="flex gap-2 justify-end mt-3">
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    on:click=handle_cancel
                >
                    {move || t!(i18n, vault.cancel)}
                </Button>
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    on:click=handle_save
                >
                    {move || t!(i18n, vault.save)}
                </Button>
            </div>
        </div>
    }
}
