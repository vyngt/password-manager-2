use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{CommonMetaDto, EntryTypeDto, LoginPayloadDto, PayloadDto};
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::primitives::tokens::{Size, Variant};

/// Build the write-side payload for a new Login entry from the form fields.
///
/// An empty URL collapses to `None` so we don't persist a blank string.
fn to_login_payload(title: String, username: String, password: String, url: String) -> PayloadDto {
    PayloadDto::Login(LoginPayloadDto {
        meta: CommonMetaDto {
            name: title,
            entry_type: EntryTypeDto::Login,
            url: (!url.is_empty()).then_some(url),
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            notes: None,
        },
        username,
        password,
        totp_secret: None,
        recovery_codes: vec![],
    })
}

/// Add-entry form. On submit it persists a Login entry through the backend
/// (`api::entry::create_entry`) against the active vault, then pings the
/// parent to refresh via `on_created` (a `Callback<()>`).
#[component]
pub fn VaultCreateForm(show: RwSignal<bool>, on_created: Callback<()>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let form_title = RwSignal::new(String::new());
    let form_identifier = RwSignal::new(String::new());
    let form_password = RwSignal::new(String::new());
    let form_url = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let reset_form = move || {
        form_title.set(String::new());
        form_identifier.set(String::new());
        form_password.set(String::new());
        form_url.set(String::new());
    };

    let handle_cancel = move |_| {
        reset_form();
        error.set(None);
        show.set(false);
    };

    let handle_save = move |_| {
        if submitting.get() {
            return;
        }
        let title = form_title.get();
        if title.trim().is_empty() {
            return;
        }

        submitting.set(true);
        error.set(None);

        let vault_path = active.path.get().unwrap_or_default();
        let payload = to_login_payload(
            title,
            form_identifier.get(),
            form_password.get(),
            form_url.get(),
        );

        spawn_local(async move {
            match api::entry::create_entry(&vault_path, &payload).await {
                Ok(_id) => {
                    reset_form();
                    show.set(false);
                    on_created.run(());
                }
                Err(e) => {
                    error.set(Some(format!("Could not save the entry: {e}")));
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
            {move || error.get().map(|e| view! {
                <p class="text-sm mt-3" style="color:var(--color-danger-text)">{e}</p>
            })}
            <div class="flex gap-2 justify-end mt-3">
                <Button variant=Variant::Ghost size=Size::Sm on:click=handle_cancel>
                    {move || t!(i18n, vault.cancel)}
                </Button>
                {move || {
                    let busy = submitting.get() || form_title.get().trim().is_empty();
                    view! {
                        <Button
                            variant=Variant::Primary
                            size=Size::Sm
                            disabled=busy
                            on:click=handle_save
                        >
                            {move || t!(i18n, vault.save)}
                        </Button>
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::to_login_payload;
    use vedge_ipc::{EntryTypeDto, PayloadDto};

    #[test]
    fn maps_login_fields() {
        let PayloadDto::Login(p) = to_login_payload(
            "GitHub".into(),
            "alice".into(),
            "s3cret".into(),
            "https://github.com".into(),
        ) else {
            panic!("expected Login variant");
        };
        assert_eq!(p.meta.name, "GitHub");
        assert_eq!(p.meta.entry_type, EntryTypeDto::Login);
        assert_eq!(p.meta.url.as_deref(), Some("https://github.com"));
        assert_eq!(p.username, "alice");
        assert_eq!(p.password, "s3cret");
        assert!(p.totp_secret.is_none());
        assert!(p.recovery_codes.is_empty());
        assert!(p.meta.tag_ids.is_empty());
        assert!(!p.meta.is_favorite);
    }

    #[test]
    fn empty_url_becomes_none() {
        let PayloadDto::Login(p) =
            to_login_payload("x".into(), "u".into(), "p".into(), String::new())
        else {
            panic!("expected Login variant");
        };
        assert!(p.meta.url.is_none());
    }
}
