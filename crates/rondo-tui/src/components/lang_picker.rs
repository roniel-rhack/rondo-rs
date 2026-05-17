//! `:lang` modal — list installed language packs and apply the chosen one.
//!
//! Rendering reads the picker's pre-populated `lang_picker_entries` from
//! `ModalsState`; scanning happens once in `open_lang_picker` so each frame
//! is a pure paint.

use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use rondo_core::i18n;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(
            format!(" {} ", i18n::t("modal.lang_picker.title")),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let entries = &app.modals.lang_picker_entries;
    let cursor = app.modals.lang_picker_cursor;
    let active = i18n::active_code();
    let marker = i18n::t("modal.lang_picker.active_marker");
    let builtin = i18n::t("modal.lang_picker.builtin_label");

    let mut lines: Vec<Line> = Vec::with_capacity(entries.len() + 3);
    if entries.is_empty() {
        lines.push(Line::from(Span::styled(
            i18n::t("modal.lang_picker.empty"),
            Style::default().fg(t.fg_muted),
        )));
    } else {
        for (i, entry) in entries.iter().enumerate() {
            let highlighted = i == cursor;
            let is_active = entry.code == active;
            let prefix = if highlighted { "▌ " } else { "  " };
            let marker_span = if is_active {
                Span::styled(
                    format!(" {} ", marker),
                    Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("   ")
            };
            let row_style = if highlighted {
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(t.fg)
            };
            let mut spans = vec![
                Span::styled(prefix, Style::default().fg(t.accent)),
                marker_span,
                Span::styled(format!("{:<10}", entry.code), row_style),
                Span::styled(entry.name.clone(), Style::default().fg(t.fg)),
            ];
            if entry.path.is_none() {
                spans.push(Span::styled(
                    format!("  {}", builtin),
                    Style::default().fg(t.fg_muted),
                ));
            }
            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("↑/↓", t.kbd()),
        Span::styled(
            format!(" {}  ", i18n::t("modal.lang_picker.hint_select")),
            t.muted(),
        ),
        Span::styled("Enter", t.kbd()),
        Span::styled(
            format!(" {}  ", i18n::t("modal.lang_picker.hint_apply")),
            t.muted(),
        ),
        Span::styled("Esc", t.kbd()),
        Span::styled(
            format!(" {}", i18n::t("modal.lang_picker.hint_close")),
            t.muted(),
        ),
    ]));

    f.render_widget(Paragraph::new(lines), inner);
}
