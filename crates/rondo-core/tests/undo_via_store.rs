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

#[test]
fn add_dependency_cycle_reports_endpoints() {
    let (_f, store) = fixture();
    let mut tasks = store.list_tasks().unwrap().into_iter();
    let a = tasks.next().unwrap();
    let b = tasks.next().unwrap();
    // a is blocked by b; trying to make b blocked by a forms the cycle.
    store.add_dependency(a.id, b.id).unwrap();
    let err = store
        .add_dependency(b.id, a.id)
        .expect_err("should refuse cycle");
    match err {
        rondo_core::error::Error::CycleDetected(x, y) => {
            assert_eq!(x, b.id);
            assert_eq!(y, a.id);
        }
        other => panic!("expected CycleDetected, got {other:?}"),
    }
}

// ----- A4: per-kind round-trips for the new payload-bearing UndoKinds ----

#[test]
fn add_dep_undo_via_remove() {
    let (_f, store) = fixture();
    let mut tasks = store.list_tasks().unwrap().into_iter();
    let a = tasks.next().unwrap();
    let b = tasks.next().unwrap();
    store.add_dependency(a.id, b.id).unwrap();
    let after = store.task_by_id(a.id).unwrap();
    assert!(after.blocked_by_ids.iter().any(|d| *d == b.id));
    store.remove_dependency(a.id, b.id).unwrap();
    let after2 = store.task_by_id(a.id).unwrap();
    assert!(!after2.blocked_by_ids.iter().any(|d| *d == b.id));
}

#[test]
fn remove_dep_undo_via_add() {
    let (_f, store) = fixture();
    let mut tasks = store.list_tasks().unwrap().into_iter();
    let a = tasks.next().unwrap();
    let b = tasks.next().unwrap();
    store.add_dependency(a.id, b.id).unwrap();
    store.remove_dependency(a.id, b.id).unwrap();
    store.add_dependency(a.id, b.id).unwrap();
    assert!(store
        .task_by_id(a.id)
        .unwrap()
        .blocked_by_ids
        .iter()
        .any(|d| *d == b.id));
}

#[test]
fn delete_subtask_undo_via_restore() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let (sid, _) = store.add_subtask(t.id, "victim").unwrap();
    let sub = store
        .task_by_id(t.id)
        .unwrap()
        .subtasks
        .iter()
        .find(|s| s.id == sid)
        .unwrap()
        .clone();
    store.delete_subtask(sid).unwrap();
    assert!(!store
        .task_by_id(t.id)
        .unwrap()
        .subtasks
        .iter()
        .any(|s| s.id == sid));
    store.restore_subtask(&sub).unwrap();
    let restored = store
        .task_by_id(t.id)
        .unwrap()
        .subtasks
        .into_iter()
        .find(|s| s.id == sid)
        .unwrap();
    assert_eq!(restored.title, "victim");
    assert_eq!(restored.id, sid);
}

#[test]
fn explicit_subtask_toggle_undo() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let (sid, _) = store.add_subtask(t.id, "togglee").unwrap();
    let before = store
        .task_by_id(t.id)
        .unwrap()
        .subtasks
        .iter()
        .find(|s| s.id == sid)
        .unwrap()
        .completed;
    store.toggle_subtask(sid).unwrap();
    store.set_subtask_completed(sid, before).unwrap();
    let after = store
        .task_by_id(t.id)
        .unwrap()
        .subtasks
        .iter()
        .find(|s| s.id == sid)
        .unwrap()
        .completed;
    assert_eq!(after, before);
}

#[test]
fn add_note_undo_via_delete() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let nid = store.add_task_note(t.id, "ephemeral note").unwrap();
    assert!(store
        .task_by_id(t.id)
        .unwrap()
        .notes
        .iter()
        .any(|n| n.id == nid));
    store.delete_task_note(nid).unwrap();
    assert!(!store
        .task_by_id(t.id)
        .unwrap()
        .notes
        .iter()
        .any(|n| n.id == nid));
}

#[test]
fn update_note_undo_restores_body() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let nid = store.add_task_note(t.id, "original").unwrap();
    store.update_task_note(nid, "changed").unwrap();
    store.update_task_note(nid, "original").unwrap();
    let body = store
        .task_by_id(t.id)
        .unwrap()
        .notes
        .into_iter()
        .find(|n| n.id == nid)
        .unwrap()
        .body;
    assert_eq!(body, "original");
}

#[test]
fn delete_note_undo_via_restore() {
    let (_f, store) = fixture();
    let t = store.list_tasks().unwrap().into_iter().next().unwrap();
    let nid = store.add_task_note(t.id, "saved note").unwrap();
    let captured = store
        .task_by_id(t.id)
        .unwrap()
        .notes
        .into_iter()
        .find(|n| n.id == nid)
        .unwrap();
    store.delete_task_note(nid).unwrap();
    store.restore_task_note(&captured).unwrap();
    let restored = store
        .task_by_id(t.id)
        .unwrap()
        .notes
        .into_iter()
        .find(|n| n.id == nid)
        .unwrap();
    assert_eq!(restored.body, "saved note");
    assert_eq!(restored.id, nid);
}

#[test]
fn journal_delete_entry_undo_via_restore() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    let eid = store.add_journal_entry(note.id, "diary line").unwrap();
    let captured = store
        .entries_for_note(note.id)
        .unwrap()
        .into_iter()
        .find(|e| e.id == eid)
        .unwrap();
    store.delete_entry(eid).unwrap();
    store.restore_journal_entry(&captured).unwrap();
    let entries = store.entries_for_note(note.id).unwrap();
    assert!(entries.iter().any(|e| e.id == eid && e.body == "diary line"));
}

#[test]
fn journal_delete_day_undo_via_restore() {
    let (_f, store) = fixture();
    let note = store.create_or_get_today_note().unwrap();
    store.add_journal_entry(note.id, "line 1").unwrap();
    store.add_journal_entry(note.id, "line 2").unwrap();
    let entries = store.entries_for_note(note.id).unwrap();
    let total_before = entries.len();
    store.delete_note(note.id).unwrap();
    store.restore_journal_day(&note, &entries).unwrap();
    let restored = store.entries_for_note(note.id).unwrap();
    assert_eq!(restored.len(), total_before);
    assert!(restored.iter().any(|e| e.body == "line 1"));
    assert!(restored.iter().any(|e| e.body == "line 2"));
}
