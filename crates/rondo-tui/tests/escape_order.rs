//! Verify modal close order is deterministic.
//!
//! Opens several modals via the public `open_*` helpers, then issues
//! `Action::EscapeContext` repeatedly and asserts the topmost modal
//! closes first (LIFO per [`ModalLayer`] priority).
//!
//! This pairs with the unit test `top_modal_respects_priority` in
//! `modals_state.rs`, which checks only the priority query; here we
//! exercise the full dispatch through `AppState::update()` so the
//! cross-cutting side-effects (resetting `ui.mode`, plugin Hide,
//! pomodoro finalisation) stay correct as layers stack and unwind.

mod common;

use common::test_app_with_writable_store;
use rondo_tui::action::Action;
use rondo_tui::app::modals_state::{DepOverlayMode, ModalLayer};

#[test]
fn escape_closes_modals_in_lifo_priority_order() {
    let (_tmp, mut app) = test_app_with_writable_store();

    // Stack several layers on. Order doesn't matter for the test —
    // `top_modal()` always returns the highest-priority one first.
    app.modals.open_help();
    app.modals.open_command_palette();
    app.modals.open_search();
    app.modals.open_quick_add();
    app.modals.open_sort_overlay();
    app.modals.open_confirm_delete();
    app.modals.open_edit_title("draft".into());
    app.modals.open_add_subtask();
    app.modals.open_dep_overlay(DepOverlayMode::Add);

    // Expected LIFO close order, descending by `ModalLayer` priority.
    let expected = [
        ModalLayer::DepOverlay,
        ModalLayer::AddSubtask,
        ModalLayer::EditTitle,
        ModalLayer::ConfirmDelete,
        ModalLayer::SortOverlay,
        ModalLayer::QuickAdd,
        ModalLayer::Search,
        ModalLayer::CommandPalette,
        ModalLayer::Help,
    ];

    for layer in expected {
        assert_eq!(
            app.modals.top_modal(),
            Some(layer),
            "top before escape should be {:?}",
            layer
        );
        app.update(Action::EscapeContext);
        assert_ne!(
            app.modals.top_modal(),
            Some(layer),
            "{:?} should be closed after escape",
            layer
        );
    }

    assert_eq!(app.modals.top_modal(), None, "all layers should be closed");
}

#[test]
fn escape_at_idle_clears_status_msg() {
    let (_tmp, mut app) = test_app_with_writable_store();
    app.status_msg = Some("a previous toast".into());
    assert_eq!(app.modals.top_modal(), None);

    app.update(Action::EscapeContext);
    assert!(
        app.status_msg.is_none(),
        "escape with no modals clears toast"
    );
}

#[test]
fn escape_pomodoro_finalizes_and_clears_session() {
    let (_tmp, mut app) = test_app_with_writable_store();
    app.update(Action::OpenPomodoro);
    assert_eq!(app.modals.top_modal(), Some(ModalLayer::Pomodoro));
    app.update(Action::EscapeContext);
    assert_eq!(app.modals.top_modal(), None);
    assert!(app.modals.pomodoro_session_id.is_none());
}
