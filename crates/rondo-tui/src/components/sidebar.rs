use crate::app::AppState;
use crate::filter::{Filter, NAV_BLOCK_LEN, SIDEBAR_ITEMS};
use crate::focus::Pane;
use crate::theme::Theme;
use crate::widgets::bracket_panel::BracketPanel;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(NAV_BLOCK_LEN as u16 + 2),
            Constraint::Min(0),
        ])
        .split(area);
    draw_nav(app, f, chunks[0]);
    draw_filters(app, f, chunks[1]);
    if app.ui.leader_goto {
        draw_leader_hint(app, f, area);
    }
}

fn draw_nav(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let focused = app.ui.focus.pane == Pane::Sidebar && app.ui.focus.sidebar_item < NAV_BLOCK_LEN;
    let panel = BracketPanel::new("navegación", t).active(focused);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let counts = compute_counts(app);
    let mut lines: Vec<Line> = Vec::new();
    for (i, filter) in SIDEBAR_ITEMS.iter().take(NAV_BLOCK_LEN).enumerate() {
        let count = counts_for(*filter, &counts);
        let is_cursor = app.ui.focus.pane == Pane::Sidebar && app.ui.focus.sidebar_item == i;
        let is_active = app.data.active_filter == *filter;
        lines.push(row(*filter, count, is_cursor, is_active, t, inner.width));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_filters(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let focused = app.ui.focus.pane == Pane::Sidebar && app.ui.focus.sidebar_item >= NAV_BLOCK_LEN;
    let panel = BracketPanel::new("filtros rápidos", t).active(focused);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let counts = compute_counts(app);
    let mut lines: Vec<Line> = Vec::new();
    for (i, filter) in SIDEBAR_ITEMS.iter().skip(NAV_BLOCK_LEN).enumerate() {
        let count = counts_for(*filter, &counts);
        let sidebar_idx = NAV_BLOCK_LEN + i;
        let is_cursor =
            app.ui.focus.pane == Pane::Sidebar && app.ui.focus.sidebar_item == sidebar_idx;
        let is_active = app.data.active_filter == *filter;
        lines.push(row(*filter, count, is_cursor, is_active, t, inner.width));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_leader_hint(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    // 1-row hint anchored at the bottom of the sidebar area.
    let h = 1u16.min(area.height);
    let hint_area = Rect {
        x: area.x,
        y: area.y + area.height - h,
        width: area.width,
        height: h,
    };
    let line = Line::from(vec![
        Span::styled(
            " f",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" → ", t.muted()),
        Span::styled(
            "pulsa letra",
            Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(line), hint_area);
}

fn row(
    filter: Filter,
    count: usize,
    is_cursor: bool,
    is_active: bool,
    t: &Theme,
    width: u16,
) -> Line<'static> {
    let cursor_mark = if is_cursor {
        Span::styled(
            "▌",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(" ")
    };
    let shortcut = Span::styled(
        format!("[{}]", filter.shortcut()),
        Style::default()
            .fg(if is_active { t.accent } else { t.warn })
            .add_modifier(Modifier::BOLD),
    );
    let icon_style = if is_active {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg_muted)
    };
    let label_style = if is_active {
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else if is_cursor {
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg)
    };
    let count_str = if count > 0 {
        format!("{:>2}", count)
    } else {
        "  ".to_string()
    };
    let count_style = if is_active {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg_muted)
    };

    let label = filter.label();
    let icon = filter.icon();
    let used =
        1 + 3 + 1 + icon.chars().count() + 1 + label.chars().count() + 1 + count_str.len() + 1;
    let pad = (width as usize).saturating_sub(used);
    let pad_str = " ".repeat(pad);

    Line::from(vec![
        cursor_mark,
        shortcut,
        Span::styled(format!(" {}", icon), icon_style),
        Span::raw(" "),
        Span::styled(label, label_style),
        Span::raw(pad_str),
        Span::styled(count_str, count_style),
        Span::raw(" "),
    ])
}

struct Counts {
    by_filter: std::collections::HashMap<Filter, usize>,
}

fn counts_for(f: Filter, c: &Counts) -> usize {
    *c.by_filter.get(&f).unwrap_or(&0)
}

fn compute_counts(app: &AppState) -> Counts {
    let mut by_filter = std::collections::HashMap::new();
    for filter in SIDEBAR_ITEMS.iter().copied() {
        let n = app
            .data
            .tasks
            .iter()
            .filter(|t| filter.applies_to(t))
            .count();
        by_filter.insert(filter, n);
    }
    // Status field is referenced only to ensure the trait is in scope.
    let _ = Status::Pending;
    Counts { by_filter }
}
