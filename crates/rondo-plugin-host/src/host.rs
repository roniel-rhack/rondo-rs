use crate::error::HostError;
use crate::manifest::FsManifest;
use rondo_plugin_api::{PluginAction, PluginContext, PluginResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A plugin loaded from disk. Holds the parsed manifest plus the (optional)
/// extism runtime instance — manifest-only registrations are allowed so the
/// host can list plugins before their `.wasm` binary has been built.
pub struct LoadedPlugin {
    pub manifest: FsManifest,
    pub dir: PathBuf,
    plugin: Option<extism::Plugin>,
    pub enabled: bool,
}

impl LoadedPlugin {
    pub fn has_wasm(&self) -> bool {
        self.plugin.is_some()
    }
}

/// Discovers, loads, enables/disables, and dispatches to WASM plugins.
///
/// The host is intentionally lightweight: it owns the extism plugin
/// instances and a `HashMap<String, LoadedPlugin>` keyed by manifest id.
/// The TUI calls `dispatch` once per relevant event; the result vector lets
/// the caller fan-out follow-up actions and overlay views.
pub struct PluginHost {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Scan `dir` for subdirectories containing a `plugin.toml`. Each valid
    /// manifest is registered. Returns the list of plugin ids that loaded
    /// successfully. Errors on individual plugins are swallowed and logged.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<Vec<String>, HostError> {
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let plugin_dir = entry.path();
            let manifest_path = plugin_dir.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }
            match self.load_one(&plugin_dir) {
                Ok(id) => out.push(id),
                Err(e) => tracing::warn!("plugin {} failed to load: {}", plugin_dir.display(), e),
            }
        }
        Ok(out)
    }

    fn load_one(&mut self, dir: &Path) -> Result<String, HostError> {
        let manifest_path = dir.join("plugin.toml");
        let manifest = FsManifest::load(&manifest_path)?;
        if !manifest.api.starts_with("0.1") {
            return Err(HostError::UnsupportedApi(manifest.api.clone()));
        }
        let wasm_path = dir.join("plugin.wasm");
        let extism_plugin = if wasm_path.exists() {
            let wasm = std::fs::read(&wasm_path)?;
            let p = extism::Plugin::new(wasm, [], false)
                .map_err(|e| HostError::Extism(e.to_string()))?;
            Some(p)
        } else {
            tracing::debug!("no wasm for {}: manifest-only registration", manifest.id);
            None
        };
        let id = manifest.id.clone();
        self.plugins.insert(
            id.clone(),
            LoadedPlugin {
                manifest,
                dir: dir.to_path_buf(),
                plugin: extism_plugin,
                enabled: true,
            },
        );
        Ok(id)
    }

    pub fn list(&self) -> Vec<&FsManifest> {
        self.plugins.values().map(|lp| &lp.manifest).collect()
    }

    pub fn get(&self, id: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(id)
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> Result<(), HostError> {
        self.plugins
            .get_mut(id)
            .map(|lp| lp.enabled = on)
            .ok_or_else(|| HostError::NotFound(id.to_string()))
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.plugins.get(id).map(|lp| lp.enabled).unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Dispatch an action to every enabled plugin that has a wasm body.
    /// Each plugin's `handle(action, ctx)` is invoked via the extism
    /// `"handle"` export. Plugins that lack a wasm binary (manifest-only)
    /// are skipped silently. Per-plugin errors are logged and do not
    /// interrupt the dispatch loop.
    ///
    /// Convenience wrapper around [`Self::dispatch_with_store`] for callers
    /// that don't have a [`SqliteStore`] available (or don't expect any
    /// plugin to emit KV follow-ups).
    pub fn dispatch(&mut self, action: &PluginAction) -> Vec<(String, PluginResult)> {
        self.dispatch_with_store(action, None)
    }

    /// Same as [`Self::dispatch`] but threads an optional [`SqliteStore`]
    /// reference so the host can resolve `KvSet` follow-up actions emitted
    /// by plugins. `KvGet` round-trips are not yet supported — the request
    /// is logged and dropped; a future extism host-function will give
    /// plugins a synchronous read path (target: M8.5).
    pub fn dispatch_with_store(
        &mut self,
        action: &PluginAction,
        store: Option<&rondo_core::store::sqlite::SqliteStore>,
    ) -> Vec<(String, PluginResult)> {
        let action_json = match serde_json::to_string(action) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("action serialize failed: {}", e);
                return vec![];
            }
        };
        let mut out = Vec::new();
        for (id, lp) in self.plugins.iter_mut() {
            if !lp.enabled {
                continue;
            }
            let Some(plugin) = lp.plugin.as_mut() else {
                continue;
            };
            let ctx_json = serde_json::to_string(&PluginContext::new(id)).unwrap_or_default();
            let input = format!(r#"{{"action":{},"ctx":{}}}"#, action_json, ctx_json);
            match plugin.call::<&str, &str>("handle", &input) {
                Ok(s) => match serde_json::from_str::<PluginResult>(s) {
                    Ok(r) => {
                        let resolved = resolve_follow_up(id, r, store);
                        out.push((id.clone(), resolved));
                    }
                    Err(e) => {
                        tracing::warn!("plugin {} returned invalid PluginResult: {}", id, e)
                    }
                },
                Err(e) => tracing::warn!("plugin {} call failed: {}", id, e),
            }
        }
        out
    }
}

/// Test/debug helper: run the same follow-up resolution loop that
/// `dispatch_with_store` uses, without going through extism. Useful for
/// integration tests that want to verify the KV-passthrough wiring without
/// loading a real `.wasm` plugin.
pub fn resolve_plugin_follow_up(
    plugin_id: &str,
    result: PluginResult,
    store: Option<&rondo_core::store::sqlite::SqliteStore>,
) -> PluginResult {
    resolve_follow_up(plugin_id, result, store)
}

/// Walks a plugin's `follow_up` vec, persisting `KvSet`s into the host store
/// when one is available, and dropping `KvGet`s (logged). Other variants are
/// passed through untouched so the caller can fan them out.
fn resolve_follow_up(
    plugin_id: &str,
    mut result: PluginResult,
    store: Option<&rondo_core::store::sqlite::SqliteStore>,
) -> PluginResult {
    let drained: Vec<PluginAction> = result.follow_up.drain(..).collect();
    let mut kept: Vec<PluginAction> = Vec::with_capacity(drained.len());
    for fa in drained {
        match fa {
            PluginAction::KvSet { key, value } => {
                if let Some(s) = store {
                    if let Err(e) = s.kv_set(plugin_id, &key, &value) {
                        tracing::warn!("kv_set {}/{} failed: {}", plugin_id, key, e);
                    }
                } else {
                    tracing::debug!(
                        "plugin {} emitted kv_set {} but no store wired",
                        plugin_id,
                        key
                    );
                }
            }
            PluginAction::KvGet { key } => {
                // KvGet from plugin means "I want the value back". The host
                // cannot synchronously answer here — the round-trip needs an
                // extism host-function import, which is a follow-up (M8.5).
                tracing::debug!(
                    "plugin {} requested kv_get {} (round-trip not yet implemented)",
                    plugin_id,
                    key
                );
            }
            other => kept.push(other),
        }
    }
    result.follow_up = kept;
    result
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}
