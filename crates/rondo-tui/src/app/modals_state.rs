use crate::action::Action;
use std::time::{Duration, Instant};
use throbber_widgets_tui::ThrobberState;

/// Modal/overlay UI state and associated buffers.
pub struct ModalsState {
    pub pomodoro_open: bool,
    pub pomodoro_started: Option<Instant>,
    pub pomodoro_total: Duration,
    pub pomodoro_throbber: ThrobberState,
    pub command_palette_open: bool,
    pub command_buf: String,
    pub help_open: bool,
    pub search_open: bool,
    pub search_buf: String,
    pub quick_actions_open: bool,
    pub quick_add_open: bool,
    pub quick_add_buf: String,
}

impl Default for ModalsState {
    fn default() -> Self {
        Self {
            pomodoro_open: false,
            pomodoro_started: None,
            pomodoro_total: Duration::from_secs(25 * 60),
            pomodoro_throbber: ThrobberState::default(),
            command_palette_open: false,
            command_buf: String::new(),
            help_open: false,
            search_open: false,
            search_buf: String::new(),
            quick_actions_open: false,
            quick_add_open: false,
            quick_add_buf: String::new(),
        }
    }
}

impl ModalsState {
    /// Any modal open?
    pub fn any_open(&self) -> bool {
        self.pomodoro_open
            || self.command_palette_open
            || self.help_open
            || self.search_open
            || self.quick_actions_open
            || self.quick_add_open
    }

    /// Pure modal mutations that don't need cross-substate access.
    /// Returns optional follow-up for the dispatcher.
    pub fn update(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::OpenHelp | Action::ToggleHelp => {
                self.help_open = !self.help_open;
                None
            }
            Action::CloseHelp => {
                self.help_open = false;
                None
            }
            Action::OpenSearch => {
                self.search_open = true;
                self.search_buf.clear();
                None
            }
            Action::CloseSearch => {
                self.search_open = false;
                self.search_buf.clear();
                None
            }
            Action::SearchUpdate(s) => {
                self.search_buf = s;
                None
            }
            Action::SearchInput(s) => {
                self.command_buf = s;
                None
            }
            Action::OpenCommandPalette => {
                self.command_palette_open = true;
                self.command_buf.clear();
                None
            }
            Action::CloseCommandPalette => {
                self.command_palette_open = false;
                None
            }
            Action::ToggleQuickActions => {
                self.quick_actions_open = !self.quick_actions_open;
                None
            }
            Action::CloseQuickActions => {
                self.quick_actions_open = false;
                None
            }
            Action::QuickAddUpdate(s) => {
                self.quick_add_buf = s;
                None
            }
            _ => None,
        }
    }
}
