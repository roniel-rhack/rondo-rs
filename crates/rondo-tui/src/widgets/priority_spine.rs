use crate::theme::Theme;
use ratatui::{style::Style, text::Span};
use rondo_core::domain::task::Priority;

/// Vertical priority spine glyph — wider for higher priority.
/// Reads aggregate priority of the list at a glance without parsing badges.
pub fn glyph(p: Priority, theme: &Theme) -> Span<'static> {
    let (ch, color) = match p {
        Priority::Urgent => ("▌", theme.danger),
        Priority::High => ("▍", theme.danger),
        Priority::Med => ("▎", theme.warn),
        Priority::Low => ("▏", theme.success),
    };
    Span::styled(ch, Style::default().fg(color))
}
