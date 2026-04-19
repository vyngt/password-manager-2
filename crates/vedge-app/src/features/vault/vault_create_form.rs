use super::types::{VaultItem, VaultItemData, VaultItemDataCredential};
use crate::i18n::*;
use leptos::prelude::*;
use uuid::Uuid;
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::primitives::tokens::{Size, Variant};

// TODO(UI adaptation): this form currently produces a local-only
// `VaultItem` without calling the shell. The real save path is
// `api::entry::create_entry(vault_path, &PayloadDto::Login(...))` returning
// the new entry id. Component tree needs a vault_path context before
// wiring; see docs/ImplementAppBridge.md "What comes after this plan".

#[component]
pub fn VaultCreateForm(show: RwSignal<bool>, on_created: Callback<VaultItem>) -> impl IntoView {
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

        let item = VaultItem {
            id: Uuid::new_v4().to_string(),
            title,
            kind: "Credential".to_owned(),
            data: VaultItemData::Credential(VaultItemDataCredential {
                identifier: form_identifier.get(),
                password: form_password.get(),
                url: form_url.get(),
            }),
            created_at: None,
            updated_at: None,
        };
        on_created.run(item);
        reset_form();
        show.set(false);
        submitting.set(false);
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
                    reveal_label=Signal::derive(move || {
                        t_string!(i18n, playground.show_password).to_string()
                    })
                    hide_label=Signal::derive(move || {
                        t_string!(i18n, playground.hide_password).to_string()
                    })
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
