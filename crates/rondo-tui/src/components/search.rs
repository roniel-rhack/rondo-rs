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
        .border_style(t.border_style(true))
        .title(Span::styled(i18n::t("search.title"), t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let match_count = matching_tasks(app);
    let key = if match_count == 1 {
        "search.match_singular"
    } else {
        "search.match_plural"
    };
    let count_label = i18n::tf(key, &[("n", &match_count.to_string())]);
    let line = Line::from(vec![
        Span::styled(" / ", t.accent_style()),
        Span::styled(app.modals.search_buf.clone(), t.fg_style()),
        Span::styled(
            "▏",
            Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
        ),
        Span::raw("   "),
        Span::styled(count_label, t.muted()),
    ]);
    f.render_widget(Paragraph::new(line), inner);
}

fn matching_tasks(app: &AppState) -> usize {
    let q = app.modals.search_buf.trim();
    if q.is_empty() {
        return app.data.visible_task_indices().len();
    }
    app.data.visible_task_indices_with_search(q).len()
}
