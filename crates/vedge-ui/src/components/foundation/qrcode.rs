use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Size;
use leptos::prelude::*;
use qrcodegen::{QrCode, QrCodeEcc};

/// QR error-correction level. Higher levels recover more from damage but pack
/// more modules into the same content length.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum QrEcc {
    /// ~7% recovery. Use only when the QR will always be clean (screen display).
    L,
    /// ~15% recovery. Default.
    #[default]
    M,
    /// ~25% recovery. For overlay / logo use (not used in Vedge currently).
    Q,
    /// ~30% recovery. For printed codes that may be damaged.
    H,
}

fn to_ecc(e: QrEcc) -> QrCodeEcc {
    match e {
        QrEcc::L => QrCodeEcc::Low,
        QrEcc::M => QrCodeEcc::Medium,
        QrEcc::Q => QrCodeEcc::Quartile,
        QrEcc::H => QrCodeEcc::High,
    }
}

const BORDER: i32 = 4;

fn matrix_to_path_d(qr: &QrCode) -> String {
    use std::fmt::Write;
    let n = qr.size();
    let mut d = String::with_capacity((n as usize).pow(2) * 12);
    for y in 0..n {
        for x in 0..n {
            if qr.get_module(x, y) {
                let _ = write!(d, "M{} {}h1v1h-1z", x + BORDER, y + BORDER);
            }
        }
    }
    d
}

#[component]
pub fn QRCode(
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] size: Size,
    #[prop(optional)] error_correction: QrEcc,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if aria_label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"QRCode: `aria_label` is required — a QR code with no label is unusable for screen reader users.".into(),
        );
    }

    let qr = Memo::new(move |_| {
        QrCode::encode_text(&value.get(), to_ecc(error_correction)).ok()
    });

    let path_d = move || {
        qr.with(|opt| opt.as_ref().map(matrix_to_path_d).unwrap_or_default())
    };

    let view_box = move || {
        qr.with(|opt| {
            opt.as_ref()
                .map(|q| {
                    let side = q.size() + BORDER * 2;
                    format!("0 0 {side} {side}")
                })
                .unwrap_or_else(|| String::from("0 0 33 33"))
        })
    };

    let root_cls = ["qrcode", size.qrcode_class(), class].join(" ");
    let label_text = move || aria_label.get();

    view! {
        <div class=root_cls role="img" aria-label=label_text>
            <svg
                viewBox=view_box
                xmlns="http://www.w3.org/2000/svg"
                preserveAspectRatio="xMidYMid meet"
            >
                <title>{label_text}</title>
                <rect width="100%" height="100%" fill="var(--color-background)" />
                <path d=path_d fill="var(--color-text-primary)" />
            </svg>
        </div>
    }
}
