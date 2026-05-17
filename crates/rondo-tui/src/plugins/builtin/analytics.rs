use crate::app::AppState;
use crate::theme::Theme;
use crate::widgets::bracket_panel::BracketPanel;
use chrono::Duration;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    Frame,
};
use rondo_core::domain::task::Status;
use rondo_plugin_api::{
    action::PluginAction,
    capabilities::{Capability, QueryScope},
    plugin::{Plugin, PluginContext, PluginManifest, PluginResult},
};
use std::collections::HashMap;

/// Direct-render builtin plugin that surfaces the bottom analytics row.
///
/// The actual ratatui rendering happens via the free [`draw`] function below,
/// which the host (`components::root`) calls into directly. `handle()` is a
/// no-op for now: the [`ViewSpec`](rondo_plugin_api::view::ViewSpec) DSL does
/// not yet expose donut / sparkline / stacked-bar blocks, so we keep the
/// existing widget code and only declare the capability surface here. Once
/// the DSL grows the needed primitives, `handle()` will return a real
/// `ViewSpec` and the host wrapper can drop the direct call.
#[derive(Default)]
pub struct AnalyticsPlugin;

impl Plugin for AnalyticsPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.analytics".into(),
            name: "Analytics Dashboard".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![
                Capability::PageView,
                Capability::QueryAccess(QueryScope::Tasks),
                Capability::QueryAccess(QueryScope::FocusSessions),
            ],
            exporter: None,
            syncer: None,
            cli: None,
        }
    }

    fn handle(&mut self, _action: PluginAction, _ctx: &PluginContext) -> PluginResult {
        PluginResult::default()
    }
}

/// Bottom analytics row: donut · 7-day bar chart · tag distribution · sync placeholder
pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Length(34),
            Constraint::Min(22),
            Constraint::Length(28),
        ])
        .split(area);

    draw_donut(app, f, cols[0]);
    draw_bars_7d(app, f, cols[1]);
    draw_distribution(app, f, cols[2]);
    draw_sync(app, f, cols[3]);
}

fn draw_donut(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("vista general", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let total = app.data.tasks.len();
    let done = app
        .data
        .tasks
        .iter()
        .filter(|x| x.status == Status::Done)
        .count();
    let in_prog = app
        .data
        .tasks
        .iter()
        .filter(|x| x.status == Status::InProgress)
        .count();
    let pending = app
        .data
        .tasks
        .iter()
        .filter(|x| x.status == Status::Pending)
        .count();
    let today = crate::clock::today();
    let overdue = app
        .data
        .tasks
        .iter()
        .filter(|x| x.status != Status::Done && x.due_date.is_some_and(|d| d < today))
        .count();

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}", total),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                "TOTAL",
                Style::default().fg(t.fg_muted).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        legend_row("◑", done, "Completadas", t.success, t),
        legend_row("◐", in_prog, "En progreso", t.accent, t),
        legend_row("○", pending, "Pendientes", t.warn, t),
        legend_row("!", overdue, "Vencidas", t.danger, t),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn legend_row(
    icon: &str,
    count: usize,
    label: &str,
    color: ratatui::style::Color,
    t: &Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            icon.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:>3}", count),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(label.to_string(), t.muted()),
    ])
}

fn draw_bars_7d(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("próximas 7 días", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let today = crate::clock::today();
    let mut counts = [0usize; 7];
    for task in &app.data.tasks {
        if let Some(d) = task.due_date {
            let delta = (d - today).num_days();
            if (0..7).contains(&delta) {
                counts[delta as usize] += 1;
            }
        }
    }
    let max = *counts.iter().max().unwrap_or(&1).max(&1);
    let bar_height = inner.height.saturating_sub(2) as usize;
    let max_h = bar_height.max(1);

    // Build bars top-down
    let mut grid: Vec<String> = vec![String::new(); max_h];
    for &c in counts.iter() {
        let h = ((c as f64 / max as f64) * max_h as f64).round() as usize;
        for (row, line) in grid.iter_mut().enumerate().take(max_h) {
            let filled = row >= (max_h - h);
            line.push(' ');
            line.push(' ');
            line.push(' ');
            line.push(if filled { '█' } else { ' ' });
            line.push(if filled { '█' } else { ' ' });
        }
    }

    let mut lines: Vec<Line> = grid
        .into_iter()
        .map(|row| Line::from(Span::styled(row, Style::default().fg(t.accent))))
        .collect();
    // value labels above bars (top row)
    if let Some(first) = lines.first_mut() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, &c) in counts.iter().enumerate() {
            let label = if c > 0 {
                format!("{}", c)
            } else {
                " ".to_string()
            };
            spans.push(Span::raw("  "));
            let _ = i;
            spans.push(Span::styled(
                format!("{:>2}", label),
                Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
        }
        *first = Line::from(spans);
    }
    // day labels
    let mut date_spans: Vec<Span<'static>> = Vec::new();
    for i in 0..7 {
        let d = today + Duration::days(i as i64);
        date_spans.push(Span::raw("  "));
        date_spans.push(Span::styled(d.format("%d").to_string(), t.muted()));
        date_spans.push(Span::raw(" "));
    }
    lines.push(Line::from(date_spans));
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_distribution(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("distribución por tag", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for task in &app.data.tasks {
        if task.tags.is_empty() {
            *tag_counts.entry("(sin tag)".into()).or_default() += 1;
        }
        for tag in &task.tags {
            *tag_counts.entry(tag.clone()).or_default() += 1;
        }
    }
    let total: usize = tag_counts.values().sum();
    let mut entries: Vec<(String, usize)> = tag_counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    entries.truncate(5);
    let palette = [t.accent, t.warn, t.success, t.danger, t.fg_muted];
    let bar_max = (inner.width as usize).saturating_sub(20).max(1);

    let mut lines: Vec<Line> = Vec::new();
    for (i, (tag, count)) in entries.iter().enumerate() {
        let pct = (count * 100).checked_div(total).unwrap_or(0);
        let filled = (bar_max * pct / 100).max(1);
        let empty = bar_max.saturating_sub(filled);
        let color = palette[i % palette.len()];
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<10}", truncate(tag, 10)),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("▰".repeat(filled), Style::default().fg(color)),
            Span::styled("▱".repeat(empty), Style::default().fg(t.border_inactive)),
            Span::styled(format!("  {:>2} ({}%)", count, pct), t.muted()),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_sync(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let panel = BracketPanel::new("sincronización", t).active(false);
    let inner = panel.inner(area);
    panel.render(area, f.buffer_mut());

    let now = crate::clock::now();
    let lines = vec![
        Line::from(vec![
            Span::styled(" ESTADO     ", t.muted()),
            Span::styled(
                "Conectado",
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ÚLTIMA     ", t.muted()),
            Span::styled(
                now.format("%H:%M:%S").to_string(),
                Style::default().fg(t.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" PRÓXIMA    ", t.muted()),
            Span::styled(
                (now + Duration::minutes(5)).format("%H:%M:%S").to_string(),
                Style::default().fg(t.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" DISPOSIT.  ", t.muted()),
            Span::styled(
                "1/1",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" local", t.muted()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("        "),
            Span::styled(
                "⌬",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
    let _ = app;
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_capabilities() {
        let p = AnalyticsPlugin;
        let m = p.manifest();
        assert_eq!(m.id, "builtin.analytics");
        assert!(m
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::PageView)));
        assert!(m
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::QueryAccess(QueryScope::Tasks))));
        assert!(m
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::QueryAccess(QueryScope::FocusSessions))));
    }

    #[test]
    fn handle_returns_empty_result() {
        let mut p = AnalyticsPlugin;
        let ctx = PluginContext::new("builtin.analytics");
        let r = p.handle(PluginAction::Show, &ctx);
        assert!(r.view.is_none());
        assert!(r.follow_up.is_empty());
    }
}
