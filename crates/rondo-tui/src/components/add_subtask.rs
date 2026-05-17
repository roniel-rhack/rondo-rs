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
    let parent_title = app
        .data
        .selected_task()
        .map(|x| x.title.clone())
        .unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" + subtask ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let context = Line::from(vec![
        Span::styled(" parent: ", t.muted()),
        Span::styled(parent_title, Style::default().fg(t.fg_muted)),
    ]);
    let line = Line::from(vec![
        Span::styled(" ↳ ", t.accent_style()),
        Span::styled(
            app.modals.add_subtask_buf.clone(),
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
        Span::styled(" add  ", t.muted()),
        Span::styled("Esc", Style::default().fg(t.accent)),
        Span::styled(" cancel", t.muted()),
    ]);
    f.render_widget(Paragraph::new(vec![context, line, hint]), inner);
}
