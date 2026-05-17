//! Full-screen overlay that renders a plugin's `Show` ViewSpec response.
//!
//! Special case: `builtin.analytics` uses direct ratatui rendering instead
//! of ViewSpec (the dashboard's donut/sparkline don't fit the DSL yet) —
//! we route it to `plugins::builtin::analytics::draw`.

use crate::app::AppState;
use crate::widgets::viewspec;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw(app: &mut AppState, f: &mut Frame<'_>, area: Rect) {
    let t_accent = app.theme.accent;
    let t_fg_muted = app.theme.fg_muted;
    let t_fg = app.theme.fg;
    let id = match app.modals.plugin_page.clone() {
        Some(s) => s,
        None => return,
    };

    f.render_widget(Clear, area);
    let title = format!(" 🧩 {} ", id);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t_accent))
        .title(Span::styled(
            title,
            Style::default().fg(t_accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Analytics renders via direct ratatui (the 4-panel dashboard row).
    if id == "builtin.analytics" {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);
        crate::plugins::builtin::analytics::draw(app, f, chunks[0]);
        let hint = Line::from(vec![
            Span::styled(
                " Esc ",
                Style::default().fg(t_accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("cerrar", Style::default().fg(t_fg)),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[1]);
        return;
    }

    // ViewSpec path: re-dispatch Show on every frame so the plugin can
    // reflect internal state changes (e.g. calendar cursor on KeyPress).
    let ctx = rondo_plugin_api::PluginContext::new(&id);
    let view = app
        .plugins
        .get_mut(&id)
        .and_then(|p| p.handle(rondo_plugin_api::PluginAction::Show, &ctx).view);

    let mut lines: Vec<Line<'static>> = match view {
        Some(v) => viewspec::render(&v, &app.theme),
        None => vec![Line::from(Span::styled(
            "(no view returned)".to_string(),
            Style::default().fg(t_fg_muted),
        ))],
    };
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            " Esc ",
            Style::default().fg(t_accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("cerrar  ", Style::default().fg(t_fg)),
        Span::styled(
            "h/j/k/l",
            Style::default().fg(t_accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navegar", Style::default().fg(t_fg)),
    ]));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
