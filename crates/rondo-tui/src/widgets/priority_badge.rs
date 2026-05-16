use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use rondo_core::domain::task::Priority;

pub fn span(p: Priority, theme: &Theme) -> Span<'static> {
    Span::styled(
        format!(" {} ", p.label()),
        Style::default()
            .fg(theme.priority_color(p))
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    )
}
