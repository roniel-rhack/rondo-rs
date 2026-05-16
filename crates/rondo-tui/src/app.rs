use crate::action::{Action, Page};
use crate::focus::{DetailSection, FocusState, Mode, Pane};
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
    pub focus: FocusState,
    pub mode: Mode,
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
    pub help_open: bool,
    pub search_open: bool,
    pub search_buf: String,
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

    /// Backward-compat shim — still used by some pane-render code paths to color borders.
    pub fn focus_left(&self) -> bool {
        self.focus.pane == Pane::List
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
            focus: FocusState::default(),
            mode: Mode::Normal,
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
            help_open: false,
            search_open: false,
            search_buf: String::new(),
            should_quit: false,
            status_msg: None,
            plugins: PluginRegistry::new(),
            store,
        })
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::JumpTop => self.jump_selection(0),
            Action::JumpBottom => self.jump_selection_end(),
            Action::HalfPageDown => self.move_selection(10),
            Action::HalfPageUp => self.move_selection(-10),
            Action::FocusLeft => {
                self.focus.pane = Pane::List;
                self.focus.section_item = 0;
            }
            Action::FocusRight => {
                self.focus.pane = Pane::Detail;
                self.focus.section_item = 0;
            }
            Action::ResetSplit => self.split_ratio = 50,
            Action::OpenHelp | Action::ToggleHelp => self.help_open = !self.help_open,
            Action::CloseHelp => self.help_open = false,
            Action::OpenSearch => {
                self.search_open = true;
                self.search_buf.clear();
            }
            Action::CloseSearch => {
                self.search_open = false;
                self.search_buf.clear();
            }
            Action::SearchUpdate(s) => self.search_buf = s,
            Action::Tick => {
                if self.pomodoro_open {
                    self.pomodoro_throbber.calc_next();
                }
                self.dispatch_plugin_ticks();
            }
            Action::NextItem => self.move_selection(1),
            Action::PrevItem => self.move_selection(-1),
            Action::TogglePage(p) => self.page = p,
            Action::NextTab => {
                self.page = match self.page {
                    Page::Tasks => Page::Journal,
                    Page::Journal => Page::Tasks,
                };
            }
            Action::PrevTab => {
                self.page = match self.page {
                    Page::Tasks => Page::Journal,
                    Page::Journal => Page::Tasks,
                };
            }
            Action::FocusNext => {
                self.focus.pane = match self.focus.pane {
                    Pane::List => Pane::Detail,
                    Pane::Detail => Pane::List,
                };
                self.focus.section_item = 0;
            }
            Action::NextSection => {
                if self.focus.pane == Pane::Detail {
                    self.focus.section = self.focus.section.next();
                    self.focus.section_item = 0;
                }
            }
            Action::PrevSection => {
                if self.focus.pane == Pane::Detail {
                    self.focus.section = self.focus.section.prev();
                    self.focus.section_item = 0;
                }
            }
            Action::ToggleSelected => self.handle_space(),
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
                if self.help_open {
                    self.help_open = false;
                } else if self.command_palette_open {
                    self.command_palette_open = false;
                } else if self.search_open {
                    self.search_open = false;
                    self.search_buf.clear();
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

    fn jump_selection(&mut self, idx: usize) {
        match self.page {
            Page::Tasks if !self.tasks.is_empty() => {
                self.selected_task = idx.min(self.tasks.len() - 1);
                self.task_list_state.select(Some(self.selected_task));
            }
            Page::Journal if !self.journal_notes.is_empty() => {
                self.selected_journal = idx.min(self.journal_notes.len() - 1);
                self.journal_list_state.select(Some(self.selected_journal));
                self.reload_journal_entries();
            }
            _ => {}
        }
    }

    fn jump_selection_end(&mut self) {
        match self.page {
            Page::Tasks if !self.tasks.is_empty() => self.jump_selection(self.tasks.len() - 1),
            Page::Journal if !self.journal_notes.is_empty() => {
                self.jump_selection(self.journal_notes.len() - 1)
            }
            _ => {}
        }
    }

    fn reload_journal_entries(&mut self) {
        if let Some(n) = self.journal_notes.get(self.selected_journal) {
            if let Ok(e) = self.store.entries_for_note(n.id) {
                self.journal_entries = e;
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.page {
            Page::Tasks => {
                if self.focus.pane == Pane::Detail {
                    self.move_detail_section_item(delta);
                    return;
                }
                if self.tasks.is_empty() {
                    return;
                }
                let len = self.tasks.len() as i32;
                let next = (self.selected_task as i32 + delta).rem_euclid(len);
                self.selected_task = next as usize;
                self.task_list_state.select(Some(self.selected_task));
                self.focus.section_item = 0;
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

    fn move_detail_section_item(&mut self, delta: i32) {
        let len = self.detail_section_len();
        if len == 0 {
            return;
        }
        let len_i = len as i32;
        let next = (self.focus.section_item as i32 + delta).rem_euclid(len_i);
        self.focus.section_item = next as usize;
    }

    fn detail_section_len(&self) -> usize {
        let Some(task) = self.tasks.get(self.selected_task) else {
            return 0;
        };
        match self.focus.section {
            DetailSection::Header => 0,
            DetailSection::Subtasks => task.subtasks.len(),
            DetailSection::Dependencies => task.blocked_by_ids.len() + task.blocks_ids.len(),
            DetailSection::Notes => task.notes.len(),
        }
    }

    /// Space-bar action: meaning depends on focus context.
    fn handle_space(&mut self) {
        if self.focus.pane != Pane::Detail {
            return;
        }
        // Subtask toggle is the headline interaction.
        if self.focus.section == DetailSection::Subtasks {
            let task_id = match self.tasks.get(self.selected_task) {
                Some(t) => t.id,
                None => return,
            };
            let item_idx = self.focus.section_item;
            if let Some(task) = self.tasks.get_mut(self.selected_task) {
                if let Some(st) = task.subtasks.get_mut(item_idx) {
                    st.completed = !st.completed;
                    self.status_msg = Some(format!(
                        "toggled subtask #{} (in-memory; read-only store)",
                        st.id
                    ));
                    let _ = task_id;
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
