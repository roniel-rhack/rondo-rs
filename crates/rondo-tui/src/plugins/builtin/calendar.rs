use chrono::{Datelike, NaiveDate, Utc};
use rondo_plugin_api::{
    action::PluginAction,
    capabilities::{Capability, QueryScope},
    plugin::{Plugin, PluginContext, PluginManifest, PluginResult},
    view::{Block, ViewKind, ViewSpec},
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
        let today = Utc::now().date_naive();
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
        let blocks = render_month(self.cursor, &dates);
        PluginResult {
            view: Some(ViewSpec {
                kind: ViewKind::Page,
                blocks,
            }),
            follow_up: vec![],
        }
    }
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
    let mut line = String::new();
    for _ in 0..first_weekday {
        line.push_str("    ");
    }
    let mut day: u32 = 1;
    while let Some(date) = first.with_day(day) {
        if date.month() != first.month() {
            break;
        }
        let marker = if dates.contains(&date) { "●" } else { " " };
        line.push_str(&format!("{:2}{} ", day, marker));
        if date.weekday().num_days_from_monday() == 6 {
            blocks.push(Block::Paragraph {
                text: line.clone(),
                style: None,
            });
            line.clear();
        }
        day += 1;
    }
    if !line.is_empty() {
        blocks.push(Block::Paragraph {
            text: line,
            style: None,
        });
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
        let joined = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { text, .. } => Some(text.as_str()),
                Block::Heading { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("January 2025"));
        assert!(joined.contains("10●"));
    }
}
