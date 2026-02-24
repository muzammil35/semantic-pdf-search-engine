# Prerequisites
- Docker
- Rust
- Pdfium Binary (https://github.com/paulocoutinhox/pdfium-lib/releases) in working directory

  

# Getting started

`docker pull qdrant/qdrant`

`docker run -p 6333:6333 -p 6334:6334 \ -v "$(pwd)/qdrant_storage:/qdrant/storage:z" \ qdrant/qdrant`

`cargo build`

# Running the Webapp

`cargo run --bin app`

# Running the CLI

`cargo run --bin repl`

  # Supported CLI Commands:
    - file ex.pdf (extract and embed file into qdrant vector database, file should be in the root folder of this repo)
    - search ex.pdf (query the vector database and get results back)
    - serve ex.pdf (render the pdf in the browser)
