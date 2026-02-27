// ── File input UX ────────────────────────────────────────────────────────────

export function initDropZone() {
    const dropZone        = document.getElementById('dropZone');
    const pdfFileInput    = document.getElementById('pdfFile');
    const fileNameDisplay = document.getElementById('fileNameDisplay');
    const fileNameText    = document.getElementById('fileNameText');

    pdfFileInput.addEventListener('change', () => {
        if (pdfFileInput.files.length > 0) {
            fileNameText.textContent = pdfFileInput.files[0].name;
            fileNameDisplay.classList.add('visible');
        } else {
            fileNameDisplay.classList.remove('visible');
        }
    });

    dropZone.addEventListener('dragover',  (e) => { e.preventDefault(); dropZone.classList.add('dragover'); });
    dropZone.addEventListener('dragleave', ()  => dropZone.classList.remove('dragover'));
    dropZone.addEventListener('drop',      ()  => dropZone.classList.remove('dragover'));
}

// ── Viewer initialisation ─────────────────────────────────────────────────────

/**
 * Creates and returns a fresh { eventBus, pdfLinkService, pdfViewer } set.
 */
export function initializeViewer() {
    const eventBus       = new pdfjsViewer.EventBus();
    const pdfLinkService = new pdfjsViewer.PDFLinkService({ eventBus });

    const pdfViewer = new pdfjsViewer.PDFViewer({
        container:         document.getElementById('viewerContainer'),
        viewer:            document.getElementById('viewer'),
        eventBus,
        linkService:       pdfLinkService,
        textLayerMode:     1,
        removePageBorders: false,
    });

    pdfLinkService.setViewer(pdfViewer);
    return { eventBus, pdfLinkService, pdfViewer };
}

// ── PDF loading ───────────────────────────────────────────────────────────────

/**
 * Loads the given ArrayBuffer into the provided pdfViewer instance.
 * Returns the resolved PDFDocumentProxy.
 */
export async function loadPDF(pdfData, pdfViewer, pdfLinkService) {
    const loading = document.getElementById('loading');
    loading.style.display = 'block';
    try {
        const loadingTask = pdfjsLib.getDocument({ data: pdfData });
        const pdfDocument = await loadingTask.promise;
        pdfViewer.setDocument(pdfDocument);
        pdfLinkService.setDocument(pdfDocument);
        pdfViewer.currentScaleValue = '1';
        loading.style.display = 'none';
        return pdfDocument;
    } catch (error) {
        console.error('Error loading PDF:', error);
        loading.innerHTML = '<p style="color:#dc2626;font-family:Inter,sans-serif">Error loading PDF</p>';
        return null;
    }
}

// ── Highlight helpers ─────────────────────────────────────────────────────────

export function clearAllHighlights() {
    document.querySelectorAll('.bbox-highlight').forEach(el => el.remove());
}

/**
 * Converts pdfium coords (bottom-left origin) to PDF.js viewport coords (top-left origin).
 */
export function pdfiumRectsToViewport(rects, viewport) {
    return rects.map(r => {
        const [x1, y1] = viewport.convertToViewportPoint(r.x, r.y + r.height);
        const [x2, y2] = viewport.convertToViewportPoint(r.x + r.width, r.y);
        return {
            left:   Math.min(x1, x2),
            top:    Math.min(y1, y2),
            width:  Math.abs(x2 - x1),
            height: Math.abs(y2 - y1),
        };
    });
}

export function mergeRectsOnSameLine(rects, tolerance = 3) {
    if (!rects.length) return rects;
    const sorted = [...rects].sort((a, b) =>
        Math.abs(a.top - b.top) < tolerance ? a.left - b.left : a.top - b.top
    );
    const merged = [];
    let current = { ...sorted[0] };
    for (let i = 1; i < sorted.length; i++) {
        const r           = sorted[i];
        const sameLine    = Math.abs(r.top - current.top) < tolerance;
        const closeEnough = r.left <= current.left + current.width + 6;
        if (sameLine && closeEnough) {
            const right  = Math.max(current.left + current.width,  r.left + r.width);
            const bottom = Math.max(current.top  + current.height, r.top  + r.height);
            current.top    = Math.min(current.top,  r.top);
            current.left   = Math.min(current.left, r.left);
            current.width  = right  - current.left;
            current.height = bottom - current.top;
        } else {
            merged.push(current);
            current = { ...r };
        }
    }
    merged.push(current);
    return merged;
}

/**
 * @param {object}  highlight            - { page, rects }
 * @param {number}  index                - position in matchResults array
 * @param {number}  selectedHighlightIndex
 * @param {object}  pdfViewer
 */
export function renderHighlight(highlight, index, selectedHighlightIndex, pdfViewer) {
    const pageView = pdfViewer.getPageView(highlight.page - 1);
    if (!pageView || !highlight.rects || !highlight.rects.length) return;

    const isSelected    = index === selectedHighlightIndex;
    const viewport      = pageView.viewport;
    const viewportRects = pdfiumRectsToViewport(highlight.rects, viewport);
    const cleanRects    = mergeRectsOnSameLine(viewportRects);

    for (const rect of cleanRects) {
        const el = document.createElement('div');
        el.className     = 'bbox-highlight';
        el.dataset.page  = highlight.page;
        el.dataset.index = index;
        el.style.cssText = `
            position:       absolute;
            left:           ${rect.left}px;
            top:            ${rect.top}px;
            width:          ${rect.width}px;
            height:         ${rect.height}px;
            background:     ${isSelected ? 'rgba(255, 140, 0, 0.45)' : 'rgba(255, 220, 0, 0.35)'};
            border-radius:  3px;
            pointer-events: none;
            mix-blend-mode: multiply;
        `;
        pageView.div.appendChild(el);
    }
}

export function applyHighlights(results, pdfViewer) {
    clearAllHighlights();
    results.forEach((h, i) => renderHighlight(h, i, -1, pdfViewer));
}

/**
 * Scrolls to and highlights the match at the given index.
 * Returns the updated selectedHighlightIndex (same as index).
 */
export function scrollToMatch(index, activeHighlights, pdfViewer) {
    const h = activeHighlights[index];
    if (!h) return index;

    clearAllHighlights();
    renderHighlight(h, index, index, pdfViewer);

    const allRects = h.rects;
    const maxY     = Math.max(...allRects.map(r => r.y + r.height));
    const padding  = 50;

    pdfViewer.scrollPageIntoView({
        pageNumber: h.page,
        destArray:  [null, { name: 'XYZ' }, null, maxY + padding, null],
    });

    return index; // new selectedHighlightIndex
}

// ── Match counter UI ──────────────────────────────────────────────────────────

export function updateMatchCounter(matchResults, currentMatchIndex, searchInput, prevButton, nextButton, matchCounter) {
    const total = matchResults.length;
    if (total === 0) {
        matchCounter.textContent = searchInput.value.trim() ? 'No matches' : '';
        prevButton.disabled = true;
        nextButton.disabled = true;
    } else {
        matchCounter.textContent = `${currentMatchIndex + 1} / ${total}`;
        prevButton.disabled = currentMatchIndex === 0;
        nextButton.disabled = currentMatchIndex === total - 1;
    }
}

// ── Backend search ────────────────────────────────────────────────────────────
export async function getBackendResults(query, documentId, signal) {
    const response = await fetch(`/api/search?q=${encodeURIComponent(query)}&id=${documentId}`, {
        signal // 👈 fetch will throw AbortError if cancelled
    });
    if (!response.ok) throw new Error('Search failed');
    return response.json();
}

// ── Poll Backend ────────────────────────────────────────────────────────────
export async function waitUntilReady(docId, intervalMs = 500, timeoutMs = 60000) {
    const start = Date.now();

    while (Date.now() - start < timeoutMs) {
        try {
            const res = await fetch(`/api/ready?id=${docId}`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);

            const data = await res.json();
            if (data.ready) return;
        } catch (err) {
            console.error("Polling error:", err);
            // Optional: decide if you want to break instead
        }

        await new Promise(r => setTimeout(r, intervalMs));
    }

    throw new Error('Timed out waiting for PDF to be indexed');
}