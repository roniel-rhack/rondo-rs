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
    app.ui.last_journal_entries_rect = chunks[1];

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
    let days_active = matches!(app.ui.journal_pane, crate::app::ui_state::JournalPane::Days);
    let entries_active = !days_active;
    let list = List::new(items).block(t.panel("Days", days_active));
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
            .block(t.panel("Journal", entries_active)),
        chunks[1],
    );
}

pub fn draw_editor_overlay(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    // Take most of the available height so multi-line entries stay visible.
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
    f.render_widget(block.clone(), editor_rect);

    // Inner split: textarea + hint footer.
    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(3),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(inner);

    // Configure textarea visuals.
    let textarea = &mut app.modals.journal_textarea;
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(
        Style::default()
            .fg(t.bg)
            .bg(t.accent)
            .add_modifier(Modifier::BOLD),
    );
    textarea.set_line_number_style(Style::default().fg(t.fg_muted));
    f.render_widget(&*textarea, layout[0]);

    let hint = Line::from(vec![
        Span::styled(" ", t.muted()),
        Span::styled("Ctrl+S", t.kbd()),
        Span::styled(" save · ", t.muted()),
        Span::styled("Esc", t.kbd()),
        Span::styled(" cancel · ", t.muted()),
        Span::styled("←↑↓→", t.kbd()),
        Span::styled(" mover · ", t.muted()),
        Span::styled("# **md** _it_", t.muted()),
        Span::styled(" soportado", t.muted()),
    ]);
    f.render_widget(Paragraph::new(hint), layout[1]);
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
