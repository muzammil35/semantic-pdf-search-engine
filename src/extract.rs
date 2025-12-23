use fastembed::TextEmbedding;
use lopdf::Document;
use pdf_oxide::PdfDocument;
use pdfium_render::prelude::*;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use unidecode::unidecode;

#[derive(Debug)]
pub struct File {
    filename: String,
    pub pages: Vec<String>,
}

pub fn extract_text_lopdf(filenames: Vec<&str>) -> Vec<File> {
    filenames
        .par_iter()
        .map(|filename| {
            // Load document once
            let document = Document::load(filename).unwrap();
            let pages_map = document.get_pages();
            let page_numbers: Vec<u32> = pages_map.keys().cloned().collect();

            // Wrap in Arc to share across threads
            let document = Arc::new(document);

            // Extract pages IN PARALLEL - THIS IS THE KEY WIN!
            let page_texts: Vec<String> = page_numbers
                .par_iter()
                .filter_map(|&page_num| {
                    //let doc = Arc::clone(&document);
                    document.extract_text(&[page_num]).ok()
                })
                .collect();

            File {
                filename: (*filename).to_string(),
                pages: page_texts,
            }
        })
        .collect()
}

pub fn extract_text(filenames: Vec<&str>) -> Vec<File> {
    filenames
        .par_iter()
        .map(|filename| {
            let page_count = PdfDocument::open(filename).unwrap().page_count().unwrap();

            // Calculate optimal chunk size based on available threads
            let num_threads = rayon::current_num_threads();
            let chunk_size = (page_count / num_threads).max(1);

            // Process pages in parallel chunks
            let page_texts: Vec<String> = (0..page_count)
                .collect::<Vec<_>>()
                .par_chunks(chunk_size)
                .flat_map(|chunk| {
                    // Open document once per chunk
                    let mut doc = PdfDocument::open(filename).unwrap();
                    chunk
                        .iter()
                        .filter_map(|&page_num| doc.extract_text(page_num).ok())
                        .collect::<Vec<_>>()
                })
                .collect();

            // Fast join
            let total_size: usize = page_texts.iter().map(|s| s.len()).sum();
            //let mut all_text = String::with_capacity(total_size + page_texts.len());

            // for page_text in page_texts {
            //     all_text.push_str(&page_text);
            //     all_text.push(' ');
            // }

            File {
                filename: (*filename).to_string(),
                pages: page_texts,
            }
        })
        .collect()
}
