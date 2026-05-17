//! TOML configuration schema for ui/pomodoro/plugins.
//!
//! Loaded from `$RONDO_CONFIG` or `$HOME/.rondo-rs/config.toml`. Missing file
//! returns defaults; malformed file logs a warning and returns defaults.
//!
//! ## Language selection
//!
//! The active UI language is stored as a string in `[ui].language` (default
//! `"en"`). The matching `~/.rondo-rs/lang/<language>.toml` pack is loaded at
//! startup. The legacy `[lang]` table from v0.x configs is silently ignored —
//! the default is now `"en"`, not Spanish; users who want Spanish must
//! install the `es` pack and set `[ui].language = "es"` (or pick it from the
//! TUI `:lang` palette command, which writes the file for them).
//!
//! ## Saving
//!
//! [`Config::save`] re-serialises the entire struct with `toml::to_string_pretty`.
//! Comments and field ordering from a hand-edited `config.toml` are lost on
//! save — acceptable trade-off because the only writer is the `:lang` modal,
//! which the user explicitly invokes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub pomodoro: PomodoroConfig,
    pub plugins: PluginsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub sidebar: bool,
    pub animations: bool,
    /// Code of the active language pack (e.g. `"en"`, `"es"`, `"pt-br"`).
    /// Resolved against `~/.rondo-rs/lang/<language>.toml`; built-in `"en"`
    /// is always available.
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroConfig {
    pub work_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    /// Number of work cycles before a long break is offered. Defaults to 4.
    pub cycles_per_long: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginsConfig {
    pub enabled: Vec<String>,
    pub permissions: HashMap<String, Vec<String>>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            sidebar: true,
            animations: true,
            language: default_language(),
        }
    }
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            cycles_per_long: 4,
        }
    }
}

fn default_language() -> String {
    "en".to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("toml-ser: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn load_or_default(path: &Path) -> Self {
        match Self::load(path) {
            Ok(c) => c,
            Err(ConfigError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!("config load failed, using defaults: {}", e);
                Self::default()
            }
        }
    }

    pub fn default_path() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".rondo-rs")
            .join("config.toml")
    }

    pub fn from_env_or_default() -> Self {
        let path = std::env::var("RONDO_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::default_path());
        Self::load_or_default(&path)
    }

    /// Persist the current config to `path`, creating parent directories as
    /// needed. Loses user comments and field ordering — see module-level
    /// docs.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let body = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_language_is_english() {
        let c = Config::default();
        assert_eq!(c.ui.language, "en");
    }

    #[test]
    fn legacy_lang_table_is_ignored() {
        // v0.x configs may carry `[lang] name = "es"`. With the schema change
        // it must parse without error and fall back to the new default.
        let src = "[lang]\nname = \"es\"\n";
        let cfg: Config = toml::from_str(src).expect("legacy lang must not break parse");
        assert_eq!(cfg.ui.language, "en");
    }

    #[test]
    fn save_round_trips_language() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.ui.language = "es".to_string();
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.ui.language, "es");
    }
}
