//! Subtask Action handlers.

use crate::action::Action;
use crate::app::{ro_msg, AppState};
use crate::focus::Mode;

pub fn request_add(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("subtask"));
    } else if app.data.selected_task_id().is_some() {
        app.modals.open_add_subtask();
        app.ui.mode = Mode::Insert;
    }
}

pub fn submit_add(app: &mut AppState, title: String) {
    let trimmed = title.trim().to_string();
    if !trimmed.is_empty() {
        if let Some(task_id) = app.data.selected_task_id() {
            match app.data.store.add_subtask(task_id, &trimmed) {
                Ok((_id, snap)) => {
                    app.undo.push(snap);
                    app.refresh_tasks();
                    app.toast("subtask added");
                }
                Err(e) => app.toast(format!("subtask failed: {}", e)),
            }
        }
    }
    app.modals.close_add_subtask();
    app.ui.mode = Mode::Normal;
}

pub fn cancel_add(app: &mut AppState) {
    app.modals.close_add_subtask();
    app.ui.mode = Mode::Normal;
}

pub fn request_edit_focused(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("subtask"));
    } else if let Some(task) = app.data.selected_task() {
        if let Some(sub) = task.subtasks.get(app.ui.focus.section_item) {
            let id = sub.id;
            let title = sub.title.clone();
            app.modals.open_edit_subtask(id, title);
            app.ui.mode = Mode::Insert;
        }
    }
}

pub fn submit_edit(app: &mut AppState, new_title: String) {
    let trimmed = new_title.trim().to_string();
    let sub_id = app.modals.edit_subtask_id;
    app.modals.close_edit_subtask();
    app.ui.mode = Mode::Normal;
    if !trimmed.is_empty() {
        if let Some(id) = sub_id {
            match app.data.store.update_subtask_title(id, &trimmed) {
                Ok(_) => {
                    app.refresh_tasks();
                    app.toast("subtask renamed");
                }
                Err(e) => app.toast(format!("rename failed: {}", e)),
            }
        }
    }
}

pub fn cancel_edit(app: &mut AppState) {
    app.modals.close_edit_subtask();
    app.ui.mode = Mode::Normal;
}

pub fn request_delete_focused(app: &mut AppState) {
    if !app.writable {
        app.toast("subtask: read-only");
        return;
    }
    let Some(task) = app.data.selected_task() else {
        return;
    };
    let Some(sub) = task.subtasks.get(app.ui.focus.section_item) else {
        return;
    };
    let sub_clone = sub.clone();
    let sub_id = sub.id;
    let task_id = task.id;
    match app.data.store.delete_subtask(sub_id) {
        Ok(_) => {
            app.undo
                .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                    rondo_core::domain::task::UndoKind::DeleteSubtask {
                        task_id,
                        subtask: sub_clone,
                    },
                ));
            app.refresh_tasks();
            let total = app
                .data
                .selected_task()
                .map(|t| t.subtasks.len())
                .unwrap_or(0);
            if app.ui.focus.section_item >= total && total > 0 {
                app.ui.focus.section_item = total - 1;
            }
            app.toast(format!("deleted subtask #{}", sub_id));
        }
        Err(e) => app.toast(format!("delete failed: {}", e)),
    }
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestAddSubtask => {
            request_add(app);
            true
        }
        Action::SubmitAddSubtask(t) => {
            submit_add(app, t);
            true
        }
        Action::CancelAddSubtask => {
            cancel_add(app);
            true
        }
        Action::RequestEditFocusedSubtask => {
            request_edit_focused(app);
            true
        }
        Action::SubmitEditSubtask(t) => {
            submit_edit(app, t);
            true
        }
        Action::CancelEditSubtask => {
            cancel_edit(app);
            true
        }
        Action::RequestDeleteFocusedSubtask => {
            request_delete_focused(app);
            true
        }
        _ => false,
    }
}
