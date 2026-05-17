//! Task due-date Action handlers.

use crate::action::Action;
use crate::app::{ro_msg, AppState};
use crate::focus::Mode;

pub fn request_edit(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("due"));
    } else if app.data.selected_task_id().is_some() {
        app.modals.open_edit_due_date();
    }
}

pub fn submit(app: &mut AppState, date: Option<chrono::NaiveDate>) {
    let task_id = app.data.selected_task_id();
    app.modals.close_edit_due_date();
    app.ui.mode = Mode::Normal;
    let Some(id) = task_id else { return };
    let patch = rondo_core::domain::task::TaskPatch {
        due_date: Some(date),
        ..Default::default()
    };
    match app.data.store.update_task(id, patch) {
        Ok(snap) => {
            app.undo.push(snap);
            app.patch_task(id);
            match date {
                Some(d) => app.toast(format!("due: {}", d.format("%Y-%m-%d"))),
                None => app.toast("due: cleared"),
            }
        }
        Err(e) => app.toast(format!("due-date failed: {}", e)),
    }
}

pub fn cancel(app: &mut AppState) {
    app.modals.close_edit_due_date();
    app.ui.mode = Mode::Normal;
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestEditDueDate => {
            request_edit(app);
            true
        }
        Action::SubmitDueDate(d) => {
            submit(app, d);
            true
        }
        Action::CancelEditDueDate => {
            cancel(app);
            true
        }
        Action::EditDueDateInput(s) => {
            app.modals.edit_due_date_buf = s;
            app.modals.edit_due_date_custom_mode = true;
            true
        }
        _ => false,
    }
}
