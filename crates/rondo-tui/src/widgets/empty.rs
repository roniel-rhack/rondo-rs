//! Shared empty-state helper.
//!
//! Renders the canonical `(empty) <key> to add` one-liner used by every
//! detail / journal sub-section that can be vacuously filled. Keeps the
//! wording, indentation, and accent styling identical across the app.

use crate::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::Paragraph,
};

/// Single-line empty hint: `  (empty)  KEY to add`.
///
/// `section` is the (lowercased) display name — currently rendered as the
/// `(empty)` literal so the hint stays terse, but kept in the signature so
/// future variants can read it without breaking call sites.
pub fn line(_section: &str, key: &str, hint: &str, t: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled("(empty)  ", t.muted()),
        Span::styled(key.to_string(), t.kbd()),
        Span::raw(" "),
        Span::styled(hint.to_string(), t.muted()),
    ])
}

/// Same as [`line`] but wrapped in a [`Paragraph`] for callers that just
/// want to render the hint directly into a rect.
pub fn paragraph(section: &str, key: &str, hint: &str, t: &Theme) -> Paragraph<'static> {
    Paragraph::new(line(section, key, hint, t))
}
