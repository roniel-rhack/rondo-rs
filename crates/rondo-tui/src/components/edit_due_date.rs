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
use rondo_core::i18n;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let task = app.data.selected_task();
    let current = task
        .and_then(|x| x.due_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| i18n::t("edit_due_date.none"));
    let title = task.map(|x| x.title.clone()).unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(
            i18n::t("edit_due_date.title"),
            t.accent_style(),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(i18n::t("edit_due_date.task_label"), t.muted()),
            Span::styled(title, Style::default().fg(t.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(i18n::t("edit_due_date.current_label"), t.muted()),
            Span::styled(current, Style::default().fg(t.fg)),
        ]),
        Line::from(""),
    ];
    if app.modals.edit_due_date_custom_mode {
        lines.push(Line::from(vec![
            Span::styled(i18n::t("edit_due_date.custom_prompt"), t.muted()),
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
            Span::styled(i18n::t("edit_due_date.hint_set"), t.muted()),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(i18n::t("edit_due_date.hint_cancel"), t.muted()),
        ]));
    } else {
        lines.push(preset_line(t, "t", &i18n::t("edit_due_date.preset_today")));
        lines.push(preset_line(
            t,
            "m",
            &i18n::t("edit_due_date.preset_tomorrow"),
        ));
        lines.push(preset_line(t, "w", &i18n::t("edit_due_date.preset_7days")));
        lines.push(preset_line(t, "M", &i18n::t("edit_due_date.preset_30days")));
        lines.push(preset_line(t, "x", &i18n::t("edit_due_date.preset_clear")));
        lines.push(preset_line(t, "c", &i18n::t("edit_due_date.preset_custom")));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", t.muted()),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(i18n::t("edit_due_date.hint_cancel"), t.muted()),
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
