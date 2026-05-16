use std::collections::HashSet;
use std::sync::Arc;

use rondo_plugin_api::{
    action::PluginAction,
    capabilities::{Capability, QueryScope},
    plugin::{Plugin, PluginContext, PluginManifest, PluginResult},
    view::{Block, ViewKind, ViewSpec},
};

/// Builtin plugin that renders an ASCII tree of the focused task and its
/// transitive `blocked_by` chain. Advertises `PageView + QueryAccess(Deps)`.
///
/// The plugin does not have access to `AppState`; the host pokes
/// [`Self::focus_task_id`] before dispatching `Show`. When unset, the plugin
/// falls back to the first task returned by the store.
pub struct DepGraphPlugin {
    store: Arc<rondo_core::store::sqlite::SqliteStore>,
    pub focus_task_id: Option<i64>,
    open: bool,
}

impl DepGraphPlugin {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Self {
        Self {
            store,
            focus_task_id: None,
            open: false,
        }
    }

    fn root_id(&self) -> Option<i64> {
        if let Some(id) = self.focus_task_id {
            return Some(id);
        }
        self.store.list_tasks().ok().and_then(|ts| ts.first().map(|t| t.id))
    }

    fn render_graph(&self, root: i64) -> Vec<Block> {
        let mut out = vec![Block::Heading {
            text: "Dependencies".into(),
            level: 1,
        }];
        let mut visited = HashSet::new();
        self.walk(root, 0, &mut out, &mut visited);
        out
    }

    fn walk(
        &self,
        id: i64,
        depth: usize,
        out: &mut Vec<Block>,
        visited: &mut HashSet<i64>,
    ) {
        let indent = "  ".repeat(depth);
        if !visited.insert(id) {
            out.push(Block::Paragraph {
                text: format!("{indent}↳ [#{id}] (cycle)"),
                style: None,
            });
            return;
        }
        let task = match self.store.task_by_id(id) {
            Ok(t) => t,
            Err(_) => {
                out.push(Block::Paragraph {
                    text: format!("{indent}↳ [#{id}] (missing)"),
                    style: None,
                });
                return;
            }
        };
        let prefix = if depth == 0 { "" } else { "↳ " };
        out.push(Block::Paragraph {
            text: format!("{indent}{prefix}#{} {}", task.id, task.title),
            style: None,
        });
        for bid in task.blocked_by_ids {
            self.walk(bid, depth + 1, out, visited);
        }
    }
}

impl Plugin for DepGraphPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.dep-graph".into(),
            name: "Dependency Graph".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![
                Capability::PageView,
                Capability::QueryAccess(QueryScope::Deps),
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
        let blocks = match self.root_id() {
            Some(id) => self.render_graph(id),
            None => vec![
                Block::Heading {
                    text: "Dependencies".into(),
                    level: 1,
                },
                Block::Paragraph {
                    text: "(no task)".into(),
                    style: None,
                },
            ],
        };
        PluginResult {
            view: Some(ViewSpec {
                kind: ViewKind::Page,
                blocks,
            }),
            follow_up: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_core::domain::task::NewTask;
    use rondo_core::store::sqlite::SqliteStore;

    fn store_with_dep() -> (Arc<SqliteStore>, i64, i64) {
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
        let s = SqliteStore::open_readwrite(f.path()).unwrap();
        let (a, _) = s.create_task(NewTask::quick("Parent")).unwrap();
        let (b, _) = s.create_task(NewTask::quick("Dep")).unwrap();
        s.add_dependency(a, b).unwrap();
        // Keep the tempfile alive for the rest of the test.
        std::mem::forget(f);
        (Arc::new(s), a, b)
    }

    #[test]
    fn manifest_declares_deps_query_access() {
        let (s, _, _) = store_with_dep();
        let p = DepGraphPlugin::new(s);
        let m = p.manifest();
        assert_eq!(m.id, "builtin.dep-graph");
        assert!(m
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::PageView)));
        assert!(m.capabilities.iter().any(|c| matches!(
            c,
            Capability::QueryAccess(QueryScope::Deps)
        )));
    }

    #[test]
    fn show_returns_page_view() {
        let (s, _, _) = store_with_dep();
        let mut p = DepGraphPlugin::new(s);
        let ctx = PluginContext::new("builtin.dep-graph");
        let r = p.handle(PluginAction::Show, &ctx);
        let v = r.view.expect("show should produce a view");
        assert!(matches!(v.kind, ViewKind::Page));
        assert!(matches!(v.blocks.first(), Some(Block::Heading { .. })));
    }

    #[test]
    fn hide_clears_view() {
        let (s, _, _) = store_with_dep();
        let mut p = DepGraphPlugin::new(s);
        let ctx = PluginContext::new("builtin.dep-graph");
        let _ = p.handle(PluginAction::Show, &ctx);
        let r = p.handle(PluginAction::Hide, &ctx);
        assert!(r.view.is_none());
    }

    #[test]
    fn focus_task_renders_blocked_by_chain() {
        let (s, parent, dep) = store_with_dep();
        let mut p = DepGraphPlugin::new(s);
        p.focus_task_id = Some(parent);
        let ctx = PluginContext::new("builtin.dep-graph");
        let r = p.handle(PluginAction::Show, &ctx);
        let v = r.view.unwrap();
        let texts: Vec<String> = v
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains(&format!("#{parent}"))),
            "missing parent in {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains(&format!("#{dep}"))),
            "missing dep in {texts:?}"
        );
    }
}
