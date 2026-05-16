use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Duration, NaiveDate};
use rondo_plugin_api::{
    Block, Capability, Plugin, PluginAction, PluginContext, PluginManifest, PluginResult,
    QueryScope, ViewKind, ViewSpec,
};

/// Builtin plugin: renders a focus-session heatmap as a full page.
///
/// Visualizes the last 5 weeks of completed Work sessions (one cell per day,
/// 7 rows × 5 columns) plus the user's current streak. Read-only: the plugin
/// only queries `focus_sessions` via the host's `SqliteStore`. The handle
/// returns a `ViewSpec` with `kind = Page`; the host is responsible for
/// rendering it.
pub struct FocusPagePlugin {
    store: Arc<rondo_core::store::sqlite::SqliteStore>,
    open: bool,
}

impl FocusPagePlugin {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Self {
        Self { store, open: false }
    }
}

impl Plugin for FocusPagePlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.focus-page".into(),
            name: "Focus Heatmap".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![
                Capability::PageView,
                Capability::QueryAccess(QueryScope::FocusSessions),
            ],
            exporter: None,
            syncer: None,
            cli: None,
        }
    }

    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext) -> PluginResult {
        match action {
            PluginAction::Show => self.open = true,
            PluginAction::Hide => {
                self.open = false;
                return PluginResult::default();
            }
            _ => {}
        }
        if !self.open {
            return PluginResult::default();
        }
        let sessions = self.store.list_focus_sessions().unwrap_or_default();
        let streak = self.store.focus_streak().unwrap_or(0);
        let counts = aggregate_by_day(&sessions, 35);
        let blocks = render_heatmap(&counts, streak);
        PluginResult {
            view: Some(ViewSpec {
                kind: ViewKind::Page,
                blocks,
            }),
            follow_up: vec![],
        }
    }
}

/// Bucket completed Work sessions into per-day counts for the last `days`
/// days (UTC), inclusive of today, oldest first.
fn aggregate_by_day(
    sessions: &[rondo_core::domain::focus::Session],
    days: i64,
) -> Vec<(NaiveDate, u32)> {
    let today = chrono::Utc::now().date_naive();
    let mut counts: HashMap<NaiveDate, u32> = HashMap::new();
    for s in sessions {
        let Some(completed) = s.completed_at else {
            continue;
        };
        if !matches!(s.kind, rondo_core::domain::focus::SessionKind::Work) {
            continue;
        }
        let date = completed.date_naive();
        *counts.entry(date).or_default() += 1;
    }
    (0..days)
        .rev()
        .map(|i| {
            let d = today - Duration::days(i);
            (d, counts.get(&d).copied().unwrap_or(0))
        })
        .collect()
}

/// Build the `ViewSpec` blocks for the heatmap. Rows are days of the week
/// (Mon..Sun), columns are weeks ordered oldest→newest.
fn render_heatmap(days: &[(NaiveDate, u32)], streak: u32) -> Vec<Block> {
    let mut blocks = vec![
        Block::Heading {
            text: "Focus Heatmap".into(),
            level: 1,
        },
        Block::Paragraph {
            text: format!(
                "current streak: {} day{}",
                streak,
                if streak == 1 { "" } else { "s" }
            ),
            style: None,
        },
    ];
    let max = days.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
    for dow in 0u32..7 {
        let mut line = String::new();
        for chunk in days.chunks(7) {
            for (date, count) in chunk {
                if date.weekday().num_days_from_monday() == dow {
                    let shade = match (*count as f64 / max as f64 * 4.0).ceil() as u32 {
                        0 => '·',
                        1 => '\u{2582}',
                        2 => '\u{2584}',
                        3 => '\u{2586}',
                        _ => '\u{2588}',
                    };
                    line.push(shade);
                    line.push(' ');
                }
            }
        }
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

    fn store() -> Arc<SqliteStore> {
        let f = tempfile::NamedTempFile::new().unwrap();
        let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
            .join("seed.sql");
        let conn = rusqlite::Connection::open(f.path()).unwrap();
        conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
            .unwrap();
        drop(conn);
        let path = f.path().to_path_buf();
        std::mem::forget(f);
        Arc::new(SqliteStore::open_readwrite(&path).unwrap())
    }

    #[test]
    fn manifest_declares_focus_query_access() {
        let p = FocusPagePlugin::new(store());
        let m = p.manifest();
        assert_eq!(m.id, "builtin.focus-page");
        assert!(m.capabilities.iter().any(|c| matches!(
            c,
            rondo_plugin_api::Capability::QueryAccess(rondo_plugin_api::QueryScope::FocusSessions)
        )));
    }

    #[test]
    fn show_returns_page_view() {
        let mut p = FocusPagePlugin::new(store());
        let ctx = rondo_plugin_api::PluginContext::new("builtin.focus-page");
        let r = p.handle(rondo_plugin_api::PluginAction::Show, &ctx);
        assert!(r.view.is_some());
        assert!(matches!(
            r.view.unwrap().kind,
            rondo_plugin_api::ViewKind::Page
        ));
    }

    #[test]
    fn hide_clears_view() {
        let mut p = FocusPagePlugin::new(store());
        let ctx = rondo_plugin_api::PluginContext::new("builtin.focus-page");
        let _ = p.handle(rondo_plugin_api::PluginAction::Show, &ctx);
        let r = p.handle(rondo_plugin_api::PluginAction::Hide, &ctx);
        assert!(r.view.is_none());
    }
}
