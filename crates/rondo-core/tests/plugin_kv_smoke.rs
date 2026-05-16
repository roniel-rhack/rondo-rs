use rondo_core::store::sqlite::SqliteStore;

fn fixture() -> (tempfile::NamedTempFile, SqliteStore) {
    let f = tempfile::NamedTempFile::new().unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(f.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
        .unwrap();
    drop(conn);
    let s = SqliteStore::open_readwrite(f.path()).unwrap();
    (f, s)
}

#[test]
fn migration_creates_plugin_kv_table() {
    let (_f, store) = fixture();
    store.kv_set("p1", "k1", b"hello").unwrap();
    let got = store.kv_get("p1", "k1").unwrap();
    assert_eq!(got.as_deref(), Some(b"hello".as_slice()));
}

#[test]
fn get_missing_returns_none() {
    let (_f, store) = fixture();
    assert!(store.kv_get("missing", "missing").unwrap().is_none());
}

#[test]
fn namespaces_are_isolated() {
    let (_f, store) = fixture();
    store.kv_set("pa", "shared", b"A").unwrap();
    store.kv_set("pb", "shared", b"B").unwrap();
    assert_eq!(
        store.kv_get("pa", "shared").unwrap().as_deref(),
        Some(b"A".as_slice())
    );
    assert_eq!(
        store.kv_get("pb", "shared").unwrap().as_deref(),
        Some(b"B".as_slice())
    );
}

#[test]
fn upsert_replaces_value() {
    let (_f, store) = fixture();
    store.kv_set("p", "k", b"v1").unwrap();
    store.kv_set("p", "k", b"v2").unwrap();
    assert_eq!(
        store.kv_get("p", "k").unwrap().as_deref(),
        Some(b"v2".as_slice())
    );
}

#[test]
fn delete_removes_value() {
    let (_f, store) = fixture();
    store.kv_set("p", "k", b"v").unwrap();
    store.kv_delete("p", "k").unwrap();
    assert!(store.kv_get("p", "k").unwrap().is_none());
}

#[test]
fn list_keys_returns_sorted() {
    let (_f, store) = fixture();
    store.kv_set("p", "z", b"3").unwrap();
    store.kv_set("p", "a", b"1").unwrap();
    store.kv_set("p", "m", b"2").unwrap();
    store.kv_set("other", "a", b"X").unwrap();
    let keys = store.kv_list_keys("p").unwrap();
    assert_eq!(keys, vec!["a", "m", "z"]);
}

#[test]
fn binary_blobs_preserved() {
    let (_f, store) = fixture();
    let bin: Vec<u8> = (0u8..=255).collect();
    store.kv_set("p", "blob", &bin).unwrap();
    assert_eq!(store.kv_get("p", "blob").unwrap().unwrap(), bin);
}
