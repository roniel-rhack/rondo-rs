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
    pub selection: std::collections::HashSet<i64>,
    pub quick_actions_open: bool,
    pub quick_add_open: bool,
    pub quick_add_buf: String,
    pub should_quit: bool,
    pub status_msg: Option<String>,
    pub plugins: PluginRegistry,
    pub store: Arc<rondo_core::store::sqlite::SqliteStore>,
    pub flash: Option<(FlashTarget, Instant)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashTarget {
    Task(i64),
    Subtask(i64),
}

pub const FLASH_DURATION_MS: u128 = 220;

impl AppState {
    /// Returns true when an animation requires periodic redraw without user input.
    pub fn needs_animation_tick(&self) -> bool {
        self.pomodoro_open || self.flash.is_some()
    }

    /// Is `target` currently flashing? Clears the flash if expired.
    pub fn is_flashing(&self, target: FlashTarget) -> bool {
        match self.flash {
            Some((t, when)) if t == target => when.elapsed().as_millis() < FLASH_DURATION_MS,
            _ => false,
        }
    }

    /// Periodic housekeeping (called on tick).
    pub fn expire_flash(&mut self) {
        if let Some((_, when)) = self.flash {
            if when.elapsed().as_millis() >= FLASH_DURATION_MS {
                self.flash = None;
            }
        }
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
            selection: std::collections::HashSet::new(),
            quick_actions_open: false,
            quick_add_open: false,
            quick_add_buf: String::new(),
            should_quit: false,
            status_msg: None,
            plugins: PluginRegistry::new(),
            store,
            flash: None,
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
                self.expire_flash();
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
            Action::ToggleQuickActions => self.quick_actions_open = !self.quick_actions_open,
            Action::CloseQuickActions => self.quick_actions_open = false,
            Action::EnterVisual => {
                if self.focus.pane == Pane::List && self.page == Page::Tasks {
                    self.mode = Mode::Visual;
                    self.selection.clear();
                    if let Some(t) = self.tasks.get(self.selected_task) {
                        self.selection.insert(t.id);
                    }
                }
            }
            Action::BulkDone => {
                if self.mode == Mode::Visual {
                    let ids: Vec<i64> = self.selection.iter().copied().collect();
                    for t in self.tasks.iter_mut() {
                        if ids.contains(&t.id) {
                            t.status = match t.status {
                                rondo_core::domain::task::Status::Done => {
                                    rondo_core::domain::task::Status::Pending
                                }
                                _ => rondo_core::domain::task::Status::Done,
                            };
                        }
                    }
                    if let Some(first) = ids.first() {
                        self.flash = Some((FlashTarget::Task(*first), Instant::now()));
                    }
                    self.status_msg = Some(format!(
                        "toggled {} tasks (in-memory)",
                        self.selection.len()
                    ));
                    self.selection.clear();
                    self.mode = Mode::Normal;
                }
            }
            Action::BulkPriority => {
                // Placeholder: cycles priority on all selected.
                if self.mode == Mode::Visual {
                    let ids: Vec<i64> = self.selection.iter().copied().collect();
                    for t in self.tasks.iter_mut() {
                        if ids.contains(&t.id) {
                            t.priority = match t.priority {
                                rondo_core::domain::task::Priority::Low => {
                                    rondo_core::domain::task::Priority::Med
                                }
                                rondo_core::domain::task::Priority::Med => {
                                    rondo_core::domain::task::Priority::High
                                }
                                rondo_core::domain::task::Priority::High => {
                                    rondo_core::domain::task::Priority::Urgent
                                }
                                rondo_core::domain::task::Priority::Urgent => {
                                    rondo_core::domain::task::Priority::Low
                                }
                            };
                        }
                    }
                    self.status_msg = Some(format!(
                        "bumped priority on {} tasks",
                        self.selection.len()
                    ));
                }
            }
            Action::OpenQuickAdd => {
                self.quick_add_open = true;
                self.quick_add_buf.clear();
                self.mode = Mode::Insert;
            }
            Action::QuickAddUpdate(s) => self.quick_add_buf = s,
            Action::SubmitQuickAdd(raw) => self.submit_quick_add(raw),
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
                } else if self.quick_actions_open {
                    self.quick_actions_open = false;
                } else if self.quick_add_open {
                    self.quick_add_open = false;
                    self.quick_add_buf.clear();
                    self.mode = Mode::Normal;
                } else if self.command_palette_open {
                    self.command_palette_open = false;
                } else if self.search_open {
                    self.search_open = false;
                    self.search_buf.clear();
                } else if self.mode == Mode::Visual {
                    self.mode = Mode::Normal;
                    self.selection.clear();
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
                if self.mode == Mode::Visual {
                    if let Some(t) = self.tasks.get(self.selected_task) {
                        self.selection.insert(t.id);
                    }
                }
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
            let mut flashed: Option<i64> = None;
            if let Some(task) = self.tasks.get_mut(self.selected_task) {
                if let Some(st) = task.subtasks.get_mut(item_idx) {
                    st.completed = !st.completed;
                    flashed = Some(st.id);
                    let _ = task_id;
                }
            }
            if let Some(id) = flashed {
                self.flash = Some((FlashTarget::Subtask(id), Instant::now()));
                self.status_msg = Some(format!(
                    "toggled subtask #{} (in-memory; read-only store)",
                    id
                ));
            }
        }
    }

    fn submit_quick_add(&mut self, raw: String) {
        self.quick_add_open = false;
        self.mode = Mode::Normal;
        let parsed = parse_quick_add(&raw);
        self.quick_add_buf.clear();
        if parsed.title.is_empty() {
            return;
        }
        self.status_msg = Some(format!(
            "queued: '{}' (tags={:?} prio={:?} due={:?}) — read-only store",
            parsed.title, parsed.tags, parsed.priority, parsed.due
        ));
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

#[derive(Debug, Default)]
pub struct QuickAddInput {
    pub title: String,
    pub tags: Vec<String>,
    pub priority: Option<rondo_core::domain::task::Priority>,
    pub due: Option<String>,
}

/// Parse quick-add syntax: `title with words #tag1 #tag2 !p3 due:tmrw`.
pub fn parse_quick_add(raw: &str) -> QuickAddInput {
    let mut out = QuickAddInput::default();
    let mut title_parts: Vec<&str> = Vec::new();
    for token in raw.split_whitespace() {
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.is_empty() {
                out.tags.push(tag.to_string());
            }
        } else if let Some(prio) = token.strip_prefix('!') {
            out.priority = match prio.to_lowercase().as_str() {
                "p1" | "low" => Some(rondo_core::domain::task::Priority::Low),
                "p2" | "med" => Some(rondo_core::domain::task::Priority::Med),
                "p3" | "high" => Some(rondo_core::domain::task::Priority::High),
                "p4" | "urg" | "urgent" => Some(rondo_core::domain::task::Priority::Urgent),
                _ => None,
            };
        } else if let Some(due) = token.strip_prefix("due:") {
            out.due = Some(due.to_string());
        } else {
            title_parts.push(token);
        }
    }
    out.title = title_parts.join(" ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_core::domain::task::Priority;

    #[test]
    fn quick_add_parses_all_fields() {
        let p = parse_quick_add("Refactor auth #work #backend !p3 due:tmrw");
        assert_eq!(p.title, "Refactor auth");
        assert_eq!(p.tags, vec!["work", "backend"]);
        assert_eq!(p.priority, Some(Priority::High));
        assert_eq!(p.due.as_deref(), Some("tmrw"));
    }

    #[test]
    fn quick_add_title_only() {
        let p = parse_quick_add("just a title");
        assert_eq!(p.title, "just a title");
        assert!(p.tags.is_empty());
        assert!(p.priority.is_none());
        assert!(p.due.is_none());
    }
}
