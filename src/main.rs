use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use anyhow;
use clap::Parser;
use clap::{arg, ArgAction};
use clap::{command, Arg};


pub mod extract;
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path to be stored
    #[arg(short, long)]
    file: String,

    
}


fn main() {
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
    extract::extract_text(args);
}

pub fn main_() {
    let  mut model = get_model().unwrap();
    let documents = vec![
        "passage: Hello, World!",
        "query: Hello, World!",
    ];

    let embeddings = model.embed(documents, None).unwrap();

    println!("{:?}", embeddings);

}

fn get_model() -> Result<TextEmbedding, anyhow::Error> {
    let model = TextEmbedding::try_new(
    InitOptions::new(EmbeddingModel::AllMiniLML6V2)
    )?;
    Ok(model)
}

