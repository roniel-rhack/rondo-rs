//! Task-note Action handlers (add, edit, delete, submit).

use crate::action::Action;
use crate::app::AppState;
use crate::focus::Mode;

pub fn request_add(app: &mut AppState) {
    if !app.writable {
        app.toast("note: read-only");
    } else if let Some(id) = app.data.selected_task_id() {
        app.modals.open_note_editor(id, None);
        app.ui.mode = Mode::Insert;
    }
}

pub fn request_edit_focused(app: &mut AppState) {
    if !app.writable {
        app.toast("note: read-only");
        return;
    }
    let Some(task) = app.data.selected_task() else {
        return;
    };
    let Some(note) = task.notes.get(app.ui.focus.section_item) else {
        return;
    };
    let task_id = task.id;
    let note_id = note.id;
    let body = note.body.clone();
    app.modals.open_note_editor(task_id, Some((note_id, &body)));
    app.ui.mode = Mode::Insert;
}

pub fn request_delete_focused(app: &mut AppState) {
    if !app.writable {
        app.toast("note: read-only");
        return;
    }
    let Some(task) = app.data.selected_task() else {
        return;
    };
    let Some(note) = task.notes.get(app.ui.focus.section_item) else {
        return;
    };
    let note_clone = note.clone();
    let task_id = task.id;
    let note_id = note.id;
    match app.data.store.delete_task_note(note_id) {
        Ok(_) => {
            app.undo
                .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                    rondo_core::domain::task::UndoKind::DeleteNote {
                        task_id,
                        note: note_clone,
                    },
                ));
            app.refresh_tasks();
            let total = app.data.selected_task().map(|t| t.notes.len()).unwrap_or(0);
            if app.ui.focus.section_item >= total && total > 0 {
                app.ui.focus.section_item = total - 1;
            }
            app.toast(format!("deleted note #{}", note_id));
        }
        Err(e) => app.toast(format!("delete failed: {}", e)),
    }
}

pub fn editor_key(app: &mut AppState, k: crossterm::event::KeyEvent) {
    app.modals
        .note_textarea
        .input(tui_textarea::Input::from(crossterm::event::Event::Key(k)));
}

pub fn submit(app: &mut AppState) {
    let body = app.modals.note_textarea.lines().join("\n");
    let editing = app.modals.note_editing_id;
    let task_id = app.modals.note_task_id;
    app.modals.close_note_editor();
    app.ui.mode = Mode::Normal;
    if body.trim().is_empty() {
        return;
    }
    match (editing, task_id) {
        (Some(note_id), Some(tid)) => {
            let before_body = app
                .data
                .selected_task()
                .and_then(|t| t.notes.iter().find(|n| n.id == note_id))
                .map(|n| n.body.clone());
            match app.data.store.update_task_note(note_id, &body) {
                Ok(_) => {
                    if let Some(before) = before_body {
                        app.undo
                            .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                                rondo_core::domain::task::UndoKind::UpdateNote {
                                    task_id: tid,
                                    note_id,
                                    before,
                                },
                            ));
                    }
                    app.refresh_tasks();
                    app.toast("note updated");
                }
                Err(e) => app.toast(format!("note failed: {}", e)),
            }
        }
        (None, Some(tid)) => match app.data.store.add_task_note(tid, &body) {
            Ok(note_id) => {
                app.undo
                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                        rondo_core::domain::task::UndoKind::AddNote {
                            task_id: tid,
                            note_id,
                        },
                    ));
                app.refresh_tasks();
                app.toast("note added");
            }
            Err(e) => app.toast(format!("note failed: {}", e)),
        },
        _ => {}
    }
}

pub fn cancel(app: &mut AppState) {
    app.modals.close_note_editor();
    app.ui.mode = Mode::Normal;
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::RequestAddNote => {
            request_add(app);
            true
        }
        Action::RequestEditFocusedNote => {
            request_edit_focused(app);
            true
        }
        Action::RequestDeleteFocusedNote => {
            request_delete_focused(app);
            true
        }
        Action::NoteEditorKey(k) => {
            editor_key(app, k);
            true
        }
        Action::SubmitNote => {
            submit(app);
            true
        }
        Action::CancelNote => {
            cancel(app);
            true
        }
        _ => false,
    }
}
