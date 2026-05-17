//! Pomodoro overlay Action handlers.

use crate::action::Action;
use crate::app::AppState;
use std::time::Instant;

pub fn toggle_or_open(app: &mut AppState, open_only: bool) {
    let was_open = app.modals.pomodoro_open;
    app.modals.pomodoro_open = !was_open || open_only;
    if app.modals.pomodoro_open && app.modals.pomodoro_started.is_none() {
        app.modals.pomodoro_started = Some(Instant::now());
        app.persist_pomodoro_start();
    }
    if !app.modals.pomodoro_open {
        app.finalize_pomodoro_close();
    }
    if app.modals.pomodoro_open && !was_open && app.ui.last_pomodoro_rect.width > 0 {
        let eff = crate::fx::presets::pomodoro_open(app.theme.accent);
        app.fx.spawn(
            crate::fx::EffectId::PomodoroOpen,
            eff,
            app.ui.last_pomodoro_rect,
        );
    }
}

pub fn close(app: &mut AppState) {
    app.modals.pomodoro_open = false;
    app.finalize_pomodoro_close();
}

/// Returns `true` if the action was a pomodoro action.
pub fn handle(app: &mut AppState, action: &Action) -> bool {
    match action {
        Action::TogglePomodoro => {
            toggle_or_open(app, false);
            true
        }
        Action::OpenPomodoro => {
            toggle_or_open(app, true);
            true
        }
        Action::ClosePomodoro => {
            close(app);
            true
        }
        _ => false,
    }
}
