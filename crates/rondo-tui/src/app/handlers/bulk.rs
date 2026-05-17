//! Bulk-op Action handlers (E10).
//!
//! Triggered from visual mode (`v` then pick a verb). Every handler
//! pushes one undo snapshot per affected task so `Ctrl+Z` rewinds the
//! bulk action row-by-row.

use crate::action::Action;
use crate::app::AppState;
use crate::focus::Mode;
use rondo_core::domain::task::{Status, TaskPatch};

fn selection(app: &AppState) -> Vec<i64> {
    app.ui.selection.iter().copied().collect()
}

fn finish(app: &mut AppState) {
    app.refresh_tasks();
    app.ui.selection.clear();
    app.ui.mode = Mode::Normal;
}

pub fn delete(app: &mut AppState) {
    if app.ui.mode != Mode::Visual {
        return;
    }
    if !app.writable {
        app.toast("bulk delete: read-only");
        app.ui.mode = Mode::Normal;
        app.ui.selection.clear();
        return;
    }
    let ids = selection(app);
    let mut ok = 0usize;
    let mut err: Option<String> = None;
    for id in &ids {
        match app.data.store.delete_task(*id) {
            Ok(snap) => {
                app.undo.push(snap);
                ok += 1;
            }
            Err(e) => err = Some(format!("{}", e)),
        }
    }
    finish(app);
    match err {
        Some(e) => app.toast(format!("bulk delete: {} ok, error: {}", ok, e)),
        None => app.toast(format!("deleted {} tasks", ok)),
    }
}

pub fn set_status(app: &mut AppState, status: Status) {
    if app.ui.mode != Mode::Visual {
        return;
    }
    if !app.writable {
        app.toast("bulk status: read-only");
        app.ui.mode = Mode::Normal;
        app.ui.selection.clear();
        return;
    }
    let ids = selection(app);
    let mut ok = 0usize;
    let mut err: Option<String> = None;
    for id in &ids {
        match app.data.store.set_status(*id, status) {
            Ok(snap) => {
                app.undo.push(snap);
                ok += 1;
            }
            Err(e) => err = Some(format!("{}", e)),
        }
    }
    finish(app);
    match err {
        Some(e) => app.toast(format!("bulk status: {} ok, error: {}", ok, e)),
        None => app.toast(format!("set status on {} tasks", ok)),
    }
}

pub fn set_due_date(app: &mut AppState, date: Option<chrono::NaiveDate>) {
    if app.ui.mode != Mode::Visual {
        return;
    }
    if !app.writable {
        app.toast("bulk due: read-only");
        app.ui.mode = Mode::Normal;
        app.ui.selection.clear();
        return;
    }
    let ids = selection(app);
    let mut ok = 0usize;
    let mut err: Option<String> = None;
    for id in &ids {
        let patch = TaskPatch {
            due_date: Some(date),
            ..Default::default()
        };
        match app.data.store.update_task(*id, patch) {
            Ok(snap) => {
                app.undo.push(snap);
                ok += 1;
            }
            Err(e) => err = Some(format!("{}", e)),
        }
    }
    finish(app);
    match err {
        Some(e) => app.toast(format!("bulk due: {} ok, error: {}", ok, e)),
        None => app.toast(format!("set due on {} tasks", ok)),
    }
}

pub fn add_tag(app: &mut AppState, tag: String) {
    if app.ui.mode != Mode::Visual {
        return;
    }
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        app.toast("bulk tag: empty tag");
        app.ui.mode = Mode::Normal;
        app.ui.selection.clear();
        return;
    }
    if !app.writable {
        app.toast("bulk tag: read-only");
        app.ui.mode = Mode::Normal;
        app.ui.selection.clear();
        return;
    }
    let ids = selection(app);
    let mut ok = 0usize;
    let mut err: Option<String> = None;
    for id in &ids {
        match app.data.store.add_tag(*id, &tag) {
            Ok(snap) => {
                app.undo.push(snap);
                ok += 1;
            }
            Err(e) => err = Some(format!("{}", e)),
        }
    }
    finish(app);
    match err {
        Some(e) => app.toast(format!("bulk tag: {} ok, error: {}", ok, e)),
        None => app.toast(format!("tagged {} tasks #{}", ok, tag)),
    }
}

pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::BulkDelete => {
            delete(app);
            true
        }
        Action::BulkSetStatus(s) => {
            set_status(app, s);
            true
        }
        Action::BulkSetDueDate(d) => {
            set_due_date(app, d);
            true
        }
        Action::BulkAddTag(t) => {
            add_tag(app, t);
            true
        }
        _ => false,
    }
}
