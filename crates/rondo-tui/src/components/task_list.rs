use crate::app::AppState;
use crate::widgets::{due_badge, priority_badge, priority_spine};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let title = format!("Tasks ({})", app.tasks.len());
    let block = t.panel(&title, app.focus_left);

    if app.tasks.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::raw(""),
            Line::from(Span::styled("  No tasks yet", t.muted())),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("?", t.kbd()),
                Span::raw(" "),
                Span::styled("for help", t.muted()),
                Span::raw("    "),
                Span::styled(":", t.kbd()),
                Span::raw(" "),
                Span::styled("commands", t.muted()),
            ]),
        ];
        f.render_widget(
            ratatui::widgets::Paragraph::new(lines).block(block),
            area,
        );
        return;
    }

    let selected = app.task_list_state.selected();
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            let is_selected = Some(idx) == selected;
            let gutter = if is_selected {
                Span::styled("▌ ", Style::default().fg(t.accent))
            } else {
                Span::raw("  ")
            };
            let spine = priority_spine::glyph(task.priority, t);
            let icon_color = match task.status {
                Status::Done => t.success,
                Status::InProgress => t.accent,
                Status::Pending => t.fg_muted,
            };
            let title_style = if task.status == Status::Done {
                Style::default()
                    .fg(t.fg_muted)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(t.fg)
            };
            let mut spans: Vec<Span<'static>> = vec![
                gutter,
                spine,
                Span::raw(" "),
                Span::styled(task.status.icon().to_string(), Style::default().fg(icon_color)),
                Span::raw("  "),
                Span::styled(task.title.clone(), title_style),
                Span::raw("   "),
                priority_badge::span(task.priority, t),
            ];
            if let Some(b) = due_badge::span(task.due_date, t) {
                spans.push(Span::raw("   "));
                spans.push(b);
            }
            if task.is_blocked() {
                spans.push(Span::raw("   "));
                spans.push(Span::styled(
                    "blocked",
                    Style::default()
                        .fg(t.danger)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            let (done, total) = task.subtask_progress();
            if total > 0 {
                spans.push(Span::styled(
                    format!("   {}/{}", done, total),
                    t.muted(),
                ));
            }
            if !task.tags.is_empty() {
                spans.push(Span::styled(
                    format!("   #{}", task.tags.join(" #")),
                    t.muted(),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(t.selection());
    f.render_stateful_widget(list, area, &mut app.task_list_state);
}
