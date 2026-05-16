use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use rondo_core::domain::task::Priority;

/// Soft inline badge: color + bold, no REVERSED.
/// `URG!` exclamation removed — color carries the urgency signal.
pub fn span(p: Priority, theme: &Theme) -> Span<'static> {
    let label = match p {
        Priority::Low => "low",
        Priority::Med => "med",
        Priority::High => "high",
        Priority::Urgent => "URG",
    };
    Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme.priority_color(p))
            .add_modifier(Modifier::BOLD),
    )
}
