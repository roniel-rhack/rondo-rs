use crate::{app::AppState, widgets::markdown};
use chrono::{Local, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    if app.data.journal_notes.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::raw(""),
            Line::from(Span::styled("  No journal entries yet", t.muted())),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("?", t.kbd()),
                Span::raw(" "),
                Span::styled("for help", t.muted()),
            ]),
        ];
        f.render_widget(Paragraph::new(lines).block(t.panel("Journal", true)), area);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(area);

    let items: Vec<ListItem> = app
        .data
        .journal_notes
        .iter()
        .map(|n| {
            let label = smart_date_label(n.date);
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<10} ", label), Style::default().fg(t.accent)),
                Span::styled(
                    n.date.format("%Y-%m-%d").to_string(),
                    Style::default().fg(t.fg_muted),
                ),
            ]))
        })
        .collect();
    let list = List::new(items)
        .block(t.panel("Days", false))
        .highlight_style(t.selection());
    f.render_stateful_widget(list, chunks[0], &mut app.data.journal_list_state);

    let mut content_lines: Vec<Line> = Vec::new();
    if let Some(note) = app.data.journal_notes.get(app.data.selected_journal) {
        content_lines.push(Line::from(Span::styled(
            note.date
                .format("%A, %B %-d, %Y")
                .to_string()
                .to_uppercase(),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )));
        content_lines.push(Line::from(Span::styled(
            "━".repeat(40),
            Style::default().fg(t.border_inactive),
        )));
        content_lines.push(Line::raw(""));
        if app.data.journal_entries.is_empty() {
            content_lines.push(Line::from(Span::styled(
                "  (no entries this day)",
                Style::default().fg(t.fg_muted),
            )));
        }
        for entry in &app.data.journal_entries {
            content_lines.push(Line::from(Span::styled(
                entry
                    .created_at
                    .with_timezone(&Local)
                    .format("  %H:%M  ")
                    .to_string(),
                Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD),
            )));
            for l in markdown::render(&entry.body, t).lines {
                content_lines.push(l);
            }
            content_lines.push(Line::raw(""));
        }
    } else {
        content_lines.push(Line::from(Span::styled(
            "No journal entries",
            Style::default().fg(t.fg_muted),
        )));
    }
    f.render_widget(
        Paragraph::new(content_lines)
            .wrap(Wrap { trim: false })
            .block(t.panel("Journal", true)),
        chunks[1],
    );
}

fn smart_date_label(date: NaiveDate) -> String {
    let today = Local::now().date_naive();
    let delta = (today - date).num_days();
    match delta {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        d if (2..7).contains(&d) => format!("{}", date.format("%A")),
        _ => date.format("%b %-d").to_string(),
    }
}
