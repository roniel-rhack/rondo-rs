use crate::{action::Page, app::AppState};
use chrono::Local;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Tabs},
    Frame,
};
use rondo_core::domain::task::Status;

const TAB_LABELS: &[(&str, &str)] = &[
    ("1", "TODAY"),
    ("2", "TASKS"),
    ("3", "JOURNAL"),
    ("4", "FOCUS"),
    ("5", "PLUGINS"),
];

/// Renders a 3-row bordered header (binsider-style):
/// Row 0: top border with centered version + right-side telemetry strip
/// Row 1: tab strip with numeric prefixes
/// Row 2: bottom border (forms a continuous panel with body)
pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;

    let title_spans = vec![
        Span::styled("│ ", Style::default().fg(t.fg_muted)),
        Span::styled(
            "rondo",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" - {}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(t.fg_muted),
        ),
        Span::styled(" │", Style::default().fg(t.fg_muted)),
    ];

    let telemetry = telemetry_line(app);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(t.border_inactive))
        .title(Line::from(title_spans).alignment(Alignment::Center))
        .title(Line::from(telemetry).alignment(Alignment::Right))
        .title_bottom(Line::from(""));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let active_index = page_index(app.page);
    let tab_titles: Vec<Line> = TAB_LABELS
        .iter()
        .enumerate()
        .map(|(i, (num, label))| tab_line(num, label, i == active_index, t))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(active_index)
        .divider(Span::styled(
            symbols::line::VERTICAL,
            Style::default().fg(t.border_inactive),
        ))
        .padding("  ", "  ")
        .highlight_style(Style::default());
    f.render_widget(tabs, inner);
}

fn tab_line(num: &str, label: &str, active: bool, t: &crate::theme::Theme) -> Line<'static> {
    if active {
        Line::from(vec![
            Span::styled(
                format!("[{}]", num),
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", label),
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("[{}]", num),
                Style::default().fg(t.fg_muted),
            ),
            Span::styled(format!(" {}", label), Style::default().fg(t.fg_muted)),
        ])
    }
}

fn page_index(page: Page) -> usize {
    match page {
        // Tasks page maps to TAB index 1 (TASKS). Future pages map similarly.
        Page::Tasks => 1,
        Page::Journal => 2,
    }
}

/// Telemetry data strip: `14:23 · ◷ 3/8 · ☑12 · ↟5d`
fn telemetry_line(app: &AppState) -> Vec<Span<'static>> {
    let t = &app.theme;
    let now = Local::now();
    let time = now.format("%H:%M").to_string();
    let today = now.date_naive();

    let due_today = app
        .tasks
        .iter()
        .filter(|x| x.due_date == Some(today) && x.status != Status::Done)
        .count();
    let done_today = app
        .tasks
        .iter()
        .filter(|x| x.status == Status::Done)
        .count();
    let total_active = app
        .tasks
        .iter()
        .filter(|x| x.status != Status::Done)
        .count();

    let pomodoro_round = if app.pomodoro_open { "⏵ P1" } else { "P—" };

    let sep = || Span::styled(" · ", Style::default().fg(t.border_inactive));

    vec![
        Span::styled(time, Style::default().fg(t.fg).add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("◷ ", Style::default().fg(t.warn)),
        Span::styled(
            format!("{}/{}", done_today, due_today + done_today),
            Style::default().fg(t.fg_muted),
        ),
        sep(),
        Span::styled(
            pomodoro_round,
            Style::default().fg(if app.pomodoro_open {
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
        Span::raw(" "),
    ]
}
