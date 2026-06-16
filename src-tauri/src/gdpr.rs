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
