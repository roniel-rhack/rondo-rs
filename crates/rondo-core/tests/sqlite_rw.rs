use rondo_core::store::sqlite::SqliteStore;

#[test]
fn open_readwrite_round_trip() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let seed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed_path).unwrap())
        .unwrap();
    drop(conn);
    let store = SqliteStore::open_readwrite(tmp.path()).unwrap();
    let tasks = store.list_tasks().unwrap();
    assert!(!tasks.is_empty());
}

/// Smoke test for D1: when a task row carries a `created_at` value that
/// `parse_dt` cannot parse, `list_tasks()` must surface an error
/// instead of silently substituting `Utc::now()` (the old behaviour).
#[test]
fn parse_dt_rejects_garbage() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let seed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed_path).unwrap())
        .unwrap();
    conn.execute("UPDATE tasks SET created_at = 'totally not a date'", [])
        .unwrap();
    drop(conn);

    let store = SqliteStore::open_readwrite(tmp.path()).unwrap();
    let err = store
        .list_tasks()
        .expect_err("garbage created_at must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("totally not a date") || msg.contains("parse") || msg.contains("conversion"),
        "expected parse-date error, got: {msg}",
    );
}
