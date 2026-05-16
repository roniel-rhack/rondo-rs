use crate::app::AppState;
use crate::filter::{Filter, NAV_BLOCK_LEN, SIDEBAR_ITEMS};
use crate::focus::Pane;
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

const NAV_GLYPHS: &[&str] = &["◉", "◷", "⏵", "⛀", "◆", "⊟", "▤", "⌬", "⊳", "✕"];
const FILTER_GLYPHS: &[&str] = &["!", "↑", "@", "#", "✓", "◷"];

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
    let focused = app.focus.pane == Pane::Sidebar && app.focus.sidebar_item < NAV_BLOCK_LEN;
    let panel = BracketPanel::new("navegación", t).active(focused);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let counts = compute_counts(app);
    let mut lines: Vec<Line> = Vec::new();
    for (i, filter) in SIDEBAR_ITEMS.iter().take(NAV_BLOCK_LEN).enumerate() {
        let icon = NAV_GLYPHS.get(i).copied().unwrap_or("·");
        let count = counts_for(filter, &counts);
        let is_cursor = app.focus.pane == Pane::Sidebar && app.focus.sidebar_item == i;
        let is_active = app.active_filter == *filter;
        lines.push(nav_row(icon, filter.label(), count, is_cursor, is_active, t, inner.width));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_filters(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let focused = app.focus.pane == Pane::Sidebar && app.focus.sidebar_item >= NAV_BLOCK_LEN;
    let panel = BracketPanel::new("filtros", t).active(focused);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let palette = [t.danger, t.warn, t.accent, t.fg_muted, t.success, t.danger];
    let mut lines: Vec<Line> = Vec::new();
    for (i, filter) in SIDEBAR_ITEMS.iter().skip(NAV_BLOCK_LEN).enumerate() {
        let icon = FILTER_GLYPHS.get(i).copied().unwrap_or("·");
        let color = palette[i % palette.len()];
        let sidebar_idx = NAV_BLOCK_LEN + i;
        let is_cursor =
            app.focus.pane == Pane::Sidebar && app.focus.sidebar_item == sidebar_idx;
        let is_active = app.active_filter == *filter;
        lines.push(filter_row(
            icon,
            filter.label(),
            color,
            is_cursor,
            is_active,
            t,
            inner.width,
        ));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn nav_row(
    icon: &'static str,
    label: &'static str,
    count: usize,
    is_cursor: bool,
    is_active: bool,
    t: &Theme,
    width: u16,
) -> Line<'static> {
    let cursor_mark = if is_cursor {
        Span::styled("▌", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    } else {
        Span::raw(" ")
    };
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

    let used = 1 + 1 + icon.chars().count() + 1 + label.chars().count() + 1 + count_str.len() + 1;
    let pad = (width as usize).saturating_sub(used);
    let pad_str = " ".repeat(pad);

    Line::from(vec![
        cursor_mark,
        Span::styled(format!(" {}", icon), icon_style),
        Span::raw(" "),
        Span::styled(label, label_style),
        Span::raw(pad_str),
        Span::styled(count_str, count_style),
        Span::raw(" "),
    ])
}

fn filter_row(
    icon: &'static str,
    label: &'static str,
    color: ratatui::style::Color,
    is_cursor: bool,
    is_active: bool,
    t: &Theme,
    _width: u16,
) -> Line<'static> {
    let cursor_mark = if is_cursor {
        Span::styled("▌", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
    } else {
        Span::raw(" ")
    };
    let label_style = if is_active {
        Style::default()
            .fg(t.accent)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(t.fg)
    };
    Line::from(vec![
        cursor_mark,
        Span::styled(
            format!(" {}", icon),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(label, label_style),
    ])
}

struct Counts {
    inbox: usize,
    today: usize,
    upcoming: usize,
    tags: usize,
    urgent: usize,
    high: usize,
    no_tag: usize,
    completed: usize,
    overdue: usize,
}

fn counts_for(f: &Filter, c: &Counts) -> usize {
    match f {
        Filter::Inbox => c.inbox,
        Filter::Today => c.today,
        Filter::Upcoming => c.upcoming,
        Filter::AllProjects | Filter::AllTags => c.tags,
        Filter::Calendar | Filter::Analysis | Filter::Graph | Filter::Automations
        | Filter::Trash => 0,
        Filter::Urgent => c.urgent,
        Filter::HighPriority => c.high,
        Filter::AssignedToMe => c.inbox,
        Filter::NoTag => c.no_tag,
        Filter::Completed => c.completed,
        Filter::Overdue => c.overdue,
    }
}

fn compute_counts(app: &AppState) -> Counts {
    use rondo_core::domain::task::Priority;
    let today = Local::now().date_naive();
    let inbox = app.tasks.iter().filter(|t| t.status != Status::Done).count();
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
    let urgent = app
        .tasks
        .iter()
        .filter(|t| t.priority == Priority::Urgent)
        .count();
    let high = app
        .tasks
        .iter()
        .filter(|t| matches!(t.priority, Priority::High | Priority::Urgent))
        .count();
    let no_tag = app.tasks.iter().filter(|t| t.tags.is_empty()).count();
    let completed = app
        .tasks
        .iter()
        .filter(|t| t.status == Status::Done)
        .count();
    let overdue = app
        .tasks
        .iter()
        .filter(|t| t.status != Status::Done && t.due_date.is_some_and(|d| d < today))
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
        urgent,
        high,
        no_tag,
        completed,
        overdue,
    }
}
