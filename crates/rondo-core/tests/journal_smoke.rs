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
fn create_or_get_today_is_idempotent() {
    let (_f, store) = fixture();
    let a = store.create_or_get_today_note().unwrap();
    let b = store.create_or_get_today_note().unwrap();
    assert_eq!(a.id, b.id);
}

#[test]
fn add_entry_and_retrieve() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    let id = store.add_journal_entry(note.id, "## Hello world").unwrap();
    assert!(id > 0);
    let entries = store.entries_for_note(note.id).unwrap();
    assert!(entries.iter().any(|e| e.body == "## Hello world"));
}

#[test]
fn hide_excludes_from_default_list() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    store.hide_note(note.id).unwrap();
    let visible = store.list_journal_notes().unwrap();
    assert!(!visible.iter().any(|n| n.id == note.id));
    let all = store.list_all_journal_notes_including_hidden().unwrap();
    assert!(all.iter().any(|n| n.id == note.id));
}

#[test]
fn unhide_restores() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    store.hide_note(note.id).unwrap();
    store.unhide_note(note.id).unwrap();
    let visible = store.list_journal_notes().unwrap();
    assert!(visible.iter().any(|n| n.id == note.id));
}

#[test]
fn delete_entry_removes_it() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    let id = store.add_journal_entry(note.id, "bye").unwrap();
    store.delete_entry(id).unwrap();
    let entries = store.entries_for_note(note.id).unwrap();
    assert!(!entries.iter().any(|e| e.id == id));
}
