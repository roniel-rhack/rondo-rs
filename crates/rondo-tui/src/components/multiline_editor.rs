//! Generic multi-line editor overlay backed by `tui-textarea`.
//! Used by description / note editors so they share a consistent look.

use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use rondo_core::i18n;
use tui_textarea::TextArea;

pub struct Config<'a> {
    pub title: &'a str,
    pub hint_extra: Option<&'a str>,
}

pub fn draw(
    app: &mut AppState,
    f: &mut Frame<'_>,
    area: Rect,
    textarea: TextAreaAccess,
    cfg: Config<'_>,
) {
    // Snapshot theme tokens up front so the textarea mutable borrow on app
    // doesn't collide with reads on `app.theme`.
    let border_style = app.theme.border_style(true);
    let accent_style = app.theme.accent_style();
    let muted = app.theme.muted();
    let kbd = app.theme.kbd();
    let cursor_fg = app.theme.bg;
    let cursor_bg = app.theme.accent;

    let h = area.height.saturating_sub(4).max(8);
    let w = area.width.saturating_sub(4);
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(cfg.title.to_string(), accent_style));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let textarea_ref = textarea.get(app);
    textarea_ref.set_cursor_line_style(Style::default());
    textarea_ref.set_cursor_style(
        Style::default()
            .fg(cursor_fg)
            .bg(cursor_bg)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(&*textarea_ref, layout[0]);

    let mut hint_spans = vec![
        Span::styled(" ", muted),
        Span::styled("Ctrl+S", kbd),
        Span::styled(i18n::t("multiline_editor.hint_save"), muted),
        Span::styled("Esc", kbd),
        Span::styled(i18n::t("multiline_editor.hint_cancel"), muted),
        Span::styled("←↑↓→", kbd),
        Span::styled(
            format!(" {}", i18n::t("multiline_editor.hint_navigate")),
            muted,
        ),
    ];
    if let Some(extra) = cfg.hint_extra {
        hint_spans.push(Span::styled(" · ", muted));
        hint_spans.push(Span::styled(extra.to_string(), muted));
    }
    f.render_widget(Paragraph::new(Line::from(hint_spans)), layout[1]);
}

/// Picks which textarea on the AppState the overlay should render.
/// Kept as an explicit enum so the borrow checker is happy.
pub enum TextAreaAccess {
    Description,
    Note,
}

impl TextAreaAccess {
    pub fn get<'a>(&self, app: &'a mut AppState) -> &'a mut TextArea<'static> {
        match self {
            TextAreaAccess::Description => &mut app.modals.description_textarea,
            TextAreaAccess::Note => &mut app.modals.note_textarea,
        }
    }
}
