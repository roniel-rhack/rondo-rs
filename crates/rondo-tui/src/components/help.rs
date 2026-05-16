use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const BINDINGS: &[(&str, &str)] = &[
    ("Navigation", ""),
    ("j  /  ↓", "next item"),
    ("k  /  ↑", "prev item"),
    ("g", "jump top"),
    ("G", "jump bottom"),
    ("Ctrl+D", "half page down"),
    ("Ctrl+U", "half page up"),
    ("", ""),
    ("Layout", ""),
    ("Tab  /  Shift+Tab", "next / prev page"),
    ("1  /  2", "Tasks / Journal"),
    ("h  /  l", "focus left / right pane"),
    ("<  >", "resize split ± 5%"),
    ("=", "reset split 50 / 50"),
    ("", ""),
    ("Actions", ""),
    ("/", "search tasks"),
    (":", "command palette"),
    ("p", "toggle pomodoro"),
    ("?", "toggle this help"),
    ("Esc", "close top modal"),
    ("q  /  Ctrl+C", "quit"),
];

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" Keybindings ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = BINDINGS
        .iter()
        .map(|(key, label)| {
            if label.is_empty() && !key.is_empty() {
                Line::from(Span::styled(
                    format!("  {}", key),
                    t.accent_style(),
                ))
            } else if key.is_empty() {
                Line::raw("")
            } else {
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("{:<22}", key), t.kbd()),
                    Span::styled(label.to_string(), t.muted()),
                ])
            }
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(t.fg)),
        inner,
    );
}
