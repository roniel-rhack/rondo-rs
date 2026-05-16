use crate::action::{Action, Page};
use crate::theme::Theme;
use color_eyre::eyre::Result;
use ratatui::widgets::ListState;
use throbber_widgets_tui::ThrobberState;
use rondo_core::domain::{
    journal::{Entry, Note},
    task::Task,
};
use rondo_plugin_api::PluginRegistry;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct AppState {
    pub theme: Theme,
    pub page: Page,
    pub tasks: Vec<Task>,
    pub selected_task: usize,
    pub task_list_state: ListState,
    pub focus_left: bool,
    pub split_ratio: u16,
    pub journal_notes: Vec<Note>,
    pub journal_entries: Vec<Entry>,
    pub selected_journal: usize,
    pub journal_list_state: ListState,
    pub pomodoro_open: bool,
    pub pomodoro_started: Option<Instant>,
    pub pomodoro_total: Duration,
    pub pomodoro_throbber: ThrobberState,
    pub command_palette_open: bool,
    pub command_buf: String,
    pub should_quit: bool,
    pub status_msg: Option<String>,
    pub plugins: PluginRegistry,
    pub store: Arc<rondo_core::store::sqlite::SqliteStore>,
}

impl AppState {
    /// Returns true when an animation requires periodic redraw without user input.
    pub fn needs_animation_tick(&self) -> bool {
        self.pomodoro_open
    }
}

impl AppState {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Result<Self> {
        let tasks = store.list_tasks()?;
        let journal_notes = store.list_journal_notes()?;
        let journal_entries = if let Some(n) = journal_notes.first() {
            store.entries_for_note(n.id)?
        } else {
            vec![]
        };
        let mut task_list_state = ListState::default();
        if !tasks.is_empty() {
            task_list_state.select(Some(0));
        }
        let mut journal_list_state = ListState::default();
        if !journal_notes.is_empty() {
            journal_list_state.select(Some(0));
        }
        Ok(Self {
            theme: Theme::dark(),
            page: Page::Tasks,
            tasks,
            selected_task: 0,
            task_list_state,
            focus_left: true,
            split_ratio: 50,
            journal_notes,
            journal_entries,
            selected_journal: 0,
            journal_list_state,
            pomodoro_open: false,
            pomodoro_started: None,
            pomodoro_total: Duration::from_secs(25 * 60),
            pomodoro_throbber: ThrobberState::default(),
            command_palette_open: false,
            command_buf: String::new(),
            should_quit: false,
            status_msg: None,
            plugins: PluginRegistry::new(),
            store,
        })
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Tick => {
                if self.pomodoro_open {
                    self.pomodoro_throbber.calc_next();
                }
                self.dispatch_plugin_ticks();
            }
            Action::NextItem => self.move_selection(1),
            Action::PrevItem => self.move_selection(-1),
            Action::TogglePage(p) => self.page = p,
            Action::FocusNext => self.focus_left = !self.focus_left,
            Action::ResizeSplit { delta } => {
                let new = self.split_ratio as i32 + delta as i32;
                self.split_ratio = new.clamp(20, 80) as u16;
            }
            Action::TogglePomodoro | Action::OpenPomodoro => {
                self.pomodoro_open = !self.pomodoro_open || matches!(action, Action::OpenPomodoro);
                if self.pomodoro_open && self.pomodoro_started.is_none() {
                    self.pomodoro_started = Some(Instant::now());
                }
                if !self.pomodoro_open {
                    self.pomodoro_started = None;
                }
            }
            Action::ClosePomodoro => {
                self.pomodoro_open = false;
                self.pomodoro_started = None;
            }
            Action::OpenCommandPalette => {
                self.command_palette_open = true;
                self.command_buf.clear();
            }
            Action::CloseCommandPalette => {
                self.command_palette_open = false;
            }
            Action::SearchInput(s) => self.command_buf = s,
            Action::SubmitCommand(cmd) => self.handle_command(cmd),
            Action::EscapeContext => {
                if self.command_palette_open {
                    self.command_palette_open = false;
                } else if self.pomodoro_open {
                    self.pomodoro_open = false;
                    self.pomodoro_started = None;
                } else if self.status_msg.is_some() {
                    self.status_msg = None;
                }
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.page {
            Page::Tasks => {
                if self.tasks.is_empty() {
                    return;
                }
                let len = self.tasks.len() as i32;
                let next = (self.selected_task as i32 + delta).rem_euclid(len);
                self.selected_task = next as usize;
                self.task_list_state.select(Some(self.selected_task));
            }
            Page::Journal => {
                if self.journal_notes.is_empty() {
                    return;
                }
                let len = self.journal_notes.len() as i32;
                let next = (self.selected_journal as i32 + delta).rem_euclid(len);
                self.selected_journal = next as usize;
                self.journal_list_state.select(Some(self.selected_journal));
                if let Ok(e) = self
                    .store
                    .entries_for_note(self.journal_notes[self.selected_journal].id)
                {
                    self.journal_entries = e;
                }
            }
        }
    }

    fn handle_command(&mut self, cmd: String) {
        self.command_palette_open = false;
        match cmd.trim() {
            "tasks" => self.page = Page::Tasks,
            "journal" => self.page = Page::Journal,
            "pomodoro" => {
                self.pomodoro_open = true;
                self.pomodoro_started = Some(Instant::now());
            }
            "quit" => self.should_quit = true,
            "" => {}
            other => self.status_msg = Some(format!("unknown: {}", other)),
        }
    }

    fn dispatch_plugin_ticks(&mut self) {
        use rondo_plugin_api::action::PluginAction;
        use rondo_plugin_api::plugin::PluginContext;
        let ctx = PluginContext::now();
        let ids: Vec<&'static str> = self.plugins.iter_meta().map(|m| m.id).collect();
        for id in ids {
            if let Some(p) = self.plugins.get_mut(id) {
                let _ = p.handle(PluginAction::Tick { delta_ms: 100 }, &ctx);
            }
        }
    }
}
