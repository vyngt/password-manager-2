//! Emergency Kit PDF.
//!
//! Renders an [`EmergencyKitContent`] as a single-page A4 portrait PDF
//! using `printpdf`. Layout is deliberately simple and readable — this
//! document lives in a safe somewhere, not on a screen, so legibility under
//! print wins over density.
//!
//! ## What goes on the page
//!
//! 1. Title: **Vedge Emergency Kit**.
//! 2. Sub-title: vault display name + `.vdb` path.
//! 3. Generated timestamp.
//! 4. Boxed Secret Key in large monospace — one hyphen-group per line so
//!    hand-copy errors are localized.
//! 5. QR code of the display string (square, ~3 cm) — optional scan path.
//! 6. Short instructions paragraph.
//! 7. Footer: KDF params summary + Vedge version.
//!
//! ## What we deliberately omit
//!
//! - The master password. Users manage that separately; embedding it would
//!   defeat the purpose.
//! - Entry data from the vault.
//! - Any tracking identifier — the kit is user-private.
//!
//! ## Clippy exemptions
//!
//! PDF layout is inherently millimeter-based (`f32`) and the QR bitmap
//! routine maps pixel coordinates to module coordinates via integer
//! division. Both are allowed at the module level rather than sprinkled
//! as per-site `#[allow]`s.

#![allow(
    clippy::float_arithmetic,
    clippy::integer_division,
    clippy::indexing_slicing
)]

use printpdf::{BuiltinFont, Image, ImageTransform, Mm, PdfDocument, PdfDocumentReference};

// `printpdf::image_crate` is re-exported when the `embedded_images`
// feature is on (see `vedge-tauri/Cargo.toml`). Alias it locally so the
// rest of the file reads cleanly.
use printpdf::image_crate as image_crate_re;
use qrcodegen::{QrCode, QrCodeEcc};

use vedge_core::domain::vault::emergency_kit::EmergencyKitContent;

use crate::dto::common::ts_to_string;
use crate::error::CommandError;

const VEDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render the kit to a PDF byte buffer. Returns `CommandError::Internal`
/// on the rare `printpdf` serialization failure (mostly impossible on
/// well-formed input, but we don't panic on pathological sizes).
pub fn render(content: &EmergencyKitContent) -> Result<Vec<u8>, CommandError> {
    // A4 portrait: 210 mm × 297 mm.
    let (doc, page_idx, layer_idx) =
        PdfDocument::new("Vedge Emergency Kit", Mm(210.0), Mm(297.0), "content");

    let sans = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| pdf_err(&e))?;
    let sans_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| pdf_err(&e))?;
    let mono = doc
        .add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| pdf_err(&e))?;
    let mono_bold = doc
        .add_builtin_font(BuiltinFont::CourierBold)
        .map_err(|e| pdf_err(&e))?;

    let page = doc.get_page(page_idx);
    let layer = page.get_layer(layer_idx);

    // ---- Title ------------------------------------------------------------
    layer.use_text("Vedge Emergency Kit", 24.0, Mm(20.0), Mm(270.0), &sans_bold);

    // ---- Sub-title (vault name + path) -----------------------------------
    let vault_line = if content.vault_name.is_empty() {
        content.vault_path.clone()
    } else {
        format!("{} — {}", content.vault_name, content.vault_path)
    };
    layer.use_text(&vault_line, 11.0, Mm(20.0), Mm(260.0), &sans);

    // ---- Generated timestamp ---------------------------------------------
    let generated_line = format!("Generated: {}", ts_to_string(content.generated_at));
    layer.use_text(&generated_line, 10.0, Mm(20.0), Mm(253.0), &sans);

    // ---- Secret Key box (large monospaced, one group per line) -----------
    layer.use_text(
        "Secret Key — copy this exactly:",
        12.0,
        Mm(20.0),
        Mm(230.0),
        &sans_bold,
    );

    // Split on hyphens and render each group on its own line at a large
    // point size. A line height of 9 mm suits 18-pt Courier.
    let mut y = 218.0_f32;
    for group in content.secret_key_display.split('-') {
        layer.use_text(group, 18.0, Mm(25.0), Mm(y), &mono_bold);
        y -= 9.0;
    }

    // ---- QR code ---------------------------------------------------------
    match QrCode::encode_text(&content.secret_key_display, QrCodeEcc::Medium) {
        Ok(qr) => {
            let png_bytes = qr_to_png_bytes(&qr)?;
            let decoder =
                image_crate_re::codecs::png::PngDecoder::new(std::io::Cursor::new(&png_bytes))
                    .map_err(|e| internal(format!("qr png decode: {e}")))?;
            let image =
                Image::try_from(decoder).map_err(|e| internal(format!("qr to image: {e}")))?;
            let transform = ImageTransform {
                translate_x: Some(Mm(150.0)),
                translate_y: Some(Mm(215.0)),
                scale_x: Some(0.5),
                scale_y: Some(0.5),
                ..Default::default()
            };
            image.add_to_layer(layer.clone(), transform);
        }
        Err(e) => {
            return Err(internal(format!("qr encode: {e}")));
        }
    }

    // ---- Instructions ----------------------------------------------------
    let paragraph = concat!(
        "Keep this document somewhere only you can reach — a safe, an encrypted drive, or a ",
        "fireproof box. Anyone with this Secret Key AND your master password can open your ",
        "vault. Anyone with just this document cannot."
    );
    let mut text_y = 140.0_f32;
    for line in wrap_lines(paragraph, 85) {
        layer.use_text(&line, 10.0, Mm(20.0), Mm(text_y), &sans);
        text_y -= 5.0;
    }

    // ---- Footer ----------------------------------------------------------
    let footer = format!(
        "KDF: {}   ·   Vedge {}",
        content.kdf_params_summary, VEDGE_VERSION,
    );
    layer.use_text(&footer, 8.0, Mm(20.0), Mm(20.0), &mono);

    serialize(doc)
}

/// Convert a QR matrix into a monochrome PNG byte buffer.
fn qr_to_png_bytes(qr: &QrCode) -> Result<Vec<u8>, CommandError> {
    let size =
        usize::try_from(qr.size()).map_err(|_| internal("qr size out of range".to_owned()))?;
    // 8× scale per module = readable at A4 print resolution.
    let scale: u32 = 8;
    let px_side = u32::try_from(size)
        .map_err(|_| internal("qr size out of range".to_owned()))?
        .saturating_mul(scale);

    let mut img = image_crate_re::GrayImage::new(px_side, px_side);
    for y in 0..px_side {
        for x in 0..px_side {
            let qr_x = x / scale;
            let qr_y = y / scale;
            let dark = qr.get_module(
                i32::try_from(qr_x).unwrap_or(0),
                i32::try_from(qr_y).unwrap_or(0),
            );
            let pixel = if dark { 0u8 } else { 255u8 };
            img.put_pixel(x, y, image_crate_re::Luma([pixel]));
        }
    }

    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    image_crate_re::DynamicImage::ImageLuma8(img)
        .write_to(&mut cursor, image_crate_re::ImageFormat::Png)
        .map_err(|e| internal(format!("png encode: {e}")))?;
    Ok(out)
}

fn serialize(doc: PdfDocumentReference) -> Result<Vec<u8>, CommandError> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut writer = std::io::BufWriter::new(&mut buf);
        doc.save(&mut writer)
            .map_err(|e| internal(format!("pdf save: {e}")))?;
    }
    Ok(buf)
}

fn pdf_err(e: &printpdf::Error) -> CommandError {
    internal(format!("pdf: {e}"))
}

/// PDF-layer operational failures map to `Invalid` so the frontend can
/// surface the cause instead of an opaque "internal error". They're still
/// unexpected — `tracing::warn` logs every occurrence.
fn internal(msg: impl Into<String>) -> CommandError {
    let msg = msg.into();
    tracing::warn!(message = %msg, "emergency kit PDF render failure");
    CommandError::Invalid(msg)
}

/// Naive greedy line wrap on word boundaries. Keeps the module
/// dep-free of a layout crate.
fn wrap_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty()
            && current.len().saturating_add(1).saturating_add(word.len()) <= max_chars
        {
            current.push(' ');
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        clippy::expect_used,
        clippy::indexing_slicing
    )]

    use super::*;
    use chrono::TimeZone;

    fn sample_content() -> EmergencyKitContent {
        EmergencyKitContent {
            vault_name: "work vault".into(),
            vault_path: "C:/Users/alice/vaults/work.vdb".into(),
            generated_at: chrono::Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap(),
            secret_key_display: "A3-00001-11122-23334-44555-66677-78889".into(),
            kdf_params_summary: "argon2id v1 (m=262144, t=3, p=4)".into(),
        }
    }

    #[test]
    fn render_produces_pdf_magic_bytes() {
        let bytes = render(&sample_content()).unwrap();
        assert!(!bytes.is_empty());
        let prefix: &[u8] = bytes.get(..5).unwrap_or_default();
        assert_eq!(prefix, b"%PDF-", "expected PDF magic bytes");
    }

    #[test]
    fn render_omits_master_password_section() {
        // Smoke: searching the PDF for words the kit should never contain.
        let bytes = render(&sample_content()).unwrap();
        let hay = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        assert!(!hay.contains("master password"));
    }
}
