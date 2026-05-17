//! End-to-end check that an external WASM plugin installed under
//! `~/.rondo-rs/plugins/` is reachable from the TUI command palette.
//!
//! We load `examples/plugins/quote-of-the-day/` directly into the
//! `AppState.external` host (bypassing `load_external_plugins`, which
//! reads from a fixed home dir), then dispatch the typed command
//! `quote-of-the-day` through the public `Action::SubmitCommand` path
//! and assert the resulting `ViewSpec` lands in `modals.plugin_overlay`.

use rondo_core::store::sqlite::SqliteStore;
use rondo_plugin_api::{Block, ViewKind};
use rondo_tui::{action::Action, app::AppState};
use std::path::Path;
use std::sync::Arc;

const EXPECTED_QUOTES: &[&str] = &[
    "Make it work, make it right, make it fast.",
    "Premature optimization is the root of all evil.",
    "Simplicity is the ultimate sophistication.",
    "Talk is cheap. Show me the code.",
    "Code is read more often than it is written.",
];

fn plugin_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("plugins")
}

fn fixture_store() -> Arc<SqliteStore> {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    let seed = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(&seed).unwrap();
    drop(conn);
    Arc::new(SqliteStore::open_readwrite(tmp.path()).unwrap())
}

#[test]
fn submit_command_routes_to_external_overlay() {
    let wasm = plugin_dir().join("quote-of-the-day").join("plugin.wasm");
    if !wasm.exists() {
        eprintln!(
            "SKIP: {} missing — run examples/plugins/quote-of-the-day/build.sh",
            wasm.display(),
        );
        return;
    }

    let mut app = AppState::with_writable(fixture_store(), true).unwrap();
    let loaded = app.external.load_from_dir(&plugin_dir()).unwrap();
    assert!(
        loaded.contains(&"quote-of-the-day".to_string()),
        "load_from_dir did not load quote-of-the-day: {:?}",
        loaded,
    );

    app.update(Action::SubmitCommand("quote-of-the-day".to_string()));

    let (id, view) = app
        .modals
        .plugin_overlay
        .as_ref()
        .expect("plugin_overlay should be populated by SubmitCommand");
    assert_eq!(id, "quote-of-the-day");
    assert_eq!(view.kind, ViewKind::Overlay);

    let paragraph = view
        .blocks
        .iter()
        .find_map(|b| match b {
            Block::Paragraph { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("expected a Paragraph block with the quote");
    assert!(
        EXPECTED_QUOTES.contains(&paragraph.as_str()),
        "unexpected quote: {:?}",
        paragraph,
    );
}

#[test]
fn unique_prefix_routes_to_external_overlay() {
    let wasm = plugin_dir().join("quote-of-the-day").join("plugin.wasm");
    if !wasm.exists() {
        eprintln!("SKIP: plugin.wasm missing");
        return;
    }
    let mut app = AppState::with_writable(fixture_store(), true).unwrap();
    app.external.load_from_dir(&plugin_dir()).unwrap();

    // "quo" only matches "quote-of-the-day" — should run it.
    app.update(Action::SubmitCommand("quo".to_string()));
    let (id, _) = app
        .modals
        .plugin_overlay
        .as_ref()
        .expect("prefix `quo` should resolve to quote-of-the-day");
    assert_eq!(id, "quote-of-the-day");
}

#[test]
fn ambiguous_prefix_toasts_matches() {
    let mut app = AppState::with_writable(fixture_store(), true).unwrap();
    // "fo" matches both `focus` and `focus-page` from the builtin set.
    app.update(Action::SubmitCommand("fo".to_string()));
    assert!(app.modals.plugin_overlay.is_none());
    assert!(app.modals.plugin_page.is_none());
    let msg = app.status_msg.as_deref().unwrap_or("");
    assert!(msg.starts_with("ambiguous:"), "got: {}", msg);
    assert!(
        msg.contains("focus"),
        "expected matches in toast, got: {}",
        msg
    );
}

#[test]
fn unknown_command_does_not_open_overlay() {
    let mut app = AppState::with_writable(fixture_store(), true).unwrap();
    app.update(Action::SubmitCommand("definitely-not-a-plugin".to_string()));
    assert!(app.modals.plugin_overlay.is_none());
    assert!(app.modals.plugin_page.is_none());
    assert!(app
        .status_msg
        .as_deref()
        .unwrap_or("")
        .starts_with("unknown:"));
}
