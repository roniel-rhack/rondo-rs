use crate::theme::Theme;
use chrono::{Local, NaiveDate};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

pub fn span(due: Option<NaiveDate>, theme: &Theme) -> Option<Span<'static>> {
    let due = due?;
    let today = Local::now().date_naive();
    let (label, color) = if due < today {
        ("OVERDUE", theme.danger)
    } else if due == today {
        ("TODAY", theme.warn)
    } else {
        ("UPCOMING", theme.fg_muted)
    };
    Some(Span::styled(
        format!(" {} ", label),
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    ))
}
