use clap::{Arg, Parser, command, arg};
use std::io::{self, Write};
use qdrant_client::Qdrant;
use std::fs;

use axum::{
    Router,
    routing::get,
    response::Html,
    Json,
};
use serde::Serialize;

pub mod chunk;
pub mod embed;
pub mod extract;
pub mod qdrant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path to be stored
    #[arg(short, long)]
    file: String,
}

#[tokio::main]
async fn main() {
    let banner = r#"
 ██████╗ ██╗   ██╗███████╗██████╗ ██╗   ██╗
██╔═══██╗██║   ██║██╔════╝██╔══██╗╚██╗ ██╔╝
██║   ██║██║   ██║█████╗  ██████╔╝ ╚████╔╝ 
██║▄▄ ██║██║   ██║██╔══╝  ██╔══██╗  ╚██╔╝  
╚██████╔╝╚██████╔╝███████╗██║  ██║   ██║   
 ╚══▀▀═╝  ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   
"#;

    println!("{}", banner);
    let result = run().await;
    println!("{:?}", result);
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        println!(">");
        let matches = command!()
            .arg(Arg::new("file").short('f').long("file"))
            .arg(Arg::new("search").short('s').long("search"))
            .try_get_matches_from(std::env::args());

        let matches = match matches {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        let file_path = matches.get_one::<String>("file");
        let collection_name = matches.get_one::<String>("search");
            
        // Handle file command
        if let Some(file) = file_path {

            let res = extract::extract_text(file);
            let pages = res.get_pages();
            let chunks = chunk::chunk_everything(pages);
            let embedded_chunks = embed::get_embeddings(chunks)?;
            let client = qdrant::setup_qdrant(&embedded_chunks, file).await?;
            let response = qdrant::store_embeddings(&client, file, embedded_chunks).await?;
            dbg!(response);
        }

        // Handle search command
        if let Some(collection) = collection_name {
            println!("search paths: {:?}", &collection);
            
            // Prompt for query
            print!("Enter your search query: ");
            std::io::stdout().flush()?;

            let client = Qdrant::from_url("http://localhost:6334").build()?;
            
            let mut query = String::new();
            std::io::stdin().read_line(&mut query)?;
            let query = query.trim();
            
            if !query.is_empty() {
               match qdrant::run_query(&client, &collection, query).await {
                Ok(resp) => {
                    let app = Router::new()
                        .route("/", get(home_page))
                        .route("/api/pages", get(get_pages));

                    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                        .await
                        .unwrap();
                    
                    println!("Server running on http://127.0.0.1:3000");
                    axum::serve(listener, app).await.unwrap();
            
                    for point in resp.result {
                            if let Some(text_value) = point.payload.get("text") {
                                if let Some(text) = text_value.as_str() {
                                    println!("-----");
                                    println!("{}", text);
                                }
                            }
                        }
                }
                Err(e) => return Err(e.into())
               }
            } else {
                println!("No query entered.");
            }
}

    }
}

async fn home_page() -> Html<String> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>
        <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>
        <script src="https://cdn.tailwindcss.com"></script>
    </head>
    <body>
        <div id="root"></div>
        <script>
            // Fetch pages and render component
            fetch('/api/pages')
                .then(r => r.json())
                .then(data => console.log(data));
        </script>
    </body>
    </html>
    "#.to_string())
}

#[derive(Serialize)]
struct PagesResponse {
    pages: Vec<String>,
}

// Using bincode
fn save_pages(pages: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let encoded = bincode::serialize(pages)?;
    fs::write("pages.bin", encoded)?;
    Ok(())
}

fn load_pages() -> Result<Vec<Page>, Box<dyn std::error::Error>> {
    let bytes = fs::read("pages.bin")?;
    let pages = bincode::deserialize(&bytes)?;
    Ok(pages)
}

// API endpoint to get your PDF pages
async fn get_pages() -> Json<PagesResponse> {
    // Replace this with your actual PDF extraction
    let pages = 
    
    Json(PagesResponse { pages })
}




