//! Per-type entry form model + field renderer, shared by the create form and
//! the detail edit mode.
//!
//! [`EntryFormData`] is a plain, host-testable snapshot of every field the UI
//! can edit, plus *carry-through* values (recovery codes, tag/folder refs,
//! favicon, favorite flag, notes, and Document blob metadata) that aren't
//! surfaced as inputs but must survive an edit round-trip so `update_entry`
//! never clobbers them. [`EntryFormData::to_payload`] builds the wire
//! [`PayloadDto`]; [`EntryFormData::from_payload`] is its inverse (seeded from
//! a `get_entry` reveal). The [`EntryForm`] component renders the active type's
//! field set over a single `RwSignal<EntryFormData>`.

use crate::i18n::*;
use leptos::prelude::*;
use vedge_ipc::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, CommonMetaDto, EntryTypeDto, EnvVarDto,
    EnvVarsPayloadDto, FolderPayloadDto, IdentityPayloadDto, LoginPayloadDto, NotePayloadDto,
    PayloadDto, SshKeyPayloadDto,
};
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::components::form::date_picker::{
    DatePicker, DatePickerValue, DatePickerVariant, YearMonth,
};
use vedge_ui::components::form::textarea::Textarea;
use vedge_ui::primitives::tokens::{Size, Variant};

/// The eight entry types the picker offers (all but `Document`, which needs a
/// blob sidecar and is handled in slice 2.1.1).
#[must_use]
pub fn editable_types() -> [EntryTypeDto; 8] {
    [
        EntryTypeDto::Login,
        EntryTypeDto::Card,
        EntryTypeDto::SshKey,
        EntryTypeDto::ApiKey,
        EntryTypeDto::EnvVars,
        EntryTypeDto::Note,
        EntryTypeDto::Identity,
        EntryTypeDto::Folder,
    ]
}

/// Stable string key for an entry type, used as the `Select` option value.
#[must_use]
pub fn type_to_key(ty: &EntryTypeDto) -> &'static str {
    match ty {
        EntryTypeDto::Login => "Login",
        EntryTypeDto::Card => "Card",
        EntryTypeDto::SshKey => "SshKey",
        EntryTypeDto::ApiKey => "ApiKey",
        EntryTypeDto::EnvVars => "EnvVars",
        EntryTypeDto::Note => "Note",
        EntryTypeDto::Document => "Document",
        EntryTypeDto::Identity => "Identity",
        EntryTypeDto::Folder => "Folder",
        EntryTypeDto::Unknown(_) => "Unknown",
    }
}

/// Inverse of [`type_to_key`]. Unrecognized keys fall back to `Login`.
#[must_use]
pub fn type_from_key(key: &str) -> EntryTypeDto {
    match key {
        "Card" => EntryTypeDto::Card,
        "SshKey" => EntryTypeDto::SshKey,
        "ApiKey" => EntryTypeDto::ApiKey,
        "EnvVars" => EntryTypeDto::EnvVars,
        "Note" => EntryTypeDto::Note,
        "Document" => EntryTypeDto::Document,
        "Identity" => EntryTypeDto::Identity,
        "Folder" => EntryTypeDto::Folder,
        _ => EntryTypeDto::Login,
    }
}

/// Validation / build failure surfaced by [`EntryFormData::to_payload`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryFormError {
    /// The name field is blank.
    NameRequired,
    /// Card expiry month/year is not a valid number.
    InvalidExpiry,
    /// The active type can't be built here (Document → 2.1.1; Unknown).
    UnsupportedType,
}

/// A flat, host-testable snapshot of an entry form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFormData {
    pub entry_type: EntryTypeDto,

    // Common meta.
    pub name: String,
    pub url: String,

    // Login.
    pub username: String,
    pub password: String,
    pub totp: String,

    // Card.
    pub cardholder_name: String,
    pub card_number: String,
    /// Card expiry in `MM/YY` form (single field).
    pub card_expiry: String,
    pub cvv: String,
    pub pin: String,

    // SshKey.
    pub ssh_private_key: String,
    pub ssh_passphrase: String,
    pub ssh_public_key: String,
    pub ssh_fingerprint: String,
    pub ssh_key_type: String,

    // ApiKey.
    pub api_key: String,
    pub api_secret: String,
    pub api_endpoint: String,
    pub api_expiry: String,
    pub api_key_type: String,

    // EnvVars.
    pub env_vars: Vec<(String, String)>,

    // Note.
    pub note_content: String,

    // Identity.
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub addr_line1: String,
    pub addr_line2: String,
    pub addr_city: String,
    pub addr_state: String,
    pub addr_postal_code: String,
    pub addr_country: String,
    pub date_of_birth: String,
    pub national_id: String,

    // Carry-through (not UI-edited, preserved across an edit round-trip).
    pub recovery_codes: Vec<String>,
    pub favicon_url: Option<String>,
    pub tag_ids: Vec<String>,
    pub folder_id: Option<String>,
    pub is_favorite: bool,
    pub notes: Option<String>,

    // Document (read-only carry-through; not editable in this slice).
    pub doc_filename: String,
    pub doc_mime_type: String,
    pub doc_size_bytes: u64,
    pub doc_blob_nonce_b64: String,
}

impl EntryFormData {
    /// An empty form for `entry_type`.
    #[must_use]
    pub fn new(entry_type: EntryTypeDto) -> Self {
        Self {
            entry_type,
            name: String::new(),
            url: String::new(),
            username: String::new(),
            password: String::new(),
            totp: String::new(),
            cardholder_name: String::new(),
            card_number: String::new(),
            card_expiry: String::new(),
            cvv: String::new(),
            pin: String::new(),
            ssh_private_key: String::new(),
            ssh_passphrase: String::new(),
            ssh_public_key: String::new(),
            ssh_fingerprint: String::new(),
            ssh_key_type: String::new(),
            api_key: String::new(),
            api_secret: String::new(),
            api_endpoint: String::new(),
            api_expiry: String::new(),
            api_key_type: String::new(),
            env_vars: Vec::new(),
            note_content: String::new(),
            first_name: String::new(),
            last_name: String::new(),
            email: String::new(),
            phone: String::new(),
            addr_line1: String::new(),
            addr_line2: String::new(),
            addr_city: String::new(),
            addr_state: String::new(),
            addr_postal_code: String::new(),
            addr_country: String::new(),
            date_of_birth: String::new(),
            national_id: String::new(),
            recovery_codes: Vec::new(),
            favicon_url: None,
            tag_ids: Vec::new(),
            folder_id: None,
            is_favorite: false,
            notes: None,
            doc_filename: String::new(),
            doc_mime_type: String::new(),
            doc_size_bytes: 0,
            doc_blob_nonce_b64: String::new(),
        }
    }

    /// Copy the shared meta (name/url) into a fresh form of a new type — used
    /// when the create-form picker switches type so the user doesn't lose what
    /// they already typed.
    #[must_use]
    pub fn switch_type(&self, entry_type: EntryTypeDto) -> Self {
        let mut next = Self::new(entry_type);
        next.name = self.name.clone();
        next.url = self.url.clone();
        // Preserve the chosen folder across a type switch (the folder select is
        // shared across all types, so switching shouldn't discard it).
        next.folder_id = self.folder_id.clone();
        next
    }

    fn meta(&self) -> CommonMetaDto {
        CommonMetaDto {
            name: self.name.trim().to_owned(),
            entry_type: self.entry_type.clone(),
            url: non_empty(&self.url),
            favicon_url: self.favicon_url.clone(),
            tag_ids: self.tag_ids.clone(),
            folder_id: self.folder_id.clone(),
            is_favorite: self.is_favorite,
            notes: self.notes.clone(),
        }
    }

    /// Build the wire payload for the active type. Empty optional fields
    /// collapse to `None`; carry-through secrets are preserved.
    ///
    /// # Errors
    /// - [`EntryFormError::NameRequired`] if the name is blank.
    /// - [`EntryFormError::InvalidExpiry`] if a Card's expiry isn't numeric.
    /// - [`EntryFormError::UnsupportedType`] for Document / Unknown.
    pub fn to_payload(&self) -> Result<PayloadDto, EntryFormError> {
        if self.name.trim().is_empty() {
            return Err(EntryFormError::NameRequired);
        }
        let meta = self.meta();
        let payload = match self.entry_type {
            EntryTypeDto::Login => PayloadDto::Login(LoginPayloadDto {
                meta,
                username: self.username.clone(),
                password: self.password.clone(),
                totp_secret: non_empty(&self.totp),
                recovery_codes: self.recovery_codes.clone(),
            }),
            EntryTypeDto::Card => {
                let (expiry_month, expiry_year) = parse_card_expiry(&self.card_expiry)?;
                PayloadDto::Card(CardPayloadDto {
                    meta,
                    cardholder_name: self.cardholder_name.clone(),
                    number: self.card_number.clone(),
                    expiry_month,
                    expiry_year,
                    cvv: self.cvv.clone(),
                    pin: non_empty(&self.pin),
                })
            }
            EntryTypeDto::SshKey => PayloadDto::SshKey(SshKeyPayloadDto {
                meta,
                private_key_pem: self.ssh_private_key.clone(),
                passphrase: non_empty(&self.ssh_passphrase),
                public_key: self.ssh_public_key.clone(),
                fingerprint: self.ssh_fingerprint.clone(),
                key_type: self.ssh_key_type.clone(),
            }),
            EntryTypeDto::ApiKey => PayloadDto::ApiKey(ApiKeyPayloadDto {
                meta,
                key: self.api_key.clone(),
                secret: non_empty(&self.api_secret),
                endpoint: non_empty(&self.api_endpoint),
                expiry: non_empty(&self.api_expiry),
                key_type: non_empty(&self.api_key_type),
            }),
            EntryTypeDto::EnvVars => PayloadDto::EnvVars(EnvVarsPayloadDto {
                meta,
                vars: self
                    .env_vars
                    .iter()
                    .filter(|(k, _)| !k.trim().is_empty())
                    .map(|(k, v)| EnvVarDto {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect(),
            }),
            EntryTypeDto::Note => PayloadDto::Note(NotePayloadDto {
                meta,
                content: self.note_content.clone(),
            }),
            EntryTypeDto::Identity => PayloadDto::Identity(IdentityPayloadDto {
                meta,
                first_name: self.first_name.clone(),
                last_name: self.last_name.clone(),
                email: self.email.clone(),
                phone: non_empty(&self.phone),
                address: self.address(),
                date_of_birth: non_empty(&self.date_of_birth),
                national_id: non_empty(&self.national_id),
            }),
            EntryTypeDto::Folder => PayloadDto::Folder(FolderPayloadDto { meta }),
            EntryTypeDto::Document | EntryTypeDto::Unknown(_) => {
                return Err(EntryFormError::UnsupportedType);
            }
        };
        Ok(payload)
    }

    fn address(&self) -> Option<AddressDto> {
        let any = [
            &self.addr_line1,
            &self.addr_line2,
            &self.addr_city,
            &self.addr_state,
            &self.addr_postal_code,
            &self.addr_country,
        ]
        .iter()
        .any(|s| !s.is_empty());
        any.then(|| AddressDto {
            line1: self.addr_line1.clone(),
            line2: non_empty(&self.addr_line2),
            city: self.addr_city.clone(),
            state: non_empty(&self.addr_state),
            postal_code: self.addr_postal_code.clone(),
            country: self.addr_country.clone(),
        })
    }

    /// Seed a form from a revealed payload (secrets included).
    #[must_use]
    pub fn from_payload(payload: &PayloadDto) -> Self {
        let (entry_type, meta) = (payload_entry_type(payload), payload_meta(payload));
        let mut d = Self::new(entry_type);
        d.name = meta.name.clone();
        d.url = meta.url.clone().unwrap_or_default();
        d.favicon_url = meta.favicon_url.clone();
        d.tag_ids = meta.tag_ids.clone();
        d.folder_id = meta.folder_id.clone();
        d.is_favorite = meta.is_favorite;
        d.notes = meta.notes.clone();

        match payload {
            PayloadDto::Login(p) => {
                d.username = p.username.clone();
                d.password = p.password.clone();
                d.totp = p.totp_secret.clone().unwrap_or_default();
                d.recovery_codes = p.recovery_codes.clone();
            }
            PayloadDto::Card(p) => {
                d.cardholder_name = p.cardholder_name.clone();
                d.card_number = p.number.clone();
                d.card_expiry = format!("{:02}/{:02}", p.expiry_month, p.expiry_year % 100);
                d.cvv = p.cvv.clone();
                d.pin = p.pin.clone().unwrap_or_default();
            }
            PayloadDto::SshKey(p) => {
                d.ssh_private_key = p.private_key_pem.clone();
                d.ssh_passphrase = p.passphrase.clone().unwrap_or_default();
                d.ssh_public_key = p.public_key.clone();
                d.ssh_fingerprint = p.fingerprint.clone();
                d.ssh_key_type = p.key_type.clone();
            }
            PayloadDto::ApiKey(p) => {
                d.api_key = p.key.clone();
                d.api_secret = p.secret.clone().unwrap_or_default();
                d.api_endpoint = p.endpoint.clone().unwrap_or_default();
                d.api_expiry = p.expiry.clone().unwrap_or_default();
                d.api_key_type = p.key_type.clone().unwrap_or_default();
            }
            PayloadDto::EnvVars(p) => {
                d.env_vars = p
                    .vars
                    .iter()
                    .map(|v| (v.key.clone(), v.value.clone()))
                    .collect();
            }
            PayloadDto::Note(p) => {
                d.note_content = p.content.clone();
            }
            PayloadDto::Identity(p) => {
                d.first_name = p.first_name.clone();
                d.last_name = p.last_name.clone();
                d.email = p.email.clone();
                d.phone = p.phone.clone().unwrap_or_default();
                if let Some(a) = &p.address {
                    d.addr_line1 = a.line1.clone();
                    d.addr_line2 = a.line2.clone().unwrap_or_default();
                    d.addr_city = a.city.clone();
                    d.addr_state = a.state.clone().unwrap_or_default();
                    d.addr_postal_code = a.postal_code.clone();
                    d.addr_country = a.country.clone();
                }
                d.date_of_birth = p.date_of_birth.clone().unwrap_or_default();
                d.national_id = p.national_id.clone().unwrap_or_default();
            }
            PayloadDto::Document(p) => {
                d.doc_filename = p.filename.clone();
                d.doc_mime_type = p.mime_type.clone();
                d.doc_size_bytes = p.size_bytes;
                d.doc_blob_nonce_b64 = p.blob_nonce_b64.clone();
            }
            PayloadDto::Folder(_) => {}
        }
        d
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    }
}

/// Parse a `MM/YY` (or `MM/YYYY`) card expiry into `(month, year)`. A 2-digit
/// year is read as `20YY`. Rejects a missing `/`, non-numeric parts, or a month
/// outside 1..=12.
fn parse_card_expiry(s: &str) -> Result<(u8, u16), EntryFormError> {
    let (m, y) = s.split_once('/').ok_or(EntryFormError::InvalidExpiry)?;
    let month: u8 = m
        .trim()
        .parse()
        .map_err(|_| EntryFormError::InvalidExpiry)?;
    let y = y.trim();
    let raw_year: u16 = y.parse().map_err(|_| EntryFormError::InvalidExpiry)?;
    let year = if y.len() <= 2 {
        2000 + raw_year
    } else {
        raw_year
    };
    if !(1..=12).contains(&month) {
        return Err(EntryFormError::InvalidExpiry);
    }
    Ok((month, year))
}

/// `card_expiry` string ("MM/YY") → the month-variant `DatePicker` value.
/// A blank/invalid string is an empty month selection.
fn expiry_to_picker(s: &str) -> DatePickerValue {
    match parse_card_expiry(s) {
        Ok((month, year)) => {
            DatePickerValue::Month(Some(YearMonth::new(year.into(), month.into())))
        }
        Err(_) => DatePickerValue::Month(None),
    }
}

/// The month-variant `DatePicker` value → a canonical `MM/YY` string.
fn picker_to_expiry(v: DatePickerValue) -> String {
    match v.as_month() {
        Some(ym) => format!("{:02}/{:02}", ym.month, ym.year.rem_euclid(100)),
        None => String::new(),
    }
}

fn payload_entry_type(p: &PayloadDto) -> EntryTypeDto {
    match p {
        PayloadDto::Login(_) => EntryTypeDto::Login,
        PayloadDto::Card(_) => EntryTypeDto::Card,
        PayloadDto::SshKey(_) => EntryTypeDto::SshKey,
        PayloadDto::ApiKey(_) => EntryTypeDto::ApiKey,
        PayloadDto::EnvVars(_) => EntryTypeDto::EnvVars,
        PayloadDto::Note(_) => EntryTypeDto::Note,
        PayloadDto::Document(_) => EntryTypeDto::Document,
        PayloadDto::Identity(_) => EntryTypeDto::Identity,
        PayloadDto::Folder(_) => EntryTypeDto::Folder,
    }
}

fn payload_meta(p: &PayloadDto) -> &CommonMetaDto {
    match p {
        PayloadDto::Login(x) => &x.meta,
        PayloadDto::Card(x) => &x.meta,
        PayloadDto::SshKey(x) => &x.meta,
        PayloadDto::ApiKey(x) => &x.meta,
        PayloadDto::EnvVars(x) => &x.meta,
        PayloadDto::Note(x) => &x.meta,
        PayloadDto::Document(x) => &x.meta,
        PayloadDto::Identity(x) => &x.meta,
        PayloadDto::Folder(x) => &x.meta,
    }
}

// --- field-rendering macros (keep the per-type arms DRY) ---------------------

/// A plain text input bound to a `String` field of the form data.
macro_rules! text_field {
    ($data:expr, $i18n:expr, $id:literal, $field:ident, $key:ident) => {{
        let data = $data;
        let i18n = $i18n;
        view! {
            <Input
                id=$id
                placeholder=Signal::derive(move || t_string!(i18n, vault.$key).to_string())
                value=Signal::derive(move || data.with(|d| d.$field.clone()))
                on_input=Callback::new(move |v: String| data.update(|d| d.$field = v))
            />
        }
    }};
}

/// A masked (password-style) input with a reveal/hide toggle.
macro_rules! secret_field {
    ($data:expr, $i18n:expr, $id:literal, $field:ident, $key:ident) => {{
        let data = $data;
        let i18n = $i18n;
        view! {
            <Input
                id=$id
                input_type="password"
                placeholder=Signal::derive(move || t_string!(i18n, vault.$key).to_string())
                value=Signal::derive(move || data.with(|d| d.$field.clone()))
                on_input=Callback::new(move |v: String| data.update(|d| d.$field = v))
                reveal_label=Signal::derive(move || t_string!(i18n, vault.reveal).to_string())
                hide_label=Signal::derive(move || t_string!(i18n, vault.hide).to_string())
            />
        }
    }};
}

/// Renders the field set for the current entry type over a single
/// `RwSignal<EntryFormData>`. The structural branch is driven by a `Memo` on
/// `entry_type` so typing in a field never rebuilds the input tree (which would
/// drop focus).
#[component]
pub fn EntryForm(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let active_type = Memo::new(move |_| data.with(|d| d.entry_type.clone()));

    view! {
        <div class="grid grid-cols-2 gap-3">
            {text_field!(data, i18n, "ef-name", name, form_title)}
            {text_field!(data, i18n, "ef-url", url, form_url)}
            {move || match active_type.get() {
                EntryTypeDto::Login => view! {
                    {text_field!(data, i18n, "ef-username", username, form_identifier)}
                    {secret_field!(data, i18n, "ef-password", password, form_password)}
                    {secret_field!(data, i18n, "ef-totp", totp, field_totp)}
                }.into_any(),
                EntryTypeDto::Card => view! {
                    {text_field!(data, i18n, "ef-cardholder", cardholder_name, field_cardholder)}
                    {secret_field!(data, i18n, "ef-number", card_number, field_card_number)}
                    <DatePicker
                        id="ef-expiry"
                        variant=DatePickerVariant::Month
                        month_numeric=true
                        value=Signal::derive(move || expiry_to_picker(&data.with(|d| d.card_expiry.clone())))
                        on_change=Callback::new(move |v: DatePickerValue| data.update(|d| d.card_expiry = picker_to_expiry(v)))
                        placeholder=Signal::derive(move || t_string!(i18n, vault.field_card_expiry).to_string())
                    />
                    {secret_field!(data, i18n, "ef-cvv", cvv, field_cvv)}
                    {secret_field!(data, i18n, "ef-pin", pin, field_pin)}
                }.into_any(),
                EntryTypeDto::SshKey => view! {
                    {secret_field!(data, i18n, "ef-ssh-priv", ssh_private_key, field_private_key)}
                    {secret_field!(data, i18n, "ef-ssh-pass", ssh_passphrase, field_passphrase)}
                    {text_field!(data, i18n, "ef-ssh-pub", ssh_public_key, field_public_key)}
                    {text_field!(data, i18n, "ef-ssh-fp", ssh_fingerprint, field_fingerprint)}
                    {text_field!(data, i18n, "ef-ssh-kt", ssh_key_type, field_key_type)}
                }.into_any(),
                EntryTypeDto::ApiKey => view! {
                    {secret_field!(data, i18n, "ef-api-key", api_key, field_api_key)}
                    {secret_field!(data, i18n, "ef-api-secret", api_secret, field_api_secret)}
                    {text_field!(data, i18n, "ef-api-endpoint", api_endpoint, field_endpoint)}
                    {text_field!(data, i18n, "ef-api-expiry", api_expiry, field_expiry)}
                    {text_field!(data, i18n, "ef-api-kt", api_key_type, field_key_type)}
                }.into_any(),
                EntryTypeDto::EnvVars => view! {
                    <div class="col-span-2">
                        <EnvVarsFields data=data />
                    </div>
                }.into_any(),
                EntryTypeDto::Note => view! {
                    <div class="col-span-2">
                        <Textarea
                            id="ef-note"
                            rows=6
                            placeholder=Signal::derive(move || t_string!(i18n, vault.field_content).to_string())
                            value=Signal::derive(move || data.with(|d| d.note_content.clone()))
                            on_change=Callback::new(move |v: String| data.update(|d| d.note_content = v))
                        />
                    </div>
                }.into_any(),
                EntryTypeDto::Identity => view! {
                    {text_field!(data, i18n, "ef-first", first_name, field_first_name)}
                    {text_field!(data, i18n, "ef-last", last_name, field_last_name)}
                    {text_field!(data, i18n, "ef-email", email, field_email)}
                    {text_field!(data, i18n, "ef-phone", phone, field_phone)}
                    {text_field!(data, i18n, "ef-addr1", addr_line1, field_address_line1)}
                    {text_field!(data, i18n, "ef-addr2", addr_line2, field_address_line2)}
                    {text_field!(data, i18n, "ef-city", addr_city, field_city)}
                    {text_field!(data, i18n, "ef-state", addr_state, field_state)}
                    {text_field!(data, i18n, "ef-postal", addr_postal_code, field_postal_code)}
                    {text_field!(data, i18n, "ef-country", addr_country, field_country)}
                    {text_field!(data, i18n, "ef-dob", date_of_birth, field_date_of_birth)}
                    {secret_field!(data, i18n, "ef-natid", national_id, field_national_id)}
                }.into_any(),
                EntryTypeDto::Folder => ().into_any(),
                EntryTypeDto::Document | EntryTypeDto::Unknown(_) => view! {
                    <p class="col-span-2 text-sm text-foreground/50">
                        {move || t!(i18n, vault.doc_not_editable)}
                    </p>
                }.into_any(),
            }}
        </div>
    }
}

/// Editable key/value rows for an `EnvVars` entry. The structural `For` is
/// driven by a `Memo` on the row count so per-field typing doesn't rebuild it.
#[component]
fn EnvVarsFields(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let count = Memo::new(move |_| data.with(|d| d.env_vars.len()));

    let add_row = move |_: web_sys::MouseEvent| {
        data.update(|d| d.env_vars.push((String::new(), String::new())));
    };

    view! {
        <div class="flex flex-col gap-2">
            <For each=move || 0..count.get() key=|i| *i let:i>
                <div class="flex gap-2 items-center">
                    <Input
                        id="ef-env-key"
                        placeholder=Signal::derive(move || t_string!(i18n, vault.field_env_key).to_string())
                        value=Signal::derive(move || data.with(|d| d.env_vars.get(i).map(|p| p.0.clone()).unwrap_or_default()))
                        on_input=Callback::new(move |v: String| data.update(|d| {
                            if let Some(p) = d.env_vars.get_mut(i) { p.0 = v; }
                        }))
                        class="flex-1"
                    />
                    <Input
                        id="ef-env-val"
                        input_type="password"
                        placeholder=Signal::derive(move || t_string!(i18n, vault.field_env_value).to_string())
                        value=Signal::derive(move || data.with(|d| d.env_vars.get(i).map(|p| p.1.clone()).unwrap_or_default()))
                        on_input=Callback::new(move |v: String| data.update(|d| {
                            if let Some(p) = d.env_vars.get_mut(i) { p.1 = v; }
                        }))
                        reveal_label=Signal::derive(move || t_string!(i18n, vault.reveal).to_string())
                        hide_label=Signal::derive(move || t_string!(i18n, vault.hide).to_string())
                        class="flex-1"
                    />
                    <Button
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |_: web_sys::MouseEvent| data.update(|d| {
                            if i < d.env_vars.len() { d.env_vars.remove(i); }
                        })
                    >
                        {move || t!(i18n, vault.remove_var)}
                    </Button>
                </div>
            </For>
            <div>
                <Button variant=Variant::Secondary size=Size::Sm on:click=add_row>
                    {move || t!(i18n, vault.add_var)}
                </Button>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryFormData, EntryFormError};
    use vedge_ipc::{EntryTypeDto, PayloadDto};

    fn base(ty: EntryTypeDto) -> EntryFormData {
        let mut d = EntryFormData::new(ty);
        d.name = "Item".into();
        d
    }

    #[test]
    fn to_payload_requires_name() {
        let d = EntryFormData::new(EntryTypeDto::Login);
        assert_eq!(d.to_payload().unwrap_err(), EntryFormError::NameRequired);
    }

    #[test]
    fn to_payload_login_empty_optionals_are_none() {
        let mut d = base(EntryTypeDto::Login);
        d.username = "alice".into();
        d.password = "pw".into();
        let PayloadDto::Login(p) = d.to_payload().unwrap() else {
            panic!("login");
        };
        assert_eq!(p.username, "alice");
        assert!(p.totp_secret.is_none());
        assert!(p.recovery_codes.is_empty());
    }

    #[test]
    fn to_payload_card_parses_expiry() {
        let mut d = base(EntryTypeDto::Card);
        d.card_number = "4111".into();
        d.card_expiry = "12/30".into();
        d.cvv = "123".into();
        let PayloadDto::Card(p) = d.to_payload().unwrap() else {
            panic!("card");
        };
        assert_eq!(p.expiry_month, 12);
        assert_eq!(p.expiry_year, 2030);
        assert!(p.pin.is_none());
    }

    #[test]
    fn to_payload_card_bad_expiry_errors() {
        let mut d = base(EntryTypeDto::Card);
        d.card_expiry = "13/30".into(); // month out of range
        assert_eq!(d.to_payload().unwrap_err(), EntryFormError::InvalidExpiry);
    }

    #[test]
    fn to_payload_document_unsupported() {
        let d = base(EntryTypeDto::Document);
        assert_eq!(d.to_payload().unwrap_err(), EntryFormError::UnsupportedType);
    }

    #[test]
    fn to_payload_env_vars_drops_blank_keys() {
        let mut d = base(EntryTypeDto::EnvVars);
        d.env_vars = vec![("DB".into(), "x".into()), ("  ".into(), "orphan".into())];
        let PayloadDto::EnvVars(p) = d.to_payload().unwrap() else {
            panic!("env");
        };
        assert_eq!(p.vars.len(), 1);
        assert_eq!(p.vars[0].key, "DB");
    }

    #[test]
    fn from_payload_roundtrip_login() {
        let mut d = base(EntryTypeDto::Login);
        d.url = "https://example.com".into();
        d.username = "alice".into();
        d.password = "s3cret".into();
        d.totp = "JBSWY3DPEHPK3PXP".into();
        d.recovery_codes = vec!["r1".into()];
        let round = EntryFormData::from_payload(&d.to_payload().unwrap());
        assert_eq!(round, d);
    }

    #[test]
    fn from_payload_roundtrip_identity_with_address() {
        let mut d = base(EntryTypeDto::Identity);
        d.first_name = "Alice".into();
        d.last_name = "A".into();
        d.email = "a@b.co".into();
        d.addr_line1 = "1 St".into();
        d.addr_city = "Town".into();
        d.addr_postal_code = "90001".into();
        d.addr_country = "US".into();
        d.national_id = "ID-1".into();
        let round = EntryFormData::from_payload(&d.to_payload().unwrap());
        assert_eq!(round, d);
    }

    #[test]
    fn from_payload_roundtrip_card() {
        let mut d = base(EntryTypeDto::Card);
        d.cardholder_name = "Alice A".into();
        d.card_number = "4111111111111111".into();
        d.card_expiry = "12/30".into();
        d.cvv = "123".into();
        d.pin = "4321".into();
        let round = EntryFormData::from_payload(&d.to_payload().unwrap());
        assert_eq!(round, d);
    }
}
