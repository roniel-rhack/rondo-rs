use rondo_core::domain::task::{NewTask, Priority, RecurFreq, Status, TaskPatch, UndoKind};
use rondo_core::store::sqlite::SqliteStore;

fn fixture_db() -> (tempfile::NamedTempFile, SqliteStore) {
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
    let store = SqliteStore::open_readwrite(f.path()).unwrap();
    (f, store)
}

#[test]
fn create_task_returns_id_and_undo() {
    let (_f, store) = fixture_db();
    let before_count = store.list_tasks().unwrap().len();
    let (id, undo) = store.create_task(NewTask::quick("new")).unwrap();
    assert!(id > 0);
    assert!(matches!(undo.kind, UndoKind::Create));
    assert_eq!(undo.created_id, Some(id));
    assert!(undo.task_before.is_none());
    assert_eq!(store.list_tasks().unwrap().len(), before_count + 1);
}

#[test]
fn update_task_captures_before_state() {
    let (_f, store) = fixture_db();
    let before = store.list_tasks().unwrap().into_iter().next().unwrap();
    let patch = TaskPatch {
        title: Some("NEW TITLE".into()),
        ..Default::default()
    };
    let undo = store.update_task(before.id, patch).unwrap();
    assert!(matches!(undo.kind, UndoKind::Update));
    assert_eq!(undo.task_before.as_ref().unwrap().title, before.title);
    let after = store.task_by_id(before.id).unwrap();
    assert_eq!(after.title, "NEW TITLE");
}

#[test]
fn delete_task_returns_full_snapshot() {
    let (_f, store) = fixture_db();
    let target = store.list_tasks().unwrap().into_iter().next().unwrap();
    let undo = store.delete_task(target.id).unwrap();
    assert!(matches!(undo.kind, UndoKind::Delete));
    assert_eq!(undo.task_before.as_ref().unwrap().id, target.id);
    assert!(store.task_by_id(target.id).is_err());
}

#[test]
fn set_status_round_trip() {
    let (_f, store) = fixture_db();
    let t = store
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.status != Status::Done)
        .unwrap();
    let _u = store.set_status(t.id, Status::Done).unwrap();
    let after = store.task_by_id(t.id).unwrap();
    assert_eq!(after.status, Status::Done);
}

#[test]
fn subtask_lifecycle() {
    let (_f, store) = fixture_db();
    let task = store.list_tasks().unwrap().into_iter().next().unwrap();
    let (sid, undo) = store.add_subtask(task.id, "new sub").unwrap();
    assert!(matches!(undo.kind, UndoKind::AddSubtask));
    let after = store.task_by_id(task.id).unwrap();
    assert!(after.subtasks.iter().any(|s| s.id == sid));
    let (now_completed, _) = store.toggle_subtask(sid).unwrap();
    assert!(now_completed);
    let (back, _) = store.toggle_subtask(sid).unwrap();
    assert!(!back);
}

#[test]
fn tag_add_remove() {
    let (_f, store) = fixture_db();
    let task = store.list_tasks().unwrap().into_iter().next().unwrap();
    let _ = store.add_tag(task.id, "freshtag").unwrap();
    let after = store.task_by_id(task.id).unwrap();
    assert!(after.tags.iter().any(|t| t == "freshtag"));
    let _ = store.remove_tag(task.id, "freshtag").unwrap();
    let after2 = store.task_by_id(task.id).unwrap();
    assert!(!after2.tags.iter().any(|t| t == "freshtag"));
}

#[test]
fn create_with_tags() {
    let (_f, store) = fixture_db();
    let new = NewTask {
        title: "tagged".into(),
        description: None,
        status: Status::Pending,
        priority: Priority::Low,
        due_date: None,
        recur_freq: RecurFreq::None,
        recur_interval: 0,
        tags: vec!["foo".into(), "bar".into()],
    };
    let (id, _) = store.create_task(new).unwrap();
    let t = store.task_by_id(id).unwrap();
    assert_eq!(t.tags.len(), 2);
}
