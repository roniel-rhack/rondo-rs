//! State-transition tests for the EditDueDate modal (E1).
//!
//! Verifies request → submit → undo cycle and the custom-mode flip from
//! the preset list. Uses an RW `SqliteStore` against a temp DB so the
//! actual mutation lands in the store and undo can revert it.

use chrono::NaiveDate;
use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::action::Action;
use rondo_tui::app::modals_state::ModalLayer;
use rondo_tui::app::AppState;
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
    // Leak the tempfile so the DB outlives the test (avoids ROLLBACK on drop).
    let path = tmp.into_temp_path().keep().unwrap();
    let store = Arc::new(SqliteStore::open_readwrite(&path).unwrap());
    AppState::with_writable(store, true).unwrap()
}

#[test]
fn request_opens_modal_as_top_layer() {
    let mut app = rw_app();
    app.update(Action::RequestEditDueDate);
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::EditDueDate));
    assert!(app.modals.edit_due_date_open);
    assert!(!app.modals.edit_due_date_custom_mode);
}

#[test]
fn submit_preset_sets_date_and_closes() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().expect("seed has tasks");
    let d = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    app.update(Action::RequestEditDueDate);
    app.update(Action::SubmitDueDate(Some(d)));
    assert!(!app.modals.edit_due_date_open);
    let task = app.data.store.task_by_id(id).unwrap();
    assert_eq!(task.due_date, Some(d));
}

#[test]
fn submit_none_clears_date() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let d = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    app.update(Action::SubmitDueDate(Some(d)));
    // sanity: first set a date.
    let task = app.data.store.task_by_id(id).unwrap();
    assert_eq!(task.due_date, Some(d));
    // now clear via SubmitDueDate(None) — wraps RequestEditDueDate so
    // the modal opens first (real path would be `D` -> `x`).
    app.update(Action::RequestEditDueDate);
    app.update(Action::SubmitDueDate(None));
    let task = app.data.store.task_by_id(id).unwrap();
    assert_eq!(task.due_date, None);
}

#[test]
fn cancel_closes_modal_without_mutation() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let before = app.data.store.task_by_id(id).unwrap().due_date;
    app.update(Action::RequestEditDueDate);
    app.update(Action::CancelEditDueDate);
    assert!(!app.modals.edit_due_date_open);
    let after = app.data.store.task_by_id(id).unwrap().due_date;
    assert_eq!(before, after);
}

#[test]
fn edit_due_date_input_enters_custom_mode() {
    let mut app = rw_app();
    app.update(Action::RequestEditDueDate);
    assert!(!app.modals.edit_due_date_custom_mode);
    app.update(Action::EditDueDateInput("2026".into()));
    assert!(app.modals.edit_due_date_custom_mode);
    assert_eq!(app.modals.edit_due_date_buf, "2026");
}

#[test]
fn undo_reverts_due_date_change() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let before = app.data.store.task_by_id(id).unwrap().due_date;
    let new_date = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
    app.update(Action::RequestEditDueDate);
    app.update(Action::SubmitDueDate(Some(new_date)));
    assert_eq!(
        app.data.store.task_by_id(id).unwrap().due_date,
        Some(new_date)
    );
    app.update(Action::Undo);
    assert_eq!(app.data.store.task_by_id(id).unwrap().due_date, before);
}
