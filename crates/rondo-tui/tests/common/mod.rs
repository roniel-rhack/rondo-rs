//! Shared test helpers for TUI integration tests.
//!
//! Currently exposes `test_app_with_writable_store()`, which provisions an
//! isolated SQLite DB seeded from `fixtures/seed.sql` inside a `TempDir`
//! and returns a writable `AppState` ready to drive `update()` calls.
//!
//! The returned `TempDir` must outlive the `AppState`; tests should bind
//! both to local variables.

#![allow(dead_code)]

use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::app::AppState;
use std::sync::Arc;
use tempfile::TempDir;

/// Build an isolated writable `AppState` over a seeded temp DB.
///
/// The `TempDir` keeps the DB file alive for the lifetime of the test;
/// callers must keep it in scope alongside the `AppState`.
pub fn test_app_with_writable_store() -> (TempDir, AppState) {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let db_path = tmp.path().join("state.db");
    let seed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let seed = std::fs::read_to_string(&seed_path).expect("read seed.sql");
    let conn = rusqlite::Connection::open(&db_path).expect("open temp db");
    conn.execute_batch(&seed).expect("apply seed");
    drop(conn);
    let store = Arc::new(SqliteStore::open_readwrite(&db_path).expect("open rw store"));
    let app = AppState::with_writable(store, true).expect("build app state");
    (tmp, app)
}
