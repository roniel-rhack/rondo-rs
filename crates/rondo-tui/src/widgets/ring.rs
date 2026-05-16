use crate::theme::Theme;
use ratatui::{style::Style, text::Span};

/// Four-state activity ring for progress visualization.
/// Maps ratio to ◯ ◔ ◑ ◕ ●
pub fn glyph(done: usize, total: usize, theme: &Theme) -> Span<'static> {
    if total == 0 {
        return Span::styled("◯", Style::default().fg(theme.fg_muted));
    }
    let ratio = done as f64 / total as f64;
    let (ch, color) = if ratio >= 1.0 {
        ("●", theme.success)
    } else if ratio >= 0.75 {
        ("◕", theme.success)
    } else if ratio >= 0.5 {
        ("◑", theme.warn)
    } else if ratio > 0.0 {
        ("◔", theme.warn)
    } else {
        ("◯", theme.fg_muted)
    };
    Span::styled(ch, Style::default().fg(color))
}
