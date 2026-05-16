use crate::theme::Theme;
use chrono::{Local, NaiveDate};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// Returns a soft due badge.
/// - `OVERDUE` and `TODAY` always shown (high-signal).
/// - Distant dates (>3 days out) return None — show date as muted text elsewhere.
/// - Near-future (1-3 days) returns `in Xd` muted hint instead of UPCOMING.
pub fn span(due: Option<NaiveDate>, theme: &Theme) -> Option<Span<'static>> {
    let due = due?;
    let today = Local::now().date_naive();
    let delta = (due - today).num_days();
    if delta < 0 {
        return Some(Span::styled(
            "overdue".to_string(),
            Style::default()
                .fg(theme.danger)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if delta == 0 {
        return Some(Span::styled(
            "today".to_string(),
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        ));
    }
    if delta <= 3 {
        return Some(Span::styled(
            format!("in {}d", delta),
            Style::default().fg(theme.fg_muted),
        ));
    }
    None
}
