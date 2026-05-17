//! Task-level Action handlers (delete, edit title, edit description).

use crate::action::Action;
use crate::app::{ro_msg, AppState};
use crate::focus::Mode;

pub fn request_delete(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("delete"));
    } else if app.data.selected_task_id().is_some() {
        app.modals.open_confirm_delete();
    }
}

pub fn confirm_delete(app: &mut AppState) {
    app.modals.close_confirm_delete();
    if let Some(id) = app.data.selected_task_id() {
        match app.data.store.delete_task(id) {
            Ok(snap) => {
                app.undo.push(snap);
                app.refresh_tasks();
                app.toast("task deleted");
            }
            Err(e) => app.toast(format!("delete failed: {}", e)),
        }
    }
}

pub fn request_edit_title(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("edit"));
    } else if let Some(t) = app.data.selected_task() {
        app.modals.open_edit_title(t.title.clone());
        app.ui.mode = Mode::Insert;
    }
}

pub fn submit_edit_title(app: &mut AppState, new_title: String) {
    let trimmed = new_title.trim().to_string();
    if !trimmed.is_empty() {
        if let Some(id) = app.data.selected_task_id() {
            let patch = rondo_core::domain::task::TaskPatch {
                title: Some(trimmed),
                ..Default::default()
            };
            match app.data.store.update_task(id, patch) {
                Ok(snap) => {
                    app.undo.push(snap);
                    app.refresh_tasks();
                    app.toast("title updated");
                }
                Err(e) => app.toast(format!("update failed: {}", e)),
            }
        }
    }
    app.modals.close_edit_title();
    app.ui.mode = Mode::Normal;
}

pub fn cancel_edit_title(app: &mut AppState) {
    app.modals.close_edit_title();
    app.ui.mode = Mode::Normal;
}

pub fn request_edit_description(app: &mut AppState) {
    if !app.writable {
        app.toast(ro_msg("description"));
    } else if let Some(task) = app.data.selected_task() {
        let body = task.description.clone().unwrap_or_default();
        let task_id = task.id;
        app.modals.open_description_editor(task_id, &body);
        app.ui.mode = Mode::Insert;
    }
}

pub fn description_editor_key(app: &mut AppState, k: crossterm::event::KeyEvent) {
    app.modals
        .description_textarea
        .input(tui_textarea::Input::from(crossterm::event::Event::Key(k)));
}

pub fn submit_edit_description(app: &mut AppState) {
    let body = app.modals.description_textarea.lines().join("\n");
    let task_id = app.modals.description_task_id;
    app.modals.close_description_editor();
    app.ui.mode = Mode::Normal;
    if let Some(id) = task_id {
        let patch = rondo_core::domain::task::TaskPatch {
            description: Some(if body.is_empty() { None } else { Some(body) }),
            ..Default::default()
        };
        match app.data.store.update_task(id, patch) {
            Ok(snap) => {
                app.undo.push(snap);
                app.refresh_tasks();
                app.toast("description updated");
            }
            Err(e) => app.toast(format!("update failed: {}", e)),
        }
    }
}

pub fn cancel_edit_description(app: &mut AppState) {
    app.modals.close_description_editor();
    app.ui.mode = Mode::Normal;
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestDeleteTask => {
            request_delete(app);
            true
        }
        Action::ConfirmDeleteTask => {
            confirm_delete(app);
            true
        }
        Action::CancelDelete => {
            app.modals.close_confirm_delete();
            true
        }
        Action::RequestEditTitle => {
            request_edit_title(app);
            true
        }
        Action::SubmitEditTitle(t) => {
            submit_edit_title(app, t);
            true
        }
        Action::CancelEditTitle => {
            cancel_edit_title(app);
            true
        }
        Action::RequestEditDescription => {
            request_edit_description(app);
            true
        }
        Action::DescriptionEditorKey(k) => {
            description_editor_key(app, k);
            true
        }
        Action::SubmitEditDescription => {
            submit_edit_description(app);
            true
        }
        Action::CancelEditDescription => {
            cancel_edit_description(app);
            true
        }
        _ => false,
    }
}
