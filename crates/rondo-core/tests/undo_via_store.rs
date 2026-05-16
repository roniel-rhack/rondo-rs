use rondo_core::domain::task::{NewTask, Status, TaskPatch};
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
    let store = SqliteStore::open_readwrite(f.path()).unwrap();
    (f, store)
}

#[test]
fn create_undo_deletes() {
    let (_f, store) = fixture();
    let before_count = store.list_tasks().unwrap().len();
    let (id, undo) = store.create_task(NewTask::quick("temp")).unwrap();
    assert_eq!(store.list_tasks().unwrap().len(), before_count + 1);
    // simulate undo: delete the created row
    store.delete_task(undo.created_id.unwrap()).unwrap();
    let _ = id;
    assert_eq!(store.list_tasks().unwrap().len(), before_count);
}

#[test]
fn update_undo_restores_title() {
    let (_f, store) = fixture();
    let original = store.list_tasks().unwrap().into_iter().next().unwrap();
    let patch = TaskPatch {
        title: Some("BANG".into()),
        ..Default::default()
    };
    let undo = store.update_task(original.id, patch).unwrap();
    let before = undo.task_before.unwrap();
    let restore = TaskPatch {
        title: Some(before.title.clone()),
        description: Some(before.description.clone()),
        status: Some(before.status),
        priority: Some(before.priority),
        due_date: Some(before.due_date),
        recur_freq: Some(before.recur_freq),
        recur_interval: Some(before.recur_interval),
    };
    store.update_task(original.id, restore).unwrap();
    let final_task = store.task_by_id(original.id).unwrap();
    assert_eq!(final_task.title, original.title);
}

#[test]
fn set_status_undo_restores_status() {
    let (_f, store) = fixture();
    let t = store
        .list_tasks()
        .unwrap()
        .into_iter()
        .find(|t| t.status != Status::Done)
        .unwrap();
    let original_status = t.status;
    let undo = store.set_status(t.id, Status::Done).unwrap();
    let before = undo.task_before.unwrap();
    store.set_status(t.id, before.status).unwrap();
    let after = store.task_by_id(t.id).unwrap();
    assert_eq!(after.status, original_status);
}

#[test]
fn add_subtask_undo_deletes() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let before_subs = t.subtasks.len();
    let (sub_id, undo) = store.add_subtask(t.id, "ephemeral").unwrap();
    assert_eq!(
        store.task_by_id(t.id).unwrap().subtasks.len(),
        before_subs + 1
    );
    // undo via delete_subtask
    let _ = undo;
    store.delete_subtask(sub_id).unwrap();
    assert_eq!(store.task_by_id(t.id).unwrap().subtasks.len(), before_subs);
}

#[test]
fn add_tag_undo_via_remove() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let before_tags = t.tags.clone();
    let new_tag = "undo-me";
    assert!(!before_tags.contains(&new_tag.to_string()));
    store.add_tag(t.id, new_tag).unwrap();
    let after = store.task_by_id(t.id).unwrap();
    assert!(after.tags.contains(&new_tag.to_string()));
    // undo: locate the new tag and remove it
    let added: Vec<String> = after
        .tags
        .iter()
        .filter(|t| !before_tags.contains(t))
        .cloned()
        .collect();
    assert_eq!(added, vec![new_tag.to_string()]);
    store.remove_tag(t.id, new_tag).unwrap();
    assert_eq!(store.task_by_id(t.id).unwrap().tags, before_tags);
}
