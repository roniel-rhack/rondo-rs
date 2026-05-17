use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use rondo_core::i18n;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(
            i18n::t("command_palette.title"),
            t.accent_style(),
        ));
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
                Span::styled(i18n::t(s.desc_key), Style::default().fg(t.fg_muted)),
            ]))
        })
        .collect();
    f.render_widget(List::new(items), chunks[1]);
}

pub(crate) struct Suggestion {
    pub(crate) cmd: &'static str,
    /// i18n key for the description shown next to `cmd` in the palette.
    pub(crate) desc_key: &'static str,
}

static ALL: &[Suggestion] = &[
    Suggestion {
        cmd: "tasks",
        desc_key: "command_palette.suggestion.tasks",
    },
    Suggestion {
        cmd: "journal",
        desc_key: "command_palette.suggestion.journal",
    },
    Suggestion {
        cmd: "pomodoro",
        desc_key: "command_palette.suggestion.pomodoro",
    },
    Suggestion {
        cmd: "plugins",
        desc_key: "command_palette.suggestion.plugins",
    },
    Suggestion {
        cmd: "calendar",
        desc_key: "command_palette.suggestion.calendar",
    },
    Suggestion {
        cmd: "focus",
        desc_key: "command_palette.suggestion.focus",
    },
    Suggestion {
        cmd: "deps",
        desc_key: "command_palette.suggestion.deps",
    },
    Suggestion {
        cmd: "analytics",
        desc_key: "command_palette.suggestion.analytics",
    },
    Suggestion {
        cmd: "theme",
        desc_key: "command_palette.suggestion.theme",
    },
    Suggestion {
        cmd: "group",
        desc_key: "command_palette.suggestion.group",
    },
    Suggestion {
        cmd: "lang",
        desc_key: "command_palette.suggestion.lang",
    },
    Suggestion {
        cmd: "help",
        desc_key: "command_palette.suggestion.help",
    },
    Suggestion {
        cmd: "quit",
        desc_key: "command_palette.suggestion.quit",
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
            let hay = format!("{} {}", s.cmd, i18n::t(s.desc_key));
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
