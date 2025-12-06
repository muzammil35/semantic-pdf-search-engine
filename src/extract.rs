use fastembed::TextEmbedding;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use unidecode::unidecode;


pub struct File {
    filename: String,
    pub text: String,
}

pub fn extract_text(filenames: Vec<&str>) -> Vec<File>{
    let v: Vec<File> = filenames
    .par_iter()
    .map(|filename| {
        let bytes = std::fs::read(filename).unwrap();
        let text = pdf_extract::extract_text_from_mem(&bytes).unwrap();

        let cleaned = unidecode(&text)
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        
        File {
            filename: filename.to_string(),
            text: cleaned,
        }
    })
    .collect();

    v
}
