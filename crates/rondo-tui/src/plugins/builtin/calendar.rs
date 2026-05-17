use chrono::{Datelike, Duration, Local, NaiveDate};
use rondo_plugin_api::{
    action::PluginAction,
    capabilities::{Capability, QueryScope},
    plugin::{Plugin, PluginContext, PluginManifest, PluginResult},
    view::{Block, ColorToken, Span as PSpan, TextStyle, ViewKind, ViewSpec},
};
use std::collections::HashSet;
use std::sync::Arc;

/// Builtin mini-calendar plugin. Renders a month grid with a dot on each day
/// that has at least one journal note. Reads the store directly because it is
/// an in-process trusted builtin; future external plugins will receive the
/// same data via the `Query` channel declared by `QueryAccess(Journal)`.
pub struct CalendarPlugin {
    store: Arc<rondo_core::store::sqlite::SqliteStore>,
    cursor: NaiveDate,
    open: bool,
}

impl CalendarPlugin {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Self {
        let today = Local::now().date_naive();
        Self {
            store,
            cursor: today,
            open: false,
        }
    }
}

impl Plugin for CalendarPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.calendar".into(),
            name: "Calendar".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![
                Capability::PageView,
                Capability::QueryAccess(QueryScope::Journal),
            ],
            exporter: None,
            syncer: None,
            cli: None,
        }
    }

    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext) -> PluginResult {
        match action {
            PluginAction::Show => {
                self.open = true;
            }
            PluginAction::Hide => {
                self.open = false;
                return PluginResult::default();
            }
            PluginAction::KeyPress { ref key } => {
                self.handle_key(key);
            }
            _ => {}
        }
        if !self.open {
            return PluginResult::default();
        }
        let dates: HashSet<NaiveDate> = self
            .store
            .list_journal_notes()
            .map(|notes| notes.into_iter().map(|n| n.date).collect())
            .unwrap_or_default();
        let mut blocks = render_month(self.cursor, &dates);

        // Preview of entries for cursor day (if any).
        let preview_note = self
            .store
            .list_all_journal_notes_including_hidden()
            .map(|notes| notes.into_iter().find(|n| n.date == self.cursor))
            .ok()
            .flatten();
        blocks.push(Block::Divider);
        let date_label = self.cursor.format("%A, %B %-d, %Y").to_string();
        blocks.push(Block::Spans(vec![
            PSpan {
                text: format!("▌ {}", date_label),
                style: Some(TextStyle {
                    fg: Some(ColorToken::Accent),
                    bold: true,
                    ..Default::default()
                }),
            },
        ]));
        if let Some(note) = preview_note {
            if let Ok(entries) = self.store.entries_for_note(note.id) {
                if entries.is_empty() {
                    blocks.push(Block::Paragraph {
                        text: "  (sin entradas)".to_string(),
                        style: Some(TextStyle {
                            fg: Some(ColorToken::Muted),
                            ..Default::default()
                        }),
                    });
                } else {
                    for entry in entries.iter().take(5) {
                        let time = entry
                            .created_at
                            .with_timezone(&Local)
                            .format("%H:%M")
                            .to_string();
                        let first_line = entry.body.lines().next().unwrap_or("").to_string();
                        blocks.push(Block::Spans(vec![
                            PSpan {
                                text: format!("  ◷ {}  ", time),
                                style: Some(TextStyle {
                                    fg: Some(ColorToken::Accent),
                                    bold: true,
                                    ..Default::default()
                                }),
                            },
                            PSpan {
                                text: first_line,
                                style: Some(TextStyle {
                                    fg: Some(ColorToken::Foreground),
                                    ..Default::default()
                                }),
                            },
                        ]));
                    }
                    if entries.len() > 5 {
                        blocks.push(Block::Paragraph {
                            text: format!("  … +{} entradas más", entries.len() - 5),
                            style: Some(TextStyle {
                                fg: Some(ColorToken::Muted),
                                ..Default::default()
                            }),
                        });
                    }
                }
            }
        } else {
            blocks.push(Block::Paragraph {
                text: "  (sin notas en este día)".to_string(),
                style: Some(TextStyle {
                    fg: Some(ColorToken::Muted),
                    ..Default::default()
                }),
            });
        }
        blocks.push(Block::Paragraph {
            text: "h/l día · j/k semana · J/K mes · t hoy".to_string(),
            style: Some(TextStyle {
                fg: Some(ColorToken::Muted),
                ..Default::default()
            }),
        });
        PluginResult {
            view: Some(ViewSpec {
                kind: ViewKind::Page,
                blocks,
            }),
            follow_up: vec![],
        }
    }
}

impl CalendarPlugin {
    fn handle_key(&mut self, key: &str) {
        let delta_days = match key {
            "h" | "Left" => -1,
            "l" | "Right" => 1,
            "j" | "Down" => 7,
            "k" | "Up" => -7,
            "J" => {
                self.cursor = shift_month(self.cursor, 1);
                return;
            }
            "K" => {
                self.cursor = shift_month(self.cursor, -1);
                return;
            }
            "t" => {
                self.cursor = Local::now().date_naive();
                return;
            }
            _ => return,
        };
        if let Some(d) = self.cursor.checked_add_signed(Duration::days(delta_days)) {
            self.cursor = d;
        }
    }
}

fn shift_month(d: NaiveDate, delta: i32) -> NaiveDate {
    let year = d.year();
    let month0 = d.month() as i32 - 1 + delta;
    let new_year = year + month0.div_euclid(12);
    let new_month = month0.rem_euclid(12) as u32 + 1;
    let day = d.day();
    NaiveDate::from_ymd_opt(new_year, new_month, day)
        .or_else(|| NaiveDate::from_ymd_opt(new_year, new_month, 28))
        .unwrap_or(d)
}

fn render_month(cursor: NaiveDate, dates: &HashSet<NaiveDate>) -> Vec<Block> {
    let first = cursor
        .with_day(1)
        .expect("day 1 is valid for any month");
    let month_name = first.format("%B %Y").to_string();
    let mut blocks = vec![
        Block::Heading {
            text: month_name,
            level: 1,
        },
        Block::Paragraph {
            text: "Mo Tu We Th Fr Sa Su".into(),
            style: None,
        },
    ];
    let first_weekday = first.weekday().num_days_from_monday();
    let today = Local::now().date_naive();
    let mut row_spans: Vec<PSpan> = Vec::new();
    for _ in 0..first_weekday {
        row_spans.push(PSpan {
            text: "    ".into(),
            style: None,
        });
    }
    let mut day: u32 = 1;
    while let Some(date) = first.with_day(day) {
        if date.month() != first.month() {
            break;
        }
        let has_entry = dates.contains(&date);
        let is_cursor = date == cursor;
        let is_today = date == today;
        let marker = if has_entry { "●" } else { " " };
        let cell = format!("{:>2}{}", day, marker);
        let style = if is_cursor {
            Some(TextStyle {
                fg: Some(ColorToken::Background),
                bg: Some(ColorToken::Accent),
                bold: true,
                ..Default::default()
            })
        } else if is_today {
            Some(TextStyle {
                fg: Some(ColorToken::Accent),
                bold: true,
                ..Default::default()
            })
        } else if has_entry {
            Some(TextStyle {
                fg: Some(ColorToken::Foreground),
                ..Default::default()
            })
        } else {
            Some(TextStyle {
                fg: Some(ColorToken::Muted),
                ..Default::default()
            })
        };
        row_spans.push(PSpan { text: cell, style });
        row_spans.push(PSpan {
            text: " ".into(),
            style: None,
        });
        if date.weekday().num_days_from_monday() == 6 {
            blocks.push(Block::Spans(std::mem::take(&mut row_spans)));
        }
        day += 1;
    }
    if !row_spans.is_empty() {
        blocks.push(Block::Spans(row_spans));
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_core::store::sqlite::SqliteStore;

    fn fixture_store() -> Arc<SqliteStore> {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let seed = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("fixtures/seed.sql"),
        )
        .unwrap();
        let path = tmp.path().to_path_buf();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(&seed).unwrap();
        drop(conn);
        std::mem::forget(tmp);
        Arc::new(SqliteStore::open_readonly(&path).unwrap())
    }

    #[test]
    fn manifest_declares_journal_query_access() {
        let p = CalendarPlugin::new(fixture_store());
        let m = p.manifest();
        assert_eq!(m.id, "builtin.calendar");
        assert!(m.capabilities.contains(&Capability::PageView));
        assert!(m
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::QueryAccess(QueryScope::Journal))));
        assert!(m.exporter.is_none());
    }

    #[test]
    fn show_returns_page_view() {
        let mut p = CalendarPlugin::new(fixture_store());
        let ctx = PluginContext::new("builtin.calendar");
        let r = p.handle(PluginAction::Show, &ctx);
        let v = r.view.expect("Show should return a view");
        assert!(matches!(v.kind, ViewKind::Page));
        assert!(
            v.blocks
                .iter()
                .any(|b| matches!(b, Block::Heading { .. })),
            "calendar page should start with a month heading"
        );
    }

    #[test]
    fn hide_returns_empty() {
        let mut p = CalendarPlugin::new(fixture_store());
        let ctx = PluginContext::new("builtin.calendar");
        p.handle(PluginAction::Show, &ctx);
        let r = p.handle(PluginAction::Hide, &ctx);
        assert!(r.view.is_none());
    }

    #[test]
    fn render_month_marks_days_with_entries() {
        let cursor = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let mut dates = HashSet::new();
        dates.insert(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
        let blocks = render_month(cursor, &dates);
        let mut joined = String::new();
        for b in &blocks {
            match b {
                Block::Heading { text, .. } | Block::Paragraph { text, .. } => {
                    joined.push_str(text);
                    joined.push('\n');
                }
                Block::Spans(parts) => {
                    for p in parts {
                        joined.push_str(&p.text);
                    }
                    joined.push('\n');
                }
                _ => {}
            }
        }
        assert!(joined.contains("January 2025"));
        // The 10th has a dot marker; with right-aligned width 2 + marker
        // the format is "10●".
        assert!(joined.contains("10●"), "joined was: {}", joined);
    }

    #[test]
    fn keypress_moves_cursor() {
        let mut p = CalendarPlugin::new(fixture_store());
        let ctx = PluginContext::new("builtin.calendar");
        p.handle(PluginAction::Show, &ctx);
        let before = p.cursor;
        p.handle(
            PluginAction::KeyPress {
                key: "l".to_string(),
            },
            &ctx,
        );
        assert_eq!(p.cursor.signed_duration_since(before).num_days(), 1);
    }
}
