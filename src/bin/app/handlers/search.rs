// src/handlers/search.rs

use anyhow::Result;
use axum::{
    extract::{Query, State},
    Json,
};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::value::Kind;
use vb::{fuzzy, qdrant};

use crate::errors::AppError;
use crate::pdf::{expand_ligatures, extract_char_bboxes, get_pdfium, snap_to_sentence_boundaries};
use crate::types::{AppState, PageHighlight, SearchResult, SearchWithBboxQuery};

pub async fn search_with_bboxes(
    State(state): State<AppState>,
    Query(params): Query<SearchWithBboxQuery>,
) -> Result<Json<Vec<PageHighlight>>, AppError> {
    if params.q.is_empty() {
        return Ok(Json(vec![]));
    }

    let file_name = resolve_file_name(&state, &params.id).await?;
    let search_results = run_search_api(&state.qdrant, &file_name, &params.q).await?;

    if search_results.is_empty() {
        return Ok(Json(vec![]));
    }

    let bytes = get_pdf_bytes(&state, &params.id).await?;
    let highlights = compute_highlights(&bytes, &search_results)?;

    Ok(Json(highlights))
}

async fn resolve_file_name(state: &AppState, id: &str) -> Result<String> {
    state
        .id_map
        .read()
        .await
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No file found for id: {}", id))
}

async fn get_pdf_bytes(state: &AppState, id: &str) -> Result<Vec<u8>> {
    state
        .bytes_map
        .read()
        .await
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No bytes found for id: {}", id))
}

async fn run_search_api(client: &Qdrant, file_name: &str, query: &str) -> Result<Vec<SearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    let resp = qdrant::run_query(client, "embedded_pdfs", file_name, query).await?;

    let results = resp
        .result
        .into_iter()
        .filter_map(|point| {
            let text = point.payload.get("text")?.as_str()?.to_string();
            let page = match &point.payload.get("page")?.kind {
                Some(Kind::DoubleValue(d)) => *d as i64,
                Some(Kind::IntegerValue(i)) => *i,
                Some(Kind::StringValue(s)) => s.parse().unwrap_or(1),
                _ => 1,
            };
            Some(SearchResult { page, text })
        })
        .collect();

    Ok(results)
}

fn compute_highlights(bytes: &[u8], search_results: &[SearchResult]) -> Result<Vec<PageHighlight>> {
    let pdfium = get_pdfium();
    let doc = pdfium.load_pdf_from_byte_slice(bytes, None)?;
    let mut highlights: Vec<PageHighlight> = Vec::new();

    for search_result in search_results {
        let page_idx = (search_result.page - 1) as u16;

        let page = match doc.pages().get(page_idx) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let text_page = match page.text() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let needle_chars: Vec<char> = search_result.text.to_lowercase().chars().collect();

        let char_entries: Vec<(usize, char)> = text_page
            .chars()
            .iter()
            .enumerate()
            .flat_map(|(pdf_idx, c)| {
                c.unicode_char()
                    .map(|ch| expand_ligatures(pdf_idx, ch))
                    .unwrap_or_default()
            })
            .collect();

        let fuzzy_matches = fuzzy::fuzzy_search(&char_entries, &needle_chars, 0.85);

        let snapped_matches: Vec<(usize, usize, f32)> = fuzzy_matches
            .into_iter()
            .map(|(start, end, score)| {
                let (new_start, new_end) = snap_to_sentence_boundaries(&char_entries, start, end);
                (new_start, new_end, score)
            })
            .collect();

        for (entry_start, entry_end, _score) in snapped_matches {
            let fuzzy_str: String = char_entries[entry_start..entry_end]
                .iter()
                .map(|(_, ch)| *ch)
                .collect();
            println!("[fuzzy match]   {:?}", fuzzy_str);
            println!("[backend match] {:?}", search_result.text);

            let pdf_char_indices: Vec<usize> = char_entries[entry_start..entry_end]
                .iter()
                .map(|(pdf_idx, _)| *pdf_idx)
                .collect();

            if let Ok(rects) = extract_char_bboxes(&text_page, &pdf_char_indices) {
                if !rects.is_empty() {
                    highlights.push(PageHighlight {
                        page: search_result.page as usize,
                        rects,
                    });
                }
            }
        }
    }

    Ok(highlights)
}