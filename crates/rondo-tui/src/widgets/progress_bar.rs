use crate::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub fn line(done: usize, total: usize, width: usize, theme: &Theme) -> Line<'static> {
    if total == 0 {
        return Line::raw("");
    }
    let ratio = done as f64 / total as f64;
    let filled = ((width as f64) * ratio).round() as usize;
    let empty = width.saturating_sub(filled);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(theme.success)),
        Span::styled("░".repeat(empty), Style::default().fg(theme.fg_muted)),
        Span::raw(format!("  {}/{}", done, total)),
    ])
}
