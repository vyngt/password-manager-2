//! Recovery Kit PDF (slice 5.7).
//!
//! A sibling of the Emergency Kit PDF ([`super::emergency_kit`]), rendered as a single A4
//! page. Unlike the Emergency Kit — whose Secret Key is re-derivable from the keychain — the
//! Recovery Key is **show-once and never stored**, so [`render`] takes the `RK1-` display
//! string that crossed once at enroll rather than reading anything back.
//!
//! ## What goes on the page
//!
//! 1. Title: **Vedge Recovery Kit** (unmistakably NOT the Emergency Kit).
//! 2. Sub-title: vault name + path, and a generated timestamp.
//! 3. The boxed `RK1-` Recovery Key in large monospace, one hyphen-group per line.
//! 4. A QR code of the display string.
//! 5. 🔴 Instructions that tell the user to keep it **separate** from the Emergency Kit —
//!    the two documents together open the vault with no password, so they must not live in
//!    the same place.
//!
//! Clippy exemptions match the Emergency Kit renderer (mm-based `f32` layout + QR integer
//! division).

#![allow(
    clippy::float_arithmetic,
    clippy::integer_division,
    clippy::indexing_slicing
)]

use printpdf::{BuiltinFont, Image, ImageTransform, Mm, PdfDocument, PdfDocumentReference};

use printpdf::image_crate as image_crate_re;
use qrcodegen::{QrCode, QrCodeEcc};

use vedge_core::domain::shared::Timestamp;

use crate::dto::common::ts_to_string;
use crate::error::CommandError;

const VEDGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the Recovery Kit PDF renders. The display string is supplied by the caller (the
/// enroll flow's transient state); nothing here is read from disk or the keychain.
pub struct RecoveryKitContent {
    pub vault_name: String,
    pub vault_path: String,
    pub generated_at: Timestamp,
    /// `RK1-XXXXX-…` — the show-once Recovery Key display.
    pub recovery_key_display: String,
}

/// Render the Recovery Kit to a PDF byte buffer.
pub fn render(content: &RecoveryKitContent) -> Result<Vec<u8>, CommandError> {
    let (doc, page_idx, layer_idx) =
        PdfDocument::new("Vedge Recovery Kit", Mm(210.0), Mm(297.0), "content");

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

    // ---- Title -----------------------------------------------------------
    layer.use_text("Vedge Recovery Kit", 24.0, Mm(20.0), Mm(270.0), &sans_bold);

    // ---- Sub-title (vault name + path) -----------------------------------
    let vault_line = if content.vault_name.is_empty() {
        content.vault_path.clone()
    } else {
        format!("{} — {}", content.vault_name, content.vault_path)
    };
    layer.use_text(&vault_line, 11.0, Mm(20.0), Mm(260.0), &sans);

    let generated_line = format!("Generated: {}", ts_to_string(content.generated_at));
    layer.use_text(&generated_line, 10.0, Mm(20.0), Mm(253.0), &sans);

    // ---- Recovery Key box (large monospaced, one group per line) ---------
    layer.use_text(
        "Recovery Key — copy this exactly:",
        12.0,
        Mm(20.0),
        Mm(238.0),
        &sans_bold,
    );

    // The `RK1-` display has 11 groups (vs the Emergency Kit's 6), so use a slightly smaller
    // point size + line height to keep all groups above the instructions.
    let mut y = 226.0_f32;
    for group in content.recovery_key_display.split('-') {
        layer.use_text(group, 16.0, Mm(25.0), Mm(y), &mono_bold);
        y -= 8.0;
    }

    // ---- QR code ---------------------------------------------------------
    match QrCode::encode_text(&content.recovery_key_display, QrCodeEcc::Medium) {
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

    // ---- Instructions (🔴 keep it SEPARATE from the Emergency Kit) --------
    let paragraph = concat!(
        "Keep this document somewhere separate from your Emergency Kit. Your Recovery Key ",
        "(RK1-) plus the Secret Key (A3-) from your Emergency Kit together open this vault ",
        "with no master password — so the two documents must never be stored in the same ",
        "place. Anyone with just this Recovery Kit cannot open your vault."
    );
    let mut text_y = 120.0_f32;
    for line in wrap_lines(paragraph, 85) {
        layer.use_text(&line, 10.0, Mm(20.0), Mm(text_y), &sans);
        text_y -= 5.0;
    }

    // ---- Footer ----------------------------------------------------------
    let footer = format!("Vedge {VEDGE_VERSION}");
    layer.use_text(&footer, 8.0, Mm(20.0), Mm(20.0), &mono);

    serialize(doc)
}

/// Convert a QR matrix into a monochrome PNG byte buffer.
fn qr_to_png_bytes(qr: &QrCode) -> Result<Vec<u8>, CommandError> {
    let size =
        usize::try_from(qr.size()).map_err(|_| internal("qr size out of range".to_owned()))?;
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

fn internal(msg: impl Into<String>) -> CommandError {
    let msg = msg.into();
    tracing::warn!(message = %msg, "recovery kit PDF render failure");
    CommandError::Invalid(msg)
}

/// Naive greedy line wrap on word boundaries.
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

    fn sample_content() -> RecoveryKitContent {
        RecoveryKitContent {
            vault_name: "work vault".into(),
            vault_path: "C:/Users/alice/vaults/work.vedge".into(),
            generated_at: chrono::Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap(),
            recovery_key_display:
                "RK1-00001-11122-23334-44555-66677-78889-9AAAB-BBCCC-DDEEE-FFGGG-HHJJJ".into(),
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
    fn render_titles_it_the_recovery_kit_not_the_emergency_kit() {
        let bytes = render(&sample_content()).unwrap();
        let hay = String::from_utf8_lossy(&bytes);
        assert!(hay.contains("Vedge Recovery Kit"));
        // It must never claim to be the Emergency Kit or leak the master password.
        let low = hay.to_ascii_lowercase();
        assert!(!low.contains("master password"));
    }
}
