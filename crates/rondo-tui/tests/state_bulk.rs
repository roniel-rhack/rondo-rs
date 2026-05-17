//! State-transition tests for visual-mode bulk ops (E10).

use chrono::NaiveDate;
use rondo_core::domain::task::Status;
use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::action::Action;
use rondo_tui::app::AppState;
use rondo_tui::focus::Mode;
use std::sync::Arc;

fn rw_app() -> AppState {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    let seed = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(&seed).unwrap();
    drop(conn);
    let path = tmp.into_temp_path().keep().unwrap();
    let store = Arc::new(SqliteStore::open_readwrite(&path).unwrap());
    AppState::with_writable(store, true).unwrap()
}

fn enter_visual_with_two(app: &mut AppState) -> Vec<i64> {
    app.update(Action::EnterVisual);
    // EnterVisual pre-selects the focused task; advance to extend.
    let mut ids: Vec<i64> = app.ui.selection.iter().copied().collect();
    app.update(Action::NextItem);
    let next = app.data.selected_task_id().unwrap();
    app.ui.selection.insert(next);
    if !ids.contains(&next) {
        ids.push(next);
    }
    ids.sort();
    ids
}

#[test]
fn bulk_delete_removes_each_id() {
    let mut app = rw_app();
    let ids = enter_visual_with_two(&mut app);
    app.update(Action::BulkDelete);
    assert_eq!(app.ui.mode, Mode::Normal);
    assert!(app.ui.selection.is_empty());
    for id in ids {
        assert!(
            app.data.store.task_by_id(id).is_err(),
            "task {} should be deleted",
            id
        );
    }
}

#[test]
fn bulk_set_status_persists() {
    let mut app = rw_app();
    let ids = enter_visual_with_two(&mut app);
    app.update(Action::BulkSetStatus(Status::Done));
    for id in &ids {
        assert_eq!(app.data.store.task_by_id(*id).unwrap().status, Status::Done);
    }
}

#[test]
fn bulk_set_due_date_persists_and_undoes() {
    let mut app = rw_app();
    let ids = enter_visual_with_two(&mut app);
    let date = NaiveDate::from_ymd_opt(2030, 1, 1).unwrap();
    let before: Vec<_> = ids
        .iter()
        .map(|id| app.data.store.task_by_id(*id).unwrap().due_date)
        .collect();
    app.update(Action::BulkSetDueDate(Some(date)));
    for id in &ids {
        assert_eq!(app.data.store.task_by_id(*id).unwrap().due_date, Some(date));
    }
    // Undo each in turn (one snapshot per task).
    for _ in 0..ids.len() {
        app.update(Action::Undo);
    }
    for (id, b) in ids.iter().zip(before.iter()) {
        assert_eq!(app.data.store.task_by_id(*id).unwrap().due_date, *b);
    }
}

#[test]
fn bulk_add_tag_attaches_tag() {
    let mut app = rw_app();
    let ids = enter_visual_with_two(&mut app);
    app.update(Action::BulkAddTag("bulk-test".into()));
    for id in &ids {
        let tags = app.data.store.task_by_id(*id).unwrap().tags;
        assert!(
            tags.contains(&"bulk-test".to_string()),
            "task {} missing tag",
            id
        );
    }
}

#[test]
fn bulk_actions_skip_when_not_visual() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let before = app.data.store.task_by_id(id).unwrap().status;
    // Not in visual mode -> no-op.
    app.update(Action::BulkSetStatus(Status::Done));
    assert_eq!(app.data.store.task_by_id(id).unwrap().status, before);
}
