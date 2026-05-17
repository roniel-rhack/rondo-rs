use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use rondo_core::i18n;

const GRID: &[&[(&str, &str)]] = &[
    &[
        ("a", "quick_actions.new_task"),
        ("e", "quick_actions.edit_title"),
        ("d", "quick_actions.delete"),
        ("space", "quick_actions.toggle_status"),
    ],
    &[
        ("A", "quick_actions.add_subtask"),
        ("B", "quick_actions.add_dependency"),
        ("v", "quick_actions.multi_select"),
        ("p", "quick_actions.pomodoro"),
    ],
    &[
        ("/", "quick_actions.search"),
        (":", "quick_actions.command"),
        ("s", "quick_actions.sort"),
        ("f<l>", "quick_actions.filter"),
    ],
    &[
        ("Ctrl+Z", "quick_actions.undo"),
        ("?", "quick_actions.help"),
        ("1/2", "quick_actions.page_switch"),
        ("Esc", "quick_actions.close_overlay"),
    ],
];

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(
            i18n::t("quick_actions.title"),
            t.accent_style(),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows: Vec<Constraint> = GRID.iter().map(|_| Constraint::Length(1)).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(rows)
        .split(inner);

    for (row_idx, row) in GRID.iter().enumerate() {
        let cols: Vec<Constraint> = (0..row.len())
            .map(|_| Constraint::Ratio(1, row.len() as u32))
            .collect();
        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(cols)
            .split(row_areas[row_idx]);
        for (col_idx, (key, label_key)) in row.iter().enumerate() {
            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", key),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(i18n::t(label_key), Style::default().fg(t.fg)),
            ]);
            f.render_widget(Paragraph::new(line), col_areas[col_idx]);
        }
    }
}
