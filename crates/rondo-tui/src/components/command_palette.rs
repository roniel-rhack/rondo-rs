use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(" : command ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " › ",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(app.modals.command_buf.clone(), Style::default().fg(t.fg)),
            Span::styled(
                "▏",
                Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK),
            ),
        ])),
        chunks[0],
    );

    let suggestions = filter_suggestions(app, &app.modals.command_buf);
    let items: Vec<ListItem> = suggestions
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {:<18} ", s.cmd),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(s.desc.clone(), Style::default().fg(t.fg_muted)),
            ]))
        })
        .collect();
    f.render_widget(List::new(items), chunks[1]);
}

struct Suggestion {
    cmd: String,
    desc: String,
}

fn builtin_suggestions() -> Vec<Suggestion> {
    [
        ("tasks", "switch to Tasks page"),
        ("journal", "switch to Journal page"),
        ("pomodoro", "start focus session overlay"),
        ("plugins", "list installed plugins + capabilities"),
        (
            "calendar",
            "open calendar plugin (journal-driven mini-month)",
        ),
        ("focus", "open focus heatmap plugin (5w×7d)"),
        ("deps", "open dependency graph plugin"),
        ("analytics", "open analytics dashboard plugin"),
        ("help", "open key-bindings reference"),
        ("quit", "exit rondo-tui"),
    ]
    .into_iter()
    .map(|(cmd, desc)| Suggestion {
        cmd: cmd.to_string(),
        desc: desc.to_string(),
    })
    .collect()
}

/// Build the suggestion list: built-in commands plus one entry per
/// external WASM plugin that declared `[cli].name`. The external entries
/// show the plugin id in their description so users can tell duplicates
/// apart when a plugin shadows a builtin command name.
fn all_suggestions(app: &AppState) -> Vec<Suggestion> {
    let mut out = builtin_suggestions();
    for m in app.external.manifests() {
        if let Some(name) = m.command_name() {
            out.push(Suggestion {
                cmd: name.to_string(),
                desc: format!("(plugin: {})", m.id),
            });
        }
    }
    // Also surface in-process plugins that declared a CLI command name
    // (none today, but the hook is here for builtins to opt in).
    for m in app.plugins.iter_manifests() {
        if let Some(cli) = m.cli.as_ref() {
            out.push(Suggestion {
                cmd: cli.name.clone(),
                desc: format!("(plugin: {})", m.id),
            });
        }
    }
    out
}

fn filter_suggestions(app: &AppState, buf: &str) -> Vec<Suggestion> {
    let all = all_suggestions(app);
    let q = buf.trim();
    if q.is_empty() {
        return all;
    }
    all.into_iter().filter(|s| s.cmd.starts_with(q)).collect()
}
