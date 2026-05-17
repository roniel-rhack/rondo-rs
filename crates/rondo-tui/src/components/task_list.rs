use crate::app::ui_state::SortOrder;
use crate::app::{AppState, FlashTarget};
use crate::theme::Theme;
use crate::widgets::{
    bracket_panel::BracketPanel, due_badge, priority_badge, priority_spine, ring,
};
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
    let visible: Vec<usize> = sorted_indices(
        &app.data.tasks,
        app.ui.sort_order,
        app.visible_task_indices(),
    );
    let filter_label = app.data.active_filter.label().to_lowercase();
    let title = if app.modals.search_open && !app.modals.search_buf.trim().is_empty() {
        format!(
            "{} · {} tareas · /{}",
            filter_label,
            visible.len(),
            app.modals.search_buf
        )
    } else {
        format!("{} · {} tareas", filter_label, visible.len())
    };
    let panel = BracketPanel::new(&title, t).active(app.ui.focus.pane == crate::focus::Pane::List);
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

    let search_query: Option<String> =
        if app.modals.search_open && !app.modals.search_buf.trim().is_empty() {
            Some(app.modals.search_buf.trim().to_string())
        } else {
            None
        };

    // Viewport-aware slice: each task expands to multiple lines, but the
    // scroll offset is task-indexed (1 task ≈ 1 cursor step). We render at
    // most `area_height` tasks — generous bound since rows are 1–5 lines.
    let area_height = layout[1].height as usize;
    let scroll = app.ui.task_list_scroll.min(visible.len().saturating_sub(1));
    let end = (scroll + area_height.max(1)).min(visible.len());
    let slice = &visible[scroll..end];

    let items = render_items(app, slice, layout[1].width, search_query.as_deref(), scroll);
    // No REVERSED highlight — bg changes are theme-fragile. The accent ▌ gutter
    // already marks the cursor row. Use a slice-local ListState so ratatui's
    // own offset math doesn't compound with our pre-slicing.
    let mut local_state = ratatui::widgets::ListState::default();
    if let Some(sel) = app.data.task_list_state.selected() {
        if sel >= scroll && sel < scroll + slice.len() {
            local_state.select(Some(sel - scroll));
        }
    }
    let list = List::new(items);
    f.render_stateful_widget(list, layout[1], &mut local_state);

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

/// Per-row data prepared for rendering. The `lines` are fully styled
/// `Line<'static>` so the renderer just wraps them in `ListItem`s.
pub(crate) struct TaskRow {
    pub lines: Vec<Line<'static>>,
    #[allow(dead_code)]
    pub is_selected: bool,
}

/// Data-prep phase: walk `visible`, compute selection & last-row flags,
/// borrow the search engine once, and produce a `TaskRow` per visible
/// task. Keeps the render phase a thin `ListItem::new(row.lines)` loop.
pub(crate) fn build_rows(
    app: &AppState,
    visible: &[usize],
    width: u16,
    query: Option<&str>,
) -> Vec<TaskRow> {
    let selected_task_idx = app.data.selected_task;
    let last_idx = visible.len().saturating_sub(1);
    let mut engine_borrow = query.map(|_| app.data.search_engine.borrow_mut());
    visible
        .iter()
        .enumerate()
        .map(|(pos, &idx)| {
            let is_selected = idx == selected_task_idx;
            let is_last = pos == last_idx;
            let lines = build_row_lines(
                &app.data.tasks[idx],
                is_selected,
                is_last,
                app,
                &app.theme,
                width,
                query,
                engine_borrow.as_deref_mut(),
            );
            TaskRow { lines, is_selected }
        })
        .collect()
}

fn render_items(
    app: &AppState,
    visible: &[usize],
    width: u16,
    query: Option<&str>,
    scroll_offset: usize,
) -> Vec<ListItem<'static>> {
    let _ = scroll_offset;
    build_rows(app, visible, width, query)
        .into_iter()
        .map(|r| ListItem::new(r.lines))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_row_lines(
    task: &Task,
    is_selected: bool,
    is_last: bool,
    app: &AppState,
    t: &Theme,
    width: u16,
    query: Option<&str>,
    engine: Option<&mut crate::search::SearchEngine>,
) -> Vec<Line<'static>> {
    let in_visual = app.ui.selection.contains(&task.id);
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
    } else if is_selected {
        // Selected: brighter accent fg + bold + underline. No bg change.
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
    };

    // ROW 1 — primary identity: gutter · spine · checkbox · title · badges
    let truncated_title = truncate(&task.title, 50);
    let title_spans = highlight_title(&truncated_title, title_style, query, engine, t);
    let mut primary: Vec<Span<'static>> = vec![
        gutter(),
        spine.clone(),
        Span::raw(" "),
        checkbox,
        Span::raw("   "),
        priority_badge::span(task.priority, t),
        Span::raw("  "),
    ];
    primary.extend(title_spans);
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

    let incomplete: Vec<_> = task
        .subtasks
        .iter()
        .filter(|s| !s.completed)
        .take(2)
        .collect();
    for st in &incomplete {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled("↳ ", Style::default().fg(t.fg_muted)),
            Span::styled(truncate(&st.title, 60).into_owned(), t.muted()),
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

    // ROW final — subtle separator unless this is the last row (no trailing line).
    if !is_last {
        let dash_width = (width as usize).saturating_sub(4);
        let sep = "╌".repeat(dash_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(sep, Style::default().fg(t.border_inactive)),
        ]));
    }

    lines
}

fn draw_progress_bar(app: &AppState, f: &mut Frame<'_>, area: Rect, t: &Theme) {
    let total = app.data.tasks.len();
    let done = app
        .data
        .tasks
        .iter()
        .filter(|x| x.status == Status::Done)
        .count();
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
        Span::styled("▱".repeat(empty), Style::default().fg(t.border_inactive)),
        Span::styled("] ", Style::default().fg(t.border_inactive)),
        Span::styled(
            format!("{}%", pct),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   {}/{} ", done, total), t.muted()),
    ];
    let _ = &mut spans;
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Build title spans, optionally splitting matched chars into accent
/// underlined runs. Falls back to a single styled span when no query is
/// active or no match is found in the (already-truncated) title.
fn highlight_title(
    title: &str,
    base_style: Style,
    query: Option<&str>,
    engine: Option<&mut crate::search::SearchEngine>,
    t: &Theme,
) -> Vec<Span<'static>> {
    let fallback = || vec![Span::styled(title.to_string(), base_style)];
    let (Some(q), Some(eng)) = (query, engine) else {
        return fallback();
    };
    let Some((_, indices)) = eng.score(q, title) else {
        return fallback();
    };
    if indices.is_empty() {
        return fallback();
    }
    let match_set: std::collections::BTreeSet<u32> = indices.into_iter().collect();
    let hl_style = Style::default()
        .fg(t.accent)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_hl = false;
    for (i, ch) in title.chars().enumerate() {
        let is_hl = match_set.contains(&(i as u32));
        if is_hl != current_hl && !current.is_empty() {
            let style = if current_hl { hl_style } else { base_style };
            spans.push(Span::styled(std::mem::take(&mut current), style));
        }
        current.push(ch);
        current_hl = is_hl;
    }
    if !current.is_empty() {
        let style = if current_hl { hl_style } else { base_style };
        spans.push(Span::styled(current, style));
    }
    spans
}

fn sorted_indices(tasks: &[Task], order: SortOrder, base: Vec<usize>) -> Vec<usize> {
    let mut out = base;
    match order {
        SortOrder::Default => {} // already in SQL order
        SortOrder::PriorityDesc => out.sort_by(|&a, &b| {
            (tasks[b].priority as i64)
                .cmp(&(tasks[a].priority as i64))
                .then(tasks[a].id.cmp(&tasks[b].id))
        }),
        SortOrder::DueAsc => out.sort_by(|&a, &b| {
            match (tasks[a].due_date, tasks[b].due_date) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then(tasks[a].id.cmp(&tasks[b].id))
        }),
        SortOrder::CreatedAtDesc => {
            out.sort_by(|&a, &b| tasks[b].created_at.cmp(&tasks[a].created_at))
        }
        SortOrder::TitleAsc => out.sort_by(|&a, &b| {
            tasks[a]
                .title
                .to_lowercase()
                .cmp(&tasks[b].title.to_lowercase())
        }),
    }
    out
}

fn truncate(s: &str, max: usize) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= max {
        std::borrow::Cow::Borrowed(s)
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        std::borrow::Cow::Owned(out)
    }
}
