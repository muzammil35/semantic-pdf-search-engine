use anyhow::Result;
use axum::http::StatusCode;
use axum::{
    Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    extract::{Json, Multipart, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use qdrant_client::Qdrant;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;
use pdfium_render::prelude::*;
use vb::chunk;
use vb::embed;
use vb::fuzzy;
use vb::qdrant;

// --- Pdfium singleton: initialized once, reused across requests ---
static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

// Shared state for ID to filename mapping
type IdToFilenameMap = Arc<RwLock<HashMap<String, String>>>;
type IdToBytesMap = Arc<RwLock<HashMap<String, Vec<u8>>>>;

#[derive(Serialize)]
struct SearchResult {
    page: i64,
    text: String,
    //rects: Vec<Rect>
}

#[derive(Serialize)]
struct UploadResponse {
    id: String,
}

#[derive(Clone)]
struct AppState {
    id_map: IdToFilenameMap,
    bytes_map: IdToBytesMap,
    qdrant: Arc<Qdrant>,
}

#[tokio::main]
async fn main() {
    let id_map: IdToFilenameMap = Arc::new(RwLock::new(HashMap::new()));
    let bytes_map: IdToBytesMap = Arc::new(RwLock::new(HashMap::new()));
    let qdrant_client = Qdrant::from_url("http://localhost:6334")
        .build()
        .expect("Failed to connect to Qdrant");
    qdrant::delete_all_collections(&qdrant_client).await;

    let _ = qdrant::init_collection(&qdrant_client, "embedded_pdfs").await;

    let shared_state = AppState {
        id_map,
        bytes_map,
        qdrant: Arc::new(qdrant_client),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/upload", post(handle_upload))
        .route("/api/search", get(search_with_bboxes))
        .nest_service("/static", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) 
        .with_state(shared_state.clone());

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C handler");
}

async fn index() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/webapp/render.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let id_map = state.id_map.clone();
        let bytes_map = state.bytes_map.clone();
        let qdrant = state.qdrant.clone();
        let name = field.name().unwrap().to_string();
        let filename = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "pdf" {
            println!("Received file: {} ({} bytes)", filename, data.len());

            // Generate unique ID for this upload
            let id = Uuid::new_v4().to_string();

            // Store a placeholder in the map
            {
                let mut map = id_map.write().await;
                map.insert(id.clone(), "processing".to_string());
            }
            {
                let mut map = bytes_map.write().await;
                map.insert(id.clone(), data.to_vec());
            }

            // Clone data for background task
            let data_clone = data.to_vec();
            let filename_clone = filename.clone();
            let id_clone = id.clone();
            let id_map_clone = id_map.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                match process_file(&filename_clone, data_clone.into(), qdrant).await {
                    Ok(unique_filename) => {
                        println!("Processing done: {:?}", start.elapsed());

                        // Update the map with the actual result
                        let mut map = id_map_clone.write().await;
                        map.insert(id_clone, unique_filename);
                    }
                    Err(e) => {
                        eprintln!("Processing failed: {:?}", e);
                        let mut map = id_map_clone.write().await;
                        map.insert(id_clone, "failed".to_string());
                    }
                }
            });

            // Return immediately with the ID
            return Ok((StatusCode::OK, Json(UploadResponse { id })));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn process_file(filename: &str, pdf_data: Bytes, client: Arc<Qdrant>) -> Result<String> {
    let chunks = chunk::extract_and_chunk(chunk::PdfSource::Bytes(pdf_data.to_vec()))?;
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let unique_filename =
        qdrant::store_embeddings(&client, "embedded_pdfs", filename, embedded_chunks).await?;

    println!("File processed successfully!");

    Ok(unique_filename)
}

// API version of search (returns JSON)
async fn run_search_api(
    client: &Qdrant,
    file_name: &str,
    query: String,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    let resp = qdrant::run_query(&client, "embedded_pdfs", file_name, &query).await?;

    let mut results = Vec::new();

    for point in resp.result {
        if let Some(text_value) = point.payload.get("text") {
            if let Some(page_value) = point.payload.get("page") {
                if let Some(text) = text_value.as_str() {
                    // Extract page number - handle different number types
                    use qdrant_client::qdrant::value::Kind;

                    let page = match &page_value.kind {
                        Some(Kind::DoubleValue(d)) => *d as i64,
                        Some(Kind::IntegerValue(i)) => *i,
                        Some(Kind::StringValue(s)) => s.parse::<i64>().unwrap_or(1),
                        _ => 1,
                    };

                    results.push(SearchResult {
                        page,
                        text: text.to_string(),
                    });
                }
            }
        }
    }

    Ok(results)
}


#[derive(Deserialize)]
pub struct BboxRequest {
    pub id: String,
    pub page: usize,  // 1-indexed
    pub start: usize, // char offset into page text (from frontend index)
    pub end: usize,
}

#[derive(Serialize)]
pub struct CharBbox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize)]
pub struct BboxResponse {
    pub rects: Vec<CharBbox>,
}


fn get_pdfium() -> &'static Pdfium {
    PDFIUM.get_or_init(|| {
        Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .expect("Failed to bind to pdfium library"),
        )
    })
}

#[derive(Deserialize)]
struct SearchWithBboxQuery {
    id: String,
    q: String,
}

#[derive(Serialize)]
pub struct PageHighlight {
    pub page: usize, // 1-indexed
    pub rects: Vec<CharBbox>,
}

async fn search_with_bboxes(
    State(state): State<AppState>,
    Query(params): Query<SearchWithBboxQuery>,
) -> Json<Vec<PageHighlight>> {
    let needle = params.q.to_lowercase();
    if needle.is_empty() {
        return Json(vec![]);
    }

    // 1. Get the filename for this ID
    let file_name = {
        let map = state.id_map.read().await;
        match map.get(&params.id).cloned() {
            Some(name) => name,
            None => return Json(vec![]),
        }
    };

    // 2. Use search_handler's underlying logic to get semantic search results
    let search_results = match run_search_api(&state.qdrant, &file_name, params.q.clone()).await {
        Ok(results) => results,
        Err(_) => return Json(vec![]),
    };

    if search_results.is_empty() {
        return Json(vec![]);
    }

    // 3. Get the PDF bytes
    let bytes = {
        let store = state.bytes_map.read().await;
        store.get(&params.id).cloned()
    };
    let Some(bytes) = bytes else {
        return Json(vec![]);
    };

    let pdfium = get_pdfium();
    let doc = match pdfium.load_pdf_from_byte_slice(&bytes, None) {
        Ok(d) => d,
        Err(_) => return Json(vec![]),
    };

    let mut highlights: Vec<PageHighlight> = Vec::new();

    // 4. For each search result, fuzzy-search only within that page's text
    for search_result in &search_results {
        let page_idx = (search_result.page - 1) as u16; // convert 1-indexed to 0-indexed

        let page = match doc.pages().get(page_idx) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let text_page = match page.text() {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Use the semantic search result text as the needle, not the raw query
        let needle_chars: Vec<char> = search_result
            .text
            .to_lowercase()
            .chars()
            .map(|c| c.to_lowercase().next().unwrap_or(c))
            .collect();

        // Build char_entries with ligature expansion / invisible char stripping
        let char_entries: Vec<(usize, char)> = text_page
            .chars()
            .iter()
            .enumerate()
            .flat_map(|(pdf_idx, c)| {
                let Some(ch) = c.unicode_char() else {
                    return vec![];
                };
                match ch {
                    '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{2060}' => {
                        vec![]
                    }
                    '\u{FB00}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f')],
                    '\u{FB01}' => vec![(pdf_idx, 'f'), (pdf_idx, 'i')],
                    '\u{FB02}' => vec![(pdf_idx, 'f'), (pdf_idx, 'l')],
                    '\u{FB03}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f'), (pdf_idx, 'i')],
                    '\u{FB04}' => vec![(pdf_idx, 'f'), (pdf_idx, 'f'), (pdf_idx, 'l')],
                    '\u{FB05}' | '\u{FB06}' => vec![(pdf_idx, 's'), (pdf_idx, 't')],
                    _ => vec![(pdf_idx, ch)],
                }
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
            // Print the fuzzy-matched string from the PDF
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

            match extract_char_bboxes(&text_page, &pdf_char_indices) {
                Ok(rects) if !rects.is_empty() => {
                    highlights.push(PageHighlight {
                        page: search_result.page as usize,
                        rects,
                    });
                }
                _ => {}
            }
        }
    }

    Json(highlights)
}

/// Extract and merge bounding boxes for specific PDF char indices on a page.
fn extract_char_bboxes(
    text_page: &PdfPageText,
    pdf_char_indices: &[usize],
) -> anyhow::Result<Vec<CharBbox>> {
    let chars = text_page.chars();
    let mut result: Vec<CharBbox> = Vec::new();
    let mut current: Option<CharBbox> = None;

    for &idx in pdf_char_indices {
        let ch = match chars.get(idx as usize) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip whitespace — don't highlight spaces, but do break the current rect
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

        match current {
            Some(ref mut cur) if (cur.y - y).abs() < 2.0 => {
                // Same line: extend the current rect rightward
                cur.width = (x + width) - cur.x;
                cur.height = cur.height.max(height);
            }
            Some(_) => {
                result.push(current.take().unwrap());
                current = Some(CharBbox {
                    x,
                    y,
                    width,
                    height,
                });
            }
            None => {
                current = Some(CharBbox {
                    x,
                    y,
                    width,
                    height,
                });
            }
        }
    }

    if let Some(r) = current {
        result.push(r);
    }

    Ok(result)
}

/// Adjust a match's start/end to align with sentence boundaries.
/// `char_entries` is your original &[(usize, char)] slice.
pub fn snap_to_sentence_boundaries(
    char_entries: &[(usize, char)],
    start: usize,
    end: usize,
) -> (usize, usize) {
    let chars: Vec<char> = char_entries.iter().map(|(_, c)| *c).collect();
    let len = chars.len();

    // Sentence-ending punctuation
    let is_sentence_end = |c: char| matches!(c, '.' | '!' | '?');
    // Whitespace/newline
    let is_whitespace = |c: char| matches!(c, ' ' | '\t' | '\r' | '\n');

    // --- Snap start: walk backward to find the end of the previous sentence,
    //     then skip whitespace to land on the first char of this sentence.
    let new_start = if start == 0 {
        0
    } else {
        // Walk backward from `start` looking for a sentence-ending punctuation
        let mut i = start.saturating_sub(1);
        loop {
            if is_sentence_end(chars[i]) {
                // Skip forward past punctuation and whitespace
                let mut j = i + 1;
                while j < len && (is_whitespace(chars[j]) || is_sentence_end(chars[j])) {
                    j += 1;
                }
                break j;
            }
            if i == 0 {
                break 0; // We're already at the document start
            }
            i -= 1;
        }
    };

    // --- Snap end: walk forward to find the next sentence-ending punctuation.
    let new_end = {
        let mut i = end;
        while i < len && !is_sentence_end(chars[i]) {
            i += 1;
        }
        // Include the punctuation itself (and any closing quotes/parens)
        while i + 1 < len && matches!(chars[i + 1], '"' | '\'' | ')') {
            i += 1;
        }
        (i + 1).min(len)
    };

    (new_start, new_end)
}
