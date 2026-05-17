//! Task recurrence Action handlers (E3).

use crate::action::Action;
use crate::app::{ro_msg, AppState};
use rondo_core::domain::task::{RecurFreq, TaskPatch};

pub fn request_edit(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("recurrence"));
    } else if app.data.selected_task_id().is_some() {
        app.modals.open_edit_recurrence();
    }
}

pub fn submit(app: &mut AppState, freq: RecurFreq, interval: i64) {
    let task_id = app.data.selected_task_id();
    app.modals.close_edit_recurrence();
    let Some(id) = task_id else { return };
    let patch = TaskPatch {
        recur_freq: Some(freq),
        recur_interval: Some(interval.max(0)),
        ..Default::default()
    };
    match app.data.store.update_task(id, patch) {
        Ok(snap) => {
            app.undo.push(snap);
            app.patch_task(id);
            match freq {
                RecurFreq::None => app.toast("recur: cleared"),
                _ => app.toast(format!("recur: every {} {:?}", interval.max(1), freq)),
            }
        }
        Err(e) => app.toast(format!("recurrence failed: {}", e)),
    }
}

pub fn cancel(app: &mut AppState) {
    app.modals.close_edit_recurrence();
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestEditRecurrence => {
            request_edit(app);
            true
        }
        Action::SubmitRecurrence(f, i) => {
            submit(app, f, i);
            true
        }
        Action::CancelEditRecurrence => {
            cancel(app);
            true
        }
        _ => false,
    }
}
