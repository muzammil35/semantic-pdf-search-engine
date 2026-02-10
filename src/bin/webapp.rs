use axum::{
    Router, body::Bytes, extract::{Json, Multipart, Path, Query}, response::{Html, IntoResponse},
    extract::DefaultBodyLimit,
    routing::{get, post}
};
use axum::{http::StatusCode, http::header};
use serde::Deserialize;
use serde::Serialize;
use std::net::SocketAddr;
use std::fs;
use vb::chunk;
use vb::embed;
use vb::extract;
use vb::qdrant;
use std::time::Instant;


#[tokio::main]
async fn main() {
    // Build our application with routes
    let app = Router::new()
        .route("/", get(index))
        .route("/upload", post(handle_upload))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/webapp/index.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_upload(mut multipart: Multipart) -> Result<impl IntoResponse, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let filename = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "pdf" {
            println!("Received file: {} ({} bytes)", filename, data.len());
            
            // Spawn background processing
            let data_clone = data.to_vec();
            let filename_clone = filename.clone();
            tokio::task::spawn(async move {
                let start = Instant::now();
                match process_file(&filename_clone, data_clone.into()).await {
                    Ok(_) => println!("Processing done: {:?}", start.elapsed()),
                    Err(e) => eprintln!("Processing failed: {:?}", e),
                }
            });
            
            // Return the PDF immediately in the response
            return Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/pdf")],
                data.to_vec()
            ));
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn process_file(filename: &str, pdf_data: Bytes) -> Result<(), Box<dyn std::error::Error>> {

    let chunks = chunk::extract_and_chunk(chunk::PdfSource::Bytes(pdf_data.to_vec()))?;
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let client = qdrant::setup_qdrant(&embedded_chunks, filename).await?;
    let response = qdrant::store_embeddings(&client, filename, embedded_chunks).await?;

    println!("File processed successfully!");
    dbg!(response);

    Ok(())
}
