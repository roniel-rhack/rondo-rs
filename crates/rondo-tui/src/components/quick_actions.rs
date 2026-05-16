use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const GRID: &[&[(&str, &str)]] = &[
    &[
        ("E", "Editar tarea"),
        ("N", "Nueva tarea"),
        ("T", "Etiquetas"),
        ("S", "Cambiar estado"),
    ],
    &[
        ("P", "Prioridad"),
        ("A", "Asignar"),
        ("D", "Fecha límite"),
        ("X", "Eliminar"),
    ],
    &[
        ("V", "Ver calendario"),
        ("G", "Grafo dependencias"),
        ("R", "Recurrente"),
        (".", "Más acciones"),
    ],
];

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(
            " ⚙ acciones rápidas ",
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
        let cols: Vec<Constraint> =
            (0..row.len()).map(|_| Constraint::Ratio(1, row.len() as u32)).collect();
        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(cols)
            .split(row_areas[row_idx]);
        for (col_idx, (key, label)) in row.iter().enumerate() {
            let line = Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", key),
                    Style::default()
                        .fg(t.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(label.to_string(), Style::default().fg(t.fg)),
            ]);
            f.render_widget(Paragraph::new(line), col_areas[col_idx]);
        }
    }
}
