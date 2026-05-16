//! Fuzzy-match helper around `nucleo` for the `/` search overlay.
//!
//! `SearchEngine` wraps a [`nucleo::Matcher`] plus the per-call scratch
//! buffers that `Utf32Str::new` requires. It is intentionally small —
//! callers feed needle + haystack strings and get back an `Option<(score,
//! match_indices_in_chars)>`. All matching is case-insensitive (smart
//! normalization, see `nucleo::Config::DEFAULT`).

use nucleo::{Config, Matcher, Utf32Str};

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
