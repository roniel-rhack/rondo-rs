use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use rondo_core::domain::task::Priority;

/// Rounded-end pill `◖URG◗` colored by priority. Sci-fi capsule aesthetic.
pub fn span(p: Priority, theme: &Theme) -> Span<'static> {
    let label = match p {
        Priority::Low => "low",
        Priority::Med => "med",
        Priority::High => "high",
        Priority::Urgent => "URG",
    };
    let color = theme.priority_color(p);
    Span::styled(
        format!("◖{}◗", label),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}
