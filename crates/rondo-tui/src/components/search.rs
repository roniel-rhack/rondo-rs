use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" / search ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let match_count = matching_tasks(app);
    let line = Line::from(vec![
        Span::styled(" / ", t.accent_style()),
        Span::styled(app.search_buf.clone(), t.fg_style()),
        Span::styled(
            "▏",
            Style::default()
                .fg(t.fg)
                .add_modifier(Modifier::SLOW_BLINK),
        ),
        Span::raw("   "),
        Span::styled(
            format!(
                "{} match{}",
                match_count,
                if match_count == 1 { "" } else { "es" }
            ),
            t.muted(),
        ),
    ]);
    f.render_widget(Paragraph::new(line), inner);
}

fn matching_tasks(app: &AppState) -> usize {
    let q = app.search_buf.trim().to_lowercase();
    if q.is_empty() {
        return app.tasks.len();
    }
    app.tasks
        .iter()
        .filter(|t| {
            t.title.to_lowercase().contains(&q)
                || t.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
        })
        .count()
}
