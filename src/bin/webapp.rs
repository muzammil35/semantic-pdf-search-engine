use axum::{
    extract::{Path, Query, Json, Multipart},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use axum::{http::StatusCode};
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use std::fs;
use vb::chunk;
use vb::embed;
use vb::extract;
use vb::qdrant;

#[tokio::main]
async fn main() {
    // Build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/upload", post(handle_upload));

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/index.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_upload(mut multipart: Multipart) {
    let mut pdf_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let filename = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "pdf" {
            pdf_data = Some((filename.clone(), data.to_vec()));
            println!("Received file: {} ({} bytes)", filename, data.len());
        }
    }
}
