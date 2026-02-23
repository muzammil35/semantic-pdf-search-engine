// src/main.rs

mod errors;
mod handlers;
mod pdf;
mod types;

use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    response::{Html},
    routing::{get, post},
    http::StatusCode,
};
use qdrant_client::Qdrant;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use vb::qdrant;

use types::{AppState, IdToBytesMap, IdToFilenameMap};
use handlers::{upload::handle_upload, search::search_with_bboxes};

#[tokio::main]
async fn main() {
    let id_map: IdToFilenameMap = Arc::new(RwLock::new(HashMap::new()));
    let bytes_map: IdToBytesMap = Arc::new(RwLock::new(HashMap::new()));

    let qdrant_client = Qdrant::from_url("http://localhost:6334")
        .build()
        .expect("Failed to connect to Qdrant");

    qdrant::delete_all_collections(&qdrant_client).await;
    let _ = qdrant::init_collection(&qdrant_client, "embedded_pdfs").await;

    let state = AppState {
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
        .with_state(state);

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
    fs::read_to_string("static/webapp/render.html")
        .map(Html)
        .map_err(|_| StatusCode::NOT_FOUND)
}
