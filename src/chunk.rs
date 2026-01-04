
use rayon::prelude::*;
use text_splitter::TextSplitter;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    page: u16,
}

pub fn create_chunks(pages: &Vec<String>) -> Vec<Chunk> {
    let max_token_size = 500;

    // Process pages in parallel and collect all chunks
    pages
        .par_iter()  // Fixed: use par_iter() instead of into_par_iter()
        .enumerate()
        .flat_map(|(page_idx, page_content)| {
            chunk_page(page_content, page_idx as u16, max_token_size)
        })
        .collect()
}

pub fn chunk_everything(pages: &Vec<String>) -> Vec<Chunk> {
    let combined = pages.join("\n");
    
    // Split into sentences first (basic sentence detection)
    let sentences: Vec<&str> = combined
        .split(|c| c == '.' || c == '?' || c == '!')
        .filter(|s| !s.trim().is_empty())
        .collect();
    
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let target_size = 200; // characters per chunk (adjust as needed)
    let min_size = 100;    // minimum before forcing a split
    
    for sentence in sentences {
        let sentence = sentence.trim();
        
        // If adding this sentence would exceed target size AND we're above minimum
        if current_chunk.len() + sentence.len() > target_size && current_chunk.len() >= min_size {
            // Save current chunk
            if !current_chunk.is_empty() {
                chunks.push(Chunk {
                    content: current_chunk.trim().to_string(),
                    page: 0,
                });
                current_chunk.clear();
            }
        }
        
        // Add sentence to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(sentence);
        current_chunk.push('.');
    }
    
    // Don't forget the last chunk
    if !current_chunk.is_empty() {
        chunks.push(Chunk {
            content: current_chunk.trim().to_string(),
            page: 0,
        });
    }
    
    chunks
}

fn chunk_page(content: &String, page_num: u16, max_size: usize) -> Vec<Chunk> {

    let splitter = TextSplitter::new(512);
    let chunks: Vec<Chunk> = splitter.chunks(content)
    .map(|s| Chunk {
        content: s.to_string(),
        page:page_num,
    })
    .collect();
    chunks
}

