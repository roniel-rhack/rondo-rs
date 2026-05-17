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

pub(crate) struct Suggestion {
    pub(crate) cmd: &'static str,
    pub(crate) desc: &'static str,
}

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
        cmd: "theme",
        desc: "switch theme: theme dark|light|high-contrast",
    },
    Suggestion {
        cmd: "group",
        desc: "group task list: group priority|status|due|none",
    },
    Suggestion {
        cmd: "lang",
        desc: "switch UI language",
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

/// Rank suggestions by fuzzy match against the user buffer (E5). Empty
/// buffer returns the full list in insertion order; otherwise we keep
/// only matches and sort by descending `nucleo` score.
pub(crate) fn filter_suggestions(buf: &str) -> Vec<&'static Suggestion> {
    let q = buf.trim();
    if q.is_empty() {
        return ALL.iter().collect();
    }
    let mut engine = crate::search::SearchEngine::new();
    let mut scored: Vec<(u16, &'static Suggestion)> = ALL
        .iter()
        .filter_map(|s| {
            let hay = format!("{} {}", s.cmd, s.desc);
            engine.score_only(q, &hay).map(|sc| (sc, s))
        })
        .collect();
    scored.sort_by_key(|x| std::cmp::Reverse(x.0));
    scored.into_iter().map(|(_, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_all() {
        assert_eq!(filter_suggestions("").len(), ALL.len());
    }

    #[test]
    fn fuzzy_filters_matches() {
        let out = filter_suggestions("plug");
        assert!(out.iter().any(|s| s.cmd == "plugins"));
    }

    #[test]
    fn description_terms_match_too() {
        // "page" only appears in descriptions, not cmd names.
        let out = filter_suggestions("page");
        assert!(out.iter().any(|s| s.cmd == "tasks"));
    }

    #[test]
    fn nonsense_returns_empty() {
        let out = filter_suggestions("xyzqzzzzz");
        assert!(out.is_empty());
    }
}
