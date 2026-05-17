use crate::app::modals_state::DepOverlayMode;
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
    let task = app.data.selected_task();
    let task_title = task.map(|x| x.title.clone()).unwrap_or_default();
    let mode_label = match app.modals.dep_overlay_mode {
        DepOverlayMode::Add => "+ dependency",
        DepOverlayMode::Remove => "- dependency",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(format!(" {} ", mode_label), t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let context = Line::from(vec![
        Span::styled(" task: ", t.muted()),
        Span::styled(task_title, Style::default().fg(t.fg_muted)),
    ]);

    let existing = task
        .map(|t| t.blocked_by_ids.clone())
        .unwrap_or_default();
    let existing_line = if existing.is_empty() {
        Line::from(vec![Span::styled(" blocked by: (none) ", t.muted())])
    } else {
        let mut spans = vec![Span::styled(" blocked by: ", t.muted())];
        for (i, id) in existing.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(", ", t.muted()));
            }
            spans.push(Span::styled(
                format!("#{}", id),
                Style::default().fg(t.danger),
            ));
        }
        Line::from(spans)
    };

    let prompt = match app.modals.dep_overlay_mode {
        DepOverlayMode::Add => " enter blocker task id: ",
        DepOverlayMode::Remove => " enter blocker id to remove: ",
    };
    let line = Line::from(vec![
        Span::styled(prompt, t.muted()),
        Span::styled(
            app.modals.dep_overlay_buf.clone(),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "▏",
            Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
        ),
    ]);
    let hint = Line::from(vec![
        Span::styled("  ", t.muted()),
        Span::styled("Enter", Style::default().fg(t.accent)),
        Span::styled(" submit  ", t.muted()),
        Span::styled("Tab", Style::default().fg(t.accent)),
        Span::styled(" toggle add/remove  ", t.muted()),
        Span::styled("Esc", Style::default().fg(t.accent)),
        Span::styled(" cancel", t.muted()),
    ]);
    f.render_widget(
        Paragraph::new(vec![context, existing_line, line, hint]),
        inner,
    );
}
