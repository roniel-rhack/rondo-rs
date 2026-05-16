use crate::theme::Theme;
use ratatui::{style::Style, text::Span};

const BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Text-based sparkline from a slice of values. Returns a single Span.
/// Empty input yields a placeholder of muted dots.
pub fn span(values: &[u64], theme: &Theme) -> Span<'static> {
    if values.is_empty() {
        return Span::styled("·······", Style::default().fg(theme.fg_muted));
    }
    let max = *values.iter().max().unwrap_or(&1).max(&1);
    let chars: String = values
        .iter()
        .map(|v| {
            let idx = ((*v as f64 / max as f64) * (BLOCKS.len() - 1) as f64).round() as usize;
            BLOCKS[idx.min(BLOCKS.len() - 1)]
        })
        .collect();
    Span::styled(chars, Style::default().fg(theme.accent))
}
