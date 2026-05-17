//! TOML configuration schema for ui/pomodoro/plugins.
//!
//! Loaded from `$RONDO_CONFIG` or `$HOME/.rondo-rs/config.toml`. Missing file
//! returns defaults; malformed file logs a warning and returns defaults.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub pomodoro: PomodoroConfig,
    pub plugins: PluginsConfig,
    pub lang: LangConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub sidebar: bool,
    pub animations: bool,
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

/// Workstream B will rename `work_min`→`work_mins` etc. when wiring config
/// into `ModalsState`. For now the four-field shape matches the plan and the
/// rest of the codebase still reads the legacy `*_min` names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Es,
    En,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LangConfig {
    pub name: Lang,
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
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
}
