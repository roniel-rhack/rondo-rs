use rondo_core::domain::focus::SessionKind;
use rondo_core::domain::task::NewTask;
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
fn migration_v2_creates_focus_sessions_table() {
    let (_f, store) = fixture();
    let id = store
        .start_focus_session(None, SessionKind::Work, 1500)
        .unwrap();
    assert!(id > 0);
}

#[test]
fn start_then_complete_persists_completed_at() {
    let (_f, store) = fixture();
    let id = store
        .start_focus_session(None, SessionKind::Work, 1500)
        .unwrap();
    store.complete_focus_session(id).unwrap();
    let sessions = store.list_focus_sessions().unwrap();
    let s = sessions
        .iter()
        .find(|s| s.id == Some(id))
        .expect("session should be in list");
    assert!(s.completed_at.is_some(), "completed_at should be populated");
    assert_eq!(s.duration_secs, 1500);
    assert!(matches!(s.kind, SessionKind::Work));
    assert!(s.task_id.is_none());
}

#[test]
fn started_without_complete_has_null_completed_at() {
    let (_f, store) = fixture();
    let id = store
        .start_focus_session(None, SessionKind::Work, 1500)
        .unwrap();
    let sessions = store.list_focus_sessions().unwrap();
    let s = sessions.iter().find(|s| s.id == Some(id)).unwrap();
    assert!(s.completed_at.is_none());
}

#[test]
fn streak_zero_when_no_completed_work_today() {
    let (_f, store) = fixture();
    assert_eq!(store.focus_streak().unwrap(), 0);
}

#[test]
fn streak_zero_when_only_incomplete() {
    let (_f, store) = fixture();
    let _ = store
        .start_focus_session(None, SessionKind::Work, 1500)
        .unwrap();
    assert_eq!(store.focus_streak().unwrap(), 0);
}

#[test]
fn streak_one_after_completing_today() {
    let (_f, store) = fixture();
    let id = store
        .start_focus_session(None, SessionKind::Work, 1500)
        .unwrap();
    store.complete_focus_session(id).unwrap();
    assert_eq!(store.focus_streak().unwrap(), 1);
}

#[test]
fn streak_ignores_break_sessions() {
    let (_f, store) = fixture();
    let id = store
        .start_focus_session(None, SessionKind::ShortBreak, 300)
        .unwrap();
    store.complete_focus_session(id).unwrap();
    assert_eq!(store.focus_streak().unwrap(), 0);
}

#[test]
fn task_deletion_nulls_task_id() {
    let (_f, store) = fixture();
    let (task_id, _) = store.create_task(NewTask::quick("focus target")).unwrap();
    let session_id = store
        .start_focus_session(Some(task_id), SessionKind::Work, 1500)
        .unwrap();
    store.complete_focus_session(session_id).unwrap();
    store.delete_task(task_id).unwrap();
    let sessions = store.list_focus_sessions().unwrap();
    let s = sessions
        .iter()
        .find(|s| s.id == Some(session_id))
        .expect("session should survive task deletion");
    assert!(
        s.task_id.is_none(),
        "task_id should be NULL after ON DELETE SET NULL, got {:?}",
        s.task_id
    );
}
