//! Integration test for the quote-of-the-day sample plugin.
//!
//! Loads `examples/plugins/quote-of-the-day/plugin.wasm` if it exists and
//! verifies a `Show` round-trip yields a non-empty overlay `ViewSpec`. Skips
//! gracefully when the wasm artifact is missing (e.g. CI without the
//! `wasm32-wasip1` toolchain installed) — run
//! `examples/plugins/quote-of-the-day/build.sh` first.

use rondo_plugin_api::{PluginAction, ViewKind};
use rondo_plugin_host::PluginHost;

fn samples_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/rondo-plugin-host
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("plugins")
}

#[test]
fn loads_quote_of_the_day_if_built() {
    let dir = samples_root();
    let plugin_wasm = dir.join("quote-of-the-day").join("plugin.wasm");
    if !plugin_wasm.exists() {
        eprintln!(
            "SKIP: {} missing — run examples/plugins/quote-of-the-day/build.sh",
            plugin_wasm.display()
        );
        return;
    }
    let mut host = PluginHost::new();
    let loaded = host.load_from_dir(&dir).unwrap();
    assert!(
        loaded.contains(&"quote-of-the-day".to_string()),
        "quote-of-the-day failed to load: {:?}",
        loaded
    );
    assert!(host.get("quote-of-the-day").unwrap().has_wasm());
    let manifest = &host.get("quote-of-the-day").unwrap().manifest;
    assert_eq!(manifest.command_name(), Some("quote-of-the-day"));
    assert_eq!(
        host.resolve_command("quote-of-the-day").as_deref(),
        Some("quote-of-the-day"),
    );

    let results = host.dispatch(&PluginAction::Show);
    assert!(!results.is_empty(), "Show dispatch returned no results");
    let (id, result) = &results[0];
    assert_eq!(id, "quote-of-the-day");
    let view = result.view.as_ref().expect("Show should return a ViewSpec");
    assert_eq!(view.kind, ViewKind::Overlay);
    assert!(
        view.blocks.len() >= 2,
        "expected at least heading + paragraph"
    );
}

#[test]
fn quote_of_the_day_tick_and_hide_are_noops() {
    let dir = samples_root();
    let plugin_wasm = dir.join("quote-of-the-day").join("plugin.wasm");
    if !plugin_wasm.exists() {
        eprintln!("SKIP: plugin.wasm missing");
        return;
    }
    let mut host = PluginHost::new();
    host.load_from_dir(&dir).unwrap();

    let tick = host.dispatch(&PluginAction::Tick { delta_ms: 100 });
    let (_, r) = &tick[0];
    assert!(r.view.is_none());
    assert!(r.follow_up.is_empty());

    let hide = host.dispatch(&PluginAction::Hide);
    let (_, r) = &hide[0];
    assert!(r.view.is_none());
    assert!(r.follow_up.is_empty());
}
