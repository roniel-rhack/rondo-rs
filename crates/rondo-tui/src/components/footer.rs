use crate::app::AppState;
use crate::focus::{DetailSection, Mode, Pane};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let mode = current_mode(app);

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        format!("[{}]", mode.tag()),
        Style::default()
            .fg(mode_color(mode, t))
            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    ));
    spans.push(Span::raw("  "));

    for (key, action) in hints(app) {
        spans.push(Span::styled("[", t.muted()));
        spans.push(Span::styled(key, t.kbd()));
        spans.push(Span::styled(" → ", t.muted()));
        spans.push(Span::styled(action, Style::default().fg(t.fg)));
        spans.push(Span::styled("] ", t.muted()));
    }

    spans.push(Span::raw(" "));
    if let Some(msg) = &app.status_msg {
        spans.push(Span::styled("· ", t.muted()));
        spans.push(Span::styled(msg.clone(), Style::default().fg(t.warn)));
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled("· ", t.muted()));
    spans.push(Span::styled("?", t.kbd()));
    spans.push(Span::styled(" more ", t.muted()));

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn current_mode(app: &AppState) -> Mode {
    if app.command_palette_open || app.search_open {
        Mode::Insert
    } else {
        app.mode
    }
}

fn mode_color(mode: Mode, t: &crate::theme::Theme) -> ratatui::style::Color {
    match mode {
        Mode::Normal => t.accent,
        Mode::Insert => t.warn,
        Mode::Visual => t.danger,
    }
}

/// Context-aware hint dispatcher. Caps at 5 hints; `?` appended separately.
fn hints(app: &AppState) -> Vec<(&'static str, &'static str)> {
    if app.help_open {
        return vec![("Esc", "close")];
    }
    if app.command_palette_open {
        return vec![
            ("Enter", "run"),
            ("Tab", "complete"),
            ("Esc", "cancel"),
        ];
    }
    if app.search_open {
        return vec![("Enter", "apply"), ("Esc", "cancel")];
    }
    if app.pomodoro_open {
        return vec![
            ("p", "toggle"),
            ("Esc", "close"),
            (":", "cmd"),
            ("q", "quit"),
        ];
    }

    match app.focus.pane {
        Pane::List => vec![
            ("j/k", "move"),
            ("l", "detail"),
            ("space", "toggle done"),
            ("/", "search"),
            (":", "cmd"),
        ],
        Pane::Detail => match app.focus.section {
            DetailSection::Header => vec![
                ("Tab", "next sect"),
                ("h", "list"),
                ("space", "done"),
                ("e", "edit"),
                (":", "cmd"),
            ],
            DetailSection::Subtasks => vec![
                ("j/k", "subtask"),
                ("space", "check"),
                ("Tab", "next sect"),
                ("h", "list"),
                ("a", "add"),
            ],
            DetailSection::Dependencies => vec![
                ("Enter", "goto dep"),
                ("Tab", "next sect"),
                ("h", "list"),
                (":", "cmd"),
                ("q", "quit"),
            ],
            DetailSection::Notes => vec![
                ("j/k", "note"),
                ("Enter", "open"),
                ("a", "add"),
                ("Tab", "next sect"),
                ("h", "list"),
            ],
        },
    }
}
