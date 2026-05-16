//! Fuzzy-match helper around `nucleo` for the `/` search overlay.
//!
//! `SearchEngine` wraps a [`nucleo::Matcher`] plus the per-call scratch
//! buffers that `Utf32Str::new` requires. It is intentionally small —
//! callers feed needle + haystack strings and get back an `Option<(score,
//! match_indices_in_chars)>`. All matching is case-insensitive (smart
//! normalization, see `nucleo::Config::DEFAULT`).

use nucleo::{Config, Matcher, Utf32Str};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

pub struct SearchEngine {
    matcher: Matcher,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        Self {
            matcher: Matcher::new(config),
        }
    }

    /// Returns `(score, match_indices)` where indices are CHAR positions
    /// (not byte offsets) into `haystack`. Returns `None` when:
    /// - `needle` is empty (we treat empty as "no filter", handled
    ///   by the caller)
    /// - no fuzzy match found
    pub fn score(&mut self, needle: &str, haystack: &str) -> Option<(u16, Vec<u32>)> {
        if needle.is_empty() {
            return None;
        }
        // nucleo's `ignore_case` only normalizes the haystack — the needle
        // is expected to already be lower-case. Doing it here keeps callers
        // honest.
        let needle_lc = needle.to_lowercase();
        let mut needle_buf: Vec<char> = Vec::new();
        let mut haystack_buf: Vec<char> = Vec::new();
        let needle_u32 = Utf32Str::new(&needle_lc, &mut needle_buf);
        let haystack_u32 = Utf32Str::new(haystack, &mut haystack_buf);
        let mut indices: Vec<u32> = Vec::new();
        let score = self
            .matcher
            .fuzzy_indices(haystack_u32, needle_u32, &mut indices)?;
        Some((score, indices))
    }

    /// Convenience: just the score, no positions. Slightly cheaper.
    pub fn score_only(&mut self, needle: &str, haystack: &str) -> Option<u16> {
        if needle.is_empty() {
            return None;
        }
        let needle_lc = needle.to_lowercase();
        let mut needle_buf: Vec<char> = Vec::new();
        let mut haystack_buf: Vec<char> = Vec::new();
        let needle_u32 = Utf32Str::new(&needle_lc, &mut needle_buf);
        let haystack_u32 = Utf32Str::new(haystack, &mut haystack_buf);
        self.matcher.fuzzy_match(haystack_u32, needle_u32)
    }
}

/// Rewrites `line`'s spans so that characters matched by the fuzzy
/// query are styled with accent foreground + bold + underline, while
/// every other char keeps its original `Span::style`. Preserves the
/// `Line::style` and `Line::alignment`. Returns the input unchanged
/// when `needle` is empty or no fuzzy match is found.
///
/// Used by the search overlay to surface matches in BOTH the task
/// list and the detail panel (title, tags, description, subtasks,
/// notes — every line gets the same treatment).
pub fn highlight_line(line: Line<'static>, needle: &str, theme: &Theme) -> Line<'static> {
    if needle.trim().is_empty() {
        return line;
    }
    let mut full = String::new();
    for s in &line.spans {
        full.push_str(s.content.as_ref());
    }
    if full.is_empty() {
        return line;
    }
    let mut engine = SearchEngine::new();
    let Some((_, indices)) = engine.score(needle.trim(), &full) else {
        return line;
    };
    if indices.is_empty() {
        return line;
    }
    let match_set: std::collections::BTreeSet<u32> = indices.into_iter().collect();
    let hl_extra = Modifier::UNDERLINED | Modifier::BOLD;
    let line_style = line.style;
    let line_alignment = line.alignment;
    let original_spans = line.spans;

    let mut out_spans: Vec<Span<'static>> = Vec::new();
    let mut char_idx: u32 = 0;
    for span in original_spans {
        let base_style = span.style;
        let hl_style = Style::default()
            .patch(base_style)
            .fg(theme.accent)
            .add_modifier(hl_extra);
        let mut buf = String::new();
        let mut buf_hl: Option<bool> = None;
        for ch in span.content.chars() {
            let is_hl = match_set.contains(&char_idx);
            char_idx += 1;
            if buf_hl != Some(is_hl) && !buf.is_empty() {
                let style = if buf_hl == Some(true) { hl_style } else { base_style };
                out_spans.push(Span::styled(std::mem::take(&mut buf), style));
            }
            buf.push(ch);
            buf_hl = Some(is_hl);
        }
        if !buf.is_empty() {
            let style = if buf_hl == Some(true) { hl_style } else { base_style };
            out_spans.push(Span::styled(buf, style));
        }
    }
    let mut out = Line::from(out_spans);
    out.style = line_style;
    out.alignment = line_alignment;
    out
}

#[cfg(test)]
mod tests {
    use super::SearchEngine;

    #[test]
    fn matches_substring() {
        let mut s = SearchEngine::new();
        assert!(s.score("api", "Review API spec").is_some());
    }

    #[test]
    fn no_match_returns_none() {
        let mut s = SearchEngine::new();
        assert!(s.score("xyz", "Review API spec").is_none());
    }

    #[test]
    fn empty_needle_returns_none() {
        let mut s = SearchEngine::new();
        assert!(s.score("", "anything").is_none());
    }

    #[test]
    fn score_higher_for_closer_match() {
        let mut s = SearchEngine::new();
        let close = s.score_only("apis", "API spec").unwrap_or(0);
        let far = s
            .score_only("apis", "abandoned project's index syntax")
            .unwrap_or(0);
        assert!(close > far, "close={} far={}", close, far);
    }

    #[test]
    fn case_insensitive() {
        let mut s = SearchEngine::new();
        assert!(s.score("api", "REVIEW api SPEC").is_some());
        assert!(s.score("API", "review api spec").is_some());
    }

    #[test]
    fn indices_are_inside_haystack() {
        let mut s = SearchEngine::new();
        let hay = "Review API spec";
        let (_, idx) = s.score("api", hay).unwrap();
        assert!(idx.iter().all(|&i| (i as usize) < hay.chars().count()));
        assert!(!idx.is_empty());
    }
}
