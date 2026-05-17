use crate::theme::Theme;
use chrono::NaiveDate;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// Pill-shaped due badge: `◖overdue◗`, `◖today◗`, or near-future muted hint.
pub fn span(due: Option<NaiveDate>, theme: &Theme) -> Option<Span<'static>> {
    let due = due?;
    let today = crate::clock::today();
    let delta = (due - today).num_days();
    if delta < 0 {
        return Some(Span::styled(
            "◖overdue◗".to_string(),
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if delta == 0 {
        return Some(Span::styled(
            "◖today◗".to_string(),
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        ));
    }
    if delta <= 3 {
        return Some(Span::styled(
            format!("◖in {}d◗", delta),
            Style::default().fg(theme.fg_muted),
        ));
    }
    None
}
