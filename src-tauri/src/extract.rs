//! Digital extraction pipeline (spec §Phase 2). Parses Office/text files
//! NATIVELY on-device — no vision AI, no cloud — and maps content to stable
//! location references the grader can cite:
//!   * .docx -> paragraphs of Markdown
//!   * .xlsx -> `Cell A1: value` lines (per sheet)
//!   * .pptx -> `## Slide N` sections
//!   * .txt  -> raw text
//!
//! Output is plain Markdown; the caller scrubs it before any upload.

use std::io::Read;
use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

use crate::error::{AppError, AppResult};

fn other<E: std::fmt::Display>(ctx: &str) -> impl Fn(E) -> AppError + '_ {
    move |e| AppError::Other(format!("{ctx}: {e}"))
}

/// Route a digital file to its parser by extension.
pub fn extract_digital(path: &Path) -> AppResult<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" => Ok(std::fs::read_to_string(path)?),
        "docx" => extract_docx(path),
        "pptx" => extract_pptx(path),
        "xlsx" => extract_xlsx(path),
        other_ext => Err(AppError::Other(format!(
            "unsupported digital format: .{other_ext}"
        ))),
    }
}

// ── .docx ───────────────────────────────────────────────────────────────────

fn extract_docx(path: &Path) -> AppResult<String> {
    let xml = read_zip_entry(path, "word/document.xml")?;
    let text = xml_text_with_paragraphs(&xml, b"t", b"p")?;
    Ok(text.trim().to_string())
}

// ── .pptx ───────────────────────────────────────────────────────────────────

fn extract_pptx(path: &Path) -> AppResult<String> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(other("open pptx"))?;

    // Collect slide entries and order them by slide number.
    let mut slides: Vec<(u32, String)> = Vec::new();
    let names: Vec<String> = zip.file_names().map(str::to_string).collect();
    for name in names {
        if let Some(n) = slide_number(&name) {
            let mut buf = String::new();
            zip.by_name(&name)
                .map_err(other("read slide"))?
                .read_to_string(&mut buf)?;
            slides.push((n, buf));
        }
    }
    slides.sort_by_key(|(n, _)| *n);

    let mut out = String::new();
    for (n, xml) in slides {
        // PowerPoint text runs are <a:t>; paragraphs are <a:p>.
        let body = xml_text_with_paragraphs(&xml, b"t", b"p")?;
        out.push_str(&format!("## Slide {n}\n{}\n\n", body.trim()));
    }
    Ok(out.trim().to_string())
}

fn slide_number(name: &str) -> Option<u32> {
    let file = name.strip_prefix("ppt/slides/slide")?;
    let num = file.strip_suffix(".xml")?;
    num.parse::<u32>().ok()
}

// ── .xlsx ───────────────────────────────────────────────────────────────────

fn extract_xlsx(path: &Path) -> AppResult<String> {
    let mut wb = open_workbook_auto(path).map_err(other("open xlsx"))?;
    let mut out = String::new();

    for sheet in wb.sheet_names().to_owned() {
        let range = wb
            .worksheet_range(&sheet)
            .map_err(other("read worksheet"))?;
        let (base_row, base_col) = range.start().unwrap_or((0, 0));

        out.push_str(&format!("## Sheet: {sheet}\n"));
        for (r, c, cell) in range.used_cells() {
            if matches!(cell, Data::Empty) {
                continue;
            }
            let a1 = a1_ref(base_col + c as u32, base_row + r as u32);
            out.push_str(&format!("Cell {a1}: {cell}\n"));
        }
        out.push('\n');
    }
    Ok(out.trim().to_string())
}

/// Convert 0-based (col, row) to an A1 reference, e.g. (2, 4) -> "C5".
fn a1_ref(col: u32, row: u32) -> String {
    let mut letters = String::new();
    let mut c = col + 1; // 1-based for the column algorithm
    while c > 0 {
        let rem = ((c - 1) % 26) as u8;
        letters.insert(0, (b'A' + rem) as char);
        c = (c - 1) / 26;
    }
    format!("{letters}{}", row + 1)
}

// ── shared XML text walker ────────────────────────────────────────────────────

/// Walk an OOXML part collecting text from `<*:text_tag>` runs, inserting a
/// newline at the end of each `<*:para_tag>`. Namespace prefixes are ignored by
/// matching on the local name.
fn xml_text_with_paragraphs(
    xml: &str,
    text_tag: &[u8],
    para_tag: &[u8],
) -> AppResult<String> {
    // Default config does not trim text — exactly what we want for runs.
    let mut reader = XmlReader::from_str(xml);

    let mut out = String::new();
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == text_tag {
                    in_text = true;
                }
            }
            Ok(Event::End(e)) => {
                let local = e.local_name();
                if local.as_ref() == text_tag {
                    in_text = false;
                } else if local.as_ref() == para_tag {
                    out.push('\n');
                }
            }
            Ok(Event::Text(t)) if in_text => {
                out.push_str(&t.unescape().map_err(other("xml text"))?);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(AppError::Other(format!("xml parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

// ── zip helper ────────────────────────────────────────────────────────────────

fn read_zip_entry(path: &Path, entry: &str) -> AppResult<String> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(other("open archive"))?;
    let mut content = String::new();
    zip.by_name(entry)
        .map_err(other("locate entry"))?
        .read_to_string(&mut content)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_references() {
        assert_eq!(a1_ref(0, 0), "A1");
        assert_eq!(a1_ref(2, 4), "C5");
        assert_eq!(a1_ref(26, 0), "AA1");
        assert_eq!(a1_ref(27, 9), "AB10");
    }

    #[test]
    fn docx_style_xml_walk() {
        let xml = r#"<w:document><w:body>
            <w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t> world</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second line</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let text = xml_text_with_paragraphs(xml, b"t", b"p").unwrap();
        assert_eq!(text.trim(), "Hello world\nSecond line");
    }
}
