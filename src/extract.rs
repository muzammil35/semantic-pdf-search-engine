use pdf_oxide::PdfDocument;
use rayon::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct PageDTO {
    page: u16,
    text: String,
}

#[derive(Debug)]
pub struct Page {
    pub content: String,
    pub page_num: u16,
}
pub struct File {
    pages: Vec<Page>,
}

#[derive(Deserialize)]
struct PythonOutput {
    pages: Vec<PageDTO>,
}

pub fn extract_pdf_file(pdf_path: &str) -> File {
    let output = std::process::Command::new("python3")
        .arg("extract_pdf.py")
        .arg(pdf_path)
        .output()
        .expect("Failed to run Python script");

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        panic!("Python error: {}", err);
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 from Python");

    println!("{:?}", stdout);

    let parsed: PythonOutput = serde_json::from_str(&stdout).expect("Failed to parse Python JSON");

    let pages = parsed
        .pages
        .into_iter()
        .map(|p| Page {
            page_num: p.page,
            content: p.text,
        })
        .collect();

    File { pages }
}

pub fn extract_text(file: &str) -> File {
    let page_count = PdfDocument::open(file).unwrap().page_count().unwrap();

    // Calculate optimal chunk size based on available threads
    let num_threads = rayon::current_num_threads();
    let chunk_size = (page_count / num_threads).max(1);

    // Process pages in parallel chunks
    let pages: Vec<Page> = (0..page_count)
        .collect::<Vec<_>>()
        .par_chunks(chunk_size)
        .flat_map(|chunk| {
            // Open document once per chunk
            let mut doc = PdfDocument::open(file).unwrap();
            chunk
                .iter()
                .filter_map(|&page_num| {
                    doc.extract_text(page_num).ok().map(|text| Page {
                        content: text,
                        page_num: page_num as u16,
                    })
                })
                .collect::<Vec<Page>>()
        })
        .collect();

    File { pages: pages }
}

impl File {
    pub fn get_pages(&self) -> &Vec<Page> {
        &self.pages
    }
}
