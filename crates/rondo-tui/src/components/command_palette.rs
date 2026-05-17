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

    let suggestions = filter_suggestions(&app.modals.command_buf);
    let items: Vec<ListItem> = suggestions
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {:<10} ", s.cmd),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(s.desc, Style::default().fg(t.fg_muted)),
            ]))
        })
        .collect();
    f.render_widget(List::new(items), chunks[1]);
}

struct Suggestion {
    cmd: &'static str,
    desc: &'static str,
}

fn filter_suggestions(buf: &str) -> Vec<&'static Suggestion> {
    static ALL: &[Suggestion] = &[
        Suggestion {
            cmd: "tasks",
            desc: "switch to Tasks page",
        },
        Suggestion {
            cmd: "journal",
            desc: "switch to Journal page",
        },
        Suggestion {
            cmd: "pomodoro",
            desc: "start focus session overlay",
        },
        Suggestion {
            cmd: "plugins",
            desc: "list installed plugins + capabilities",
        },
        Suggestion {
            cmd: "calendar",
            desc: "open calendar plugin (journal-driven mini-month)",
        },
        Suggestion {
            cmd: "focus",
            desc: "open focus heatmap plugin (5w×7d)",
        },
        Suggestion {
            cmd: "deps",
            desc: "open dependency graph plugin",
        },
        Suggestion {
            cmd: "analytics",
            desc: "open analytics dashboard plugin",
        },
        Suggestion {
            cmd: "help",
            desc: "open key-bindings reference",
        },
        Suggestion {
            cmd: "quit",
            desc: "exit rondo-tui",
        },
    ];
    let q = buf.trim();
    if q.is_empty() {
        return ALL.iter().collect();
    }
    ALL.iter().filter(|s| s.cmd.starts_with(q)).collect()
}
