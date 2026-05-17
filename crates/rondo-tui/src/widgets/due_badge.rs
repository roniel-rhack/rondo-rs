use crate::theme::Theme;
use chrono::NaiveDate;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// Pill-shaped due badge: `◖overdue◗`, `◖today◗`, or near-future muted hint.
///
/// `today` is injected from the caller so deterministic tests can pin the
/// rendered text without consulting wall-clock state.
pub fn span(due: Option<NaiveDate>, today: NaiveDate, theme: &Theme) -> Option<Span<'static>> {
    let due = due?;
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
