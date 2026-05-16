use crate::action::PluginAction;
use crate::capabilities::Capability;
use crate::view::ViewSpec;
use serde::{Deserialize, Serialize};

/// Manifest advertised by every plugin. Owned strings + serde so the same
/// manifest can be loaded from a WASM module's exported metadata or hand-
/// constructed by a builtin in-process plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Semver of `rondo-plugin-api` the plugin was built against. The host
    /// rejects modules with incompatible API versions.
    pub api_version: String,
    pub capabilities: Vec<Capability>,
    pub exporter: Option<ExporterMeta>,
    pub syncer: Option<SyncerMeta>,
    pub cli: Option<CliMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExporterMeta {
    pub format_id: String,
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncerMeta {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliMeta {
    pub name: String,
    /// Simplified clap-like positional/flag specification. The host parses it.
    pub args_spec: Vec<String>,
}

/// Fully owned, serializable context handed to a plugin on every `handle`
/// call. The previous lifetime-parameterised version is gone — the new
/// shape is what travels across the WASM boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub now: chrono::DateTime<chrono::Utc>,
    /// Which plugin is being invoked (mirrors `manifest.id`).
    pub manifest_id: String,
    /// `CARGO_PKG_VERSION` of the host crate.
    pub host_version: String,
}

impl PluginContext {
    /// Convenience constructor. Real host code should build a context with
    /// the actual `manifest_id` of the plugin it's calling.
    pub fn new(manifest_id: impl Into<String>) -> Self {
        Self {
            now: chrono::Utc::now(),
            manifest_id: manifest_id.into(),
            host_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn handle(&mut self, action: PluginAction, ctx: &PluginContext) -> PluginResult;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PluginResult {
    pub view: Option<ViewSpec>,
    /// Host-bound calls emitted by the plugin (`KvGet`, `KvSet`, `Notify`,
    /// `Query`, …). The host drains this queue after every `handle`.
    pub follow_up: Vec<PluginAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_context_round_trips() {
        let ctx = PluginContext::new("test.plugin");
        let json = serde_json::to_string(&ctx).unwrap();
        let back: PluginContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.manifest_id, "test.plugin");
        assert_eq!(back.host_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn manifest_round_trips() {
        let m = PluginManifest {
            id: "x".into(),
            name: "X".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![Capability::OverlayView],
            exporter: Some(ExporterMeta {
                format_id: "md".into(),
                mime: "text/markdown".into(),
            }),
            syncer: None,
            cli: Some(CliMeta {
                name: "x".into(),
                args_spec: vec!["--flag".into()],
            }),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, m.id);
        assert_eq!(back.capabilities.len(), 1);
        assert_eq!(back.exporter.unwrap().format_id, "md");
    }
}
