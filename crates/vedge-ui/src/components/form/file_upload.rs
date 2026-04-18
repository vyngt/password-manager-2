use crate::components::foundation::badge::Badge;
use crate::components::foundation::icon_button::IconButton;
use crate::components::foundation::progress_bar::ProgressBar;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{BadgeSize, BadgeVariant, Size, Variant};
use crate::utils::format::format_bytes;
use crate::utils::id::new_uuid;
use crate::utils::text::text_or;
use icondata as i;
use leptos::ev::Targeted;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use web_sys::{DragEvent, Event, HtmlInputElement};

// -------------------------------------------------------------------------
// Public types
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FileUploadVariant {
    #[default]
    Single,
    Multiple,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FileStatus {
    #[default]
    Idle,
    Uploading,
    Done,
    Error,
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub id: String,
    pub file: web_sys::File,
    pub status: FileStatus,
    pub progress: Option<u8>,
    pub error: Option<String>,
}

impl FileItem {
    pub fn new(file: web_sys::File) -> Self {
        Self {
            id: new_uuid(),
            file,
            status: FileStatus::Idle,
            progress: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Added(Vec<web_sys::File>),
    Removed { id: String },
    Cancelled { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationReason {
    Type,
    Size,
    MaxFiles,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub file: web_sys::File,
    pub reason: ValidationReason,
}

// -------------------------------------------------------------------------
// Internal state
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragState {
    Idle,
    Valid,
    Invalid,
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Check whether a MIME type + filename satisfies an accept pattern.
/// Accept forms: `"*"`, `"*/*"`, `"image/*"`, `"application/pdf"`, `".pem"`, `".pem,.crt,image/*"`.
fn matches_accept(accept: &str, mime: &str, name: &str) -> bool {
    let accept = accept.trim();
    if accept.is_empty() || accept == "*" || accept == "*/*" {
        return true;
    }
    let lower_name = name.to_ascii_lowercase();
    let lower_mime = mime.to_ascii_lowercase();
    for pat in accept.split(',') {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        let lower_pat = pat.to_ascii_lowercase();
        if let Some(prefix) = lower_pat.strip_suffix("/*") {
            if !lower_mime.is_empty() && lower_mime.starts_with(&format!("{}/", prefix)) {
                return true;
            }
        } else if lower_pat.starts_with('.') {
            if lower_name.ends_with(&lower_pat) {
                return true;
            }
        } else if !lower_mime.is_empty() && lower_mime == lower_pat {
            return true;
        }
    }
    false
}

/// Best-effort MIME-only check during dragover (no filename available yet).
/// Returns `true` if the mime plausibly matches, or if the accept rules rely on
/// extensions (in which case we can't know until drop — be permissive).
fn matches_accept_mime_only(accept: &str, mime: &str) -> bool {
    let accept = accept.trim();
    if accept.is_empty() || accept == "*" || accept == "*/*" {
        return true;
    }
    let lower_mime = mime.to_ascii_lowercase();
    let mut has_ext_rule = false;
    for pat in accept.split(',') {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        let lower_pat = pat.to_ascii_lowercase();
        if let Some(prefix) = lower_pat.strip_suffix("/*") {
            if !lower_mime.is_empty() && lower_mime.starts_with(&format!("{}/", prefix)) {
                return true;
            }
        } else if lower_pat.starts_with('.') {
            has_ext_rule = true;
        } else if !lower_mime.is_empty() && lower_mime == lower_pat {
            return true;
        }
    }
    has_ext_rule
}

fn files_from_filelist(list: web_sys::FileList) -> Vec<web_sys::File> {
    let len = list.length();
    let mut out = Vec::with_capacity(len as usize);
    for idx in 0..len {
        if let Some(f) = list.item(idx) {
            out.push(f);
        }
    }
    out
}

fn is_duplicate(existing: &[FileItem], file: &web_sys::File) -> bool {
    let name = file.name();
    let size = file.size() as u64;
    existing
        .iter()
        .any(|it| it.file.name() == name && (it.file.size() as u64) == size)
}

fn auto_hint(accept: &str, max_size: Option<u64>, up_to: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let trimmed = accept.trim();
    if !trimmed.is_empty() && trimmed != "*" && trimmed != "*/*" {
        parts.push(trimmed.to_string());
    }
    if let Some(m) = max_size {
        parts.push(format!("{} {}", up_to, format_bytes(m)));
    }
    parts.join(" \u{00B7} ")
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

#[component]
pub fn FileUpload(
    #[prop(optional)] variant: FileUploadVariant,
    #[prop(optional, default = "*")] accept: &'static str,
    #[prop(optional, default = None)] max_size: Option<u64>,
    #[prop(optional, default = None)] max_files: Option<u32>,
    #[prop(into, default = None)] items: Option<Signal<Vec<FileItem>>>,
    #[prop(into, default = None)] on_change: Option<Callback<FileChangeEvent>>,
    #[prop(into, default = None)] on_validation_error: Option<Callback<ValidationError>>,
    #[prop(optional)] allow_cancel: bool,
    // i18n-ready visible strings — defaults fall back to English at render.
    #[prop(into, default = TextProp::default())] placeholder: TextProp,
    #[prop(into, default = TextProp::default())] hint: TextProp,
    #[prop(into, default = TextProp::default())] done_label: TextProp,
    #[prop(into, default = TextProp::default())] error_label: TextProp,
    #[prop(into, default = TextProp::default())] replace_label: TextProp,
    // i18n-ready aria / live-region labels. Combined as "{prefix} {filename}"
    // or "{filename} {message}" at render time — these templates are simple
    // enough that prefix/suffix concatenation works for most languages; for
    // right-to-left word-order languages, provide the full reactive string.
    #[prop(into, default = TextProp::default())] remove_label: TextProp,
    #[prop(into, default = TextProp::default())] uploading_label: TextProp,
    #[prop(into, default = TextProp::default())] added_message: TextProp,
    #[prop(into, default = TextProp::default())] removed_message: TextProp,
    #[prop(into, default = TextProp::default())] files_added_message: TextProp,
    #[prop(into, default = TextProp::default())] up_to_label: TextProp,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let is_multiple = variant == FileUploadVariant::Multiple;

    let internal_items: RwSignal<Vec<FileItem>> = RwSignal::new(Vec::new());
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();
    let drag_state = RwSignal::new(DragState::Idle);
    let drag_counter = StoredValue::new(0i32);
    let live_message = RwSignal::new(String::new());
    let invalid_flash_ver: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    // Effective file list (controlled vs uncontrolled). Using a Signal here so
    // child rows can subscribe reactively — if we passed a plain FnOnce
    // snapshot through <For>, each row's internal view would freeze at its
    // initial state and never show progress/status updates.
    let items_sig: Signal<Vec<FileItem>> = Signal::derive(move || match items {
        Some(s) => s.get(),
        None => internal_items.get(),
    });
    let effective_items = move || items_sig.get();

    let effective_len = move || effective_items().len() as u32;

    let is_max_reached = move || {
        is_multiple
            && max_files
                .map(|m| effective_len() >= m)
                .unwrap_or(false)
    };

    // Compact zone = single mode with 1 file present
    let is_compact = move || {
        variant == FileUploadVariant::Single && !effective_items().is_empty()
    };

    // ---------- Validation & commit pipeline ----------
    let emit_validation_error = move |file: web_sys::File, reason: ValidationReason| {
        if let Some(cb) = on_validation_error {
            cb.run(ValidationError { file, reason });
        }
    };

    let commit_files = move |raw: Vec<web_sys::File>| {
        if disabled || raw.is_empty() {
            return;
        }

        let current = effective_items();
        let mut accepted: Vec<web_sys::File> = Vec::new();
        let mut slots_remaining: Option<u32> = if is_multiple {
            max_files.map(|m| m.saturating_sub(current.len() as u32))
        } else {
            Some(1)
        };

        for f in raw.into_iter() {
            // Type check
            if !matches_accept(accept, &f.type_(), &f.name()) {
                emit_validation_error(f, ValidationReason::Type);
                continue;
            }
            // Size check
            if let Some(max) = max_size {
                if (f.size() as u64) > max {
                    emit_validation_error(f, ValidationReason::Size);
                    continue;
                }
            }
            // Duplicate
            if is_multiple && is_duplicate(&current, &f) {
                continue;
            }
            // Max files
            if is_multiple {
                if let Some(rem) = slots_remaining {
                    if rem == 0 {
                        emit_validation_error(f, ValidationReason::MaxFiles);
                        continue;
                    }
                    slots_remaining = Some(rem - 1);
                }
            }
            accepted.push(f);
        }

        if accepted.is_empty() {
            return;
        }

        // Single mode: always replace
        if !is_multiple {
            accepted.truncate(1);
        }

        let announce = if accepted.len() == 1 {
            format!(
                "{} {}",
                accepted[0].name(),
                text_or(added_message, "added")
            )
        } else {
            format!(
                "{} {}",
                accepted.len(),
                text_or(files_added_message, "files added")
            )
        };
        live_message.set(announce);

        // Uncontrolled: mutate internal state
        if items.is_none() {
            let new_items: Vec<FileItem> =
                accepted.iter().cloned().map(FileItem::new).collect();
            if is_multiple {
                internal_items.update(|v| v.extend(new_items));
            } else {
                internal_items.set(new_items);
            }
        }

        if let Some(cb) = on_change {
            cb.run(FileChangeEvent::Added(accepted));
        }
    };

    // ---------- Input change handler ----------
    let handle_input_change = move |ev: Targeted<Event, HtmlInputElement>| {
        let input = ev.target();
        let files = input.files().map(files_from_filelist).unwrap_or_default();
        commit_files(files);
        // Reset input value so selecting the same file again still fires change
        input.set_value("");
    };

    // ---------- Drag handlers ----------
    let flash_ver = invalid_flash_ver.clone();
    let trigger_invalid_flash = move || {
        let ver = flash_ver.fetch_add(1, Ordering::Relaxed) + 1;
        drag_state.set(DragState::Invalid);
        let fv = flash_ver.clone();
        set_timeout(
            move || {
                if fv.load(Ordering::Relaxed) == ver {
                    drag_state.set(DragState::Idle);
                }
            },
            Duration::from_millis(600),
        );
    };

    let on_dragenter = move |ev: DragEvent| {
        if disabled || is_max_reached() {
            return;
        }
        ev.prevent_default();
        drag_counter.update_value(|c| *c += 1);

        // Inspect items' MIME types (files are not yet accessible during drag)
        let mut any_valid = false;
        let mut any_present = false;
        if let Some(dt) = ev.data_transfer() {
            let list = dt.items();
            for idx in 0..list.length() {
                if let Some(it) = list.get(idx) {
                    if it.kind() == "file" {
                        any_present = true;
                        if matches_accept_mime_only(accept, &it.type_()) {
                            any_valid = true;
                        }
                    }
                }
            }
        }

        drag_state.set(if !any_present || any_valid {
            DragState::Valid
        } else {
            DragState::Invalid
        });
    };

    let on_dragover = move |ev: DragEvent| {
        if disabled || is_max_reached() {
            return;
        }
        ev.prevent_default();
        if let Some(dt) = ev.data_transfer() {
            dt.set_drop_effect("copy");
        }
    };

    let on_dragleave = move |_ev: DragEvent| {
        if disabled || is_max_reached() {
            return;
        }
        drag_counter.update_value(|c| *c -= 1);
        if drag_counter.get_value() <= 0 {
            drag_counter.set_value(0);
            drag_state.set(DragState::Idle);
        }
    };

    let on_drop = move |ev: DragEvent| {
        if disabled || is_max_reached() {
            return;
        }
        ev.prevent_default();
        drag_counter.set_value(0);

        let raw_files = ev
            .data_transfer()
            .and_then(|dt| dt.files())
            .map(files_from_filelist)
            .unwrap_or_default();

        // Detect all-invalid-by-type case for the 600ms flash
        let any_type_match = raw_files
            .iter()
            .any(|f| matches_accept(accept, &f.type_(), &f.name()));

        if !raw_files.is_empty() && !any_type_match {
            trigger_invalid_flash();
            for f in raw_files {
                emit_validation_error(f, ValidationReason::Type);
            }
            return;
        }

        drag_state.set(DragState::Idle);
        commit_files(raw_files);
    };

    // ---------- Keyboard activation on zone ----------
    let on_zone_keydown = move |ev: web_sys::KeyboardEvent| {
        if disabled || is_max_reached() {
            return;
        }
        let key = ev.key();
        if key == "Enter" || key == " " || key == "Spacebar" {
            ev.prevent_default();
            if let Some(el) = input_ref.get() {
                el.click();
            }
        }
    };

    // ---------- Remove handler ----------
    let handle_remove = move |item: FileItem| {
        let was_uploading = item.status == FileStatus::Uploading;
        let filename = item.file.name();
        let id = item.id.clone();

        if items.is_none() {
            internal_items.update(|v| v.retain(|it| it.id != id));
        }

        live_message.set(format!("{} {}", filename, text_or(removed_message, "removed")));

        if let Some(cb) = on_change {
            if was_uploading {
                cb.run(FileChangeEvent::Cancelled { id });
            } else {
                cb.run(FileChangeEvent::Removed { id });
            }
        }
    };

    // ---------- Classes ----------
    let root_cls = ["file-upload", class].join(" ");

    let zone_cls = move || {
        let mut c = String::from("file-upload__zone");
        match drag_state.get() {
            DragState::Valid => c.push_str(" file-upload__zone--drag-over"),
            DragState::Invalid => c.push_str(" file-upload__zone--drag-invalid"),
            DragState::Idle => {}
        }
        if disabled {
            c.push_str(" file-upload__zone--disabled");
        }
        if is_max_reached() {
            c.push_str(" file-upload__zone--max-reached");
        }
        if is_compact() {
            c.push_str(" file-upload__zone--compact");
        }
        c
    };

    // Native input is disabled when the component is disabled OR max-reached
    // (max-reached only applies in Multiple mode; in Single+file we still want
    // the label click to open the picker for replacement).
    let input_disabled = move || disabled || is_max_reached();

    let aria_disabled_attr = move || {
        if disabled {
            Some("true")
        } else {
            None
        }
    };

    // Resolved hint text — prefer explicit prop, else auto-generate from
    // accept/max_size using the reactive `up_to_label`.
    let resolved_hint = move || {
        let h = hint.get();
        if h.is_empty() {
            let up_to = text_or(up_to_label, "up to");
            auto_hint(accept, max_size, &up_to)
        } else {
            h
        }
    };

    // Compact zone item (Single mode only reads first element)
    let compact_item = move || effective_items().into_iter().next();

    view! {
        <div class=root_cls>
            <label
                class=zone_cls
                role="button"
                tabindex=move || if disabled || is_max_reached() { "-1" } else { "0" }
                aria-disabled=aria_disabled_attr
                on:dragenter=on_dragenter
                on:dragover=on_dragover
                on:dragleave=on_dragleave
                on:drop=on_drop
                on:keydown=on_zone_keydown
            >
                <input
                    node_ref=input_ref
                    type="file"
                    class="file-upload__input"
                    accept=accept
                    multiple=is_multiple
                    disabled=input_disabled
                    on:change:target=handle_input_change
                />

                <Show
                    when=move || is_compact()
                    fallback=move || {
                        view! {
                            <span class="file-upload__zone-icon" aria-hidden="true">
                                <Icon icon=i::FaCloudArrowUpSolid />
                            </span>
                            <span class="file-upload__prompt">
                                {move || text_or(placeholder, "Drag files here or click to browse")}
                            </span>
                            {move || {
                                let h = resolved_hint();
                                (!h.is_empty())
                                    .then(|| view! { <span class="file-upload__hint">{h}</span> })
                            }}
                        }
                    }
                >
                    {move || {
                        compact_item()
                            .map(|item| {
                                view! {
                                    <CompactItem
                                        id=item.id.clone()
                                        file=item.file.clone()
                                        items_sig=items_sig
                                        allow_cancel=allow_cancel
                                        disabled=disabled
                                        on_remove=Callback::new(handle_remove)
                                        done_label=done_label
                                        error_label=error_label
                                        replace_label=replace_label
                                        remove_label=remove_label
                                        uploading_label=uploading_label
                                    />
                                }
                            })
                    }}
                </Show>
            </label>

            // File list (Multiple mode only)
            <Show when=move || !is_compact() && !effective_items().is_empty()>
                <div class="file-upload__list">
                    <For
                        each=move || effective_items()
                        key=|it| it.id.clone()
                        children=move |item| {
                            view! {
                                <FileRow
                                    id=item.id.clone()
                                    file=item.file.clone()
                                    items_sig=items_sig
                                    allow_cancel=allow_cancel
                                    disabled=disabled
                                    on_remove=Callback::new(handle_remove)
                                    done_label=done_label
                                    error_label=error_label
                                    remove_label=remove_label
                                    uploading_label=uploading_label
                                />
                            }
                        }
                    />
                </div>
            </Show>

            // aria-live region for accessibility announcements
            <div class="file-upload__live" aria-live="polite" aria-atomic="true">
                {move || live_message.get()}
            </div>
        </div>
    }
}

// -------------------------------------------------------------------------
// File row (multiple mode)
// -------------------------------------------------------------------------

#[component]
fn FileRow(
    id: String,
    file: web_sys::File,
    items_sig: Signal<Vec<FileItem>>,
    allow_cancel: bool,
    disabled: bool,
    on_remove: Callback<FileItem>,
    done_label: TextProp,
    error_label: TextProp,
    remove_label: TextProp,
    uploading_label: TextProp,
) -> impl IntoView {
    let name = file.name();
    let size_text = format_bytes(file.size() as u64);
    let id_stored = StoredValue::new(id);
    let name_stored = StoredValue::new(name.clone());

    // Reactive lookup of this row's current state from the parent signal.
    let lookup = move || {
        let id = id_stored.get_value();
        items_sig.with(|v| v.iter().find(|it| it.id == id).cloned())
    };
    let status = move || lookup().map(|it| it.status).unwrap_or(FileStatus::Idle);
    let progress_sig: Signal<f64> = Signal::derive(move || {
        lookup().and_then(|it| it.progress).unwrap_or(0) as f64
    });
    let error_text = move || lookup().and_then(|it| it.error.clone()).unwrap_or_default();
    let show_remove = move || !(status() == FileStatus::Uploading && !allow_cancel);

    let size_text_c = size_text.clone();

    view! {
        <div class="file-upload__item">
            <span class="file-upload__item-icon" aria-hidden="true">
                <Icon icon=i::FaFileSolid />
            </span>
            <span class="file-upload__item-name" title=name.clone()>
                {name.clone()}
            </span>
            <span class="file-upload__item-status">
                {move || match status() {
                    FileStatus::Idle => {
                        view! { <span class="file-upload__item-size">{size_text_c.clone()}</span> }
                            .into_any()
                    }
                    FileStatus::Uploading => {
                        let aria = format!(
                            "{} {}",
                            text_or(uploading_label, "Uploading"),
                            name_stored.get_value(),
                        );
                        view! {
                            <span class="file-upload__item-progress">
                                <ProgressBar value=progress_sig size=Size::Sm aria_label=aria />
                                <span class="file-upload__item-progress-value">
                                    {move || format!("{}%", progress_sig.get() as u8)}
                                </span>
                            </span>
                        }
                            .into_any()
                    }
                    FileStatus::Done => {
                        view! {
                            <Badge variant=BadgeVariant::Success size=BadgeSize::Sm>
                                {move || text_or(done_label, "Done")}
                            </Badge>
                        }
                            .into_any()
                    }
                    FileStatus::Error => {
                        let err = error_text();
                        let fallback = text_or(error_label, "Error");
                        let display = if err.is_empty() { fallback } else { err.clone() };
                        view! {
                            <span class="file-upload__item-error" title=err>
                                {display}
                            </span>
                        }
                            .into_any()
                    }
                }}
                {move || {
                    if !show_remove() {
                        return None;
                    }
                    let it = lookup()?;
                    let lbl = format!(
                        "{} {}",
                        text_or(remove_label, "Remove"),
                        name_stored.get_value(),
                    );
                    Some(
                        view! {
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Sm
                                aria_label=lbl
                                disabled=disabled
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_remove.run(it.clone());
                                }
                            >
                                <Icon icon=i::FaXmarkSolid />
                            </IconButton>
                        },
                    )
                }}
            </span>
        </div>
    }
}

// -------------------------------------------------------------------------
// Compact item (single mode)
// -------------------------------------------------------------------------

#[component]
fn CompactItem(
    id: String,
    file: web_sys::File,
    items_sig: Signal<Vec<FileItem>>,
    allow_cancel: bool,
    disabled: bool,
    on_remove: Callback<FileItem>,
    done_label: TextProp,
    error_label: TextProp,
    replace_label: TextProp,
    remove_label: TextProp,
    uploading_label: TextProp,
) -> impl IntoView {
    let name = file.name();
    let size_text = format_bytes(file.size() as u64);
    let id_stored = StoredValue::new(id);
    let name_stored = StoredValue::new(name.clone());

    let lookup = move || {
        let id = id_stored.get_value();
        items_sig.with(|v| v.iter().find(|it| it.id == id).cloned())
    };
    let status = move || lookup().map(|it| it.status).unwrap_or(FileStatus::Idle);
    let progress_sig: Signal<f64> = Signal::derive(move || {
        lookup().and_then(|it| it.progress).unwrap_or(0) as f64
    });
    let error_text = move || lookup().and_then(|it| it.error.clone()).unwrap_or_default();
    let show_remove = move || !(status() == FileStatus::Uploading && !allow_cancel);

    let size_text_c = size_text.clone();

    view! {
        <span class="file-upload__zone-icon" aria-hidden="true">
            <Icon icon=i::FaFileSolid />
        </span>
        <span class="file-upload__compact-name" title=name.clone()>
            {name.clone()}
        </span>
        {move || match status() {
            FileStatus::Idle => {
                view! {
                    <span class="file-upload__compact-size">{size_text_c.clone()}</span>
                    <span class="file-upload__compact-replace">
                        {move || text_or(replace_label, "Replace")}
                    </span>
                }
                    .into_any()
            }
            FileStatus::Uploading => {
                let aria = format!(
                    "{} {}",
                    text_or(uploading_label, "Uploading"),
                    name_stored.get_value(),
                );
                view! {
                    <span class="file-upload__item-progress">
                        <ProgressBar value=progress_sig size=Size::Sm aria_label=aria />
                        <span class="file-upload__item-progress-value">
                            {move || format!("{}%", progress_sig.get() as u8)}
                        </span>
                    </span>
                }
                    .into_any()
            }
            FileStatus::Done => {
                view! {
                    <Badge variant=BadgeVariant::Success size=BadgeSize::Sm>
                        {move || text_or(done_label, "Done")}
                    </Badge>
                    <span class="file-upload__compact-replace">
                        {move || text_or(replace_label, "Replace")}
                    </span>
                }
                    .into_any()
            }
            FileStatus::Error => {
                let err = error_text();
                let fallback = text_or(error_label, "Error");
                let display = if err.is_empty() { fallback } else { err.clone() };
                view! {
                    <span class="file-upload__item-error" title=err>
                        {display}
                    </span>
                    <span class="file-upload__compact-replace">
                        {move || text_or(replace_label, "Replace")}
                    </span>
                }
                    .into_any()
            }
        }}
        {move || {
            if !show_remove() {
                return None;
            }
            let it = lookup()?;
            let lbl = format!("{} {}", text_or(remove_label, "Remove"), name_stored.get_value());
            Some(
                view! {
                    <IconButton
                        variant=Variant::Ghost
                        size=Size::Sm
                        aria_label=lbl
                        disabled=disabled
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            ev.prevent_default();
                            on_remove.run(it.clone());
                        }
                    >
                        <Icon icon=i::FaXmarkSolid />
                    </IconButton>
                },
            )
        }}
    }
}
