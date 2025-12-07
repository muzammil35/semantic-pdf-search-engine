use fastembed::TextEmbedding;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use unidecode::unidecode;
use pdf_oxide::PdfDocument;
use std::time::Instant;


#[derive(Debug)]
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

pub fn extract_text_(filenames: Vec<&str>) -> Vec<File> {
    filenames
        .par_iter()
        .map(|filename| {
            let mut doc = PdfDocument::open(filename).unwrap();
            let page_count = doc.page_count().unwrap();
            
            // Calculate optimal chunk size based on available threads
            let num_threads = rayon::current_num_threads();
            let chunk_size = (page_count / num_threads).max(1);
            
            println!("Processing {} pages in chunks of {}", page_count, chunk_size);
            
            // Process pages in parallel chunks
            let page_texts: Vec<String> = (0..page_count)
                .collect::<Vec<_>>()
                .par_chunks(chunk_size)
                .flat_map(|chunk| {
                    // Open document once per chunk
                    let mut doc = PdfDocument::open(filename).unwrap();
                    chunk.iter().filter_map(|&page_num| {
                        doc.extract_text(page_num).ok()
                    }).collect::<Vec<_>>()
                })
                .collect();
            
            // Fast join
            let total_size: usize = page_texts.iter().map(|s| s.len()).sum();
            let mut all_text = String::with_capacity(total_size + page_texts.len());
            
            for page_text in page_texts {
                all_text.push_str(&page_text);
                all_text.push(' ');
            }
            
            File {
                filename: (*filename).to_string(),
                text: clean_text(&all_text),
            }
        })
        .collect()
}

pub fn extract_text_slow(filenames: Vec<&str>) -> Vec<File> {
    filenames
        .par_iter()
        .map(|filename| {
            // Get page count
            let page_count = {
                let mut doc = PdfDocument::open(filename).unwrap();
                doc.page_count().unwrap()
            };
            
            // Extract pages in parallel - HUGE speedup
            let page_texts: Vec<String> = (0..page_count)
                .into_par_iter()
                .filter_map(|page_num| {
                    // Each thread opens its own document handle
                    let mut doc = PdfDocument::open(filename).ok()?;
                    doc.extract_text(page_num).ok()
                })
                .collect();
            
            // Fast join - pre-allocate
            let total_size: usize = page_texts.iter().map(|s| s.len()).sum();
            let mut all_text = String::with_capacity(total_size + page_texts.len());
            
            for page_text in page_texts {
                all_text.push_str(&page_text);
                all_text.push(' ');
            }
            
            File {
                filename: (*filename).to_string(),
                text: clean_text(&all_text),
            }
        })
        .collect()
}



pub fn extract_text_profile(filenames: Vec<&str>) -> Vec<File> {
    filenames
        .par_iter()
        .map(|filename| {
            let start = Instant::now();
            
            let t1 = Instant::now();
            let mut doc = PdfDocument::open(filename).unwrap();
            println!("Open: {:?}", t1.elapsed());
            
            let t2 = Instant::now();
            let page_count = doc.page_count().unwrap();
            let mut all_text = String::with_capacity(page_count * 2048);
            
            for page_num in 0..page_count {
                if let Ok(text) = doc.extract_text(page_num) {
                    all_text.push_str(&text);
                    all_text.push(' ');
                }
            }
            println!("Extract: {:?}", t2.elapsed());
            
            let t3 = Instant::now();
            let cleaned = clean_text(&all_text);
            println!("Clean: {:?}", t3.elapsed());
            
            println!("Total for {}: {:?}", filename, start.elapsed());
            
            File {
                filename: (*filename).to_string(),
                text: cleaned,
            }
        })
        .collect()
}

fn clean_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true;
    
    for c in text.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else if c.is_ascii() {
            result.push(c);
            last_was_space = false;
        }
        // Skip non-ASCII characters entirely (or handle differently)
    }
    
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_single_file() {
        // Create a test PDF file (you'll need a sample PDF in your test fixtures)
        let filenames = vec!["src/soc.pdf"];
        
        let results = extract_text_(filenames);

        //println!("{:?}", results);
    }

}
