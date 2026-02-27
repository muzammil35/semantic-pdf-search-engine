// src/types.rs

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};

// --- Type aliases for shared state maps ---
pub type IdToFilenameMap = Arc<RwLock<HashMap<String, String>>>;
pub type IdToBytesMap = Arc<RwLock<HashMap<String, Vec<u8>>>>;
pub type IdReadyMap = Arc<RwLock<HashSet<String>>>;

// --- App state shared across handlers ---
#[derive(Clone)]
pub struct AppState {
    pub id_map: IdToFilenameMap,
    pub bytes_map: IdToBytesMap,
    pub qdrant: Arc<Qdrant>,
    pub ready_set: IdReadyMap,
}

// --- Request types ---
#[derive(Deserialize)]
pub struct SearchWithBboxQuery {
    pub id: String,
    pub q: String,
}

// --- Response types ---
#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub page: i64,
    pub text: String,
}

#[derive(Serialize)]
pub struct CharBbox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize)]
pub struct PageHighlight {
    pub page: usize,
    pub rects: Vec<CharBbox>,
}