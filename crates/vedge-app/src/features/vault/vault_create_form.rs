use super::types::VaultItem;
use crate::api::tauri;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use serde_wasm_bindgen::{from_value, to_value as to_js_value};
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::primitives::tokens::{Size, Variant};

#[component]
pub fn VaultCreateForm(
    show: RwSignal<bool>,
    on_created: Callback<VaultItem>,
) -> impl IntoView {
    let i18n = use_i18n();

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
        <div class="border border-border rounded-lg p-4 bg-primary-muted">
            <h3 class="text-sm font-semibold mb-3 text-text-secondary">
                {move || t!(i18n, vault.create_title)}
            </h3>
            <div class="grid grid-cols-2 gap-3">
                <Input
                    id="vault-form-title"
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, vault.form_title).to_string()
                    })
                    value=Signal::derive(move || form_title.get())
                    on_input=Callback::new(move |v: String| form_title.set(v))
                />
                <Input
                    id="vault-form-identifier"
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, vault.form_identifier).to_string()
                    })
                    value=Signal::derive(move || form_identifier.get())
                    on_input=Callback::new(move |v: String| form_identifier.set(v))
                />
                <Input
                    id="vault-form-password"
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, vault.form_password).to_string()
                    })
                    input_type="password"
                    value=Signal::derive(move || form_password.get())
                    on_input=Callback::new(move |v: String| form_password.set(v))
                />
                <Input
                    id="vault-form-url"
                    placeholder=Signal::derive(move || t_string!(i18n, vault.form_url).to_string())
                    value=Signal::derive(move || form_url.get())
                    on_input=Callback::new(move |v: String| form_url.set(v))
                />
            </div>
            <div class="flex gap-2 justify-end mt-3">
                <Button variant=Variant::Ghost size=Size::Sm on:click=handle_cancel>
                    {move || t!(i18n, vault.cancel)}
                </Button>
                <Button variant=Variant::Primary size=Size::Sm on:click=handle_save>
                    {move || t!(i18n, vault.save)}
                </Button>
            </div>
        </div>
    }
}
