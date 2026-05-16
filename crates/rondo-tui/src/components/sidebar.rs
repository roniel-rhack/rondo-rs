use crate::app::AppState;
use crate::theme::Theme;
use crate::widgets::bracket_panel::BracketPanel;
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    Frame,
};
use rondo_core::domain::task::Status;

/// Two-section sidebar:
///   NAVEGACIÓN  (icon + label + counter)
///   FILTROS RÁPIDOS (icon + label, no counter)
pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(11)])
        .split(area);

    draw_nav(app, f, chunks[0]);
    draw_filters(app, f, chunks[1]);
}

fn draw_nav(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("navegación", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let counts = compute_counts(app);
    let items = [
        ("◉", "INBOX", counts.inbox, true),
        ("◷", "HOY", counts.today, false),
        ("⏵", "PRÓXIMAS", counts.upcoming, false),
        ("⛀", "PROYECTOS", counts.tags, false),
        ("◆", "ETIQUETAS", counts.tags, false),
        ("⊟", "CALENDARIO", 0, false),
        ("▤", "ANÁLISIS", 0, false),
        ("⌬", "GRAFO", 0, false),
        ("⊳", "AUTOMAT.", 0, false),
        ("✕", "PAPELERA", 0, false),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (icon, label, count, active) in items {
        lines.push(nav_row(icon, label, count, active, t, inner.width));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn nav_row(
    icon: &'static str,
    label: &'static str,
    count: usize,
    active: bool,
    t: &Theme,
    width: u16,
) -> Line<'static> {
    let icon_style = if active {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg_muted)
    };
    let label_style = if active {
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(t.fg)
    };
    let count_str = if count > 0 {
        format!("{:>2}", count)
    } else {
        "  ".to_string()
    };
    let count_style = if active {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg_muted)
    };

    // Compute padding so counter sits flush right.
    let used = 1 + icon.chars().count() + 1 + label.chars().count() + 1 + count_str.len() + 1;
    let pad = (width as usize).saturating_sub(used);
    let pad_str = " ".repeat(pad);

    Line::from(vec![
        Span::styled(format!(" {}", icon), icon_style),
        Span::raw(" "),
        Span::styled(label, label_style),
        Span::raw(pad_str),
        Span::styled(count_str, count_style),
        Span::raw(" "),
    ])
}

fn draw_filters(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("filtros", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let items = [
        ("!", "URGENTES", t.danger),
        ("↑", "ALTA PRIO", t.warn),
        ("@", "ASIG. A MÍ", t.accent),
        ("#", "SIN ETIQUETA", t.fg_muted),
        ("✓", "COMPLETADAS", t.success),
        ("◷", "VENCIDAS", t.danger),
    ];
    let mut lines: Vec<Line> = Vec::new();
    for (icon, label, color) in items {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {}", icon),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(label, Style::default().fg(t.fg)),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

struct Counts {
    inbox: usize,
    today: usize,
    upcoming: usize,
    tags: usize,
}

fn compute_counts(app: &AppState) -> Counts {
    let today = Local::now().date_naive();
    let inbox = app
        .tasks
        .iter()
        .filter(|t| t.status != Status::Done)
        .count();
    let today_count = app
        .tasks
        .iter()
        .filter(|t| t.status != Status::Done && t.due_date == Some(today))
        .count();
    let upcoming = app
        .tasks
        .iter()
        .filter(|t| {
            t.status != Status::Done
                && t.due_date.is_some_and(|d| {
                    let delta = (d - today).num_days();
                    (1..=7).contains(&delta)
                })
        })
        .count();
    let mut tag_set: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for t in &app.tasks {
        for tag in &t.tags {
            tag_set.insert(tag.as_str());
        }
    }
    Counts {
        inbox,
        today: today_count,
        upcoming,
        tags: tag_set.len(),
    }
}
