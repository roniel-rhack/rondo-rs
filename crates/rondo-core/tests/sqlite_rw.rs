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
