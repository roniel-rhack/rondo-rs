use crate::app::AppState;
use crate::focus::{DetailSection, Pane};
use crate::widgets::{
    bracket_panel::BracketPanel, due_badge, markdown, priority_badge, ring, sparkline,
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let active = !app.focus_left();

    let task = match app.tasks.get(app.selected_task) {
        Some(x) => x,
        None => {
            let panel = BracketPanel::new("detail", t).active(active);
            let inner = panel.inner(area);
            panel.render(area, f.buffer_mut());
            let lines = vec![
                Line::raw(""),
                Line::from(Span::styled("  No task selected", t.muted())),
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("j/k", t.kbd()),
                    Span::raw(" "),
                    Span::styled("navigate", t.muted()),
                    Span::raw("    "),
                    Span::styled("/", t.kbd()),
                    Span::raw(" "),
                    Span::styled("search", t.muted()),
                    Span::raw("    "),
                    Span::styled("?", t.kbd()),
                    Span::raw(" "),
                    Span::styled("help", t.muted()),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), inner);
            return;
        }
    };

    let title = format!("detail · #{}", task.id);
    let badge = format!(
        "{} · {}",
        task.status.label().to_lowercase(),
        task.priority.label().to_lowercase()
    );
    let panel = BracketPanel::new(&title, t).active(active).badge(&badge);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        task.title.clone(),
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        Span::styled("Status   ", Style::default().fg(t.fg_muted)),
        Span::styled(
            format!("{}  ", task.status.icon()),
            Style::default().fg(match task.status {
                Status::Done => t.success,
                Status::InProgress => t.accent,
                Status::Pending => t.fg_muted,
            }),
        ),
        Span::styled(task.status.label(), Style::default().fg(t.fg)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Priority ", Style::default().fg(t.fg_muted)),
        priority_badge::span(task.priority, t),
    ]));

    if let Some(b) = due_badge::span(task.due_date, t) {
        lines.push(Line::from(vec![
            Span::styled("Due      ", Style::default().fg(t.fg_muted)),
            b,
            Span::styled(
                format!("  {}", task.due_date.unwrap().format("%Y-%m-%d")),
                Style::default().fg(t.fg_muted),
            ),
        ]));
    }

    if !task.tags.is_empty() {
        let mut row: Vec<Span> = vec![Span::styled("Tags     ", Style::default().fg(t.fg_muted))];
        for (i, tag) in task.tags.iter().enumerate() {
            if i > 0 {
                row.push(Span::styled("  ·  ", Style::default().fg(t.fg_muted)));
            }
            row.push(Span::styled(
                format!("#{}", tag),
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(row));
    }
    lines.push(Line::raw(""));

    if let Some(desc) = &task.description {
        if !desc.is_empty() {
            lines.push(Line::from(Span::styled(
                "Description",
                t.accent_style(),
            )));
            for l in markdown::render(desc, t).lines {
                lines.push(l);
            }
            lines.push(Line::raw(""));
        }
    }

    let (done, total) = task.subtask_progress();
    if total > 0 {
        let section_active = app.focus.pane == Pane::Detail
            && app.focus.section == DetailSection::Subtasks;
        let mut header = section_header(
            "Subtasks",
            &format!("{}/{}", done, total),
            section_active,
            t,
        );
        // append ring glyph
        header.spans.insert(1, Span::raw(" "));
        header.spans.insert(2, ring::glyph(done, total, t));
        lines.push(header);
        for (i, st) in task.subtasks.iter().enumerate() {
            let cursor_here = section_active && app.focus.section_item == i;
            let gutter = if cursor_here {
                Span::styled("▌ ", Style::default().fg(t.accent))
            } else {
                Span::raw("  ")
            };
            let (icon, st_style) = if st.completed {
                (
                    "✓",
                    Style::default()
                        .fg(t.fg_muted)
                        .add_modifier(Modifier::CROSSED_OUT),
                )
            } else {
                ("○", Style::default().fg(t.fg))
            };
            lines.push(Line::from(vec![
                gutter,
                Span::styled(
                    icon,
                    Style::default().fg(if st.completed {
                        t.success
                    } else {
                        t.fg_muted
                    }),
                ),
                Span::raw("  "),
                Span::styled(st.title.clone(), st_style),
            ]));
        }
        lines.push(Line::raw(""));
    }

    if task.is_blocked() {
        let ids = task
            .blocked_by_ids
            .iter()
            .map(|i| format!("#{}", i))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(vec![
            Span::styled("Blocked by ", Style::default().fg(t.fg_muted)),
            Span::styled(
                ids,
                Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));
    }

    if !task.time_logs.is_empty() {
        let total_secs: i64 = task.time_logs.iter().map(|tl| tl.duration_secs).sum();
        let spark_values: Vec<u64> = task
            .time_logs
            .iter()
            .rev()
            .take(7)
            .map(|tl| tl.duration_secs.max(0) as u64)
            .collect();
        lines.push(section_header("Time", &format_duration(total_secs), false, t));
        lines.push(Line::from(vec![
            Span::raw("  "),
            sparkline::span(&spark_values, t),
            Span::raw("  "),
            Span::styled(
                format!("{} sessions", task.time_logs.len()),
                t.muted(),
            ),
        ]));
        lines.push(Line::raw(""));
    }

    if !task.notes.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("Notes", t.accent_style())));
        for n in task.notes.iter().take(3) {
            lines.push(Line::from(Span::styled(
                format!("  · {}", n.body.lines().next().unwrap_or("")),
                Style::default().fg(t.fg_muted),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn section_header(name: &str, count: &str, active: bool, t: &crate::theme::Theme) -> Line<'static> {
    let underline = if active {
        Modifier::BOLD | Modifier::UNDERLINED
    } else {
        Modifier::BOLD
    };
    Line::from(vec![
        Span::styled(
            format!("── {} ", name),
            Style::default()
                .fg(if active { t.accent } else { t.fg_muted })
                .add_modifier(underline),
        ),
        Span::styled(
            count.to_string(),
            Style::default().fg(t.fg_muted),
        ),
        Span::styled(" ──", Style::default().fg(t.border_inactive)),
    ])
}

fn format_duration(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}
