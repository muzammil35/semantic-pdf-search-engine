pub fn fuzzy_search(
    char_entries: &[(usize, char)],
    needle_chars: &[char],
    threshold: f32,
) -> Vec<(usize, usize, f32)> {
    let needle_len = needle_chars.len();
    if needle_len == 0 || char_entries.is_empty() {
        return vec![];
    }

    // Precompute lowercased haystack once
    let haystack: Vec<char> = char_entries
        .iter()
        .map(|(_, ch)| ch.to_lowercase().next().unwrap_or(*ch))
        .collect();
    let haystack_len = haystack.len();

    let needle_lower: Vec<char> = needle_chars
        .iter()
        .map(|ch| ch.to_lowercase().next().unwrap_or(*ch))
        .collect();

    // 1. Exact match
    if let Some(pos) = find_exact(&haystack, &needle_lower) {
        return vec![(pos, pos + needle_len, 1.0)];
    }

    let window_min = needle_len;
    let window_max = (needle_len as f32 * 1.3).ceil() as usize;

    // 2. Anchor search — use a short prefix to find candidate positions cheaply
    let anchor_len = (needle_len / 6).clamp(2, 8);
    let anchor = &needle_lower[..anchor_len];

    let mut candidate_starts: Vec<usize> = Vec::new();

    for pos in 0..haystack_len.saturating_sub(anchor_len - 1) {
        if haystack[pos..pos + anchor_len] == *anchor {
            // Evaluate a small neighbourhood around the anchor hit
            let start = pos.saturating_sub(3);
            candidate_starts.push(start);
        }
    }

    // 3. Sparse fallback if anchor found nothing
    if candidate_starts.is_empty() {
        let step = (needle_len / 4).max(1);
        for i in (0..haystack_len.saturating_sub(window_min - 1)).step_by(step) {
            candidate_starts.push(i);
        }
    }

    // Score only the candidate windows
    let mut matches: Vec<(usize, usize, f32)> = Vec::new();

    // Dedup candidates (multiple anchors can produce nearby starts)
    candidate_starts.sort_unstable();
    candidate_starts.dedup();

    let mut last_match_end = 0usize;

    for start in candidate_starts {
        if start < last_match_end {
            continue;
        }
        if start >= haystack_len {
            continue;
        }

        let mut best_score = 0.0f32;
        let mut best_end = 0usize;

        for window_size in window_min..=window_max {
            let end = start + window_size;
            if end > haystack_len {
                break;
            }

            let score = jaro_winkler(&needle_lower, &haystack[start..end]);
            if score >= threshold && score > best_score {
                best_score = score;
                best_end = end;
                if score > 0.97 {
                    break; // good enough, stop expanding
                }
            }
        }

        if best_score >= threshold {
            matches.push((start, best_end, best_score));
            last_match_end = best_end;
        }
    }

    deduplicate_matches(matches)
}

/// Naive exact subsequence search — O(n*m) but m is small and this exits early
fn find_exact(haystack: &[char], needle: &[char]) -> Option<usize> {
    let n = needle.len();
    haystack.windows(n).position(|w| w == needle)
}

pub fn jaro_winkler(a: &[char], b: &[char]) -> f32 {
    let jaro = jaro(a, b);
    let prefix = a
        .iter()
        .zip(b.iter())
        .take(4)
        .take_while(|(x, y)| x == y)
        .count();
    jaro + (prefix as f32 * 0.1 * (1.0 - jaro))
}

fn jaro(a: &[char], b: &[char]) -> f32 {
    let a_len = a.len();
    let b_len = b.len();
    if a_len == 0 && b_len == 0 {
        return 1.0;
    }
    if a_len == 0 || b_len == 0 {
        return 0.0;
    }

    let match_dist = (a_len.max(b_len) / 2).saturating_sub(1);
    let mut a_matches = vec![false; a_len];
    let mut b_matches = vec![false; b_len];
    let mut matches = 0usize;
    let mut transpositions = 0usize;

    for i in 0..a_len {
        let start = i.saturating_sub(match_dist);
        let end = (i + match_dist + 1).min(b_len);
        for j in start..end {
            if b_matches[j] || a[i] != b[j] {
                continue;
            }
            a_matches[i] = true;
            b_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut k = 0;
    for i in 0..a_len {
        if !a_matches[i] {
            continue;
        }
        while !b_matches[k] {
            k += 1;
        }
        if a[i] != b[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let m = matches as f32;
    let t = (transpositions / 2) as f32;
    (m / a_len as f32 + m / b_len as f32 + (m - t) / m) / 3.0
}

pub fn deduplicate_matches(mut matches: Vec<(usize, usize, f32)>) -> Vec<(usize, usize, f32)> {
    matches.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    let mut kept: Vec<(usize, usize, f32)> = Vec::new();
    'outer: for (start, end, score) in matches {
        for &(ks, ke, _) in &kept {
            if start < ke && end > ks {
                continue 'outer;
            }
        }
        kept.push((start, end, score));
    }
    kept.sort_by_key(|m| m.0);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// Wrap a plain string into the (pdf_char_idx, char) format fuzzy_search expects.
    fn char_entries(s: &str) -> Vec<(usize, char)> {
        s.chars().enumerate().collect()
    }

    #[test]
    fn fuzzy_search_hyphenated_word_matches_clean() {
        // PDF sometimes breaks "information" as "infor-\nmation"
        let haystack = char_entries("infor-\nmation");
        let needle = chars("information");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(
            !results.is_empty(),
            "hyphenated word should fuzzy-match clean needle"
        );
    }

    #[test]
    fn fuzzy_search_extra_spaces_between_words() {
        // PDF columns or justified text can produce runs of spaces
        let haystack = char_entries("the  quick   brown  fox");
        let needle = chars("quick brown");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(!results.is_empty(), "extra spaces should still fuzzy-match");
    }

    #[test]
    fn fuzzy_search_mid_word_hyphen_with_newline() {
        // "self-\ncontained" from a PDF line break
        let haystack = char_entries("a self-\ncontained system");
        let needle = chars("self-contained");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(
            !results.is_empty(),
            "soft-hyphen line break should still match"
        );
    }

    #[test]
    fn fuzzy_search_soft_hyphen_invisible() {
        // U+00AD soft hyphen is sometimes injected by PDF encoders mid-word
        let haystack = char_entries("compre\u{00AD}hensive");
        let needle = chars("comprehensive");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(
            !results.is_empty(),
            "soft hyphen mid-word should still fuzzy-match"
        );
    }

    #[test]
    fn fuzzy_search_double_spaced_sentence() {
        // Old PDF typewriter convention: two spaces after a period
        let haystack = char_entries("the  result.  The  value  is  correct");
        let needle = chars("result. The value");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(
            !results.is_empty(),
            "double-spaced text should fuzzy-match clean needle"
        );
    }

    #[test]
    fn fuzzy_search_missing_space_after_extraction() {
        // PDF glyph spacing sometimes causes spaces to be dropped entirely
        let haystack = char_entries("wordswithoutspaces");
        let needle = chars("without");
        let results = fuzzy_search(&haystack, &needle, 0.99);
        assert!(
            !results.is_empty(),
            "substring without spaces should still exact-match"
        );
    }

    #[test]
    fn fuzzy_search_curly_quotes_vs_straight() {
        // PDF fonts often encode " as \u{201C}/\u{201D}
        let haystack = char_entries("\u{201C}quoted text\u{201D}");
        let needle = chars("\"quoted text\"");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(
            !results.is_empty(),
            "curly quotes should fuzzy-match straight quotes"
        );
    }

    #[test]
    fn fuzzy_search_em_dash_vs_hyphen() {
        // PDFs frequently encode em-dashes (—) where the source had a hyphen
        let haystack = char_entries("well\u{2014}known");
        let needle = chars("well-known");
        let results = fuzzy_search(&haystack, &needle, 0.75);
        assert!(!results.is_empty(), "em-dash should fuzzy-match hyphen");
    }
}
