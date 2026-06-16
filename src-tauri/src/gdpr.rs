//! GDPR pre-processing scrub (spec §4).
//!
//! Before ANY bytes leave the machine for the cloud AI, strip EXIF, author
//! metadata, and file-tracking signatures so the model sees "only raw pixels
//! and text tokens". This module produces sanitized payloads; it never reads
//! the SQLite DB and never sees student PII (names/IDs live local-only).

use std::io::Cursor;
use std::path::Path;

use image::ImageReader;

use crate::error::{AppError, AppResult};

/// A sanitized, ready-to-upload payload. Carries no file path, no original
/// filename, no metadata — just the raw bytes and a MIME type.
#[derive(Debug, Clone)]
pub struct ScrubbedPayload {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl ScrubbedPayload {
    /// Standard, padded base64 of the sanitized bytes — ready for an
    /// `inlineData` part in a Vertex AI request.
    pub fn base64(&self) -> String {
        base64_encode(&self.bytes)
    }
}

/// Tiny dependency-free base64 encoder (standard alphabet, padded).
pub fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Decode an image and re-encode it as clean PNG. Decoding to a raw pixel
/// buffer and re-encoding inherently discards EXIF/XMP/ICC and any tracking
/// chunks — the re-encoded file contains only pixels.
pub fn scrub_image_bytes(input: &[u8]) -> AppResult<ScrubbedPayload> {
    let reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(AppError::Io)?;
    let decoded = reader.decode()?;

    let mut out = Cursor::new(Vec::new());
    decoded.write_to(&mut out, image::ImageFormat::Png)?;

    Ok(ScrubbedPayload {
        bytes: out.into_inner(),
        mime_type: "image/png".to_string(),
    })
}

/// Convenience: scrub an image file from disk.
pub fn scrub_image_file(path: &Path) -> AppResult<ScrubbedPayload> {
    let raw = std::fs::read(path)?;
    scrub_image_bytes(&raw)
}

/// Strip identifying metadata from a PDF (the `/Info` dictionary holding
/// Author/Producer/Creator/timestamps) before it leaves the machine, then
/// re-serialize. Gemini accepts PDF inline, so we keep it as application/pdf.
pub fn scrub_pdf_file(path: &Path) -> AppResult<ScrubbedPayload> {
    let mut doc = lopdf::Document::load(path)
        .map_err(|e| AppError::Other(format!("could not read PDF: {e}")))?;

    // Drop the document information dictionary (author, producer, dates, ...).
    doc.trailer.remove(b"Info");

    // Drop the XMP metadata stream referenced from the catalog, if present.
    if let Ok(catalog) = doc.catalog() {
        if let Ok(meta_ref) = catalog.get(b"Metadata").and_then(|o| o.as_reference()) {
            doc.objects.remove(&meta_ref);
        }
    }

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes)
        .map_err(|e| AppError::Other(format!("could not write scrubbed PDF: {e}")))?;

    Ok(ScrubbedPayload {
        bytes,
        mime_type: "application/pdf".to_string(),
    })
}

/// Dispatch a file headed for the vision pipeline to the right scrubber by
/// extension. Returns a sanitized payload ready for an `inlineData` part.
pub fn scrub_for_vision(path: &Path) -> AppResult<ScrubbedPayload> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => scrub_pdf_file(path),
        // HEIC/HEIF can't be decoded without libheif; ask for a common format.
        "heic" | "heif" => Err(AppError::Other(
            "HEIC/HEIF isn't supported yet — please convert the photo to JPG or PNG \
             (most phones can export/share as JPEG)."
                .into(),
        )),
        // Everything else (jpg/jpeg/png/webp/bmp/tiff) re-encodes to clean PNG.
        _ => scrub_image_file(path),
    }
}

/// Strip metadata from extracted digital text. Document parsing (.docx/.xlsx/
/// .pptx) happens in the digital pipeline; by the time text reaches the cloud
/// it must be metadata-free. This normalizes away zero-width/tracking markers
/// and control characters that can carry fingerprints.
pub fn scrub_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            // Drop zero-width and BOM-style markers often used for tracking.
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        // Keep normal whitespace; drop other C0/C1 control chars.
        .filter(|c| !c.is_control() || matches!(*c, '\n' | '\r' | '\t'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_scrub_removes_zero_width() {
        let dirty = "hel\u{200B}lo\u{FEFF} world";
        assert_eq!(scrub_text(dirty), "hello world");
    }

    #[test]
    fn reencoded_png_has_no_exif_marker() {
        // 2x2 red image encoded as PNG, then scrubbed -> still valid PNG,
        // and contains no JPEG/EXIF APP1 marker.
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut buf = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        let scrubbed = scrub_image_bytes(&buf.into_inner()).unwrap();
        assert_eq!(scrubbed.mime_type, "image/png");
        assert!(!scrubbed.bytes.windows(4).any(|w| w == b"Exif"));
    }
}
