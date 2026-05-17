use crate::app::{AppState, FlashTarget};
use crate::focus::{DetailSection, Pane};
use crate::theme::Theme;
use crate::widgets::{
    bracket_panel::BracketPanel, due_badge, empty, markdown, priority_badge, sparkline,
};
use chrono::Local;
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

    let task = match app.data.tasks.get(app.data.selected_task) {
        Some(x) => x,
        None => {
            return draw_empty(t, active, f, area);
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

    let inner_width = inner.width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();

    // ─── HEADER ─────────────────────────────────────────────
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("TSK-{:04}", task.id),
            Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    ", Style::default()),
        priority_badge::span(task.priority, t),
        Span::raw("   "),
        status_pill(task.status, t),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            task.title.clone(),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(rule(inner_width, t));
    lines.push(Line::raw(""));

    // ─── METADATA ───────────────────────────────────────────
    push_meta(&mut lines, "creada", created_line(task, t), t);
    if let Some(due) = task.due_date {
        let mut spans: Vec<Span<'static>> = Vec::new();
        if let Some(b) = due_badge::span(Some(due), app.clock.today(), t) {
            spans.push(b);
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            due.format("%Y-%m-%d (%A)").to_string(),
            Style::default().fg(t.fg_muted),
        ));
        push_meta(&mut lines, "vence", Line::from(spans), t);
    }
    if matches!(
        task.recur_freq,
        rondo_core::domain::task::RecurFreq::Daily
            | rondo_core::domain::task::RecurFreq::Weekly
            | rondo_core::domain::task::RecurFreq::Monthly
            | rondo_core::domain::task::RecurFreq::Yearly
    ) {
        push_meta(
            &mut lines,
            "recurre",
            Line::from(Span::styled(
                format!("{:?} · cada {}", task.recur_freq, task.recur_interval),
                Style::default().fg(t.fg),
            )),
            t,
        );
    }
    if !task.tags.is_empty() {
        let mut tag_spans: Vec<Span<'static>> = Vec::new();
        for (i, tag) in task.tags.iter().enumerate() {
            if i > 0 {
                tag_spans.push(Span::styled("  ·  ", Style::default().fg(t.fg_muted)));
            }
            tag_spans.push(Span::styled(
                format!("#{}", tag),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ));
        }
        push_meta(&mut lines, "tags", Line::from(tag_spans), t);
    }
    if !task.metadata.is_empty() {
        for (k, v) in &task.metadata {
            push_meta(
                &mut lines,
                k.as_str(),
                Line::from(Span::styled(v.clone(), Style::default().fg(t.fg))),
                t,
            );
        }
    }

    // ─── DESCRIPCIÓN ────────────────────────────────────────
    let header_active =
        app.ui.focus.pane == Pane::Detail && app.ui.focus.section == DetailSection::Header;
    let desc_body = task.description.as_deref().unwrap_or("");
    let has_desc = !desc_body.trim().is_empty();
    lines.push(Line::raw(""));
    section_header(
        &mut lines,
        "descripción",
        if has_desc { None } else { Some("vacía") },
        header_active,
        inner_width,
        t,
    );
    lines.push(Line::raw(""));
    if has_desc {
        for l in markdown::render(desc_body, t).lines {
            let mut spans = vec![Span::raw("    ")];
            spans.extend(l.spans);
            lines.push(Line::from(spans));
        }
    } else {
        lines.push(empty::line("description", "E", "to add", t));
    }

    // ─── SUBTASKS ───────────────────────────────────────────
    let (done, total) = task.subtask_progress();
    let section_active =
        app.ui.focus.pane == Pane::Detail && app.ui.focus.section == DetailSection::Subtasks;
    lines.push(Line::raw(""));
    section_header(
        &mut lines,
        "subtareas",
        if total > 0 {
            Some(format!("{}/{}", done, total))
        } else {
            Some("vacía".to_string())
        }
        .as_deref(),
        section_active,
        inner_width,
        t,
    );
    if total == 0 {
        lines.push(empty::line("subtasks", "A", "to add", t));
    }
    if total > 0 {
        // Mini progress bar
        let bar_width = inner_width.saturating_sub(8);
        let filled = ((bar_width as f64) * (done as f64 / total as f64)).round() as usize;
        let empty = bar_width.saturating_sub(filled);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "▰".repeat(filled),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("▱".repeat(empty), Style::default().fg(t.border_inactive)),
            Span::styled(
                format!("  {:>3.0}%", (done as f64 / total as f64) * 100.0),
                Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));
        for (i, st) in task.subtasks.iter().enumerate() {
            let cursor_here = section_active && app.ui.focus.section_item == i;
            let flashing = app.is_flashing(FlashTarget::Subtask(st.id));
            let gutter = if flashing {
                Span::styled(
                    "◉ ",
                    Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
                )
            } else if cursor_here {
                Span::styled("▌ ", Style::default().fg(t.accent))
            } else {
                Span::raw("  ")
            };
            let checkbox = if st.completed {
                Span::styled("[x]", Style::default().fg(t.success))
            } else {
                Span::styled("[ ]", Style::default().fg(t.fg_muted))
            };
            let title_style = if st.completed {
                Style::default()
                    .fg(t.fg_muted)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if cursor_here {
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(t.fg)
            };
            let mut effective_title_style = title_style;
            if flashing {
                effective_title_style = effective_title_style.add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(vec![
                gutter,
                checkbox,
                Span::raw("  "),
                Span::styled(st.title.clone(), effective_title_style),
            ]));
        }
    }

    // ─── DEPENDENCIES ───────────────────────────────────────
    let deps_section_active =
        app.ui.focus.pane == Pane::Detail && app.ui.focus.section == DetailSection::Dependencies;
    let has_deps = task.is_blocked() || !task.blocks_ids.is_empty();
    lines.push(Line::raw(""));
    section_header(
        &mut lines,
        "dependencias",
        if has_deps { None } else { Some("vacía") },
        deps_section_active,
        inner_width,
        t,
    );
    if task.is_blocked() {
        let blockers: Vec<Span<'static>> = task
            .blocked_by_ids
            .iter()
            .flat_map(|id| {
                [
                    Span::styled("  ⛒ ", Style::default().fg(t.danger)),
                    Span::styled(
                        format!("blocked by #{}", id),
                        Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
                    ),
                ]
            })
            .collect();
        lines.push(Line::from(blockers));
    }
    if !task.blocks_ids.is_empty() {
        let blocks: Vec<Span<'static>> = task
            .blocks_ids
            .iter()
            .flat_map(|id| {
                [
                    Span::styled("  ⏵ ", Style::default().fg(t.warn)),
                    Span::styled(
                        format!("blocks #{}", id),
                        Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
                    ),
                ]
            })
            .collect();
        lines.push(Line::from(blocks));
    }
    if !has_deps {
        lines.push(empty::line("dependencies", "B", "to add/remove", t));
    }

    // ─── TIME LOG ───────────────────────────────────────────
    lines.push(Line::raw(""));
    if task.time_logs.is_empty() {
        section_header(&mut lines, "tiempo", Some("vacía"), false, inner_width, t);
        lines.push(empty::line("time-log", "p", "to start a pomodoro", t));
    } else {
        let total_secs: i64 = task.time_logs.iter().map(|tl| tl.duration_secs).sum();
        section_header(
            &mut lines,
            "tiempo",
            Some(&format_duration(total_secs)),
            false,
            inner_width,
            t,
        );
        let spark_values: Vec<u64> = task
            .time_logs
            .iter()
            .rev()
            .take(7)
            .map(|tl| tl.duration_secs.max(0) as u64)
            .collect();
        lines.push(Line::from(vec![
            Span::raw("  "),
            sparkline::span(&spark_values, t),
            Span::raw("   "),
            Span::styled(
                format!("{} sessions", task.time_logs.len()),
                Style::default().fg(t.fg_muted),
            ),
            Span::styled("  ·  ", Style::default().fg(t.border_inactive)),
            Span::styled(
                format!(
                    "media {}",
                    format_duration(total_secs / task.time_logs.len() as i64)
                ),
                Style::default().fg(t.fg_muted),
            ),
        ]));
    }

    // ─── NOTAS ─────────────────────────────────────────────
    let notes_section_active =
        app.ui.focus.pane == Pane::Detail && app.ui.focus.section == DetailSection::Notes;
    lines.push(Line::raw(""));
    let notes_count_lbl = if task.notes.is_empty() {
        "vacía".to_string()
    } else {
        format!("{}", task.notes.len())
    };
    section_header(
        &mut lines,
        "notas",
        Some(notes_count_lbl.as_str()),
        notes_section_active,
        inner_width,
        t,
    );
    if task.notes.is_empty() {
        lines.push(empty::line("notes", "a", "to add", t));
    }
    if !task.notes.is_empty() {
        let section_active = notes_section_active;
        for (i, n) in task.notes.iter().take(3).enumerate() {
            let cursor_here = section_active && app.ui.focus.section_item == i;
            let gutter = if cursor_here {
                Span::styled(
                    "▌",
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(" ")
            };
            let when = n
                .created_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string();
            lines.push(Line::from(vec![
                gutter,
                Span::styled(" · ", Style::default().fg(t.accent)),
                Span::styled(when, Style::default().fg(t.fg_muted)),
            ]));
            let first_line = n.body.lines().next().unwrap_or("");
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(first_line.to_string(), Style::default().fg(t.fg)),
            ]));
        }
    }

    let lines = if app.modals.search_open && !app.modals.search_buf.trim().is_empty() {
        let q = app.modals.search_buf.trim().to_string();
        lines
            .into_iter()
            .map(|l| crate::search::highlight_line(l, &q, t))
            .collect::<Vec<_>>()
    } else {
        lines
    };
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_empty(t: &Theme, active: bool, f: &mut Frame<'_>, area: Rect) {
    let panel = BracketPanel::new("detail", t).active(active);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());
    let lines = vec![
        Line::raw(""),
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
}

/// `  LABEL    value...`  (label fixed-width, fg_muted UPPERCASE)
fn push_meta(lines: &mut Vec<Line<'static>>, label: &str, value: Line<'static>, t: &Theme) {
    let mut spans: Vec<Span<'static>> = vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", label.to_uppercase()),
            Style::default().fg(t.fg_muted),
        ),
    ];
    spans.extend(value.spans);
    lines.push(Line::from(spans));
}

/// Section header: '  LABEL  count  ─────────────────'  (UPPERCASE accent)
fn section_header(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    count: Option<&str>,
    active: bool,
    width: usize,
    t: &Theme,
) {
    let bar_style = if active {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.border_inactive)
    };
    let label_style = if active {
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    };
    let mut spans = vec![
        Span::styled("▌ ", bar_style),
        Span::styled(format!("§ {}", label.to_uppercase()), label_style),
    ];
    let mut used = 2 + 2 + label.chars().count();
    if let Some(c) = count {
        spans.push(Span::styled("  ", Style::default()));
        spans.push(Span::styled(
            format!("({})", c),
            Style::default().fg(t.fg_muted),
        ));
        used += 2 + c.chars().count() + 2;
    }
    spans.push(Span::raw(" "));
    used += 1;
    let dash_count = width.saturating_sub(used + 2);
    spans.push(Span::styled(
        "─".repeat(dash_count),
        Style::default().fg(t.border_inactive),
    ));
    lines.push(Line::from(spans));
}

fn rule(width: usize, t: &Theme) -> Line<'static> {
    let w = width.saturating_sub(4);
    Line::from(vec![
        Span::raw("  "),
        Span::styled("━".repeat(w), Style::default().fg(t.border_inactive)),
    ])
}

fn status_pill(s: Status, t: &Theme) -> Span<'static> {
    let (label, color) = match s {
        Status::Done => ("done", t.success),
        Status::InProgress => ("in-progress", t.accent),
        Status::Pending => ("pending", t.fg_muted),
    };
    Span::styled(
        format!("◖{}◗", label),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn created_line(task: &rondo_core::domain::task::Task, t: &Theme) -> Line<'static> {
    let when = task
        .created_at
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string();
    Line::from(Span::styled(when, Style::default().fg(t.fg_muted)))
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
