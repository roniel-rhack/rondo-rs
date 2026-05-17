//! Journal-page Action handlers.

use crate::action::{Action, Page};
use crate::app::AppState;
use crate::focus::Mode;

pub fn start_entry(app: &mut AppState) {
    if app.ui.page == Page::Journal {
        app.modals.open_journal_editor(None);
        app.ui.mode = Mode::Insert;
    }
}

pub fn edit_focused_entry(app: &mut AppState) {
    if app.ui.page == Page::Journal && !app.data.journal_entries.is_empty() {
        let idx = app
            .data
            .selected_journal_entry
            .min(app.data.journal_entries.len() - 1);
        let entry = &app.data.journal_entries[idx];
        let id = entry.id;
        let body = entry.body.clone();
        app.modals.open_journal_editor(Some((id, &body)));
        app.ui.mode = Mode::Insert;
    }
}

pub fn editor_key(app: &mut AppState, k: crossterm::event::KeyEvent) {
    let input = tui_textarea::Input::from(crossterm::event::Event::Key(k));
    app.modals.journal_textarea.input(input);
}

pub fn next_entry(app: &mut AppState) {
    let n = app.data.journal_entries.len();
    if n > 0 {
        app.data.selected_journal_entry = (app.data.selected_journal_entry + 1).min(n - 1);
    }
}

pub fn prev_entry(app: &mut AppState) {
    if app.data.selected_journal_entry > 0 {
        app.data.selected_journal_entry -= 1;
    }
}

pub fn cancel_entry(app: &mut AppState) {
    app.modals.close_journal_editor();
    app.ui.mode = Mode::Normal;
}

pub fn toggle_hidden(app: &mut AppState) {
    app.data.journal_show_hidden = !app.data.journal_show_hidden;
    app.data.refresh_journal_notes();
    let label = if app.data.journal_show_hidden {
        "showing hidden"
    } else {
        "hiding hidden"
    };
    app.toast(format!("journal: {}", label));
}

pub fn goto_top(app: &mut AppState) {
    if app.ui.page == Page::Journal && !app.data.journal_notes.is_empty() {
        app.data.selected_journal = 0;
        app.data.journal_list_state.select(Some(0));
        app.data.reload_journal_entries();
    }
}

pub fn goto_bottom(app: &mut AppState) {
    if app.ui.page == Page::Journal && !app.data.journal_notes.is_empty() {
        let last = app.data.journal_notes.len() - 1;
        app.data.selected_journal = last;
        app.data.journal_list_state.select(Some(last));
        app.data.reload_journal_entries();
    }
}

/// Dispatch a Journal* action group to the correct handler.
/// Returns `true` if the action was handled here.
pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action.clone() {
        Action::JournalStartEntry => {
            start_entry(app);
            true
        }
        Action::JournalEditFocusedEntry => {
            edit_focused_entry(app);
            true
        }
        Action::JournalEditorKey(k) => {
            editor_key(app, k);
            true
        }
        Action::JournalNextEntry => {
            next_entry(app);
            true
        }
        Action::JournalPrevEntry => {
            prev_entry(app);
            true
        }
        Action::JournalSubmitEntry => {
            app.submit_journal_entry();
            true
        }
        Action::JournalCancelEntry => {
            cancel_entry(app);
            true
        }
        Action::JournalDeleteDay => {
            app.delete_focused_journal_day();
            true
        }
        Action::JournalToggleHidden => {
            toggle_hidden(app);
            true
        }
        Action::JournalGotoTop => {
            goto_top(app);
            true
        }
        Action::JournalGotoBottom => {
            goto_bottom(app);
            true
        }
        Action::JournalDeleteEntry => {
            app.delete_focused_journal_entry();
            true
        }
        Action::JournalNextDay => {
            app.move_journal_day(1);
            true
        }
        Action::JournalPrevDay => {
            app.move_journal_day(-1);
            true
        }
        _ => false,
    }
}
