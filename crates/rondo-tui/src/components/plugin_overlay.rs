//! Transient floating overlay that renders a plugin's cached `Show`
//! response when its `ViewKind` is `Overlay`.
//!
//! Unlike `plugin_page`, this component does NOT re-dispatch `Show` on
//! every frame: the `ViewSpec` is captured by `handle_command` at the
//! moment the user invoked the plugin, and the overlay is purely a
//! read of that cached value. This keeps WASM round-trips off the
//! render path and matches the static "snapshot" nature of overlays
//! like quote-of-the-day.

use crate::app::AppState;
use crate::widgets::viewspec;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let (id, view) = match app.modals.plugin_overlay.as_ref() {
        Some(t) => t,
        None => return,
    };
    let t = &app.theme;
    f.render_widget(Clear, area);
    let title = format!(" 🧩 {} ", id);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(
            title,
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line<'static>> = viewspec::render(view, t);
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            " Esc ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("cerrar", Style::default().fg(t.fg)),
    ]));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
