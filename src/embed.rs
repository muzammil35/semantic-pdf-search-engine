use anyhow::Error;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use fastembed::ModelTrait;

use crate::chunk::Chunk;

pub struct Embeddings {
    pub original: Vec<Chunk>,
    pub embedded: Vec<Vec<f32>>,
}

pub fn get_embeddings(original: Vec<Chunk>) -> Result<Embeddings, Error> {
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            
    )?;

    let contents: Vec<&str> = original
        .iter()
        .map(|chunk| chunk.content.as_str())
        .collect();

    let embedded = model.embed(contents, None)?;

    Ok(Embeddings { original, embedded })
}

impl Embeddings {
    pub fn get_dim(&self) -> usize {
        let model_info = EmbeddingModel::get_model_info(&EmbeddingModel::AllMiniLML6V2);
        model_info.expect("Model info should always exist").dim

    }
}
