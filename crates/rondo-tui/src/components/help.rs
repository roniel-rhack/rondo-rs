use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use rondo_core::i18n;

const BINDINGS: &[(&str, &str)] = &[
    ("Navigation", ""),
    ("j  /  ↓", "next item"),
    ("k  /  ↑", "prev item"),
    ("g", "jump top"),
    ("G", "jump bottom"),
    ("Ctrl+D", "half page down"),
    ("Ctrl+U", "half page up"),
    ("", ""),
    ("Filter (any focus)", ""),
    ("f i", "Inbox"),
    ("f t", "Today"),
    ("f p", "Upcoming"),
    ("f u", "Urgent"),
    ("f H", "High priority"),
    ("f o", "Overdue"),
    ("f c", "Done"),
    ("f n", "Untagged"),
    ("f A", "All"),
    ("", ""),
    ("Layout", ""),
    ("Tab  /  Shift+Tab", "next / prev section"),
    ("1  /  2", "Tasks / Journal"),
    ("h  /  l", "focus Sidebar / List / Detail"),
    ("<  >", "resize split ± 5%"),
    ("=", "reset split 50 / 50"),
    ("", ""),
    ("Edit", ""),
    ("a", "quick-add task"),
    ("e", "edit title"),
    ("d", "delete task (confirm)"),
    ("A", "add subtask"),
    ("B", "add/remove dependency"),
    ("space", "toggle status / subtask"),
    (
        "v  /  V·d  /  V·P",
        "visual multi-select / bulk done / bulk priority",
    ),
    ("Ctrl+Z", "undo last mutation"),
    ("", ""),
    ("Search & navigation", ""),
    ("/", "fuzzy search (highlights in list + detail)"),
    (":", "command palette"),
    (".", "quick actions grid"),
    (
        "s",
        "sort overlay (default / priority / due / newest / title)",
    ),
    ("p", "toggle pomodoro"),
    ("", ""),
    ("Plugins", ""),
    ("(CLI)", "rondo-rs plugins list / info / install / remove"),
    (
        "(built-in)",
        "pomodoro · bell · calendar · focus-page · dep-graph · analytics",
    ),
    (
        "config",
        "~/.rondo-rs/config.toml  ·  permissions table gates dangerous caps",
    ),
    (
        "install dir",
        "~/.rondo-rs/plugins/<id>/  with plugin.toml + plugin.wasm",
    ),
    ("", ""),
    ("Misc", ""),
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
        .title(Span::styled(i18n::t("help.title"), t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = BINDINGS
        .iter()
        .map(|(key, label)| {
            if label.is_empty() && !key.is_empty() {
                Line::from(Span::styled(format!("  {}", key), t.accent_style()))
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
