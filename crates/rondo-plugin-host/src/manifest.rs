use crate::error::HostError;
use rondo_plugin_api::{Capability, CliMeta, ExporterMeta, SyncerMeta};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// On-disk plugin manifest (`plugin.toml`).
///
/// Mirrors [`rondo_plugin_api::PluginManifest`] but lives in the host crate
/// because we want to keep TOML parsing out of the plugin SDK and because
/// some field names (`api` vs `api_version`) are more ergonomic from a
/// hand-edited TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsManifest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub version: String,
    /// Semver of `rondo-plugin-api` the plugin was built against. The host
    /// only accepts the `0.1.x` series for now.
    pub api: String,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub wasi: Wasi,
    #[serde(default)]
    pub exporter: Option<ExporterMeta>,
    #[serde(default)]
    pub syncer: Option<SyncerMeta>,
    #[serde(default)]
    pub cli: Option<CliMeta>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wasi {
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

impl FsManifest {
    pub fn load(path: &Path) -> Result<Self, HostError> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_manifest_parses() {
        let raw = r#"
id = "quote-of-the-day"
version = "0.1.0"
api = "0.1"
capabilities = ["OverlayView", "TickHandler"]
"#;
        let m: FsManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.id, "quote-of-the-day");
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn scoped_capability_parses() {
        let raw = r#"
id = "x"
version = "0.1.0"
api = "0.1"
capabilities = [{ QueryAccess = "Tasks" }, "OverlayView"]
"#;
        let m: FsManifest = toml::from_str(raw).unwrap();
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn wasi_defaults_to_empty() {
        let raw = r#"
id = "x"
version = "0.1.0"
api = "0.1"
capabilities = []
"#;
        let m: FsManifest = toml::from_str(raw).unwrap();
        assert!(m.wasi.allowed_paths.is_empty());
        assert!(m.wasi.allowed_hosts.is_empty());
    }
}
