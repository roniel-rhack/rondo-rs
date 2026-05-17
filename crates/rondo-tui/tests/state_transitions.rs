//! End-to-end state-transition tests for the TUI.
//!
//! Drive `AppState::update(Action::...)` against an isolated writable
//! store and assert state invariants. Complements the snapshot tests
//! (which assert *rendered* output) by validating the action → mutation
//! → store → refresh pipeline directly.

mod common;

use common::test_app_with_writable_store;
use rondo_core::domain::task::Status;
use rondo_tui::action::{Action, Page};
use rondo_tui::app::modals_state::ModalLayer;
use rondo_tui::filter::Filter;
use rondo_tui::focus::{DetailSection, Mode, Pane};

#[test]
fn quick_add_submit_then_undo() {
    let (_tmp, mut app) = test_app_with_writable_store();
    let before = app.data.tasks.len();

    app.update(Action::OpenQuickAdd);
    assert!(app.modals.quick_add_open);
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::QuickAdd));
    assert_eq!(app.ui.mode, Mode::Insert);

    app.update(Action::SubmitQuickAdd("brand new task #demo !p3".into()));
    assert!(!app.modals.quick_add_open);
    assert_eq!(app.data.tasks.len(), before + 1);
    let added = app
        .data
        .tasks
        .iter()
        .find(|t| t.title == "brand new task")
        .expect("created task should be visible");
    assert!(added.tags.iter().any(|t| t == "demo"));

    let depth_before = app.undo.len();
    assert!(depth_before > 0, "submit_quick_add should push an undo");

    app.update(Action::Undo);
    assert_eq!(app.data.tasks.len(), before);
    assert_eq!(app.undo.len(), depth_before - 1);
}

#[test]
fn open_search_type_filters_visible_indices() {
    let (_tmp, mut app) = test_app_with_writable_store();
    app.apply_filter(Filter::All);

    app.update(Action::OpenSearch);
    assert!(app.modals.search_open);
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::Search));

    let baseline = app.visible_task_indices().len();
    assert!(baseline > 0, "fixture must have visible tasks");

    // Type something that cannot possibly match any seeded task; fuzzy
    // search should narrow the result set.
    app.update(Action::SearchUpdate("zzqqxx-nope".into()));
    let narrowed = app.visible_task_indices();
    assert!(
        narrowed.len() < baseline,
        "search filter should narrow visible tasks (baseline={}, narrowed={})",
        baseline,
        narrowed.len()
    );

    // Clearing the buffer restores the baseline.
    app.update(Action::SearchUpdate(String::new()));
    assert_eq!(app.visible_task_indices().len(), baseline);

    // Close clears the buffer.
    app.update(Action::CloseSearch);
    assert!(!app.modals.search_open);
    assert!(app.modals.search_buf.is_empty());
}

#[test]
fn visual_mode_extend_then_bulk_done() {
    let (_tmp, mut app) = test_app_with_writable_store();
    app.apply_filter(Filter::All);
    app.data.selected_task = 0;

    app.update(Action::EnterVisual);
    assert_eq!(app.ui.mode, Mode::Visual);
    assert_eq!(app.ui.selection.len(), 1);

    // Move down -> extend selection to two tasks.
    let first_id = app.data.tasks[0].id;
    let second_id = app.data.tasks[1].id;
    app.ui.selection.insert(second_id);

    app.update(Action::BulkDone);
    // BulkDone resets to Normal and clears the visual selection.
    assert_eq!(app.ui.mode, Mode::Normal);
    assert!(app.ui.selection.is_empty());

    let first = app.data.tasks.iter().find(|t| t.id == first_id).unwrap();
    let second = app.data.tasks.iter().find(|t| t.id == second_id).unwrap();
    assert_eq!(first.status, Status::Done);
    assert_eq!(second.status, Status::Done);
}

#[test]
fn next_section_cycles_detail_sections() {
    let (_tmp, mut app) = test_app_with_writable_store();
    app.ui.focus.pane = Pane::Detail;
    app.ui.focus.section = DetailSection::Header;

    let start = app.ui.focus.section;
    app.update(Action::NextSection);
    assert_ne!(app.ui.focus.section, start);

    // Cycle a few times — should always stay on a valid section.
    for _ in 0..6 {
        app.update(Action::NextSection);
    }

    // Direct jump back to Header.
    app.update(Action::JumpDetailSection(0));
    assert_eq!(app.ui.focus.section, DetailSection::Header);
}

#[test]
fn pomodoro_start_then_close_finalizes() {
    let (_tmp, mut app) = test_app_with_writable_store();
    assert!(!app.modals.pomodoro_open);

    app.update(Action::OpenPomodoro);
    assert!(app.modals.pomodoro_open);
    assert!(app.modals.pomodoro_started.is_some());
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::Pomodoro));

    // Closing via EscapeContext finalizes the session.
    app.update(Action::EscapeContext);
    assert!(!app.modals.pomodoro_open);
    assert!(app.modals.pomodoro_session_id.is_none());
}

#[test]
fn quick_add_with_due_token_sets_due_date() {
    let (_tmp, mut app) = test_app_with_writable_store();
    let before = app.data.tasks.len();

    app.update(Action::OpenQuickAdd);
    app.update(Action::SubmitQuickAdd("ship it due:tmrw".into()));

    assert_eq!(app.data.tasks.len(), before + 1);
    let added = app
        .data
        .tasks
        .iter()
        .find(|t| t.title == "ship it")
        .expect("ship-it task added");
    let today = app.clock.today();
    let expected = today + chrono::Duration::days(1);
    assert_eq!(
        added.due_date,
        Some(expected),
        "due:tmrw should resolve to today+1 via the injected clock"
    );
}

#[test]
fn toggle_page_to_journal_and_back() {
    let (_tmp, mut app) = test_app_with_writable_store();
    assert_eq!(app.ui.page, Page::Tasks);
    app.update(Action::TogglePage(Page::Journal));
    assert_eq!(app.ui.page, Page::Journal);
    app.update(Action::TogglePage(Page::Tasks));
    assert_eq!(app.ui.page, Page::Tasks);
}
