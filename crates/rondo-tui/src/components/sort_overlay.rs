use crate::app::ui_state::SortOrder;
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
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(
            " sort by ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = SortOrder::ALL
        .iter()
        .enumerate()
        .map(|(i, &order)| {
            let active = order == app.ui.sort_order;
            let prefix = if active { "▌ " } else { "  " };
            let key = format!("[{}] ", char::from(b'1' + i as u8));
            let style = if active {
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(t.fg)
            };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(t.accent)),
                Span::styled(key, Style::default().fg(t.fg_muted)),
                Span::styled(order.label(), style),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}
