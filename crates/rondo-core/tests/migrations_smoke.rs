use rondo_core::store::migrations::{migrate, user_version, CURRENT_VERSION};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join(name)
}

#[test]
fn fresh_db_migrates_to_current_version() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(fixture("seed.sql")).unwrap())
        .unwrap();
    let v = migrate(&conn).unwrap();
    assert_eq!(v, CURRENT_VERSION);
    assert_eq!(user_version(&conn).unwrap(), CURRENT_VERSION);
}

#[test]
fn v0_seed_migrates_to_v1_adds_metadata() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(fixture("seed-v1.sql")).unwrap())
        .unwrap();
    assert_eq!(user_version(&conn).unwrap(), 0);
    migrate(&conn).unwrap();
    assert_eq!(user_version(&conn).unwrap(), CURRENT_VERSION);
    let mut stmt = conn.prepare("PRAGMA table_info(tasks)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(cols.contains(&"metadata".to_string()));
}

#[test]
fn migrate_is_idempotent() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(fixture("seed.sql")).unwrap())
        .unwrap();
    migrate(&conn).unwrap();
    let v = migrate(&conn).unwrap();
    assert_eq!(v, CURRENT_VERSION);
}

#[test]
fn future_version_errors() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch("PRAGMA user_version = 999").unwrap();
    assert!(migrate(&conn).is_err());
}
