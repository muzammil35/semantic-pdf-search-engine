use uuid::Uuid;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct SmallChunk {
    id: String,
    content: String,
    start_pos: usize, // position in parent
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
            small_chunk_size: 50,
            small_chunk_overlap: 5,
            parent_chunk_size: 10000,
            parent_chunk_overlap: 200,
        }
    }
}

pub fn create_chunks(text: &str, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    let config = config.unwrap_or_default();
    let words: Vec<&str> = text.split_whitespace().collect();
    let text_len = words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let parent_step = config.parent_chunk_size - config.parent_chunk_overlap;
    let num_parent_chunks = ((text_len - config.parent_chunk_size) / parent_step) + 1;
    let mut parent_chunks = Vec::with_capacity(num_parent_chunks.max(1));
    
    let mut parent_start = 0;
    let mut parent_id = 0;
    let mut small_id = 0;

    while parent_start < text_len {
        let parent_end = (parent_start + config.parent_chunk_size).min(text_len);
        let parent_words = &words[parent_start..parent_end];
        
        let small_chunks = create_small_chunks(
            parent_words,
            parent_start,
            config.small_chunk_size,
            config.small_chunk_overlap,
            &mut small_id,
        );
        
        parent_chunks.push(ParentChunk {
            id: format!("parent_{}", parent_id),
            content: parent_words.join(" "),
            small_chunks,
        });

        parent_id += 1;
        parent_start += parent_step;

        if parent_end == text_len {
            break;
        }
    }

    parent_chunks
}

fn create_small_chunks(
    parent_words: &[&str],
    parent_offset: usize,
    chunk_size: usize,
    overlap: usize,
    id_counter: &mut usize,
) -> Vec<SmallChunk> {
    let text_len = parent_words.len();
    
    if text_len == 0 {
        return Vec::new();
    }
    
    let step = chunk_size - overlap;
    let num_chunks = if text_len <= chunk_size {
        1
    } else {
        ((text_len - chunk_size) / step) + 2
    };
    
    let mut chunks = Vec::with_capacity(num_chunks);
    let mut start = 0;

    while start < text_len {
        let end = (start + chunk_size).min(text_len);

        chunks.push(SmallChunk {
            id: format!("small_{}", *id_counter),
            content: parent_words[start..end].join(" "),
            start_pos: parent_offset + start,
            end_pos: parent_offset + end,
        });

        *id_counter += 1;
        start += step;

        if end == text_len {
            break;
        }
    }

    chunks
}