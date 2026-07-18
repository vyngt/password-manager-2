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

use crate::api;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_generator::{RandomConfig, generate_random};
use vedge_ipc::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, CommonMetaDto, EntryTypeDto, EnvVarUpdateDto,
    EnvVarsPayloadDto, FolderPayloadDto, IdentityPayloadDto, LoginPayloadDto, NotePayloadDto,
    PayloadDto, SecretListUpdateDto, SecretUpdateDto, SshKeyPayloadDto, TotpAlgorithmDto,
    TotpUpdateDto,
};
use vedge_ui::components::Button;
use vedge_ui::components::IconButton;
use vedge_ui::components::Input;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::form::date_picker::{
    DatePicker, DatePickerValue, DatePickerVariant, YearMonth,
};
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::form::textarea::Textarea;
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

use crate::features::date_i18n::{calendar_labels, locale_tag};
use crate::features::generator::generator_panel::GeneratorPanel;
use crate::features::settings::generator_prefs::GeneratorPrefsCtx;

/// The entry types the create picker offers. Excludes `Document` (needs a blob
/// sidecar — slice 2.1.1) and `Folder` (created from the folder tree's "+", not
/// this form — slice 2.5).
#[must_use]
pub fn editable_types() -> [EntryTypeDto; 7] {
    [
        EntryTypeDto::Login,
        EntryTypeDto::Card,
        EntryTypeDto::SshKey,
        EntryTypeDto::ApiKey,
        EntryTypeDto::EnvVars,
        EntryTypeDto::Note,
        EntryTypeDto::Identity,
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
// A flat form snapshot legitimately carries many independent bool toggles
// (editing + per-optional-secret presence flags + favorite/totp); grouping them
// into sub-structs would obscure, not clarify, this DTO-like model.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFormData {
    pub entry_type: EntryTypeDto,

    /// True when this form was seeded from an EXISTING entry (`from_payload`),
    /// false for a fresh create (`new`). Sealed secrets (slice 5.4) no longer
    /// cross on `get_entry`, so an editing form starts with EMPTY secret fields;
    /// `editing` tells `to_payload` that an untouched (empty) required field means
    /// "keep the stored secret" (`Unchanged`) rather than "set it empty".
    pub editing: bool,

    // Common meta.
    pub name: String,
    pub url: String,

    // Login.
    pub username: String,
    /// Sealed secret. Empty on an editing form until the user types (Set) or
    /// clicks Reveal (fills it → Set of the same value, harmless).
    pub password: String,
    // TOTP (slice 4.2 door): the seed never round-trips through the form. The
    // form tracks the *presence* + params + an enrolment *intent*; the stored
    // seed is preserved server-side when the intent is `Unchanged`.
    pub has_totp: bool,
    pub totp_intent: TotpUpdateDto,
    pub totp_algorithm: TotpAlgorithmDto,
    pub totp_digits: u8,
    pub totp_period: u32,

    // Card.
    pub cardholder_name: String,
    pub card_number: String,
    /// Card expiry in `MM/YY` form (single field).
    pub card_expiry: String,
    pub cvv: String,
    pub pin: String,
    /// Optional-secret presence: a stored pin exists (drives the placeholder +
    /// Reveal button). `false` on create and when the entry has no pin.
    pub has_pin: bool,

    // SshKey.
    pub ssh_private_key: String,
    pub ssh_passphrase: String,
    pub has_passphrase: bool,
    pub ssh_public_key: String,
    pub ssh_fingerprint: String,
    pub ssh_key_type: String,

    // ApiKey.
    pub api_key: String,
    pub api_secret: String,
    pub has_secret: bool,
    pub api_endpoint: String,
    pub api_expiry: String,
    pub api_key_type: String,

    // EnvVars. `(key, value, had_stored_value)` — after sealing, an editing form
    // loads each row's key with an EMPTY value + `had_stored_value = true`, so an
    // untouched value carries forward (`Unchanged`). New rows start `false`.
    pub env_vars: Vec<(String, String, bool)>,

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
    pub has_national_id: bool,

    // Carry-through. Recovery codes have NO editor and (after sealing) their
    // values never cross — so the form only knows the COUNT and always sends
    // `Unchanged`; `update_entry` carries the stored codes forward.
    pub recovery_codes_count: u32,
    pub favicon_url: Option<String>,
    pub tag_ids: Vec<String>,
    pub folder_id: Option<String>,
    pub is_favorite: bool,
    pub notes: Option<String>,
    /// Folder presentation: a CSS color string (set from the tree's Customize
    /// dialog). Carried through every round-trip so a rename doesn't drop it.
    pub color: Option<String>,
    /// Folder presentation: a curated icon key. Same carry-through rationale.
    pub icon: Option<String>,
    /// Manual ordering position within the folder (set by drag-reorder).
    pub sort_order: u32,

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
            editing: false,
            name: String::new(),
            url: String::new(),
            username: String::new(),
            password: String::new(),
            has_totp: false,
            totp_intent: TotpUpdateDto::Unchanged,
            totp_algorithm: TotpAlgorithmDto::Sha1,
            totp_digits: 6,
            totp_period: 30,
            cardholder_name: String::new(),
            card_number: String::new(),
            card_expiry: String::new(),
            cvv: String::new(),
            pin: String::new(),
            has_pin: false,
            ssh_private_key: String::new(),
            ssh_passphrase: String::new(),
            has_passphrase: false,
            ssh_public_key: String::new(),
            ssh_fingerprint: String::new(),
            ssh_key_type: String::new(),
            api_key: String::new(),
            api_secret: String::new(),
            has_secret: false,
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
            has_national_id: false,
            recovery_codes_count: 0,
            favicon_url: None,
            tag_ids: Vec::new(),
            folder_id: None,
            is_favorite: false,
            notes: None,
            color: None,
            icon: None,
            sort_order: 0,
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
        // Folder presentation is likewise type-agnostic carry-through.
        next.color = self.color.clone();
        next.icon = self.icon.clone();
        next.sort_order = self.sort_order;
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
            color: self.color.clone(),
            icon: self.icon.clone(),
            sort_order: self.sort_order,
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
        let editing = self.editing;
        let payload = match self.entry_type {
            EntryTypeDto::Login => PayloadDto::Login(LoginPayloadDto {
                meta,
                username: self.username.clone(),
                password: required_intent(&self.password, editing),
                has_totp: self.has_totp,
                totp_algorithm: self.totp_algorithm,
                totp_digits: self.totp_digits,
                totp_period: self.totp_period,
                totp: self.totp_intent.clone(),
                // No editor for recovery codes: always carry the stored list forward.
                recovery_codes: SecretListUpdateDto::Unchanged,
                recovery_codes_count: self.recovery_codes_count,
            }),
            EntryTypeDto::Card => {
                let (expiry_month, expiry_year) = parse_card_expiry(&self.card_expiry)?;
                PayloadDto::Card(CardPayloadDto {
                    meta,
                    cardholder_name: self.cardholder_name.clone(),
                    number: required_intent(&self.card_number, editing),
                    expiry_month,
                    expiry_year,
                    cvv: required_intent(&self.cvv, editing),
                    pin: optional_intent(&self.pin, self.has_pin),
                    has_pin: self.has_pin,
                })
            }
            EntryTypeDto::SshKey => PayloadDto::SshKey(SshKeyPayloadDto {
                meta,
                private_key_pem: required_intent(&self.ssh_private_key, editing),
                passphrase: optional_intent(&self.ssh_passphrase, self.has_passphrase),
                has_passphrase: self.has_passphrase,
                public_key: self.ssh_public_key.clone(),
                fingerprint: self.ssh_fingerprint.clone(),
                key_type: self.ssh_key_type.clone(),
            }),
            EntryTypeDto::ApiKey => PayloadDto::ApiKey(ApiKeyPayloadDto {
                meta,
                key: required_intent(&self.api_key, editing),
                secret: optional_intent(&self.api_secret, self.has_secret),
                has_secret: self.has_secret,
                endpoint: non_empty(&self.api_endpoint),
                expiry: non_empty(&self.api_expiry),
                key_type: non_empty(&self.api_key_type),
            }),
            EntryTypeDto::EnvVars => PayloadDto::EnvVars(EnvVarsPayloadDto {
                meta,
                vars: self
                    .env_vars
                    .iter()
                    .filter(|(k, _, _)| !k.trim().is_empty())
                    .map(|(k, v, had_value)| EnvVarUpdateDto {
                        key: k.clone(),
                        value: env_value_intent(v, *had_value),
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
                national_id: optional_intent(&self.national_id, self.has_national_id),
                has_national_id: self.has_national_id,
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
        d.editing = true;
        d.name = meta.name.clone();
        d.url = meta.url.clone().unwrap_or_default();
        d.favicon_url = meta.favicon_url.clone();
        d.tag_ids = meta.tag_ids.clone();
        d.folder_id = meta.folder_id.clone();
        d.is_favorite = meta.is_favorite;
        d.notes = meta.notes.clone();
        d.color = meta.color.clone();
        d.icon = meta.icon.clone();
        d.sort_order = meta.sort_order;

        // Sealed secrets (slice 5.4) are ABSENT from the DTO — the form starts with
        // empty secret fields + presence flags; an untouched field carries the
        // stored secret forward via `Unchanged` in `to_payload`.
        match payload {
            PayloadDto::Login(p) => {
                d.username = p.username.clone();
                // password stays empty (sealed).
                d.has_totp = p.has_totp;
                d.totp_algorithm = p.totp_algorithm;
                d.totp_digits = p.totp_digits;
                d.totp_period = p.totp_period;
                d.totp_intent = TotpUpdateDto::Unchanged;
                d.recovery_codes_count = p.recovery_codes_count;
            }
            PayloadDto::Card(p) => {
                d.cardholder_name = p.cardholder_name.clone();
                d.card_expiry = format!("{:02}/{:02}", p.expiry_month, p.expiry_year % 100);
                // number / cvv stay empty (sealed, required); pin sealed (optional).
                d.has_pin = p.has_pin;
            }
            PayloadDto::SshKey(p) => {
                // private_key / passphrase stay empty (sealed).
                d.has_passphrase = p.has_passphrase;
                d.ssh_public_key = p.public_key.clone();
                d.ssh_fingerprint = p.fingerprint.clone();
                d.ssh_key_type = p.key_type.clone();
            }
            PayloadDto::ApiKey(p) => {
                // key / secret stay empty (sealed).
                d.has_secret = p.has_secret;
                d.api_endpoint = p.endpoint.clone().unwrap_or_default();
                d.api_expiry = p.expiry.clone().unwrap_or_default();
                d.api_key_type = p.key_type.clone().unwrap_or_default();
            }
            PayloadDto::EnvVars(p) => {
                // Keys cross (the schema); values are sealed → empty + had_value.
                d.env_vars = p
                    .vars
                    .iter()
                    .map(|v| (v.key.clone(), String::new(), true))
                    .collect();
            }
            PayloadDto::Note(p) => {
                // Note.content is NOT sealed — it crosses.
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
                // national_id stays empty (sealed).
                d.has_national_id = p.has_national_id;
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

/// Intent for a REQUIRED sealed field (password, card number/cvv, ssh private
/// key, api key). A non-empty value is a `Set`; an empty field on an EDITING
/// form means "keep the stored secret" (`Unchanged`) — the safe failure mode
/// (slice 5.4). On create (`!editing`) an empty required field is `Set("")` (the
/// form's own required-field validation guards genuinely-empty submissions).
fn required_intent(value: &str, editing: bool) -> SecretUpdateDto {
    if value.is_empty() && editing {
        SecretUpdateDto::Unchanged
    } else {
        SecretUpdateDto::Set(value.to_owned())
    }
}

/// Intent for an OPTIONAL sealed field (pin, passphrase, api secret,
/// `national_id`). Non-empty → `Set`; empty with a stored value → keep it
/// (`Unchanged`); empty with none stored → `Clear` (⇒ `None`).
fn optional_intent(value: &str, has_stored: bool) -> SecretUpdateDto {
    if !value.is_empty() {
        SecretUpdateDto::Set(value.to_owned())
    } else if has_stored {
        SecretUpdateDto::Unchanged
    } else {
        SecretUpdateDto::Clear
    }
}

/// Intent for one env-var row's value. Non-empty → `Set`; empty with a stored
/// value (an untouched existing row) → `Unchanged`; empty new row → `Set("")`.
/// (Renaming a key requires re-entering its value — the sealed value can't
/// follow a rename, or the resolver would reject the now-unknown key.)
fn env_value_intent(value: &str, had_value: bool) -> SecretUpdateDto {
    if value.is_empty() && had_value {
        SecretUpdateDto::Unchanged
    } else {
        SecretUpdateDto::Set(value.to_owned())
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

/// A plain text input bound to a `String` field of the form data, wrapped in a
/// `FormField` so it has a visible `<label for=id>` (slice 5.4.1 ⑤ — the label
/// used to be the placeholder). No placeholder: the label carries the name.
macro_rules! text_field {
    ($data:expr, $i18n:expr, $id:literal, $field:ident, $key:ident) => {{
        let data = $data;
        let i18n = $i18n;
        view! {
            <FormField id=$id label=Signal::derive(move || t_string!(i18n, vault.$key).to_owned())>
                <Input
                    id=$id
                    value=Signal::derive(move || data.with(|d| d.$field.clone()))
                    on_input=Callback::new(move |v: String| data.update(|d| d.$field = v))
                />
            </FormField>
        }
    }};
}

/// A masked (password-style) input with a reveal/hide mask toggle, wrapped in a
/// `FormField` (⑤). When EDITING, the field starts empty (the secret is sealed —
/// slice 5.4), so the placeholder says "Unchanged — type to replace"; on create
/// there is no placeholder (the label carries the name). The edit form is inputs
/// only — reading a stored secret (reveal-to-view / copy) lives in the read
/// drawer, so there is no per-field Reveal button here.
macro_rules! secret_field {
    ($data:expr, $i18n:expr, $id:literal, $field:ident, $key:ident) => {{
        let data = $data;
        let i18n = $i18n;
        view! {
            <FormField id=$id label=Signal::derive(move || t_string!(i18n, vault.$key).to_owned())>
                <Input
                    id=$id
                    input_type="password"
                    placeholder=Signal::derive(move || {
                        if data.with(|d| d.editing) {
                            t_string!(i18n, vault.form_sealed_placeholder).to_owned()
                        } else {
                            String::new()
                        }
                    })
                    value=Signal::derive(move || data.with(|d| d.$field.clone()))
                    on_input=Callback::new(move |v: String| data.update(|d| d.$field = v))
                    reveal_label=Signal::derive(move || t_string!(i18n, vault.reveal).to_owned())
                    hide_label=Signal::derive(move || t_string!(i18n, vault.hide).to_owned())
                />
            </FormField>
        }
    }};
}

/// TOTP enrolment field (slice 4.2 door). The seed never round-trips through the
/// form — it tracks presence + params + an intent (`Unchanged`/`Set`/`Clear`).
/// Pasting an `otpauth://` URI or bare Base32 secret parses (server-side,
/// stateless) into `Set` + params; "Remove" sets `Clear`; an untouched field
/// stays `Unchanged`, so `update_entry` preserves the stored seed.
#[component]
fn TotpEnrolField(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let paste = RwSignal::new(String::new());
    let parse_error = RwSignal::new(false);

    let status = move || {
        data.with(|d| match &d.totp_intent {
            TotpUpdateDto::Set(_) => t_string!(i18n, vault.totp_will_set).to_owned(),
            TotpUpdateDto::Clear => t_string!(i18n, vault.totp_will_remove).to_owned(),
            TotpUpdateDto::Unchanged => {
                if d.has_totp {
                    t_string!(i18n, vault.totp_configured).to_owned()
                } else {
                    t_string!(i18n, vault.totp_none).to_owned()
                }
            }
        })
    };

    // Parse is stateless (no session) — the raw string already came from WASM.
    let do_parse = move || {
        let raw = paste.get_untracked();
        if raw.trim().is_empty() {
            return;
        }
        parse_error.set(false);
        spawn_local(async move {
            match api::totp::parse_totp_enrolment(&raw).await {
                Ok(e) => {
                    data.update(|d| {
                        d.totp_intent = TotpUpdateDto::Set(e.secret);
                        d.totp_algorithm = e.algorithm;
                        d.totp_digits = e.digits;
                        d.totp_period = e.period;
                        d.has_totp = true;
                    });
                    paste.set(String::new());
                }
                Err(_) => parse_error.set(true),
            }
        });
    };

    let remove =
        move |_: web_sys::MouseEvent| data.update(|d| d.totp_intent = TotpUpdateDto::Clear);
    let undo =
        move |_: web_sys::MouseEvent| data.update(|d| d.totp_intent = TotpUpdateDto::Unchanged);

    view! {
        <div class="col-span-2 space-y-1">
            <div class="flex items-center gap-3">
                <label class="text-xs text-foreground/60">
                    {move || t!(i18n, vault.field_totp)}
                </label>
                <span class="text-xs text-text-secondary" data-testid="ef-totp-status">
                    {status}
                </span>
                {move || {
                    data.with(|d| d.has_totp && matches!(d.totp_intent, TotpUpdateDto::Unchanged))
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    class="text-xs text-danger hover:underline"
                                    on:click=remove
                                >
                                    {move || t!(i18n, vault.totp_remove)}
                                </button>
                            }
                        })
                }}
                {move || {
                    data.with(|d| !matches!(d.totp_intent, TotpUpdateDto::Unchanged))
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    class="text-xs text-text-tertiary hover:underline"
                                    on:click=undo
                                >
                                    {move || t!(i18n, vault.totp_undo)}
                                </button>
                            }
                        })
                }}
            </div>
            <div class="flex items-center gap-2">
                <div class="flex-1">
                    <Input
                        id="ef-totp"
                        placeholder=Signal::derive(move || {
                            t_string!(i18n, vault.totp_paste_placeholder).to_owned()
                        })
                        value=Signal::derive(move || paste.get())
                        on_input=Callback::new(move |v: String| paste.set(v))
                    />
                </div>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    attr:data-testid="ef-totp-apply"
                    on:click=move |_: web_sys::MouseEvent| do_parse()
                >
                    {move || t!(i18n, vault.totp_apply)}
                </Button>
            </div>
            {move || {
                parse_error
                    .get()
                    .then(|| {
                        view! {
                            <p class="text-xs text-danger">
                                {move || t!(i18n, vault.totp_parse_error)}
                            </p>
                        }
                    })
            }}
        </div>
    }
}

/// The SSH private key (PEM) field: a roomy `col-span-2` multi-line monospace
/// textarea with a reveal toggle — unlike the single-line masked `secret_field!`,
/// a PEM block needs several lines. Masking is display-only
/// (`-webkit-text-security` via `.textarea--masked`). After sealing (slice 5.4)
/// the key no longer crosses on `get_entry`, so an editing form starts EMPTY with
/// the "unchanged — type to replace" placeholder; leaving it empty preserves the
/// stored key (`Unchanged`). (No Reveal button: the private key isn't a
/// `FieldSelector` field — deferred.)
#[component]
fn SshPrivateKeyField(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let (revealed, set_revealed) = signal(false);

    view! {
        <div class="col-span-2">
            <div class="flex items-center justify-between mb-1">
                <label for="ef-ssh-priv" class="text-xs text-foreground/60">
                    {move || t!(i18n, vault.field_private_key)}
                </label>
                // Mask toggle only — reading the stored key lives in the read
                // drawer (reveal-to-view), not the edit form (which is inputs only).
                <IconButton
                    variant=Variant::Ghost
                    size=Size::Sm
                    aria_label=Signal::derive(move || {
                        if revealed.get() {
                            t_string!(i18n, vault.hide).to_owned()
                        } else {
                            t_string!(i18n, vault.reveal).to_owned()
                        }
                    })
                    on:click=move |_| set_revealed.update(|r| *r = !*r)
                >
                    // No `attr:aria-hidden` on these two icons: it trips a Leptos
                    // RPIT capture bound at this `<Show>`-returned position, and the
                    // IconButton's `aria_label` already names the control (matches the
                    // design-system `Input` password toggle).
                    <Show
                        when=move || revealed.get()
                        fallback=|| view! { <Icon icon=i::FaEyeSolid /> }
                    >
                        <Icon icon=i::FaEyeSlashSolid />
                    </Show>
                </IconButton>
            </div>
            <Textarea
                id="ef-ssh-priv"
                rows=4
                max_rows=12
                class="font-jetbrains-mono"
                masked=Signal::derive(move || !revealed.get())
                placeholder=Signal::derive(move || {
                    if data.with(|d| d.editing) {
                        t_string!(i18n, vault.form_sealed_placeholder).to_owned()
                    } else {
                        t_string!(i18n, vault.field_private_key).to_owned()
                    }
                })
                value=Signal::derive(move || data.with(|d| d.ssh_private_key.clone()))
                on_change=Callback::new(move |v: String| data.update(|d| d.ssh_private_key = v))
            />
        </div>
    }
}

/// The Login password field with inline generate affordances (slice 3.3): the
/// masked `Input`, a one-click **Generate** wand that fills from the last-used
/// preset, and a caret that opens a [`Dialog`]-hosted [`GeneratorPanel`] to tune
/// + "Use". The buttons are siblings (the password `Input` renders its own eye
/// and ignores a trailing slot) — mirrors [`SshPrivateKeyField`]'s layout. The
/// generator is a **modal** (not an anchored popover) so it renders above the
/// Edit dialog it is opened from and its long body scrolls.
#[component]
fn LoginPasswordField(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let prefs = expect_context::<GeneratorPrefsCtx>().0;
    let generator_open = RwSignal::new(false);

    // One click, no dialog: draw from the shared preset and fill the field signal.
    // The engine's `Zeroizing<String>` is copied into the form's existing plain
    // `String` field (pre-existing Phase-2 model) — no new persistence/exposure.
    let quick_generate = Callback::new(move |()| {
        if let Ok(g) = generate_random(&RandomConfig::from(prefs.get_untracked())) {
            let pw = g.secret.as_str().to_owned();
            data.update(|d| d.password = pw);
        }
    });
    let on_use = Callback::new(move |secret: String| {
        data.update(|d| d.password = secret);
        generator_open.set(false);
    });

    view! {
        // `items-end`: `secret_field!` now renders a label row above the input
        // (⑤), so the sibling buttons must align to the input, not the block.
        <div class="col-span-2 flex items-end gap-2">
            <div class="flex-1">
                {secret_field!(data, i18n, "ef-password", password, form_password)}
            </div>
            <IconButton
                variant=Variant::Ghost
                size=Size::Sm
                attr:data-testid="login-generate-wand"
                on_click=quick_generate
                aria_label=Signal::derive(move || {
                    t_string!(i18n, vault.generate_password).to_owned()
                })
            >
                <span aria-hidden="true">
                    <Icon icon=i::FaWandMagicSparklesSolid />
                </span>
            </IconButton>
            <IconButton
                variant=Variant::Ghost
                size=Size::Sm
                attr:data-testid="login-generate-tune"
                on_click=Callback::new(move |()| generator_open.update(|o| *o = !*o))
                aria_label=Signal::derive(move || t_string!(i18n, vault.generate_tune).to_owned())
            >
                <span aria-hidden="true">
                    <Icon icon=i::FaAngleDownSolid />
                </span>
            </IconButton>
        </div>
        // A centered modal (not an anchored popover): it portals above the Edit
        // dialog it is opened from, and its `DialogBody` scrolls when the mode
        // picker + controls + bulk/recent sections exceed the viewport.
        <Dialog
            open=generator_open
            on_close=Callback::new(move |()| generator_open.set(false))
            size=DialogSize::Md
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.generate_tune)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <GeneratorPanel on_use=on_use />
            </DialogBody>
        </Dialog>
    }
}

/// Renders the field set for the current entry type over a single
/// `RwSignal<EntryFormData>`. The structural branch is driven by a `Memo` on
/// `entry_type` so typing in a field never rebuilds the input tree (which would
/// drop focus).
#[component]
pub fn EntryForm(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let active_type = Memo::new(move |_| data.with(|d| d.entry_type.clone()));
    // Localized calendar for the card-expiry month picker (slice 4.9a P1).
    let cal_locale = locale_tag(i18n);
    let cal_labels = calendar_labels(i18n);

    view! {
        <div class="grid grid-cols-2 gap-3">
            {text_field!(data, i18n, "ef-name", name, form_title)}
            {text_field!(data, i18n, "ef-url", url, form_url)}
            {move || match active_type.get() {
                EntryTypeDto::Login => {
                    view! {
                        {text_field!(data, i18n, "ef-username", username, form_identifier)}
                        <LoginPasswordField data=data />
                        <TotpEnrolField data=data />
                    }
                        .into_any()
                }
                EntryTypeDto::Card => {
                    view! {
                        {text_field!(
                            data, i18n, "ef-cardholder", cardholder_name, field_cardholder
                        )}
                        {secret_field!(data, i18n, "ef-number", card_number, field_card_number)}
                        <DatePicker
                            id="ef-expiry"
                            variant=DatePickerVariant::Month
                            month_numeric=true
                            locale=cal_locale
                            dialog_label=cal_labels.dialog
                            prev_month_label=cal_labels.prev_month
                            next_month_label=cal_labels.next_month
                            prev_year_label=cal_labels.prev_year
                            next_year_label=cal_labels.next_year
                            value=Signal::derive(move || expiry_to_picker(
                                &data.with(|d| d.card_expiry.clone()),
                            ))
                            on_change=Callback::new(move |v: DatePickerValue| {
                                data.update(|d| d.card_expiry = picker_to_expiry(v));
                            })
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, vault.field_card_expiry).to_owned()
                            })
                        />
                        {secret_field!(data, i18n, "ef-cvv", cvv, field_cvv)}
                        {secret_field!(data, i18n, "ef-pin", pin, field_pin)}
                    }
                        .into_any()
                }
                EntryTypeDto::SshKey => {
                    view! {
                        <SshPrivateKeyField data=data />
                        {secret_field!(data, i18n, "ef-ssh-pass", ssh_passphrase, field_passphrase)}
                        {text_field!(data, i18n, "ef-ssh-pub", ssh_public_key, field_public_key)}
                        {text_field!(data, i18n, "ef-ssh-fp", ssh_fingerprint, field_fingerprint)}
                        {text_field!(data, i18n, "ef-ssh-kt", ssh_key_type, field_key_type)}
                    }
                        .into_any()
                }
                EntryTypeDto::ApiKey => {
                    view! {
                        {secret_field!(data, i18n, "ef-api-key", api_key, field_api_key)}
                        {secret_field!(data, i18n, "ef-api-secret", api_secret, field_api_secret)}
                        {text_field!(data, i18n, "ef-api-endpoint", api_endpoint, field_endpoint)}
                        {text_field!(data, i18n, "ef-api-expiry", api_expiry, field_expiry)}
                        {text_field!(data, i18n, "ef-api-kt", api_key_type, field_key_type)}
                    }
                        .into_any()
                }
                EntryTypeDto::EnvVars => {
                    view! {
                        <div class="col-span-2">
                            <EnvVarsFields data=data />
                        </div>
                    }
                        .into_any()
                }
                EntryTypeDto::Note => {
                    view! {
                        <div class="col-span-2">
                            <Textarea
                                id="ef-note"
                                rows=6
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, vault.field_content).to_owned()
                                })
                                value=Signal::derive(move || data.with(|d| d.note_content.clone()))
                                on_change=Callback::new(move |v: String| {
                                    data.update(|d| d.note_content = v);
                                })
                            />
                        </div>
                    }
                        .into_any()
                }
                EntryTypeDto::Identity => {
                    view! {
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
                    }
                        .into_any()
                }
                EntryTypeDto::Folder => ().into_any(),
                EntryTypeDto::Document | EntryTypeDto::Unknown(_) => {
                    view! {
                        <p class="col-span-2 text-sm text-foreground/50">
                            {move || t!(i18n, vault.doc_not_editable)}
                        </p>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

/// Editable key/value rows for an `EnvVars` entry. The structural `For` is
/// driven by a `Memo` on the row count so per-field typing doesn't rebuild it.
/// The fields are label-less, so a column header + per-input `aria_label` give
/// them accessible names (⑤). Reading the stored set (copy/reveal as `.env` or
/// JSON, slice 5.4.1 ⑥) lives in the read drawer, not this edit form.
#[component]
fn EnvVarsFields(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let count = Memo::new(move |_| data.with(|d| d.env_vars.len()));

    let add_row = move |_: web_sys::MouseEvent| {
        // A new row has no stored value → its value intent is `Set`, not carry.
        data.update(|d| d.env_vars.push((String::new(), String::new(), false)));
    };

    view! {
        <div class="flex flex-col gap-2">
            // Column headers (⑤) — the inputs are label-less; the header plus a
            // per-input `aria_label` give them accessible names. Reading the set
            // (copy/reveal as .env/JSON) lives in the read drawer, not this form.
            <div class="flex gap-2">
                <span class="flex-1 text-xs text-foreground/60">
                    {move || t!(i18n, vault.field_env_key)}
                </span>
                <span class="flex-1 text-xs text-foreground/60">
                    {move || t!(i18n, vault.field_env_value)}
                </span>
            </div>
            <For each=move || 0..count.get() key=|i| *i let:i>
                <div class="flex gap-2 items-center">
                    <Input
                        id="ef-env-key"
                        aria_label=Signal::derive(move || {
                            t_string!(i18n, vault.field_env_key).to_owned()
                        })
                        value=Signal::derive(move || {
                            data.with(|d| {
                                d.env_vars.get(i).map(|p| p.0.clone()).unwrap_or_default()
                            })
                        })
                        on_input=Callback::new(move |v: String| {
                            data.update(|d| {
                                if let Some(p) = d.env_vars.get_mut(i) {
                                    p.0 = v;
                                }
                            });
                        })
                        class="flex-1"
                    />
                    <Input
                        id="ef-env-val"
                        input_type="password"
                        aria_label=Signal::derive(move || {
                            t_string!(i18n, vault.field_env_value).to_owned()
                        })
                        value=Signal::derive(move || {
                            data.with(|d| {
                                d.env_vars.get(i).map(|p| p.1.clone()).unwrap_or_default()
                            })
                        })
                        on_input=Callback::new(move |v: String| {
                            data.update(|d| {
                                if let Some(p) = d.env_vars.get_mut(i) {
                                    p.1 = v;
                                }
                            });
                        })
                        reveal_label=Signal::derive(move || {
                            t_string!(i18n, vault.reveal).to_owned()
                        })
                        hide_label=Signal::derive(move || t_string!(i18n, vault.hide).to_owned())
                        class="flex-1"
                    />
                    <Button
                        variant=Variant::Ghost
                        size=Size::Sm
                        on:click=move |_: web_sys::MouseEvent| {
                            data.update(|d| {
                                if i < d.env_vars.len() {
                                    d.env_vars.remove(i);
                                }
                            });
                        }
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
    use vedge_ipc::{
        CommonMetaDto, EntryTypeDto, LoginPayloadDto, PayloadDto, SecretListUpdateDto,
        SecretUpdateDto, TotpAlgorithmDto, TotpUpdateDto,
    };

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
    fn to_payload_login_sends_set_password_on_create_and_unchanged_recovery() {
        let mut d = base(EntryTypeDto::Login);
        d.username = "alice".into();
        d.password = "pw".into();
        let PayloadDto::Login(p) = d.to_payload().unwrap() else {
            panic!("login");
        };
        assert_eq!(p.username, "alice");
        assert!(!p.has_totp);
        assert!(matches!(p.totp, TotpUpdateDto::Unchanged));
        // Create → a typed password is a `Set`.
        assert!(matches!(p.password, SecretUpdateDto::Set(ref s) if s == "pw"));
        // Recovery codes have no editor → always carried forward.
        assert!(matches!(p.recovery_codes, SecretListUpdateDto::Unchanged));
    }

    #[test]
    fn editing_untouched_secret_stays_unchanged_but_typed_is_set() {
        // 🔴 Spec #5 at the form layer: an editing form with an EMPTY (untouched)
        // required secret sends `Unchanged` (keep the stored secret), never a
        // fake value; typing sends `Set`.
        let mut d = base(EntryTypeDto::Login);
        d.editing = true;
        d.username = "alice".into();
        // password left empty → Unchanged.
        let PayloadDto::Login(p) = d.to_payload().unwrap() else {
            panic!("login");
        };
        assert!(matches!(p.password, SecretUpdateDto::Unchanged));

        d.password = "hunter3".into();
        let PayloadDto::Login(p) = d.to_payload().unwrap() else {
            panic!("login");
        };
        assert!(matches!(p.password, SecretUpdateDto::Set(ref s) if s == "hunter3"));
    }

    #[test]
    fn editing_optional_secret_empty_with_stored_stays_unchanged() {
        let mut d = base(EntryTypeDto::Card);
        d.editing = true;
        d.card_number = "4111".into();
        d.card_expiry = "12/30".into();
        d.cvv = "123".into();
        d.has_pin = true; // a stored pin exists
        // pin left empty on an editing form → keep it.
        let PayloadDto::Card(p) = d.to_payload().unwrap() else {
            panic!("card");
        };
        assert!(matches!(p.pin, SecretUpdateDto::Unchanged));
    }

    #[test]
    fn to_payload_card_parses_expiry_and_clears_absent_optional() {
        let mut d = base(EntryTypeDto::Card);
        d.card_number = "4111".into();
        d.card_expiry = "12/30".into();
        d.cvv = "123".into();
        let PayloadDto::Card(p) = d.to_payload().unwrap() else {
            panic!("card");
        };
        assert_eq!(p.expiry_month, 12);
        assert_eq!(p.expiry_year, 2030);
        // Create, empty pin, none stored → Clear (⇒ None).
        assert!(matches!(p.pin, SecretUpdateDto::Clear));
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
    fn to_payload_env_vars_drops_blank_keys_and_sets_values() {
        let mut d = base(EntryTypeDto::EnvVars);
        d.env_vars = vec![
            ("DB".into(), "x".into(), false),
            ("  ".into(), "orphan".into(), false),
        ];
        let PayloadDto::EnvVars(p) = d.to_payload().unwrap() else {
            panic!("env");
        };
        assert_eq!(p.vars.len(), 1);
        assert_eq!(p.vars[0].key, "DB");
        assert!(matches!(p.vars[0].value, SecretUpdateDto::Set(ref s) if s == "x"));
    }

    /// After sealing, `from_payload` no longer receives secrets: it leaves the
    /// secret fields EMPTY, flips `editing`, and records the presence flags.
    #[test]
    fn from_payload_seals_secrets_and_records_presence() {
        let dto = PayloadDto::Login(LoginPayloadDto {
            meta: CommonMetaDto {
                name: "gh".into(),
                entry_type: EntryTypeDto::Login,
                url: Some("https://example.com".into()),
                favicon_url: None,
                tag_ids: vec![],
                folder_id: None,
                is_favorite: false,
                notes: None,
                color: None,
                icon: None,
                sort_order: 0,
            },
            username: "alice".into(),
            password: SecretUpdateDto::Unchanged,
            has_totp: true,
            totp_algorithm: TotpAlgorithmDto::Sha256,
            totp_digits: 8,
            totp_period: 60,
            totp: TotpUpdateDto::Unchanged,
            recovery_codes: SecretListUpdateDto::Unchanged,
            recovery_codes_count: 3,
        });
        let d = EntryFormData::from_payload(&dto);
        assert!(d.editing, "seeded from an existing entry");
        assert_eq!(d.username, "alice");
        assert!(d.password.is_empty(), "the sealed password does not cross");
        assert_eq!(d.url, "https://example.com");
        assert!(d.has_totp);
        assert_eq!(d.totp_digits, 8);
        assert_eq!(d.recovery_codes_count, 3);
    }

    /// Non-secret fields still round-trip through the form; secrets are sealed.
    #[test]
    fn from_payload_roundtrips_nonsecret_fields() {
        let mut d = base(EntryTypeDto::Card);
        d.cardholder_name = "Alice A".into();
        d.card_expiry = "12/30".into();
        // Seed secrets so to_payload produces Set intents (create direction)...
        d.card_number = "4111".into();
        d.cvv = "123".into();
        // ...but from_payload reads a SEALED DTO, so build one via the outbound shape:
        let sealed = PayloadDto::Card(match d.to_payload().unwrap() {
            PayloadDto::Card(mut c) => {
                // Simulate the sealed outbound door: intents become Unchanged.
                c.number = SecretUpdateDto::Unchanged;
                c.cvv = SecretUpdateDto::Unchanged;
                c.pin = SecretUpdateDto::Unchanged;
                c
            }
            _ => panic!("expected card"),
        });
        let round = EntryFormData::from_payload(&sealed);
        assert_eq!(round.cardholder_name, "Alice A");
        assert_eq!(round.card_expiry, "12/30");
        assert!(round.card_number.is_empty(), "number is sealed");
        assert!(round.cvv.is_empty(), "cvv is sealed");
        assert!(round.editing);
    }
}
