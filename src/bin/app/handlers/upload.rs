// src/handlers/upload.rs

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use std::time::Instant;
use qdrant_client::Qdrant;
use uuid::Uuid;
use vb::{chunk, embed, qdrant};

use crate::errors::AppError;
use crate::types::{AppState, UploadResponse};

pub async fn handle_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    while let Some(field) = multipart.next_field().await? {
        if field.name() != Some("pdf") {
            continue;
        }

        let filename = field.file_name().ok_or_else(|| anyhow::anyhow!("Missing filename"))?.to_string();
        let data = field.bytes().await?;

        println!("Received file: {} ({} bytes)", filename, data.len());

        let id = Uuid::new_v4().to_string();

        {
            let mut map = state.id_map.write().await;
            map.insert(id.clone(), "processing".to_string());
        }
        {
            let mut map = state.bytes_map.write().await;
            map.insert(id.clone(), data.to_vec());
        }

        let data_clone = data.to_vec();
        let filename_clone = filename.clone();
        let id_clone = id.clone();
        let id_map_clone = state.id_map.clone();
        let qdrant = state.qdrant.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            match process_file(&filename_clone, data_clone.into(), qdrant).await {
                Ok(unique_filename) => {
                    println!("Processing done: {:?}", start.elapsed());
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

        return Ok((StatusCode::OK, Json(UploadResponse { id })));
    }

    Err(AppError::from(anyhow::anyhow!("No PDF field found in multipart body")))
}

async fn process_file(filename: &str, pdf_data: Bytes, client: Arc<Qdrant>) -> Result<String> {
    let chunks = chunk::extract_and_chunk(chunk::PdfSource::Bytes(pdf_data.to_vec()))?;
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let unique_filename =
        qdrant::store_embeddings(&client, "embedded_pdfs", filename, embedded_chunks).await?;

    println!("File processed successfully!");

    Ok(unique_filename)
}