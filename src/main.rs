use anyhow;
use clap::Parser;
use clap::{Arg, command};
use clap::{ArgAction, arg};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lopdf::{Document, Object, ObjectId};


pub mod chunk;
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

use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>>{
    loop {
        let matches = command!() // requires `cargo` feature
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

            // extract and embed
            println!("extracting text");
            let res = extract::extract_text(file_paths);
            
            println!("getting text chunks");
            let parent_chunks = chunk::create_chunks(&res[0].pages);
            println!("{:?}", &parent_chunks[2..50]);
        }

        // Handle search command
        if let Some(search_paths) = search_args {
            println!("search paths: {:?}", &search_paths);
            // Add your search logic here
        }

        // Prompt for next command
        print!("\nEnter command (or Ctrl+C to exit): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        // Break if input is empty (allows clean exit)
        if input.trim().is_empty() {
            continue;
        }
    }
    
    Ok(())
}




