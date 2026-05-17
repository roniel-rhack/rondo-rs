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
    let mode_label = if mode == Mode::Visual && !app.ui.selection.is_empty() {
        format!("[VIS·{}]", app.ui.selection.len())
    } else {
        format!("[{}]", mode.tag())
    };
    spans.push(Span::styled(
        mode_label,
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
    if app.modals.command_palette_open || app.modals.search_open {
        Mode::Insert
    } else {
        app.ui.mode
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
    if app.modals.help_open {
        return vec![("Esc", "close")];
    }
    if app.modals.command_palette_open {
        return vec![("Enter", "run"), ("Tab", "complete"), ("Esc", "cancel")];
    }
    if app.modals.search_open {
        return vec![("Enter", "apply"), ("Esc", "cancel")];
    }
    if app.modals.pomodoro_open {
        return vec![
            ("p", "toggle"),
            ("Esc", "close"),
            (":", "cmd"),
            ("q", "quit"),
        ];
    }

    if app.ui.mode == Mode::Visual {
        return vec![
            ("j/k", "extend"),
            ("d", "bulk done"),
            ("P", "bulk prio"),
            ("Esc", "cancel"),
            (":", "cmd"),
        ];
    }
    match app.ui.focus.pane {
        Pane::Sidebar => vec![
            ("j/k", "move"),
            ("Enter", "aplicar filtro"),
            ("l", "list"),
            ("Esc", "cancel"),
            (":", "cmd"),
        ],
        Pane::List => vec![
            ("a", "add"),
            ("A", "+ subtarea"),
            ("B", "+ dep"),
            ("v", "select"),
            ("f<l>", "filtro"),
        ],
        Pane::Detail => match app.ui.focus.section {
            DetailSection::Header => vec![
                ("Tab", "next sect"),
                ("h", "list"),
                ("space", "done"),
                ("e", "edit"),
                ("f<l>", "filtro"),
            ],
            DetailSection::Subtasks => vec![
                ("A", "+ subtarea"),
                ("space", "check"),
                ("Tab", "next sect"),
                ("h", "list"),
                ("f<l>", "filtro"),
            ],
            DetailSection::Dependencies => vec![
                ("B", "+ dep"),
                ("Tab", "next sect"),
                ("h", "list"),
                (":", "cmd"),
                ("f<l>", "filtro"),
            ],
            DetailSection::Notes => vec![
                ("j/k", "note"),
                ("Enter", "open"),
                ("Tab", "next sect"),
                ("h", "list"),
                ("f<l>", "filtro"),
            ],
        },
    }
}
