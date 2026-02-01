use axum::extract::Query;
use clap::{Arg, Parser, arg, command};
use qdrant_client::Qdrant;
use serde::Deserialize;
use std::fs;
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use axum::{Json, Router, body::Body, http::StatusCode, response::Html, routing::get};
use serde::Serialize;

pub mod chunk;
pub mod embed;
pub mod extract;
pub mod qdrant;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<String>,

    #[arg(short, long)]
    search: Option<String>,
}

#[tokio::main]
async fn main() {
    let banner = r#"
 ██████╗ ██╗   ██╗███████╗██████╗ ██╗   ██╗
██╔═══██╗██║   ██║██╔════╝██╔══██╗╚██╗ ██╔╝
██║   ██║██║   ██║█████╗  ██████╔╝ ╚████╔╝ 
██║▄▄ ██║██║   ██║██╔══╝  ██╔══██╗  ╚██╔╝  
╚██████╔╝╚██████╔╝███████╗██║  ██║   ██║   
 ╚══▀▀═╝  ╚═════╝ ╚══════╝╚═╝  ╚═╝   ╚═╝   
"#;

    println!("{}", banner);
    println!("Type 'help' for available commands, 'exit' to quit");

    if let Err(e) = run_repl().await {
        eprintln!("Error: {}", e);
    }
}

async fn run_repl() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Parse the command
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.first().map(|s| *s) {
            Some("exit") | Some("quit") => {
                println!("Goodbye!");
                break;
            }
            Some("help") => {
                print_help();
            }
            Some("file") => {
                if parts.len() < 2 {
                    println!("Usage: file <path>");
                    continue;
                }
                let file_path = parts[1];
                if let Err(e) = process_file(file_path).await {
                    eprintln!("Error processing file: {}", e);
                }
            }
            Some("search") => {
                if parts.len() < 3 {
                    println!("Usage: search <collection_name> <query>");
                    continue;
                }
                let collection_name = parts[1];
                let query = parts[2..].join(" ");
                if let Err(e) = run_search(collection_name, query).await {
                    eprintln!("Error searching: {}", e);
                }
            }
            Some("serve") => {
                if parts.len() < 2 {
                    println!("Usage: serve <file_path>");
                    continue;
                }
                let file_path = parts[1];
                if let Err(e) = start_server(file_path).await {
                    eprintln!("Error starting server: {}", e);
                }
            }
            Some(cmd) => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    cmd
                );
            }
            None => {}
        }
    }

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("  file <path>              - Process and index a file");
    println!("  search <collection>      - Search in a collection");
    println!("  serve <file_path>        - Start web server to view PDF");
    println!("  help                     - Show this help message");
    println!("  exit/quit                - Exit the program");
}

async fn process_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing file: {}", file_path);

    let file = extract::extract_pdf_file(file_path);

    let pages = file.get_pages();
    let chunks = chunk::chunk_per_page(pages);
    let embedded_chunks = embed::get_embeddings(chunks)?;
    let client = qdrant::setup_qdrant(&embedded_chunks, file_path).await?;
    let response = qdrant::store_embeddings(&client, file_path, embedded_chunks).await?;

    println!("File processed successfully!");
    dbg!(response);

    Ok(())
}

async fn run_search(
    collection_name: &str,
    query: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = query.trim();
    if query.is_empty() {
        println!("No query entered.");
        return Ok(());
    }

    let client = Qdrant::from_url("http://localhost:6334").build()?;
    let resp = qdrant::run_query(&client, collection_name, query).await?;

    println!("\nSearch Results:");
    println!("===============");
    for point in resp.result {
        if let Some(text_value) = point.payload.get("text") {
            let page = point.payload.get("page").unwrap();
            if let Some(text) = text_value.as_str() {
                println!("-----");
                println!("{:?}", page);
                println!("{}", text);
            }
        }
    }

    Ok(())
}

async fn start_server(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = file_path.to_string();

    // Verify the file exists
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    let app = Router::new().route("/", get(render_pdf)).route(
        "/api/pdf",
        get({
            let path = file_path.clone();
            move || serve_pdf(path.clone())
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    println!("Server running on http://127.0.0.1:3000");
    println!("Serving PDF: {}", file_path);
    println!("Press Ctrl+C to stop the server");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_pdf(file_path: String) -> Result<(StatusCode, Body), StatusCode> {
    match fs::read(&file_path) {
        Ok(contents) => Ok((StatusCode::OK, Body::from(contents))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn render_pdf() -> Result<Html<String>, StatusCode> {
    match fs::read_to_string("static/index.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn render_pdf_() -> Html<String> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>PDF Viewer</title>
        <script src="https://cdn.tailwindcss.com"></script>
        <script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
        <style>
            canvas {
                position: absolute;
                top: 0;
                left: 0;
            }
            .textLayer {
                position: absolute;
                top: 0;
                left: 0;
                overflow: hidden;
                opacity: 0.2;
                line-height: 1.0;
            }
            .textLayer > span {
                color: transparent;
                position: absolute;
                white-space: pre;
                cursor: text;
                transform-origin: 0% 0%;
            }
            .textLayer .highlight {
                background-color: rgba(255, 255, 0, 0.6);
                border-radius: 2px;
            }
            .textLayer .highlight.selected {
                background-color: rgba(255, 153, 0, 0.8);
            }
            .page-wrapper {
                position: relative;
                margin-bottom: 20px;
                box-shadow: 0 2px 8px rgba(0,0,0,0.1);
                background: white;
            }
            #pdf-container {
                display: flex;
                flex-direction: column;
                align-items: center;
            }
            #search-bar {
                position: sticky;
                top: 0;
                z-index: 100;
                background: white;
                padding: 1rem;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            }
        </style>
    </head>
    <body class="bg-gray-100">
        <div id="search-bar" class="container mx-auto">
            <div class="flex gap-2 items-center">
                <input 
                    type="text" 
                    id="search-input" 
                    placeholder="Search in PDF..." 
                    class="flex-1 border rounded px-3 py-2"
                />
                <button 
                    id="prev-match" 
                    class="bg-blue-500 text-white px-3 py-2 rounded hover:bg-blue-600 disabled:bg-gray-300"
                    disabled
                >
                    ← Previous
                </button>
                <button 
                    id="next-match" 
                    class="bg-blue-500 text-white px-3 py-2 rounded hover:bg-blue-600 disabled:bg-gray-300"
                    disabled
                >
                    Next →
                </button>
                <span id="match-counter" class="text-sm text-gray-600 min-w-[100px]"></span>
                <select id="zoom-select" class="border rounded px-2 py-1 text-sm">
                    <option value="0.5">50%</option>
                    <option value="0.75">75%</option>
                    <option value="1">100%</option>
                    <option value="1.5" selected>150%</option>
                    <option value="2">200%</option>
                    <option value="2.5">250%</option>
                    <option value="3">300%</option>
                </select>
            </div>
        </div>

        <div class="container mx-auto p-4">
            <div id="loading" class="text-center py-8">
                <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-gray-900"></div>
                <p class="mt-4 text-gray-600">Loading PDF...</p>
            </div>
            
            <div id="pdf-container"></div>
        </div>

        <script>
        pdfjsLib.GlobalWorkerOptions.workerSrc =
        'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

        let pdfDoc = null;
        let currentScale = 1.5;
        const MAX_OUTPUT_SCALE = 2;
        const renderedPages = new Map();
        const renderTasks = new Map();
        const textLayers = new Map();
        
        let searchMatches = [];
        let currentMatchIndex = -1;

        const container = document.getElementById('pdf-container');
        const loading = document.getElementById('loading');
        const searchInput = document.getElementById('search-input');
        const prevButton = document.getElementById('prev-match');
        const nextButton = document.getElementById('next-match');
        const matchCounter = document.getElementById('match-counter');

        async function loadPDF() {
            const loadingTask = pdfjsLib.getDocument('/api/pdf');
            pdfDoc = await loadingTask.promise;
            loading.style.display = 'none';

            createPagePlaceholders();
            setupObserver();
        }

        function createPagePlaceholders() {
            for (let i = 1; i <= pdfDoc.numPages; i++) {
                const wrapper = document.createElement('div');
                wrapper.className = 'page-wrapper';
                wrapper.dataset.page = i;
                
                const canvas = document.createElement('canvas');
                canvas.dataset.page = i;
                
                const textLayerDiv = document.createElement('div');
                textLayerDiv.className = 'textLayer';
                textLayerDiv.dataset.page = i;
                
                wrapper.appendChild(canvas);
                wrapper.appendChild(textLayerDiv);
                container.appendChild(wrapper);
            }
        }

        function setupObserver() {
            const observer = new IntersectionObserver(
                entries => {
                    entries.forEach(entry => {
                        if (entry.isIntersecting) {
                            const pageNum = Number(entry.target.dataset.page);
                            renderPage(pageNum);
                        }
                    });
                },
                { rootMargin: '200px' }
            );

            document.querySelectorAll('.page-wrapper').forEach(w => observer.observe(w));
        }

        async function renderPage(pageNum) {
            if (renderedPages.get(pageNum) === currentScale) return;

            if (renderTasks.has(pageNum)) {
                renderTasks.get(pageNum).cancel();
            }

            const page = await pdfDoc.getPage(pageNum);
            const viewport = page.getViewport({ scale: currentScale });

            const wrapper = document.querySelector(`.page-wrapper[data-page="${pageNum}"]`);
            const canvas = wrapper.querySelector('canvas');
            const textLayerDiv = wrapper.querySelector('.textLayer');

            const outputScale = Math.min(window.devicePixelRatio || 1, MAX_OUTPUT_SCALE);
            const ctx = canvas.getContext('2d');

            canvas.width = Math.floor(viewport.width * outputScale);
            canvas.height = Math.floor(viewport.height * outputScale);
            canvas.style.width = `${viewport.width}px`;
            canvas.style.height = `${viewport.height}px`;
            
            wrapper.style.width = `${viewport.width}px`;
            wrapper.style.height = `${viewport.height}px`;
            textLayerDiv.style.width = `${viewport.width}px`;
            textLayerDiv.style.height = `${viewport.height}px`;

            const renderTask = page.render({
                canvasContext: ctx,
                viewport,
                transform: outputScale !== 1 ? [outputScale, 0, 0, outputScale, 0, 0] : null
            });

            renderTasks.set(pageNum, renderTask);

            try {
                await renderTask.promise;
                renderedPages.set(pageNum, currentScale);
                
                // Render text layer
                await renderTextLayer(page, viewport, textLayerDiv, pageNum);
            } catch (e) {
                if (e?.name !== 'RenderingCancelledException') {
                    console.error(e);
                }
            }
        }

        async function renderTextLayer(page, viewport, textLayerDiv, pageNum) {
            const textContent = await page.getTextContent();
            textLayerDiv.innerHTML = '';
            
            const textLayer = {
                textContent,
                container: textLayerDiv,
                viewport,
                textDivs: []
            };

            textContent.items.forEach((item) => {
                const tx = pdfjsLib.Util.transform(viewport.transform, item.transform);
                const style = textContent.styles[item.fontName];
                
                const span = document.createElement('span');
                span.textContent = item.str;
                span.style.left = `${tx[4]}px`;
                span.style.top = `${tx[5]}px`;
                span.style.fontSize = `${Math.sqrt(tx[2] * tx[2] + tx[3] * tx[3])}px`;
                span.style.fontFamily = style ? style.fontFamily : 'sans-serif';
                
                textLayerDiv.appendChild(span);
                textLayer.textDivs.push(span);
            });

            textLayers.set(pageNum, textLayer);
            
            // Re-apply search highlights if there's an active search
            if (searchInput.value) {
                highlightSearchInPage(pageNum, searchInput.value);
            }
        }

        function highlightSearchInPage(pageNum, searchText) {
            const textLayer = textLayers.get(pageNum);
            if (!textLayer) return;

            const matches = [];
            const searchLower = searchText.toLowerCase();

            textLayer.textDivs.forEach((span, index) => {
                span.classList.remove('highlight', 'selected');
                
                const text = span.textContent.toLowerCase();
                if (text.includes(searchLower)) {
                    span.classList.add('highlight');
                    matches.push({ pageNum, span, index });
                }
            });

            return matches;
        }

        function performSearch(searchText) {
            searchMatches = [];
            currentMatchIndex = -1;

            if (!searchText) {
                clearHighlights();
                updateMatchCounter();
                return;
            }

            // Search in all rendered pages
            textLayers.forEach((_, pageNum) => {
                const pageMatches = highlightSearchInPage(pageNum, searchText);
                if (pageMatches) {
                    searchMatches.push(...pageMatches);
                }
            });

            updateMatchCounter();
            
            if (searchMatches.length > 0) {
                currentMatchIndex = 0;
                scrollToMatch(0);
            }
        }

        function clearHighlights() {
            textLayers.forEach(textLayer => {
                textLayer.textDivs.forEach(span => {
                    span.classList.remove('highlight', 'selected');
                });
            });
        }

        function scrollToMatch(index) {
            if (index < 0 || index >= searchMatches.length) return;

            // Remove 'selected' from previous match
            searchMatches.forEach(match => {
                match.span.classList.remove('selected');
            });

            // Add 'selected' to current match
            const match = searchMatches[index];
            match.span.classList.add('selected');
            
            // Scroll to the match
            match.span.scrollIntoView({ behavior: 'smooth', block: 'center' });
            
            currentMatchIndex = index;
            updateMatchCounter();
        }

        function updateMatchCounter() {
            const total = searchMatches.length;
            if (total === 0) {
                matchCounter.textContent = '';
                prevButton.disabled = true;
                nextButton.disabled = true;
            } else {
                matchCounter.textContent = `${currentMatchIndex + 1} / ${total}`;
                prevButton.disabled = false;
                nextButton.disabled = false;
            }
        }

        // Event listeners
        searchInput.addEventListener('input', (e) => {
            performSearch(e.target.value);
        });

        prevButton.addEventListener('click', () => {
            if (searchMatches.length === 0) return;
            const newIndex = (currentMatchIndex - 1 + searchMatches.length) % searchMatches.length;
            scrollToMatch(newIndex);
        });

        nextButton.addEventListener('click', () => {
            if (searchMatches.length === 0) return;
            const newIndex = (currentMatchIndex + 1) % searchMatches.length;
            scrollToMatch(newIndex);
        });

        // Keyboard shortcuts
        searchInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                if (e.shiftKey) {
                    prevButton.click();
                } else {
                    nextButton.click();
                }
            }
        });

        document.getElementById('zoom-select').addEventListener('change', e => {
            currentScale = Math.min(parseFloat(e.target.value), 2);
            renderedPages.clear();
            textLayers.clear();

            renderTasks.forEach(task => task.cancel());
            renderTasks.clear();

            document.querySelectorAll('.page-wrapper').forEach(wrapper => {
                const rect = wrapper.getBoundingClientRect();
                if (rect.top < window.innerHeight + 200 && rect.bottom > -200) {
                    renderPage(Number(wrapper.dataset.page));
                }
            });
        });

        loadPDF();
        </script>

    </body>
    </html>
    "#.to_string())
}

async fn home_page() -> Html<String> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>PDF Viewer</title>
        <script src="https://cdn.tailwindcss.com"></script>
        <script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
        <style>
            canvas {
                display: block;
                margin-bottom: 20px;
                box-shadow: 0 2px 8px rgba(0,0,0,0.1);
            }
            #pdf-container {
                display: flex;
                flex-direction: column;
                align-items: center;
            }
        </style>
    </head>
    <body class="bg-gray-100">
        <div class="container mx-auto p-4">
            <div class="flex justify-end mb-4">
                <select id="zoom-select" class="border rounded px-2 py-1 text-sm">
                    <option value="0.5">50%</option>
                    <option value="0.75">75%</option>
                    <option value="1">100%</option>
                    <option value="1.5" selected>150%</option>
                    <option value="2">200%</option>
                    <option value="2.5">250%</option>
                    <option value="3">300%</option>
                </select>
            </div>
            
            <div id="loading" class="text-center py-8">
                <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-gray-900"></div>
                <p class="mt-4 text-gray-600">Loading PDF...</p>
            </div>
            
            <div id="pdf-container"></div>
        </div>

        <script>
        pdfjsLib.GlobalWorkerOptions.workerSrc =
        'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

        let pdfDoc = null;
        let currentScale = 1.5;
        const MAX_OUTPUT_SCALE = 2;
        const renderedPages = new Map();
        const renderTasks = new Map();

        const container = document.getElementById('pdf-container');
        const loading = document.getElementById('loading');

        async function loadPDF() {
        const loadingTask = pdfjsLib.getDocument('/api/pdf');
        pdfDoc = await loadingTask.promise;
        loading.style.display = 'none';

        createPagePlaceholders();
        setupObserver();
        }

        function createPagePlaceholders() {
        for (let i = 1; i <= pdfDoc.numPages; i++) {
            const canvas = document.createElement('canvas');
            canvas.dataset.page = i;
            canvas.style.marginBottom = '20px';
            container.appendChild(canvas);
        }
        }

        function setupObserver() {
        const observer = new IntersectionObserver(
            entries => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                const pageNum = Number(entry.target.dataset.page);
                renderPage(pageNum, entry.target);
                }
            });
            },
            { rootMargin: '200px' }
        );

        document.querySelectorAll('canvas').forEach(c => observer.observe(c));
        }

        async function renderPage(pageNum, canvas) {
        if (renderedPages.get(pageNum) === currentScale) return;

        // cancel previous render
        if (renderTasks.has(pageNum)) {
            renderTasks.get(pageNum).cancel();
        }

        const page = await pdfDoc.getPage(pageNum);
        const viewport = page.getViewport({ scale: currentScale });

        const outputScale = Math.min(
            window.devicePixelRatio || 1,
            MAX_OUTPUT_SCALE
        );

        const ctx = canvas.getContext('2d');

        canvas.width = Math.floor(viewport.width * outputScale);
        canvas.height = Math.floor(viewport.height * outputScale);
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;

        const renderTask = page.render({
            canvasContext: ctx,
            viewport,
            transform:
            outputScale !== 1 ? [outputScale, 0, 0, outputScale, 0, 0] : null
        });

        renderTasks.set(pageNum, renderTask);

        try {
            await renderTask.promise;
            renderedPages.set(pageNum, currentScale);
        } catch (e) {
            if (e?.name !== 'RenderingCancelledException') {
            console.error(e);
            }
        }
        }

        document.getElementById('zoom-select').addEventListener('change', e => {
        currentScale = Math.min(parseFloat(e.target.value), 2);
        renderedPages.clear();

        // cancel all in-flight renders
        renderTasks.forEach(task => task.cancel());
        renderTasks.clear();

        // re-render only visible pages
        document.querySelectorAll('canvas').forEach(canvas => {
            const rect = canvas.getBoundingClientRect();
            if (rect.top < window.innerHeight + 200 && rect.bottom > -200) {
            renderPage(Number(canvas.dataset.page), canvas);
            }
        });
        });

        loadPDF();
        </script>

    </body>
    </html>
    "#.to_string())
}
