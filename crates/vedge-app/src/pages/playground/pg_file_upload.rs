use leptos::prelude::*;
use std::time::Duration;
use vedge_ui::components::form::file_upload::{
    FileChangeEvent, FileItem, FileStatus, FileUpload, FileUploadVariant, ValidationError,
    ValidationReason,
};
use vedge_ui::components::form::helper_text::HelperText;
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

fn reason_label(reason: ValidationReason) -> &'static str {
    match reason {
        ValidationReason::Type => "wrong file type",
        ValidationReason::Size => "file too large",
        ValidationReason::MaxFiles => "too many files",
    }
}

#[component]
pub fn FileUploadPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"FileUpload"</h1>

            <VariantsSection />
            <AcceptSection />
            <MaxSizeSection />
            <MaxFilesSection />
            <DisabledSection />
            <ControlledSection />
            <AutoHintSection />
        </div>
    }
}

#[component]
fn VariantsSection() -> impl IntoView {
    let single_log = RwSignal::new(String::new());
    let multi_log = RwSignal::new(String::new());

    let on_single = Callback::new(move |ev: FileChangeEvent| match ev {
        FileChangeEvent::Added(files) => {
            let names: Vec<String> = files.iter().map(web_sys::File::name).collect();
            single_log.set(format!("Added: {}", names.join(", ")));
        }
        FileChangeEvent::Removed { id } => single_log.set(format!("Removed: {id}")),
        FileChangeEvent::Cancelled { id } => single_log.set(format!("Cancelled: {id}")),
    });

    let on_multi = Callback::new(move |ev: FileChangeEvent| match ev {
        FileChangeEvent::Added(files) => {
            let names: Vec<String> = files.iter().map(web_sys::File::name).collect();
            multi_log.set(format!("Added: {}", names.join(", ")));
        }
        FileChangeEvent::Removed { id } => multi_log.set(format!("Removed: {id}")),
        FileChangeEvent::Cancelled { id } => multi_log.set(format!("Cancelled: {id}")),
    });

    view! {
        <Section title="Variants — Single and Multiple (uncontrolled)">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"variant=\"single\""</p>
                    <FileUpload variant=FileUploadVariant::Single on_change=on_single />
                    <p class="text-xs text-text-tertiary min-h-[1em]">{move || single_log.get()}</p>
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"variant=\"multiple\""</p>
                    <FileUpload variant=FileUploadVariant::Multiple on_change=on_multi />
                    <p class="text-xs text-text-tertiary min-h-[1em]">{move || multi_log.get()}</p>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn AcceptSection() -> impl IntoView {
    view! {
        <Section title="Accept filter — MIME types & file extensions">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">
                        "accept=\"image/*\" — any image (try dragging a PDF to see invalid)"
                    </p>
                    <FileUpload
                        variant=FileUploadVariant::Multiple
                        accept="image/*"
                        placeholder="Drop images here"
                    />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">
                        "accept=\".pem,.crt,.cer,.key\" — certificate files"
                    </p>
                    <FileUpload
                        variant=FileUploadVariant::Multiple
                        accept=".pem,.crt,.cer,.key"
                        placeholder="Drop cert files here"
                    />
                </div>
            </div>
        </Section>
    }
}

#[component]
fn MaxSizeSection() -> impl IntoView {
    let error_msg = RwSignal::new(String::new());

    let on_err = Callback::new(move |err: ValidationError| {
        error_msg.set(format!(
            "Rejected \"{}\": {}",
            err.file.name(),
            reason_label(err.reason)
        ));
    });

    let on_change = Callback::new(move |_ev: FileChangeEvent| {
        error_msg.set(String::new());
    });

    view! {
        <Section title="max_size — 1 MB limit (helper text surfaces validation errors)">
            <FileUpload
                variant=FileUploadVariant::Multiple
                max_size=1_000_000u64
                on_change=on_change
                on_validation_error=on_err
            />
            <div class="mt-2">
                <Show when=move || !error_msg.get().is_empty()>
                    <HelperText
                        id="file-upload-max-size-error"
                        status=Status::Error
                        message=Signal::derive(move || error_msg.get())
                        error_label="Error"
                    />
                </Show>
            </div>
        </Section>
    }
}

#[component]
fn MaxFilesSection() -> impl IntoView {
    view! {
        <Section title="max_files=3 — zone becomes inert after 3 files">
            <FileUpload variant=FileUploadVariant::Multiple max_files=3u32 hint="Up to 3 files" />
        </Section>
    }
}

#[component]
fn DisabledSection() -> impl IntoView {
    view! {
        <Section title="Disabled">
            <FileUpload
                variant=FileUploadVariant::Single
                disabled=true
                hint="This zone is disabled"
            />
        </Section>
    }
}

#[component]
fn ControlledSection() -> impl IntoView {
    let items = RwSignal::new(Vec::<FileItem>::new());
    let items_sig: Signal<Vec<FileItem>> = items.into();

    fn start_progress_simulation(id: String, items: RwSignal<Vec<FileItem>>) {
        // One tick of simulated progress
        fn tick(id: String, items: RwSignal<Vec<FileItem>>) {
            let current_pct = items
                .with_untracked(|v| v.iter().find(|it| it.id == id).and_then(|it| it.progress))
                .unwrap_or(0);

            if current_pct >= 100 {
                items.update(|v| {
                    if let Some(it) = v.iter_mut().find(|it| it.id == id) {
                        it.status = FileStatus::Done;
                        it.progress = Some(100);
                    }
                });
                return;
            }

            // Simulate an error at 40% on files whose name contains "fail"
            let name = items
                .with_untracked(|v| v.iter().find(|it| it.id == id).map(|it| it.file.name()))
                .unwrap_or_default();

            let next = (current_pct + 12).min(100);
            let fail_here = name.to_ascii_lowercase().contains("fail") && current_pct >= 36;

            items.update(|v| {
                if let Some(it) = v.iter_mut().find(|it| it.id == id) {
                    if fail_here {
                        it.status = FileStatus::Error;
                        it.error = Some("Simulated upload failure".to_owned());
                    } else {
                        it.status = FileStatus::Uploading;
                        it.progress = Some(next);
                    }
                }
            });

            // Stop if we errored or reached 100
            let should_continue = items.with_untracked(|v| {
                v.iter().find(|it| it.id == id).is_some_and(|it| {
                    it.status == FileStatus::Uploading && it.progress != Some(100)
                })
            });

            if should_continue {
                let id_next = id;
                set_timeout(move || tick(id_next, items), Duration::from_millis(220));
            }
        }

        tick(id, items);
    }

    let on_change = Callback::new(move |ev: FileChangeEvent| match ev {
        FileChangeEvent::Added(files) => {
            let mut ids = Vec::new();
            items.update(|v| {
                for f in files {
                    let item = FileItem {
                        id: uuid::Uuid::new_v4().to_string(),
                        file: f,
                        status: FileStatus::Uploading,
                        progress: Some(0),
                        error: None,
                    };
                    ids.push(item.id.clone());
                    v.push(item);
                }
            });
            for id in ids {
                let delay_id = id.clone();
                set_timeout(
                    move || start_progress_simulation(delay_id, items),
                    Duration::from_millis(150),
                );
            }
        }
        FileChangeEvent::Removed { id } | FileChangeEvent::Cancelled { id } => {
            items.update(|v| v.retain(|it| it.id != id));
        }
    });

    view! {
        <Section title="Controlled — simulated upload progress (name with \"fail\" will error)">
            <FileUpload
                variant=FileUploadVariant::Multiple
                allow_cancel=true
                items=items_sig
                on_change=on_change
                hint="Drop any files · progress is simulated client-side"
            />
            <p class="text-xs text-text-tertiary mt-2">
                {move || format!("{} file(s) in list", items.get().len())}
            </p>
        </Section>
    }
}

#[component]
fn AutoHintSection() -> impl IntoView {
    view! {
        <Section title="Hint — auto-generated vs explicit">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">
                        "No `hint` prop → derived from `accept` + `max_size`"
                    </p>
                    <FileUpload
                        variant=FileUploadVariant::Single
                        accept="image/png,image/jpeg"
                        max_size=5_000_000u64
                    />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">
                        "Explicit `hint=\"PNG or JPG · max 5 MB\"` overrides auto-gen"
                    </p>
                    <FileUpload
                        variant=FileUploadVariant::Single
                        accept="image/png,image/jpeg"
                        max_size=5_000_000u64
                        hint="PNG or JPG · max 5 MB"
                    />
                </div>
            </div>
        </Section>
    }
}
