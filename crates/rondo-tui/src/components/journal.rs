use crate::{app::AppState, widgets::markdown};
use chrono::{Local, NaiveDate};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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
        .enumerate()
        .map(|(i, n)| {
            let is_cursor = i == app.data.selected_journal;
            let label = smart_date_label(n.date);
            let cursor_mark = if is_cursor { "▌" } else { " " };
            let label_style = if is_cursor {
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(t.fg)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    cursor_mark,
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {:<10}", label), label_style),
                Span::styled(
                    n.date.format(" %Y-%m-%d").to_string(),
                    Style::default().fg(t.fg_muted),
                ),
            ]))
        })
        .collect();
    let list = List::new(items).block(t.panel("Days", true));
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
        let body_width = chunks[1].width.saturating_sub(4) as usize;
        let cursor_idx = app
            .data
            .selected_journal_entry
            .min(app.data.journal_entries.len().saturating_sub(1));
        for (i, entry) in app.data.journal_entries.iter().enumerate() {
            if i > 0 {
                content_lines.push(Line::raw(""));
                content_lines.push(Line::from(Span::styled(
                    "╌".repeat(body_width.min(60)),
                    Style::default().fg(t.border_inactive),
                )));
                content_lines.push(Line::raw(""));
            }
            let is_focused = i == cursor_idx;
            let gutter = if is_focused { "▌ ◷ " } else { "  ◷ " };
            let time_style = if is_focused {
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
            };
            content_lines.push(Line::from(vec![
                Span::styled(gutter, Style::default().fg(t.accent)),
                Span::styled(
                    entry
                        .created_at
                        .with_timezone(&Local)
                        .format("%H:%M")
                        .to_string(),
                    time_style,
                ),
                Span::styled(
                    format!("  · #{}", entry.id),
                    Style::default().fg(t.fg_muted),
                ),
            ]));
            content_lines.push(Line::raw(""));
            for l in markdown::render(&entry.body, t).lines {
                let mut spans = vec![Span::raw("  ")];
                spans.extend(l.spans);
                content_lines.push(Line::from(spans));
            }
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

pub fn draw_editor_overlay(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    // Take most of the available height so multi-line entries stay visible.
    // Leave 2 rows of margin and 1 hint row inside.
    let h = area.height.saturating_sub(4).max(8);
    let w = area.width.saturating_sub(4);
    let editor_rect = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, editor_rect);
    let title = if app.modals.journal_editor_entry_id.is_some() {
        " ✎ edit entry "
    } else {
        " + journal entry "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(title, t.accent_style()));
    let inner = block.inner(editor_rect);
    f.render_widget(block, editor_rect);

    let buf = &app.modals.journal_editor_buf;
    let mut lines: Vec<Line> = Vec::new();
    for (i, segment) in buf.split('\n').enumerate() {
        let prefix = if i == 0 { " > " } else { "   " };
        lines.push(Line::from(vec![
            Span::styled(prefix, t.accent_style()),
            Span::styled(segment.to_string(), Style::default().fg(t.fg)),
        ]));
    }
    if let Some(last) = lines.last_mut() {
        last.spans.push(Span::styled(
            "▏",
            Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
        ));
    }
    // Auto-scroll: reserve 1 row for the hint at bottom, take the last N
    // lines so the cursor row stays visible no matter how much was typed.
    let visible_rows = inner.height.saturating_sub(1) as usize;
    let body_lines = if lines.len() > visible_rows {
        lines.split_off(lines.len() - visible_rows)
    } else {
        lines
    };
    let mut all = body_lines;
    all.push(Line::raw(""));
    all.push(Line::from(vec![
        Span::styled("  ", t.muted()),
        Span::styled("Ctrl+S", t.kbd()),
        Span::styled(" save · ", t.muted()),
        Span::styled("Esc", t.kbd()),
        Span::styled(" cancel · ", t.muted()),
        Span::styled("Enter", t.kbd()),
        Span::styled(" newline · ", t.muted()),
        Span::styled("# **md**", t.muted()),
        Span::styled(" supported ", t.muted()),
    ]));
    f.render_widget(Paragraph::new(all).wrap(Wrap { trim: false }), inner);
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
