use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};
use throbber_widgets_tui::Throbber;

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.danger))
        .title(Span::styled(
            " 🍅 Focus Session ",
            Style::default()
                .fg(t.danger)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let task_label = app
        .tasks
        .get(app.selected_task)
        .map(|t| t.title.as_str())
        .unwrap_or("(no task selected)");
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Task: ", Style::default().fg(t.fg_muted)),
            Span::styled(
                task_label,
                Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
            ),
        ])),
        chunks[0],
    );

    let (elapsed, total) = (
        app.pomodoro_started
            .map(|s| s.elapsed().as_secs())
            .unwrap_or(0),
        app.pomodoro_total.as_secs(),
    );
    let remaining = total.saturating_sub(elapsed);
    let ratio = (elapsed as f64 / total as f64).clamp(0.0, 1.0);

    let throbber = Throbber::default()
        .label(format!(
            "  {:02}:{:02} remaining",
            remaining / 60,
            remaining % 60
        ))
        .style(Style::default().fg(t.accent))
        .throbber_style(
            Style::default()
                .fg(t.danger)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(throbber, chunks[1], &mut app.pomodoro_throbber);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(t.danger).bg(t.border_inactive))
        .ratio(ratio)
        .label(format!("{:.0}%", ratio * 100.0));
    f.render_widget(gauge, chunks[2]);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " p ",
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::REVERSED),
            ),
            Span::styled(" toggle  ", Style::default().fg(t.fg_muted)),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::REVERSED),
            ),
            Span::styled(" close", Style::default().fg(t.fg_muted)),
        ])),
        chunks[3],
    );
}
