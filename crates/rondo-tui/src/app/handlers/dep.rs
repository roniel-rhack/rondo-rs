//! Task dependency Action handlers.

use crate::action::Action;
use crate::app::{modals_state::DepOverlayMode, ro_msg, AppState};
use crate::focus::Mode;

pub fn request_add(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("dep"));
    } else if app.data.selected_task_id().is_some() {
        app.modals.open_dep_overlay(DepOverlayMode::Add);
        app.ui.mode = Mode::Insert;
    }
}

pub fn submit_add(app: &mut AppState, buf: String) {
    let parsed = buf.trim().parse::<i64>();
    match (parsed, app.data.selected_task_id()) {
        (Ok(blocker), Some(task_id)) if blocker > 0 && blocker != task_id => {
            match app.data.store.add_dependency(task_id, blocker) {
                Ok(()) => {
                    app.undo
                        .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                            rondo_core::domain::task::UndoKind::AddDep {
                                task_id,
                                blocker_id: blocker,
                            },
                        ));
                    app.refresh_tasks();
                    app.toast(format!("dep added: #{} blocks #{}", blocker, task_id));
                }
                Err(rondo_core::error::Error::CycleDetected(a, b)) => {
                    app.toast(format!("can't add: would create cycle #{a} → #{b}"));
                }
                Err(e) => app.toast(format!("dep add failed: {}", e)),
            }
        }
        (Ok(_), _) => app.toast("dep: invalid id"),
        (Err(_), _) => app.toast("dep: enter a numeric task id"),
    }
    app.modals.close_dep_overlay();
    app.ui.mode = Mode::Normal;
}

pub fn submit_remove(app: &mut AppState, buf: String) {
    let parsed = buf.trim().parse::<i64>();
    match (parsed, app.data.selected_task_id()) {
        (Ok(blocker), Some(task_id)) => match app.data.store.remove_dependency(task_id, blocker) {
            Ok(()) => {
                app.undo
                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                        rondo_core::domain::task::UndoKind::RemoveDep {
                            task_id,
                            blocker_id: blocker,
                        },
                    ));
                app.refresh_tasks();
                app.toast(format!("dep removed: #{}", blocker));
            }
            Err(e) => app.toast(format!("dep remove failed: {}", e)),
        },
        _ => app.toast("dep: enter a numeric task id"),
    }
    app.modals.close_dep_overlay();
    app.ui.mode = Mode::Normal;
}

pub fn cancel(app: &mut AppState) {
    app.modals.close_dep_overlay();
    app.ui.mode = Mode::Normal;
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestAddDependency => {
            request_add(app);
            true
        }
        Action::SubmitAddDependency(buf) => {
            submit_add(app, buf);
            true
        }
        Action::SubmitRemoveDependency(buf) => {
            submit_remove(app, buf);
            true
        }
        Action::CancelDepOverlay => {
            cancel(app);
            true
        }
        Action::ToggleDepOverlayMode => {
            // Handled inside ModalsState::update; consume here so the main
            // match no longer needs an arm for it.
            true
        }
        _ => false,
    }
}
