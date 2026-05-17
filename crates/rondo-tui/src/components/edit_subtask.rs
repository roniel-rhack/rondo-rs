use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" ✎ rename subtask ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let line = Line::from(vec![
        Span::styled(" ↳ ", t.accent_style()),
        Span::styled(
            app.modals.edit_subtask_buf.clone(),
            Style::default().fg(t.fg),
        ),
        Span::styled(
            "▏",
            Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);
    let hint = Line::from(vec![
        Span::styled("  ", t.muted()),
        Span::styled("Enter", Style::default().fg(t.accent)),
        Span::styled(" save  ", t.muted()),
        Span::styled("Esc", Style::default().fg(t.accent)),
        Span::styled(" cancel", t.muted()),
    ]);
    f.render_widget(Paragraph::new(vec![line, hint]), inner);
}
