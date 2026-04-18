use leptos::prelude::*;
use vedge_ui::components::foundation::qrcode::{QRCode, QrEcc};
use vedge_ui::primitives::tokens::Size;

use super::common::Section;

const URL: &str = "https://vedge.app";
const TOTP: &str =
    "otpauth://totp/Vedge:demo@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Vedge&algorithm=SHA1&digits=6&period=30";

#[component]
pub fn QRCodePage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"QRCode"</h1>

            <SizesSection />
            <ErrorCorrectionSection />
            <ExamplesSection />
            <LiveEditSection />
        </div>
    }
}

#[component]
fn SizesSection() -> impl IntoView {
    view! {
        <Section title="Sizes">
            <div class="flex items-end gap-6">
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        size=Size::Sm
                        aria_label="QR code to vedge.app (small)"
                    />
                    <span class="text-xs text-text-tertiary">"sm (128px)"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        size=Size::Md
                        aria_label="QR code to vedge.app (medium)"
                    />
                    <span class="text-xs text-text-tertiary">"md (192px)"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        size=Size::Lg
                        aria_label="QR code to vedge.app (large)"
                    />
                    <span class="text-xs text-text-tertiary">"lg (256px)"</span>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn ErrorCorrectionSection() -> impl IntoView {
    view! {
        <Section title="Error correction levels (L / M / Q / H) at md size">
            <div class="flex items-end gap-6 flex-wrap">
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        error_correction=QrEcc::L
                        aria_label="L error correction"
                    />
                    <span class="text-xs text-text-tertiary">"L — ~7%"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        error_correction=QrEcc::M
                        aria_label="M error correction"
                    />
                    <span class="text-xs text-text-tertiary">"M — ~15% (default)"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        error_correction=QrEcc::Q
                        aria_label="Q error correction"
                    />
                    <span class="text-xs text-text-tertiary">"Q — ~25%"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        error_correction=QrEcc::H
                        aria_label="H error correction"
                    />
                    <span class="text-xs text-text-tertiary">"H — ~30%"</span>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn ExamplesSection() -> impl IntoView {
    view! {
        <Section title="Examples">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="flex items-start gap-4">
                    <QRCode
                        value=Signal::stored(URL.to_string())
                        aria_label="QR code for https://vedge.app"
                    />
                    <div class="text-xs text-text-secondary">
                        <div class="font-medium text-text-primary mb-1">"URL"</div>
                        <code class="text-text-tertiary break-all">{URL}</code>
                    </div>
                </div>
                <div class="flex items-start gap-4">
                    <QRCode
                        value=Signal::stored(TOTP.to_string())
                        error_correction=QrEcc::Q
                        aria_label="QR code for TOTP setup — scan with your authenticator app"
                    />
                    <div class="text-xs text-text-secondary">
                        <div class="font-medium text-text-primary mb-1">"TOTP otpauth URI"</div>
                        <code class="text-text-tertiary break-all">{TOTP}</code>
                    </div>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn LiveEditSection() -> impl IntoView {
    let text = RwSignal::new(String::from("hello world"));
    view! {
        <Section title="Live edit — typing regenerates the QR code">
            <div class="flex items-start gap-6">
                <QRCode
                    value=Signal::derive(move || text.get())
                    size=Size::Md
                    aria_label="Live-editable QR code"
                />
                <div class="flex-1 space-y-2">
                    <label for="qr-live-input" class="text-sm text-text-secondary">
                        "Encode any text:"
                    </label>
                    <input
                        id="qr-live-input"
                        type="text"
                        class="w-full px-3 py-2 text-sm rounded border border-border bg-background text-text-primary focus:outline-none focus:ring-2 focus:ring-primary/30"
                        prop:value=move || text.get()
                        on:input:target=move |ev| text.set(ev.target().value())
                    />
                    <p class="text-xs text-text-tertiary">
                        "Length: " {move || text.get().len()} " chars"
                    </p>
                </div>
            </div>
        </Section>
    }
}
