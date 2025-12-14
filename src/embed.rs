use anyhow::Error;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

use crate::chunk::Chunk;

pub struct Embeddings {
    original: Vec<Chunk>,
    embedded: Vec<Vec<f32>>
}

pub fn GetEmbeddings(original: Vec<Chunk>) -> Result<Embeddings, Error> {

    let mut model = TextEmbedding::try_new(Default::default())?;

    let contents: Vec<&str> = original
        .iter()
        .map(|chunk| chunk.content.as_str())
        .collect();
    
    let embedded = model.embed(contents, None)?;

    Ok(Embeddings { original, embedded })




}