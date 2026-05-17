//! State-transition tests for EditRecurrence modal (E3).

use rondo_core::domain::task::RecurFreq;
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
    let path = tmp.into_temp_path().keep().unwrap();
    let store = Arc::new(SqliteStore::open_readwrite(&path).unwrap());
    AppState::with_writable(store, true).unwrap()
}

#[test]
fn request_opens_modal_as_top_layer() {
    let mut app = rw_app();
    app.update(Action::RequestEditRecurrence);
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::EditRecurrence));
}

#[test]
fn submit_weekly_persists_freq_and_interval() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    app.update(Action::RequestEditRecurrence);
    app.update(Action::SubmitRecurrence(RecurFreq::Weekly, 1));
    let task = app.data.store.task_by_id(id).unwrap();
    assert_eq!(task.recur_freq, RecurFreq::Weekly);
    assert_eq!(task.recur_interval, 1);
    assert!(!app.modals.edit_recurrence_open);
}

#[test]
fn submit_none_clears_recurrence() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    app.update(Action::SubmitRecurrence(RecurFreq::Daily, 1));
    assert_eq!(
        app.data.store.task_by_id(id).unwrap().recur_freq,
        RecurFreq::Daily
    );
    app.update(Action::RequestEditRecurrence);
    app.update(Action::SubmitRecurrence(RecurFreq::None, 0));
    assert_eq!(
        app.data.store.task_by_id(id).unwrap().recur_freq,
        RecurFreq::None
    );
}

#[test]
fn cancel_closes_without_mutation() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let before = app.data.store.task_by_id(id).unwrap().recur_freq;
    app.update(Action::RequestEditRecurrence);
    app.update(Action::CancelEditRecurrence);
    assert!(!app.modals.edit_recurrence_open);
    let after = app.data.store.task_by_id(id).unwrap().recur_freq;
    assert_eq!(before, after);
}

#[test]
fn undo_reverts_recurrence_change() {
    let mut app = rw_app();
    let id = app.data.selected_task_id().unwrap();
    let before = app.data.store.task_by_id(id).unwrap().recur_freq;
    app.update(Action::RequestEditRecurrence);
    app.update(Action::SubmitRecurrence(RecurFreq::Monthly, 1));
    assert_eq!(
        app.data.store.task_by_id(id).unwrap().recur_freq,
        RecurFreq::Monthly
    );
    app.update(Action::Undo);
    assert_eq!(app.data.store.task_by_id(id).unwrap().recur_freq, before);
}
