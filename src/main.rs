use anyhow;
use clap::Parser;
use clap::{Arg, command};
use clap::{ArgAction, arg};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use lopdf::{Document, Object, ObjectId};

pub mod chunk;
pub mod extract;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path to be stored
    #[arg(short, long)]
    file: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let matches = command!() // requires `cargo` feature
        .arg(
            Arg::new("file")
                .short('f')
                .long("file path")
                .action(ArgAction::Append),
        )
        .get_matches();

    let args = matches
        .get_many::<String>("file")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();

    println!("file paths: {:?}", &args);

    // extract and embed
    let res = extract::extract_text(args);

   let parent_chunks = chunk::create_chunks(res[0].text.clone(), None);
    println!("{:?}", parent_chunks[0]);

    let doc = Document::load("soc.pdf")?;
    
    Ok(())
   
}




