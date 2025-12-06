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
            parent_chunk_size: 200,
            parent_chunk_overlap: 20,
        }
    }
}

pub fn create_chunks(text: String, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    let mut parent_chunks = Vec::new();
    let words: Vec<&str> = text.unicode_words().collect();
    let text_len = words.len();
    let config = config.unwrap_or_default();
    let parent_step = config.parent_chunk_size - config.parent_chunk_overlap;
    let mut parent_start = 0;

    while parent_start < text_len {
        let parent_end = (parent_start + config.parent_chunk_size).min(text_len);
        let parent_content = words[parent_start..parent_end].join(" ");

        // Create small chunks within this parent chunk
        let small_chunks = create_small_chunks(
            &parent_content,
            parent_start,
            config.small_chunk_size,
            config.small_chunk_overlap,
        );

        parent_chunks.push(ParentChunk {
            id: Uuid::new_v4().to_string(),
            content: parent_content,
            small_chunks,
        });

        // Move to next parent chunk
        parent_start += parent_step;

        // Break if we've processed the entire text
        if parent_end == text_len {
            break;
        }
    }

    parent_chunks
}

fn create_small_chunks(
    parent_text: &str,
    parent_offset: usize,
    chunk_size: usize,
    overlap: usize,
) -> Vec<SmallChunk> {
    let mut chunks = Vec::new();
    let words: Vec<&str> = parent_text.unicode_words().collect();
    let text_len = words.len();
    let step = chunk_size - overlap;
    let mut start = 0;

    while start < text_len {
        let end = (start + chunk_size).min(text_len);
        let content = words[start..end].join(" ");

        chunks.push(SmallChunk {
            id: Uuid::new_v4().to_string(),
            content,
            start_pos: parent_offset + start,
            end_pos: parent_offset + end,
        });

        start += step;

        // Break if we've processed the entire parent text
        if end == text_len {
            break;
        }
    }

    chunks
}
