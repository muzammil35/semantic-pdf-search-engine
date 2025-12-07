use rayon::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct SmallChunk {
    id: String,
    content: String,
    start_pos: usize,
    end_pos: usize,
}

#[derive(Debug)]
pub struct ParentChunk {
    id: String,
    content: String,
    small_chunks: Vec<SmallChunk>,
}

pub struct ChunkConfig {
    small_chunk_size: usize,
    small_chunk_overlap: usize,
    parent_chunk_size: usize,
    parent_chunk_overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            small_chunk_size: 20,
            small_chunk_overlap: 5,
            parent_chunk_size: 100,
            parent_chunk_overlap: 50,
        }
    }
}

pub fn create_chunks(text: &str, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    let config = config.unwrap_or_default();
    
    // Tokenize ONCE for the entire text
    let words: Vec<&str> = text.split_whitespace().collect();
    let text_len = words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let parent_step = config.parent_chunk_size - config.parent_chunk_overlap;
    
    // Calculate all parent chunk boundaries upfront
    let mut parent_ranges = Vec::new();
    let mut parent_start = 0;
    
    while parent_start < text_len {
        let parent_end = (parent_start + config.parent_chunk_size).min(text_len);
        parent_ranges.push((parent_start, parent_end));
        
        parent_start += parent_step;
        if parent_end == text_len {
            break;
        }
    }
    
    // Process parent chunks in parallel
    parent_ranges
        .par_iter()
        .map(|&(start, end)| {
            let parent_words = &words[start..end];
            
            let small_chunks = create_small_chunks_optimized(
                parent_words,
                start,
                config.small_chunk_size,
                config.small_chunk_overlap,
            );
            
            ParentChunk {
                id: Uuid::new_v4().to_string(),
                content: parent_words.join(" "),
                small_chunks,
            }
        })
        .collect()
}

// Even faster: parallel with sequential IDs
pub fn create_chunks_parallel_fast(text: &str, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    let config = config.unwrap_or_default();
    let words: Vec<&str> = text.split_whitespace().collect();
    let text_len = words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let parent_step = config.parent_chunk_size - config.parent_chunk_overlap;
    
    // Calculate all parent chunk boundaries upfront
    let mut parent_ranges = Vec::new();
    let mut parent_start = 0;
    
    while parent_start < text_len {
        let parent_end = (parent_start + config.parent_chunk_size).min(text_len);
        parent_ranges.push((parent_start, parent_end));
        
        parent_start += parent_step;
        if parent_end == text_len {
            break;
        }
    }
    
    // Process parent chunks in parallel with indexed mapping
    parent_ranges
        .par_iter()
        .enumerate()
        .map(|(parent_idx, &(start, end))| {
            let parent_words = &words[start..end];
            
            // Calculate small chunk ID offset for this parent
            let small_id_offset = parent_idx * estimate_small_chunks(
                parent_words.len(),
                config.small_chunk_size,
                config.small_chunk_overlap,
            );
            
            let small_chunks = create_small_chunks_with_base_id(
                parent_words,
                start,
                config.small_chunk_size,
                config.small_chunk_overlap,
                small_id_offset,
            );
            
            ParentChunk {
                id: format!("parent_{}", parent_idx),
                content: parent_words.join(" "),
                small_chunks,
            }
        })
        .collect()
}

fn estimate_small_chunks(text_len: usize, chunk_size: usize, overlap: usize) -> usize {
    if text_len == 0 {
        return 0;
    }
    if text_len <= chunk_size {
        return 1;
    }
    let step = chunk_size - overlap;
    ((text_len - chunk_size) / step) + 2
}

fn create_small_chunks_optimized(
    parent_words: &[&str],
    parent_offset: usize,
    chunk_size: usize,
    overlap: usize,
) -> Vec<SmallChunk> {
    let text_len = parent_words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let step = chunk_size - overlap;
    let num_chunks = estimate_small_chunks(text_len, chunk_size, overlap);
    
    let mut chunks = Vec::with_capacity(num_chunks);
    let mut start = 0;

    while start < text_len {
        let end = (start + chunk_size).min(text_len);

        chunks.push(SmallChunk {
            id: Uuid::new_v4().to_string(),
            content: parent_words[start..end].join(" "),
            start_pos: parent_offset + start,
            end_pos: parent_offset + end,
        });

        start += step;
        if end == text_len {
            break;
        }
    }

    chunks
}

fn create_small_chunks_with_base_id(
    parent_words: &[&str],
    parent_offset: usize,
    chunk_size: usize,
    overlap: usize,
    base_id: usize,
) -> Vec<SmallChunk> {
    let text_len = parent_words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let step = chunk_size - overlap;
    let num_chunks = estimate_small_chunks(text_len, chunk_size, overlap);
    
    let mut chunks = Vec::with_capacity(num_chunks);
    let mut start = 0;
    let mut chunk_idx = 0;

    while start < text_len {
        let end = (start + chunk_size).min(text_len);

        chunks.push(SmallChunk {
            id: format!("small_{}", base_id + chunk_idx),
            content: parent_words[start..end].join(" "),
            start_pos: parent_offset + start,
            end_pos: parent_offset + end,
        });

        chunk_idx += 1;
        start += step;
        if end == text_len {
            break;
        }
    }

    chunks
}

// Ultimate performance: parallel processing of small chunks too
pub fn create_chunks_fully_parallel(text: &str, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    let config = config.unwrap_or_default();
    let words: Vec<&str> = text.split_whitespace().collect();
    let text_len = words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let parent_step = config.parent_chunk_size - config.parent_chunk_overlap;
    
    let mut parent_ranges = Vec::new();
    let mut parent_start = 0;
    
    while parent_start < text_len {
        let parent_end = (parent_start + config.parent_chunk_size).min(text_len);
        parent_ranges.push((parent_start, parent_end));
        
        parent_start += parent_step;
        if parent_end == text_len {
            break;
        }
    }
    
    parent_ranges
        .par_iter()
        .enumerate()
        .map(|(parent_idx, &(start, end))| {
            let parent_words = &words[start..end];
            let parent_len = parent_words.len();
            
            // Calculate small chunk ranges
            let small_step = config.small_chunk_size - config.small_chunk_overlap;
            let mut small_ranges = Vec::new();
            let mut small_start = 0;
            
            while small_start < parent_len {
                let small_end = (small_start + config.small_chunk_size).min(parent_len);
                small_ranges.push((small_start, small_end));
                
                small_start += small_step;
                if small_end == parent_len {
                    break;
                }
            }
            
            // Process small chunks in parallel
            let small_chunks: Vec<SmallChunk> = small_ranges
                .par_iter()
                .enumerate()
                .map(|(small_idx, &(s_start, s_end))| {
                    SmallChunk {
                        id: format!("small_{}_{}", parent_idx, small_idx),
                        content: parent_words[s_start..s_end].join(" "),
                        start_pos: start + s_start,
                        end_pos: start + s_end,
                    }
                })
                .collect();
            
            ParentChunk {
                id: format!("parent_{}", parent_idx),
                content: parent_words.join(" "),
                small_chunks,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_comparison() {
        let large_text = "word ".repeat(100_000);
        let config = ChunkConfig::default();
        
        let start = std::time::Instant::now();
        let _chunks1 = create_chunks(&large_text, Some(config));
        println!("Parallel (UUID): {:?}", start.elapsed());
        
        let start = std::time::Instant::now();
        let _chunks2 = create_chunks_parallel_fast(&large_text, None);
        println!("Parallel (sequential): {:?}", start.elapsed());
        
        let start = std::time::Instant::now();
        let _chunks3 = create_chunks_fully_parallel(&large_text, None);
        println!("Fully parallel: {:?}", start.elapsed());
    }
}