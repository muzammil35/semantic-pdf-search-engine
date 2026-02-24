# Semantic PDF Search Engine

End-to-end semantic search system for PDFs using vector embeddings and Qdrant, with precise in-document highlight rendering in the browser.

This project enables users to query a PDF using natural language and instantly see relevant passages highlighted directly within the rendered document.

---

## Overview

Traditional PDF search relies on keyword matching. This system implements semantic search by:

1. Extracting text from PDF documents  
2. Generating vector embeddings for document chunks  
3. Indexing embeddings in a Qdrant vector database  
4. Performing similarity search on user queries  
5. Returning precise bounding boxes for matched text  
6. Rendering and highlighting results in the browser  

The result is a full-stack retrieval pipeline from document ingestion to interactive visualization.

---

## Architecture

```
PDF Document
     ↓
Text Extraction
     ↓
Chunking + Embedding Generation
     ↓
Qdrant Vector Index
     ↓
User Query → Embedding
     ↓
Vector Similarity Search
     ↓
Fuzzy Matching Refinement
     ↓
Bounding Box Resolution
     ↓
Frontend Highlighting (pdf.js)
```

### Key Components

- **Text Extraction**: Extracts structured text and positional data 
- **Embedding Pipeline**: Converts text chunks into vector representations  
- **Vector Search**: Stores and retrieves embeddings via Qdrant using similarity metrics  
- **Fuzzy Matching**: Improves recall by matching approximate text spans  
- **Bounding Box Mapping**: Maps matches to precise PDF coordinates  
- **Frontend Rendering**: Displays PDF and overlays dynamic highlight regions using pdf.js  

---

## Features

- Semantic similarity search over PDF documents  
- Precise text highlighting via bounding box extraction  
- Fuzzy search to recover approximate or partial matches  
- CLI for document ingestion and querying  
- Dockerized vector database setup  
- Interactive browser-based PDF rendering  

---

## Tech Stack

### Backend
- Rust  
- pdfium-render  
- Qdrant (vector database)  

### Frontend
- pdf.js  
- JavaScript  

### Infrastructure
- Docker  

---

## Quick Start

### Prerequisites

- Docker  
- Rust  
- Pdfium binary in project root  
  https://github.com/paulocoutinhox/pdfium-lib/releases  

---

### 1. Start Qdrant

```bash
docker pull qdrant/qdrant

docker run -p 6333:6333 -p 6334:6334 \
  -v "$(pwd)/qdrant_storage:/qdrant/storage:z" \
  qdrant/qdrant
```

---

### 2. Build

```bash
cargo build
```

---

### 3. Run Web App

```bash
cargo run --bin app
```

---

### 4. Run CLI

```bash
cargo run --bin repl
```

---

## CLI Commands

```bash
file <filename.pdf>    # Extract and embed PDF into Qdrant
search <filename.pdf>  # Query indexed document
serve <filename.pdf>   # Render PDF in browser
```

---

## Design Considerations

- Chunk sizing affects embedding accuracy and retrieval precision  
- Bounding box recovery requires preserving positional metadata during extraction  
- Fuzzy search improves robustness against tokenization or chunk-boundary issues  
- Vector search enables semantic retrieval beyond exact keyword matching  

---

## Future Improvements

- REST API layer for external integrations
- Hybrid Keyword + Semantic search retrieval functionality for more rich query matches  
- Support for multiple embedding models  
- Pagination and multi-document indexing  
- Query latency benchmarking  
- Authentication + multi-user support  

---

## 📄 License

MIT License
