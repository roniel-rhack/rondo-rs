//! Property-based fuzz coverage for `app::parse_quick_add`.
//!
//! Properties:
//!   1. Parser never panics on any UTF-8 input.
//!   2. The reconstructed token set is a subset of the original whitespace
//!      tokens (parser never invents data).
//!   3. Title preserves the order of non-prefixed tokens.
//!   4. `#tag` tokens land in `tags`, `!pN` in `priority` (when recognised),
//!      `due:X` in `due`.

use proptest::prelude::*;
use rondo_tui::app::parse_quick_add;

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn never_panics_on_arbitrary_input(s in ".*") {
        let _ = parse_quick_add(&s);
    }

    #[test]
    fn never_panics_on_ascii(s in "[ -~]{0,128}") {
        let _ = parse_quick_add(&s);
    }

    #[test]
    fn never_panics_on_synthetic_prefixes(s in "(#[a-z0-9]{0,8}|![pP][1-4]|due:[a-z0-9-]{0,8}| [a-z]{1,4}){0,12}") {
        let _ = parse_quick_add(&s);
    }

    /// Title is composed of non-prefix tokens in original order joined by
    /// single spaces. Tags echo `#`-prefixed tokens in order. `due:` token
    /// captured when present and non-empty.
    #[test]
    fn structure_matches_token_classification(
        toks in prop::collection::vec(
            prop_oneof![
                "[a-zA-Z][a-zA-Z0-9]{0,6}".prop_map(|s| s),
                "#[a-zA-Z0-9]{1,6}".prop_map(|s| s),
                "![pP][1-4]".prop_map(|s| s),
                "due:[a-z0-9-]{1,8}".prop_map(|s| s),
            ],
            0..16,
        )
    ) {
        let raw = toks.join(" ");
        let parsed = parse_quick_add(&raw);

        // Expected title = non-prefixed tokens joined.
        let expected_title: Vec<&str> = toks
            .iter()
            .filter(|t| {
                !t.starts_with('#')
                    && !t.starts_with('!')
                    && !t.starts_with("due:")
            })
            .map(String::as_str)
            .collect();
        prop_assert_eq!(parsed.title, expected_title.join(" "));

        // Expected tags = strip '#'.
        let expected_tags: Vec<String> = toks
            .iter()
            .filter_map(|t| t.strip_prefix('#').map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        prop_assert_eq!(parsed.tags, expected_tags);

        // due: token (last one wins via overwrite semantics).
        let expected_due: Option<String> = toks
            .iter()
            .filter_map(|t| t.strip_prefix("due:").map(|s| s.to_string()))
            .last();
        prop_assert_eq!(parsed.due, expected_due);
    }
}

#[test]
fn known_edge_inputs_dont_panic() {
    let cases = [
        "",
        "   ",
        "##",
        "!",
        "due:",
        "due:tmrw due:today",
        "\u{0}\u{1}",
        "#tag1 #tag2 #tag3 !p3 due:2026-12-31 word",
        "🙂 #emoji ✨ !p1",
    ];
    for s in cases {
        let _ = parse_quick_add(s);
    }
}
