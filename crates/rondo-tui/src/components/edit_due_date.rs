//! EditDueDate modal: preset list (today / tomorrow / +7d / +30d /
//! clear / custom) plus a free-form `YYYY-MM-DD` input mode the user
//! enters via `c`.

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
    let current = task
        .and_then(|x| x.due_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "(none)".into());
    let title = task.map(|x| x.title.clone()).unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" ⌚ set due date ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(" task: ", t.muted()),
            Span::styled(title, Style::default().fg(t.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(" current: ", t.muted()),
            Span::styled(current, Style::default().fg(t.fg)),
        ]),
        Line::from(""),
    ];
    if app.modals.edit_due_date_custom_mode {
        lines.push(Line::from(vec![
            Span::styled(" custom YYYY-MM-DD: ", t.muted()),
            Span::styled(
                app.modals.edit_due_date_buf.clone(),
                Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "▏",
                Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", t.muted()),
            Span::styled("Enter", Style::default().fg(t.accent)),
            Span::styled(" set  ", t.muted()),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(" cancel", t.muted()),
        ]));
    } else {
        lines.push(preset_line(t, "t", "today"));
        lines.push(preset_line(t, "m", "tomorrow"));
        lines.push(preset_line(t, "w", "+7 days"));
        lines.push(preset_line(t, "M", "+30 days"));
        lines.push(preset_line(t, "x", "clear"));
        lines.push(preset_line(t, "c", "custom YYYY-MM-DD…"));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", t.muted()),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(" cancel", t.muted()),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn preset_line(t: &crate::theme::Theme, key: &str, label: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ", t.muted()),
        Span::styled(
            key.to_string(),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", t.muted()),
        Span::styled(label.to_string(), Style::default().fg(t.fg)),
    ])
}
