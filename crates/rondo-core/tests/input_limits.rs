//! D5: store-boundary length validation. Each text field accepted by
//! the store has an explicit cap; anything beyond it is rejected with
//! `Error::InputTooLong` rather than being silently persisted.

use rondo_core::domain::task::{NewTask, TaskPatch};
use rondo_core::error::Error;
use rondo_core::store::sqlite::SqliteStore;

fn fresh_store() -> (tempfile::NamedTempFile, SqliteStore) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
        .unwrap();
    drop(conn);
    let store = SqliteStore::open_readwrite(tmp.path()).unwrap();
    (tmp, store)
}

fn assert_too_long<T>(result: rondo_core::error::Result<T>, expected_field: &str) {
    match result {
        Err(Error::InputTooLong { field, .. }) => {
            assert_eq!(field, expected_field, "wrong field reported");
        }
        Err(other) => panic!("expected InputTooLong, got: {other}"),
        Ok(_) => panic!("expected InputTooLong, got Ok"),
    }
}

#[test]
fn create_task_rejects_oversize_title() {
    let (_tmp, store) = fresh_store();
    let huge = "x".repeat(501);
    let mut nt = NewTask::quick(&huge);
    nt.tags.clear();
    assert_too_long(store.create_task(nt), "title");
}

#[test]
fn create_task_rejects_oversize_description() {
    let (_tmp, store) = fresh_store();
    let mut nt = NewTask::quick("ok title");
    nt.description = Some("x".repeat(50_001));
    assert_too_long(store.create_task(nt), "description");
}

#[test]
fn create_task_rejects_oversize_tag() {
    let (_tmp, store) = fresh_store();
    let mut nt = NewTask::quick("ok title");
    nt.tags = vec!["x".repeat(65)];
    assert_too_long(store.create_task(nt), "tag");
}

#[test]
fn update_task_rejects_oversize_title() {
    let (_tmp, store) = fresh_store();
    let (id, _) = store.create_task(NewTask::quick("ok")).unwrap();
    let patch = TaskPatch {
        title: Some("x".repeat(501)),
        ..Default::default()
    };
    assert_too_long(store.update_task(id, patch), "title");
}

#[test]
fn add_subtask_rejects_oversize_title() {
    let (_tmp, store) = fresh_store();
    let (id, _) = store.create_task(NewTask::quick("parent")).unwrap();
    let huge = "x".repeat(501);
    assert_too_long(store.add_subtask(id, &huge), "subtask title");
}

#[test]
fn add_task_note_rejects_oversize_body() {
    let (_tmp, store) = fresh_store();
    let (id, _) = store.create_task(NewTask::quick("parent")).unwrap();
    let huge = "x".repeat(50_001);
    assert_too_long(store.add_task_note(id, &huge), "note");
}

#[test]
fn add_journal_entry_rejects_oversize_body() {
    let (_tmp, store) = fresh_store();
    let note = store.create_or_get_today_note().unwrap();
    let huge = "x".repeat(100_001);
    assert_too_long(store.add_journal_entry(note.id, &huge), "journal entry");
}

#[test]
fn add_tag_rejects_oversize_name() {
    let (_tmp, store) = fresh_store();
    let (id, _) = store.create_task(NewTask::quick("parent")).unwrap();
    let huge = "x".repeat(65);
    assert_too_long(store.add_tag(id, &huge), "tag");
}

#[test]
fn boundary_lengths_are_accepted() {
    let (_tmp, store) = fresh_store();
    let mut nt = NewTask::quick("a".repeat(500));
    nt.description = Some("b".repeat(50_000));
    nt.tags = vec!["c".repeat(64)];
    store
        .create_task(nt)
        .expect("boundary values must be accepted");
}
