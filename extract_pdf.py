import pdfplumber
import re
import sys
import json
from collections import Counter
from typing import List, Dict, Tuple


def _post_process_text(text: str) -> str:
    """Shared cleanup logic."""
    text = re.sub(r"-\s*\n\s*", "", text)
    text = re.sub(r"([.!?])([A-Z0-9])", r"\1 \2", text)
    text = re.sub(r"([a-z])([A-Z])", r"\1 \2", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def extract_clean_pdf_text(
    pdf_path: str,
    header_margin: float = 50,
    footer_margin: float = 50,
    x_tolerance: float = 1,
    y_tolerance: float = 1
) -> Tuple[str, List[Dict[int, str]]]:
    """
    Returns:
    - full cleaned document text
    - list of dicts: [{page_number: page_text}, ...]
    """

    raw_page_lines = []   # [(page_num, [lines])]
    all_lines = []

    # 1️⃣ Extract per-page lines
    with pdfplumber.open(pdf_path) as pdf:
        for page in pdf.pages:
            page_num = page.page_number

            words = page.extract_words(
                x_tolerance=x_tolerance,
                y_tolerance=y_tolerance
            )

            filtered_words = [
                w for w in words
                if header_margin <= w["top"] <= page.height - footer_margin
            ]

            lines_dict = {}
            for w in filtered_words:
                y_key = round(w["top"], 1)
                lines_dict.setdefault(y_key, []).append((w["x0"], w["text"]))

            lines = []
            for line_words in sorted(lines_dict.values(), key=lambda x: x[0]):
                sorted_words = sorted(line_words, key=lambda x: x[0])
                line_text = " ".join(word for _, word in sorted_words)
                lines.append(line_text)

            raw_page_lines.append((page_num, lines))
            all_lines.extend(lines)

    # 2️⃣ Detect repeating headers/footers
    line_counts = Counter(all_lines)
    repeating_lines = {line for line, count in line_counts.items() if count > 2}

    # 3️⃣ Build per-page cleaned output
    pages: List[Dict[int, str]] = []
    cleaned_pages_text = []

    for page_num, lines in raw_page_lines:
        page_lines = [line for line in lines if line not in repeating_lines]
        page_text = "\n".join(page_lines)
        page_text = _post_process_text(page_text)

        pages.append({
            "page": page_num,
            "text": page_text
        })
        cleaned_pages_text.append(page_text)

    # 4️⃣ Full document text
    full_text = "\n\n".join(cleaned_pages_text)
    
    return full_text, pages


# ─────────────────────────────────────────────
# CLI entrypoint (subprocess-friendly)
# ─────────────────────────────────────────────

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(
            "Usage: python extract_pdf.py <pdf_path>",
            file=sys.stderr
        )
        sys.exit(1)

    pdf_path = sys.argv[1]

    try:
        full_text, pages = extract_clean_pdf_text(pdf_path)

        output = {
            "full_text": full_text,
            "pages": pages
        }

        # stdout is the API
        print(json.dumps(output, ensure_ascii=False))

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(2)
