use crate::app::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Thin filter / count strip that lives between the header panel and the body.
/// Left: active filter chips (`◖#work◗  ◖due≤7d◗`) — empty for now (Fase 5).
/// Right: counts (`4 active · 1 done`).
pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(28)])
        .split(area);

    let mut chips: Vec<Span<'static>> = vec![Span::raw(" ")];
    if !app.search_buf.is_empty() {
        chips.push(Span::styled(" filter: ", Style::default().fg(t.fg_muted)));
        chips.push(chip(&format!("/{}", app.search_buf), t.accent, t));
    }
    f.render_widget(Paragraph::new(Line::from(chips)), chunks[0]);

    let active = app.tasks.iter().filter(|x| {
        matches!(
            x.status,
            rondo_core::domain::task::Status::Pending
                | rondo_core::domain::task::Status::InProgress
        )
    }).count();
    let done = app.tasks.len() - active;
    let counts = Line::from(vec![
        Span::styled(
            format!("{} active", active),
            Style::default()
                .fg(t.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(t.border_inactive)),
        Span::styled(format!("{} done ", done), Style::default().fg(t.fg_muted)),
    ]);
    f.render_widget(
        Paragraph::new(counts).alignment(Alignment::Right),
        chunks[1],
    );
}

fn chip(label: &str, color: ratatui::style::Color, _t: &crate::theme::Theme) -> Span<'static> {
    Span::styled(
        format!("◖{}◗", label),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}
