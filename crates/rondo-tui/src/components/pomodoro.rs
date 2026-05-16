use crate::app::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    // Dim backdrop over everything outside the modal
    let backdrop =
        Block::default().style(Style::default().fg(t.fg_muted).add_modifier(Modifier::DIM));
    f.render_widget(backdrop, f.area());

    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.danger))
        .title(Span::styled(
            " ◉ Focus Session ",
            Style::default().fg(t.danger).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    // Row 0: phase label
    let phase = "Focus 1/4";
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            phase,
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center),
        chunks[0],
    );

    // Row 1: task title
    let task_label = app
        .data
        .tasks
        .get(app.data.selected_task)
        .map(|t| t.title.as_str())
        .unwrap_or("(no task selected)");
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(task_label, t.muted())]))
            .alignment(Alignment::Center),
        chunks[1],
    );

    // Row 2-5: big text timer (mm:ss)
    let (elapsed, total) = (
        app.modals
            .pomodoro_started
            .map(|s| s.elapsed().as_secs())
            .unwrap_or(0),
        app.modals.pomodoro_total.as_secs(),
    );
    let remaining = total.saturating_sub(elapsed);
    let mm_ss = format!("{:02}:{:02}", remaining / 60, remaining % 60);

    let big = BigText::builder()
        .pixel_size(PixelSize::Quadrant)
        .style(Style::default().fg(t.danger).add_modifier(Modifier::BOLD))
        .lines(vec![mm_ss.into()])
        .alignment(Alignment::Center)
        .build();
    f.render_widget(big, chunks[2]);

    // Row -2: gauge
    let ratio = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(t.danger).bg(t.border_inactive))
        .ratio(ratio)
        .label("");
    f.render_widget(gauge, chunks[4]);
}
