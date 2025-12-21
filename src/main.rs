use anyhow;
use clap::Parser;
use clap::{Arg, command};
use clap::{ArgAction, arg};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lopdf::{Document, Object, ObjectId};
use qdrant_client::Qdrant;
use qdrant_client::QdrantError;
use qdrant_client::qdrant::Distance;
use qdrant_client::qdrant::UpsertPointsBuilder;
use qdrant_client::qdrant::{CreateCollectionBuilder, VectorParamsBuilder};
use qdrant_client::qdrant::{PointStruct, Value};
use std::collections::HashMap;
use std::io::{self, Write};

pub mod chunk;
pub mod embed;
pub mod extract;
pub mod render;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path to be stored
    #[arg(short, long)]
    file: String,
}

fn main_() {
    let home = dirs::home_dir().unwrap();
    println!("User home directory: {}", home.display());
    let _ = render::render();
}

#[tokio::main]
async fn main()  {
    let result = run().await;
    println!("{:?}", result);
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let matches = command!()
            .arg(
                Arg::new("file")
                    .short('f')
                    .long("file")
                    .action(ArgAction::Append),
            )
            .arg(
                Arg::new("search")
                    .short('s')
                    .long("search")
                    .action(ArgAction::Append),
            )
            .try_get_matches_from(std::env::args());

        let matches = match matches {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}", e);
                prompt_for_next()?;
                continue;
            }
        };

        let file_args = matches
            .get_many::<String>("file")
            .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>());

        let search_args = matches
            .get_many::<String>("search")
            .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>());

        // Handle file command
        if let Some(file_paths) = file_args {
            println!("file paths: {:?}", &file_paths);

            println!("extracting text");
            let res = extract::extract_text(file_paths);

            println!("getting text chunks");
            let chunks = chunk::create_chunks(&res[0].pages);

            println!("getting embedded chunks");
            let embedded_chunks = embed::get_embeddings(chunks)?;

            println!("getting client");
            let client = setup_qdrant(&embedded_chunks).await?;
            println!("got client");

            println!("getting response");
            let response = store_embeddings(&client, "test_collection3", embedded_chunks).await?;
            dbg!(response);
        }

        // Handle search command
        if let Some(search_paths) = search_args {
            println!("search paths: {:?}", &search_paths);
            // Add your search logic here
        }

        prompt_for_next()?;
    }
}

fn prompt_for_next() -> Result<(), Box<dyn std::error::Error>> {
    print!("\nEnter command (or Ctrl+C to exit): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(())
}




async fn setup_qdrant(embedded_chunks: &embed::Embeddings) -> Result<Qdrant, QdrantError> {
    let client = Qdrant::from_url("http://localhost:6334").build()?;

    client
        .create_collection(
            CreateCollectionBuilder::new("test_collection3").vectors_config(
                VectorParamsBuilder::new(embedded_chunks.get_dim() as u64, Distance::Dot),
            ),
        )
        .await?;

    Ok(client)
}

async fn store_embeddings(
    client: &Qdrant,
    collection_name: &str,
    embeddings: embed::Embeddings,
) -> Result<(), QdrantError> {
    // Ensure both vectors have the same length
    assert_eq!(
        embeddings.original.len(),
        embeddings.embedded.len(),
        "Original and embedded vectors must have the same length"
    );

    let points: Vec<PointStruct> = embeddings
        .original
        .into_iter()
        .zip(embeddings.embedded)
        .enumerate()
        .map(|(id, (chunk, embedding))| {
            // Create payload with original chunk data from Embeddings.original
            let mut payload = HashMap::new();
            payload.insert("text".to_string(), Value::from(chunk.content.clone()));
            // Add any other chunk fields you want to store

            PointStruct::new(
                id as u64, // Use index as ID
                embedding, payload,
            )
        })
        .collect();

    // Insert points into collection
    let response = client
        .upsert_points(UpsertPointsBuilder::new("test_collection3", points).wait(true))
        .await?;
    dbg!(response);

    Ok(())
}
