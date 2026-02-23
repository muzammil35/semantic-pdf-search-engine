// src/pdf.rs

use std::sync::OnceLock;
use anyhow::Result;
use pdfium_render::prelude::*;
use crate::types::CharBbox;

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

pub fn get_pdfium() -> &'static Pdfium {
    PDFIUM.get_or_init(|| {
        Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .expect("Failed to bind to pdfium library"),
        )
    })
}

pub fn expand_ligatures(pdf_idx: usize, ch: char) -> Vec<(usize, char)> {
    match ch {
        '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{2060}' => vec![],
        '\u{FB00}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f')],
        '\u{FB01}' => vec![(pdf_idx, 'f'), (pdf_idx, 'i')],
        '\u{FB02}' => vec![(pdf_idx, 'f'), (pdf_idx, 'l')],
        '\u{FB03}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f'), (pdf_idx, 'i')],
        '\u{FB04}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f'), (pdf_idx, 'l')],
        '\u{FB05}' | '\u{FB06}' => vec![(pdf_idx, 's'), (pdf_idx, 't')],
        _ => vec![(pdf_idx, ch)],
    }
}

pub fn extract_char_bboxes(
    text_page: &PdfPageText,
    pdf_char_indices: &[usize],
) -> Result<Vec<CharBbox>> {
    let chars = text_page.chars();
    let mut result: Vec<CharBbox> = Vec::new();
    let mut current: Option<CharBbox> = None;

    for &idx in pdf_char_indices {
        let ch = match chars.get(idx) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if ch.unicode_char().map_or(false, |c| c.is_whitespace()) {
            if let Some(r) = current.take() {
                result.push(r);
            }
            continue;
        }

        let bounds = ch.loose_bounds()?;
        let x = bounds.left().value;
        let y = bounds.bottom().value;
        let width = (bounds.right() - bounds.left()).value;
        let height = (bounds.top() - bounds.bottom()).value;

        if let Some(ref mut cur) = current {
            if (cur.y - y).abs() < 2.0 {
                // Same line: extend the rect rightward
                cur.width = (x + width) - cur.x;
                cur.height = cur.height.max(height);
                continue;
            }
            result.push(current.take().unwrap());
        }
        current = Some(CharBbox { x, y, width, height });
    }

    if let Some(r) = current {
        result.push(r);
    }

    Ok(result)
}

pub fn snap_to_sentence_boundaries(
    char_entries: &[(usize, char)],
    start: usize,
    end: usize,
) -> (usize, usize) {
    let chars: Vec<char> = char_entries.iter().map(|(_, c)| *c).collect();
    let len = chars.len();

    let is_sentence_end = |c: char| matches!(c, '.' | '!' | '?');
    let is_whitespace = |c: char| matches!(c, ' ' | '\t' | '\r' | '\n');

    let new_start = if start == 0 {
        0
    } else {
        let mut i = start.saturating_sub(1);
        loop {
            if is_sentence_end(chars[i]) {
                let mut j = i + 1;
                while j < len && (is_whitespace(chars[j]) || is_sentence_end(chars[j])) {
                    j += 1;
                }
                break j;
            }
            if i == 0 {
                break 0;
            }
            i -= 1;
        }
    };

    let new_end = {
        let mut i = end;
        while i < len && !is_sentence_end(chars[i]) {
            i += 1;
        }
        while i + 1 < len && matches!(chars[i + 1], '"' | '\'' | ')') {
            i += 1;
        }
        (i + 1).min(len)
    };

    (new_start, new_end)
}