use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let mut spans = vec![Span::raw(" ")];
    for (key, label) in hints(app) {
        spans.push(Span::styled(key, t.kbd()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, t.muted()));
        spans.push(Span::raw("  "));
    }
    if let Some(msg) = &app.status_msg {
        spans.push(Span::styled("· ", t.muted()));
        spans.push(Span::styled(
            msg.clone(),
            Style::default().fg(t.warn),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Context-aware hints: 4-5 most relevant for the current page/overlay.
fn hints(app: &AppState) -> Vec<(&'static str, &'static str)> {
    if app.command_palette_open {
        return vec![
            ("Enter", "run"),
            ("Esc", "cancel"),
        ];
    }
    if app.pomodoro_open {
        return vec![
            ("p", "toggle"),
            ("Esc", "close"),
            ("?", "help"),
            ("q", "quit"),
        ];
    }
    vec![
        ("j/k", "nav"),
        ("Tab", "focus"),
        ("/", "search"),
        (":", "cmd"),
        ("?", "help"),
        ("q", "quit"),
    ]
}
