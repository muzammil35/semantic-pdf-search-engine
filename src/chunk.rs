use rayon::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct SmallChunk {
    id: String,
    content: String,
    page: u16
}

#[derive(Debug)]
pub struct ParentChunk {
    id: String,
    content: String,
    page: u16,
    small_chunks: Vec<SmallChunk>,
}




// pub fn create_chunks( pages: Vec<String>, config: Option<ChunkConfig>) -> Vec<ParentChunk> {
    


// }


    
    