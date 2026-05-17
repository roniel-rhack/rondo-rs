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
    let title = app
        .data
        .selected_task()
        .map(|x| x.title.clone())
        .unwrap_or_default();
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.danger))
        .title(Span::styled(
            i18n::t("confirm_delete.title"),
            Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
        ));
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw(format!("  {} ", i18n::t("confirm_delete.prompt"))),
            Span::styled(
                format!("\"{}\"", title),
                Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
            ),
            Span::raw("?"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " y ",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(i18n::t("confirm_delete.yes")),
            Span::styled(
                " n/Esc ",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(i18n::t("confirm_delete.no")),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).block(block), area);
}
