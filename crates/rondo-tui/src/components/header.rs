use crate::app::AppState;
use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use rondo_core::domain::task::Status;

/// Single-row brand strip: left = product mark, center = subtitle, right = telemetry.
pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(0),
            Constraint::Length(56),
        ])
        .split(area);

    let mark = Line::from(vec![
        Span::styled(
            " ▌",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "rondo",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(t.fg_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(mark), cols[0]);

    let subtitle = Line::from(vec![
        Span::styled("// ", Style::default().fg(t.border_inactive)),
        Span::styled(
            "SISTEMA DE GESTIÓN DE TAREAS AVANZADO",
            Style::default().fg(t.fg_muted),
        ),
        Span::styled(" //", Style::default().fg(t.border_inactive)),
    ]);
    f.render_widget(
        Paragraph::new(subtitle).alignment(Alignment::Center),
        cols[1],
    );

    f.render_widget(
        Paragraph::new(Line::from(telemetry(app))).alignment(Alignment::Right),
        cols[2],
    );
}

fn telemetry(app: &AppState) -> Vec<Span<'static>> {
    let t = &app.theme;
    let now = Local::now();
    let time = now.format("%H:%M:%S").to_string();
    let today = now.date_naive();
    let mut due_today = 0usize;
    let mut done_today = 0usize;
    let mut total_active = 0usize;
    for x in &app.data.tasks {
        if x.status == Status::Done {
            done_today += 1;
        } else {
            total_active += 1;
            if x.due_date == Some(today) {
                due_today += 1;
            }
        }
    }
    let pomodoro = if app.modals.pomodoro_open {
        "⏵ P1"
    } else {
        "P—"
    };
    let sep = || Span::styled(" · ", Style::default().fg(t.border_inactive));

    vec![
        Span::styled("⊙ ", Style::default().fg(t.success)),
        Span::styled(
            "ONLINE",
            Style::default().fg(t.success).add_modifier(Modifier::BOLD),
        ),
        sep(),
        Span::styled(time, Style::default().fg(t.fg).add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("◷ ", Style::default().fg(t.warn)),
        Span::styled(
            format!("{}/{}", done_today, due_today + done_today),
            Style::default().fg(t.fg_muted),
        ),
        sep(),
        Span::styled(
            pomodoro,
            Style::default().fg(if app.modals.pomodoro_open {
                t.danger
            } else {
                t.fg_muted
            }),
        ),
        sep(),
        Span::styled("☑", Style::default().fg(t.success)),
        Span::styled(
            format!("{} ", total_active),
            Style::default().fg(t.fg_muted),
        ),
    ]
}
