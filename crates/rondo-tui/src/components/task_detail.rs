use crate::app::AppState;
use crate::widgets::{due_badge, markdown, priority_badge, progress_bar};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let border = if !app.focus_left {
        t.border_active
    } else {
        t.border_inactive
    };

    let task = match app.tasks.get(app.selected_task) {
        Some(x) => x,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(" Detail ");
            f.render_widget(
                Paragraph::new(Span::styled(
                    "No task selected",
                    Style::default().fg(t.fg_muted),
                ))
                .block(block),
                area,
            );
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            format!(" Detail · #{} ", task.id),
            t.accent_style(),
        ));

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
        lines.push(Line::from(Span::styled(
            format!("Subtasks ({}/{})", done, total),
            t.accent_style(),
        )));
        lines.push(progress_bar::line(done, total, 30, t));
        for st in &task.subtasks {
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
                Span::raw("  "),
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
        lines.push(Line::from(vec![
            Span::styled("Time logged ", Style::default().fg(t.fg_muted)),
            Span::styled(
                format_duration(total_secs),
                Style::default()
                    .fg(t.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
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

    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(block),
        area,
    );
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
