import pdfplumber
import re
from collections import Counter

def extract_clean_pdf_text(
    pdf_path: str,
    header_margin: float = 50,
    footer_margin: float = 50,
    x_tolerance: float = 1,
    y_tolerance: float = 1
) -> str:
    """
    Extracts clean text from a PDF:
    - Ignores images
    - Removes headers/footers
    - Fixes hyphenated words split across lines
    - Fixes missing spaces after punctuation
    - Fixes merged camelCase words
    - Normalizes whitespace
    - Preserves paragraph breaks
    """
    page_texts = []
    all_lines = []

    # 1️⃣ Extract words per page, filter by vertical position
    with pdfplumber.open(pdf_path) as pdf:
        for page in pdf.pages:
            words = page.extract_words(x_tolerance=x_tolerance, y_tolerance=y_tolerance)

            # Filter out headers/footers
            filtered_words = [
                w for w in words
                if header_margin <= w['top'] <= page.height - footer_margin
            ]

            # Group words by approximate y-coordinate (lines)
            lines_dict = {}
            for w in filtered_words:
                y_key = round(w['top'], 1)  # cluster words in the same line
                lines_dict.setdefault(y_key, []).append((w['x0'], w['text']))

            # Sort words horizontally and join them into lines
            lines = []
            for line_words in sorted(lines_dict.values(), key=lambda x: x[0]):
                sorted_words = sorted(line_words, key=lambda x: x[0])
                line_text = " ".join(word for _, word in sorted_words)
                lines.append(line_text)

            page_texts.append("\n".join(lines))
            all_lines.extend(lines)

    # 2️⃣ Remove repeated headers/footers across pages
    line_counts = Counter(all_lines)
    repeating_lines = {line for line, count in line_counts.items() if count > 2}

    cleaned_pages = []
    for page_text in page_texts:
        page_lines = page_text.split("\n")
        page_lines = [line for line in page_lines if line not in repeating_lines]
        cleaned_pages.append("\n".join(page_lines))

    text = "\n\n".join(cleaned_pages)

    # 3️⃣ Fix hyphenated words across lines: "rejec-\nting" → "rejecting"
    text = re.sub(r"-\s*\n\s*", "", text)

    # 4️⃣ Fix missing spaces after punctuation: "solve.Let" → "solve. Let"
    text = re.sub(r"([.!?])([A-Z0-9])", r"\1 \2", text)

    # 5️⃣ Fix camelCase / merged words: "usefulRelevant" → "useful Relevant"
    text = re.sub(r"([a-z])([A-Z])", r"\1 \2", text)

    # 6️⃣ Normalize whitespace
    text = re.sub(r"\s+", " ", text)

    return text.strip()

extracted = extract_clean_pdf_text("z2p.pdf")
print(extracted)

        

