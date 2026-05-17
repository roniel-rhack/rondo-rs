//! EditRecurrence modal: preset list to set recur_freq + default
//! interval=1 on the selected task. `x` clears.

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
    let title = task.map(|x| x.title.clone()).unwrap_or_default();
    let current = task
        .map(|x| match x.recur_freq {
            rondo_core::domain::task::RecurFreq::None => {
                format!("({})", i18n::t("edit_recurrence.none"))
            }
            other => format!("every {} {:?}", x.recur_interval.max(1), other),
        })
        .unwrap_or_default();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(
            i18n::t("edit_recurrence.title"),
            t.accent_style(),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(i18n::t("edit_recurrence.task_label"), t.muted()),
            Span::styled(title, Style::default().fg(t.fg_muted)),
        ]),
        Line::from(vec![
            Span::styled(i18n::t("edit_recurrence.current_label"), t.muted()),
            Span::styled(current, Style::default().fg(t.fg)),
        ]),
        Line::from(""),
        preset_line(t, "d", &i18n::t("edit_recurrence.preset_daily")),
        preset_line(t, "w", &i18n::t("edit_recurrence.preset_weekly")),
        preset_line(t, "m", &i18n::t("edit_recurrence.preset_monthly")),
        preset_line(t, "y", &i18n::t("edit_recurrence.preset_yearly")),
        preset_line(t, "x", &i18n::t("edit_recurrence.preset_clear")),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", t.muted()),
            Span::styled("Esc", Style::default().fg(t.accent)),
            Span::styled(i18n::t("edit_recurrence.hint_cancel"), t.muted()),
        ]),
    ];
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
