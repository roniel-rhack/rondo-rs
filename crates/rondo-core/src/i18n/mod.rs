//! User-installable language packs.
//!
//! English is baked into the binary via `include_str!("en.toml")` and is the
//! only source of truth for string keys — every key referenced from `t()` /
//! `tf()` MUST exist in `en.toml`. External packs live as plain TOML files
//! under `~/.rondo-rs/lang/<code>.toml`; the binary reads them at startup and
//! installs them via the `lang install` CLI subcommand.
//!
//! Missing keys never panic: `t()` falls back to the English baseline and, if
//! the key is missing there too, returns the key verbatim plus a `tracing`
//! warning. This keeps the UI alive even when a translator forgets an entry.
//!
//! Hot-swapping a pack at runtime works because the active `Translations` is
//! held behind `arc_swap::ArcSwap`. The `:lang` palette command calls
//! [`set_active`] and the next render frame redraws against the new map.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use arc_swap::ArcSwap;
use serde::Deserialize;

/// In-memory representation of a parsed language pack.
#[derive(Debug, Clone)]
pub struct Translations {
    pub code: String,
    pub name: String,
    pub map: HashMap<String, String>,
}

/// Raw TOML shape used for both the baked-in baseline and external packs.
#[derive(Debug, Deserialize)]
pub struct PackFile {
    pub meta: PackMeta,
    #[serde(default)]
    pub strings: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PackMeta {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid code `{0}` (must match [a-z][a-z0-9_-]*)")]
    InvalidCode(String),
}

/// English baseline, embedded at compile time. Every key in the codebase must
/// be present here.
pub const EN_TOML: &str = include_str!("en.toml");

static BASELINE: OnceLock<Translations> = OnceLock::new();
static ACTIVE: OnceLock<ArcSwap<Translations>> = OnceLock::new();

fn baseline() -> &'static Translations {
    BASELINE.get_or_init(|| {
        parse_pack(EN_TOML).expect("baked-in en.toml must parse; this is a build-time invariant")
    })
}

fn active_handle() -> &'static ArcSwap<Translations> {
    ACTIVE.get_or_init(|| ArcSwap::from_pointee(baseline().clone()))
}

/// Replace the active pack. Subsequent `t()` / `tf()` calls see the new map.
pub fn set_active(t: Translations) {
    let handle = active_handle();
    handle.store(std::sync::Arc::new(t));
}

/// Read-only view of the currently active pack code (e.g. `"en"`, `"es"`).
pub fn active_code() -> String {
    active_handle().load().code.clone()
}

/// Resolve a key in the active pack. Falls back to the English baseline, then
/// to the key itself with a `tracing::warn!` if both miss.
pub fn t(key: &str) -> String {
    let act = active_handle().load();
    if let Some(v) = act.map.get(key) {
        return v.clone();
    }
    if let Some(v) = baseline().map.get(key) {
        return v.clone();
    }
    tracing::warn!("missing i18n key: {}", key);
    key.to_string()
}

/// Resolve a key and interpolate `{name}` placeholders. Replacement is naïve
/// string substitution — sufficient for v1; ICU pluralisation lives in a
/// future PR layered behind this same call site.
pub fn tf(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(key);
    for (k, v) in args {
        let needle = format!("{{{}}}", k);
        if s.contains(&needle) {
            s = s.replace(&needle, v);
        }
    }
    s
}

/// Parse a pack from a TOML string. Used by both the baseline initialiser and
/// the `lang install` CLI helper.
pub fn parse_pack(src: &str) -> Result<Translations, I18nError> {
    let file: PackFile = toml::from_str(src)?;
    if !is_valid_code(&file.meta.code) {
        return Err(I18nError::InvalidCode(file.meta.code));
    }
    Ok(Translations {
        code: file.meta.code,
        name: file.meta.name,
        map: file.strings,
    })
}

/// Load a pack from a path. Used by `main.rs` at startup and by the palette
/// `:lang` modal when the user picks a non-built-in entry.
pub fn load_pack(path: &Path) -> Result<Translations, I18nError> {
    let raw = std::fs::read_to_string(path)?;
    parse_pack(&raw)
}

/// Built-in English pack. Used as the runtime default and as the seed for the
/// `lang scaffold` CLI.
pub fn builtin_en() -> Translations {
    baseline().clone()
}

/// Strict regex for pack codes: lowercase letter followed by lowercase
/// alphanum / `_` / `-`. Used by the CLI to reject `../etc/passwd` style
/// inputs before they hit the filesystem.
pub fn is_valid_code(code: &str) -> bool {
    let mut chars = code.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Default location for external packs under the user's confinement root.
pub fn default_lang_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join(".rondo-rs")
        .join("lang")
}

/// Resolve the on-disk path for a given pack code (no existence check).
pub fn pack_path(code: &str) -> std::path::PathBuf {
    default_lang_dir().join(format!("{code}.toml"))
}

/// Test-only entry point. Forces the active pack to the baked English baseline
/// regardless of host config — keeps insta snapshots locale-stable.
pub fn force_for_tests() {
    set_active(baseline().clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_parses_and_has_keys() {
        let b = baseline();
        assert_eq!(b.code, "en");
        assert!(b.map.contains_key("modal.lang_picker.title"));
    }

    #[test]
    fn t_falls_back_to_key_on_missing() {
        let out = t("does.not.exist.key");
        assert_eq!(out, "does.not.exist.key");
    }

    #[test]
    fn tf_interpolates_placeholders() {
        let out = tf("cli.lang.installed", &[("code", "es"), ("path", "/tmp/x")]);
        assert_eq!(out, "installed es pack at /tmp/x");
    }

    #[test]
    fn valid_code_regex() {
        assert!(is_valid_code("en"));
        assert!(is_valid_code("es"));
        assert!(is_valid_code("pt-br"));
        assert!(is_valid_code("zh_hans"));
        assert!(!is_valid_code(""));
        assert!(!is_valid_code("EN"));
        assert!(!is_valid_code("-en"));
        assert!(!is_valid_code("../etc"));
        assert!(!is_valid_code("en/passwd"));
    }

    #[test]
    fn parse_rejects_invalid_code() {
        let src = "[meta]\ncode = \"../etc\"\nname = \"x\"\n[strings]\n";
        assert!(matches!(parse_pack(src), Err(I18nError::InvalidCode(_))));
    }

    #[test]
    fn set_active_swaps_pack() {
        force_for_tests();
        assert_eq!(active_code(), "en");
        let custom = Translations {
            code: "xx".into(),
            name: "Test".into(),
            map: [(String::from("modal.lang_picker.title"), String::from("XX"))]
                .into_iter()
                .collect(),
        };
        set_active(custom);
        assert_eq!(active_code(), "xx");
        assert_eq!(t("modal.lang_picker.title"), "XX");
        // missing key still falls back to the English baseline
        assert_eq!(t("modal.lang_picker.hint_close"), "close");
        force_for_tests();
    }
}
