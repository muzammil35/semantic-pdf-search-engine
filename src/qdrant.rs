use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::Distance;
use qdrant_client::qdrant::SearchPointsBuilder;
use qdrant_client::qdrant::SearchResponse;
use qdrant_client::qdrant::UpsertPointsBuilder;
use qdrant_client::qdrant::{Condition, CreateCollectionBuilder, Filter, VectorParamsBuilder};
use qdrant_client::qdrant::{PointStruct, Value};
use std::collections::HashMap;

use crate::embed;

pub async fn setup_qdrant() -> Result<Qdrant, QdrantError> {
    let client = Qdrant::from_url("http://localhost:6334").build()?;
    client
        .create_collection(CreateCollectionBuilder::new("repl").vectors_config(
            VectorParamsBuilder::new(embed::get_dim() as u64, Distance::Dot),
        ))
        .await?;

    Ok(client)
}

pub async fn init_collection(client: &Qdrant, collection_name: &str) -> Result<(), QdrantError> {
    client
        .create_collection(
            CreateCollectionBuilder::new(collection_name).vectors_config(VectorParamsBuilder::new(
                embed::get_dim() as u64,
                Distance::Dot,
            )),
        )
        .await?;
    Ok(())
}

pub async fn store_embeddings(
    client: &Qdrant,
    collection_name: &str,
    filename: &str,
    embeddings: embed::Embeddings,
) -> Result<String, QdrantError> {
    assert_eq!(
        embeddings.original.len(),
        embeddings.embedded.len(),
        "Original and embedded vectors must have the same length"
    );

    let unique_filename = format!("{}_{}", filename, uuid::Uuid::new_v4());

    let points: Vec<PointStruct> = embeddings
        .original
        .into_iter()
        .zip(embeddings.embedded)
        .map(|(chunk, embedding)| {
            let mut payload = HashMap::new();
            payload.insert("filename".to_string(), Value::from(unique_filename.clone()));
            payload.insert("text".to_string(), Value::from(chunk.content.clone()));
            payload.insert("page".to_string(), Value::from(chunk.page as f32));
            PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload)
        })
        .collect();

    let response = client
        .upsert_points(UpsertPointsBuilder::new(collection_name, points).wait(true))
        .await?;
    dbg!(response);
    Ok(unique_filename)
}

pub async fn run_query(
    client: &Qdrant,
    collection_name: &str,
    filename: &str,
    query: &str,
) -> Result<SearchResponse, anyhow::Error> {
    let emb_query = match embed::embed_query(query) {
        Ok(embedding) => embedding,
        Err(e) => {
            eprintln!("Failed to embed query: {}", e);
            return Err(e);
        }
    };

    let filename_filter = Filter::must([Condition::matches("filename", filename.to_string())]);

    let search_result = client
        .search_points(
            SearchPointsBuilder::new(collection_name, emb_query, 5)
                .filter(filename_filter)
                .with_payload(true)
                .build(),
        )
        .await?;

    Ok(search_result)
}

pub async fn delete_all_collections(client: &Qdrant) -> Result<(), Box<dyn std::error::Error>> {
    // Get list of all collections
    let collections = client.list_collections().await?;

    // Delete each collection
    for collection in collections.collections {
        println!("Deleting collection: {}", collection.name);
        client.delete_collection(&collection.name).await?;
    }

    println!("All collections deleted!");
    Ok(())
}
