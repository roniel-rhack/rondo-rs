use crate::app::{AppState, FlashTarget};
use crate::theme::Theme;
use crate::widgets::{bracket_panel::BracketPanel, due_badge, priority_badge, priority_spine, ring};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Widget},
    Frame,
};
use rondo_core::domain::task::{Status, Task};

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let visible: Vec<usize> = app.visible_task_indices();
    let filter_label = app.active_filter.label().to_lowercase();
    let title = format!("{} · {} tareas", filter_label, visible.len());
    let panel = BracketPanel::new(&title, t)
        .active(app.focus.pane == crate::focus::Pane::List);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    if visible.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  Sin tareas para '{}'", filter_label),
                t.muted(),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("h", t.kbd()),
                Span::raw(" "),
                Span::styled("cambiar filtro", t.muted()),
                Span::raw("    "),
                Span::styled("?", t.kbd()),
                Span::raw(" "),
                Span::styled("ayuda", t.muted()),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    // Split inner: column header + list + progress bar
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // column header
            Constraint::Min(1),    // body
            Constraint::Length(2), // progress bar
        ])
        .split(inner);

    draw_column_header(f, layout[0], t);

    let items = render_items(app, &visible);
    let highlight = app.theme.selection();
    let list = List::new(items).highlight_style(highlight);
    f.render_stateful_widget(list, layout[1], &mut app.task_list_state);

    draw_progress_bar(app, f, layout[2], t);
}

fn draw_column_header(f: &mut Frame<'_>, area: Rect, t: &Theme) {
    // Columns:  [ ] gutter | [pri] 8 | tarea flex | [tags] 18 | [vence] 10
    let header = Line::from(vec![
        Span::raw("    "),
        Span::styled("ESTADO  ", t.muted()),
        Span::styled("PRI     ", t.muted()),
        Span::styled("TAREA", t.muted()),
    ]);
    f.render_widget(Paragraph::new(header), area);
}

fn render_items(app: &AppState, visible: &[usize]) -> Vec<ListItem<'static>> {
    let selected = app.task_list_state.selected();
    visible
        .iter()
        .map(|&idx| build_row(&app.tasks[idx], idx, selected, app, &app.theme))
        .collect()
}

fn build_row(
    task: &Task,
    idx: usize,
    selected: Option<usize>,
    app: &AppState,
    t: &Theme,
) -> ListItem<'static> {
    let is_selected = Some(idx) == selected;
    let in_visual = app.selection.contains(&task.id);
    let flashing = app.is_flashing(FlashTarget::Task(task.id));
    let gutter = || -> Span<'static> {
        if flashing {
            Span::styled(
                "◉ ",
                Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
            )
        } else if in_visual {
            Span::styled(
                "● ",
                Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
            )
        } else if is_selected {
            Span::styled("▌ ", Style::default().fg(t.accent))
        } else {
            Span::raw("  ")
        }
    };
    let spine = priority_spine::glyph(task.priority, t);
    let checkbox = match task.status {
        Status::Done => Span::styled("[x]", Style::default().fg(t.success)),
        Status::InProgress => Span::styled("[•]", Style::default().fg(t.accent)),
        Status::Pending => Span::styled("[ ]", Style::default().fg(t.fg_muted)),
    };
    let title_style = if task.status == Status::Done {
        Style::default()
            .fg(t.fg_muted)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
    };

    // ROW 1 — primary identity: gutter · spine · checkbox · title · badges
    let mut primary: Vec<Span<'static>> = vec![
        gutter(),
        spine.clone(),
        Span::raw(" "),
        checkbox,
        Span::raw("   "),
        priority_badge::span(task.priority, t),
        Span::raw("  "),
        Span::styled(truncate(&task.title, 50), title_style),
    ];
    if let Some(b) = due_badge::span(task.due_date, t) {
        primary.push(Span::raw("   "));
        primary.push(b);
    }
    if task.is_blocked() {
        primary.push(Span::raw("   "));
        primary.push(Span::styled(
            "⛒ blocked",
            Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
        ));
    }
    let (done, total) = task.subtask_progress();
    if total > 0 {
        primary.push(Span::raw("   "));
        primary.push(ring::glyph(done, total, t));
        primary.push(Span::styled(format!(" {}/{}", done, total), t.muted()));
    }

    // ROW 2..N — indented subtask previews + tag chips
    let mut lines = vec![Line::from(primary)];

    // Indent depth matches gutter(2) + spine(1) + space(1) + checkbox(3) + 3 = 10 cols
    let indent = "          ";

    let incomplete: Vec<_> = task.subtasks.iter().filter(|s| !s.completed).take(2).collect();
    for st in &incomplete {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled("↳ ", Style::default().fg(t.fg_muted)),
            Span::styled(truncate(&st.title, 60), t.muted()),
        ]));
    }

    if !task.tags.is_empty() {
        let mut tag_spans: Vec<Span<'static>> = vec![Span::raw(indent)];
        for (i, tag) in task.tags.iter().enumerate() {
            if i > 0 {
                tag_spans.push(Span::raw("  "));
            }
            tag_spans.push(Span::styled(
                format!("#{}", tag),
                Style::default().fg(t.accent),
            ));
        }
        lines.push(Line::from(tag_spans));
    }

    // ROW final — blank separator for breathing room (NEXUS-style).
    lines.push(Line::raw(""));

    ListItem::new(lines)
}

fn draw_progress_bar(app: &AppState, f: &mut Frame<'_>, area: Rect, t: &Theme) {
    let total = app.tasks.len();
    let done = app.tasks.iter().filter(|x| x.status == Status::Done).count();
    let ratio = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    let bar_width = area.width.saturating_sub(34) as usize;
    let filled = ((bar_width as f64) * ratio).round() as usize;
    let empty = bar_width.saturating_sub(filled);
    let pct = (ratio * 100.0).round() as u32;
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("PROGRESO GENERAL  ", t.muted()),
        Span::styled("[", Style::default().fg(t.border_inactive)),
        Span::styled(
            "▰".repeat(filled),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "▱".repeat(empty),
            Style::default().fg(t.border_inactive),
        ),
        Span::styled("] ", Style::default().fg(t.border_inactive)),
        Span::styled(
            format!("{}%", pct),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   {}/{} ", done, total),
            t.muted(),
        ),
    ];
    let _ = &mut spans;
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
