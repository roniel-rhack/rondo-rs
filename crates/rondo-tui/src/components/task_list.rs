use crate::app::AppState;
use crate::widgets::{due_badge, priority_badge};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let border_color = if app.focus_left {
        t.border_active
    } else {
        t.border_inactive
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" Tasks ({}) ", app.tasks.len()),
            t.accent_style(),
        ));

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|task| {
            let icon_color = match task.status {
                Status::Done => t.success,
                Status::InProgress => t.accent,
                Status::Pending => t.fg_muted,
            };
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled(
                    format!(" {} ", task.status.icon()),
                    Style::default().fg(icon_color),
                ),
                Span::styled(
                    task.title.clone(),
                    if task.status == Status::Done {
                        Style::default()
                            .fg(t.fg_muted)
                            .add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default().fg(t.fg)
                    },
                ),
                Span::raw("  "),
                priority_badge::span(task.priority, t),
            ];
            if let Some(b) = due_badge::span(task.due_date, t) {
                spans.push(Span::raw(" "));
                spans.push(b);
            }
            if task.is_blocked() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    " BLOCKED ",
                    Style::default()
                        .fg(t.danger)
                        .add_modifier(Modifier::REVERSED | Modifier::BOLD),
                ));
            }
            let (done, total) = task.subtask_progress();
            if total > 0 {
                spans.push(Span::styled(
                    format!("  {}/{}", done, total),
                    Style::default().fg(t.fg_muted),
                ));
            }
            if !task.tags.is_empty() {
                spans.push(Span::styled(
                    format!("  [{}]", task.tags.join(",")),
                    Style::default().fg(t.fg_muted),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default();
    state.select(Some(app.selected_task));
    f.render_stateful_widget(list, area, &mut state);
}
