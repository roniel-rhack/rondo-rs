pub mod data_state;
pub mod modals_state;
pub mod ui_state;

pub use data_state::DataState;
pub use modals_state::ModalsState;
pub use ui_state::{FlashTarget, UiState, FLASH_DURATION_MS};

use crate::action::{Action, Page};
use crate::filter::Filter;
use crate::focus::{DetailSection, Mode, Pane};
use crate::theme::Theme;
use color_eyre::eyre::Result;
use rondo_plugin_api::PluginRegistry;
use std::sync::Arc;
use std::time::Instant;

pub struct AppState {
    pub data: DataState,
    pub ui: UiState,
    pub modals: ModalsState,
    pub fx: crate::fx::FxManager,
    pub plugins: PluginRegistry,
    pub theme: Theme,
    pub should_quit: bool,
    pub status_msg: Option<String>,
}

impl AppState {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Result<Self> {
        Ok(Self {
            data: DataState::new(store)?,
            ui: UiState::default(),
            modals: ModalsState::default(),
            fx: crate::fx::FxManager::new(),
            plugins: PluginRegistry::new(),
            theme: Theme::dark(),
            should_quit: false,
            status_msg: None,
        })
    }

    /// Returns true when an animation requires periodic redraw without user input.
    pub fn needs_animation_tick(&self) -> bool {
        self.modals.pomodoro_open || self.ui.flash.is_some() || self.fx.any_running()
    }

    /// Backward-compat shim — still used by some pane-render code paths to color borders.
    pub fn focus_left(&self) -> bool {
        self.ui.focus_left()
    }

    /// Spawn a sweep transition over the body when changing pages.
    fn spawn_page_swap(&mut self) {
        if self.ui.last_body_rect.width > 0 {
            let eff = crate::fx::presets::page_swap(self.theme.bg);
            self.fx
                .spawn(crate::fx::EffectId::PageSwap, eff, self.ui.last_body_rect);
        }
    }

    /// Spawn a status-toast effect on the footer area.
    pub fn toast(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
        if self.ui.last_footer_rect.width > 0 {
            let effect = crate::fx::presets::status_toast(self.theme.accent, self.theme.fg_muted);
            self.fx.spawn(
                crate::fx::EffectId::StatusToast,
                effect,
                self.ui.last_footer_rect,
            );
        }
    }

    pub fn visible_task_indices(&self) -> Vec<usize> {
        self.data.visible_task_indices()
    }

    pub fn visible_count(&self) -> usize {
        self.data.visible_count()
    }

    pub fn is_flashing(&self, target: FlashTarget) -> bool {
        self.ui.is_flashing(target)
    }

    pub fn expire_flash(&mut self) {
        self.ui.expire_flash();
    }

    pub fn update(&mut self, action: Action) {
        // Clear pending leader on any action other than the leader itself.
        if !matches!(action, Action::LeaderGoto) {
            self.ui.leader_goto = false;
        }

        // Route to substate handlers first; they own the pure mutations.
        let mut follow_up = self.ui.update(action.clone());
        if follow_up.is_none() {
            follow_up = self.modals.update(action.clone());
        }
        if follow_up.is_none() {
            follow_up = self.data.update(action.clone());
        }

        // Cross-cutting actions handled here (need access to multiple substates,
        // theme, fx, plugins, etc.).
        match action {
            Action::Quit => self.should_quit = true,
            Action::JumpTop => self.jump_selection(0),
            Action::JumpBottom => self.jump_selection_end(),
            Action::HalfPageDown => self.move_selection(10),
            Action::HalfPageUp => self.move_selection(-10),
            Action::Tick => {
                if self.modals.pomodoro_open {
                    self.modals.pomodoro_throbber.calc_next();
                }
                self.ui.expire_flash();
                self.dispatch_plugin_ticks();
            }
            Action::NextItem => self.move_selection(1),
            Action::PrevItem => self.move_selection(-1),
            Action::TogglePage(p) => {
                if p != self.ui.page {
                    self.ui.page = p;
                    self.spawn_page_swap();
                }
            }
            Action::NextTab | Action::PrevTab => {
                let next = match self.ui.page {
                    Page::Tasks => Page::Journal,
                    Page::Journal => Page::Tasks,
                };
                if next != self.ui.page {
                    self.ui.page = next;
                    self.spawn_page_swap();
                }
            }
            Action::ToggleSelected => self.handle_space(),
            Action::ApplySidebarSelection => {
                if self.ui.focus.pane == Pane::Sidebar {
                    self.apply_sidebar_selection();
                }
            }
            Action::ApplyFilter(f) => self.apply_filter(f),
            Action::EnterVisual => {
                if self.ui.focus.pane == Pane::List && self.ui.page == Page::Tasks {
                    self.ui.mode = Mode::Visual;
                    self.ui.selection.clear();
                    if let Some(t) = self.data.tasks.get(self.data.selected_task) {
                        self.ui.selection.insert(t.id);
                    }
                }
            }
            Action::BulkDone => {
                if self.ui.mode == Mode::Visual {
                    let ids: Vec<i64> = self.ui.selection.iter().copied().collect();
                    for t in self.data.tasks.iter_mut() {
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
                        self.ui.flash = Some((FlashTarget::Task(*first), Instant::now()));
                    }
                    if self.ui.last_task_list_rect.width > 0 {
                        let eff = crate::fx::presets::task_done_sweep(self.theme.fg_muted);
                        self.fx.spawn(
                            crate::fx::EffectId::TaskDone(ids.first().copied().unwrap_or(0)),
                            eff,
                            self.ui.last_task_list_rect,
                        );
                    }
                    let count = self.ui.selection.len();
                    self.toast(format!("toggled {} tasks (in-memory)", count));
                    self.ui.selection.clear();
                    self.ui.mode = Mode::Normal;
                }
            }
            Action::BulkPriority => {
                if self.ui.mode == Mode::Visual {
                    let ids: Vec<i64> = self.ui.selection.iter().copied().collect();
                    for t in self.data.tasks.iter_mut() {
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
                        self.ui.selection.len()
                    ));
                }
            }
            Action::OpenQuickAdd => {
                self.modals.quick_add_open = true;
                self.modals.quick_add_buf.clear();
                self.ui.mode = Mode::Insert;
            }
            Action::SubmitQuickAdd(raw) => self.submit_quick_add(raw),
            Action::TogglePomodoro | Action::OpenPomodoro => {
                let was_open = self.modals.pomodoro_open;
                self.modals.pomodoro_open = !was_open || matches!(action, Action::OpenPomodoro);
                if self.modals.pomodoro_open && self.modals.pomodoro_started.is_none() {
                    self.modals.pomodoro_started = Some(Instant::now());
                }
                if !self.modals.pomodoro_open {
                    self.modals.pomodoro_started = None;
                }
                if self.modals.pomodoro_open && !was_open && self.ui.last_pomodoro_rect.width > 0 {
                    let eff = crate::fx::presets::pomodoro_open(self.theme.accent);
                    self.fx.spawn(
                        crate::fx::EffectId::PomodoroOpen,
                        eff,
                        self.ui.last_pomodoro_rect,
                    );
                }
            }
            Action::ClosePomodoro => {
                self.modals.pomodoro_open = false;
                self.modals.pomodoro_started = None;
            }
            Action::SubmitCommand(cmd) => self.handle_command(cmd),
            Action::JournalStartEntry => {
                if self.ui.page == Page::Journal {
                    self.modals.journal_editor_open = true;
                    self.modals.journal_editor_buf.clear();
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::JournalSubmitEntry => self.submit_journal_entry(),
            Action::JournalCancelEntry => {
                self.modals.journal_editor_open = false;
                self.modals.journal_editor_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::JournalToggleHidden => {
                self.data.journal_show_hidden = !self.data.journal_show_hidden;
                self.data.refresh_journal_notes();
                let label = if self.data.journal_show_hidden {
                    "showing hidden"
                } else {
                    "hiding hidden"
                };
                self.toast(format!("journal: {}", label));
            }
            Action::JournalGotoTop => {
                if self.ui.page == Page::Journal && !self.data.journal_notes.is_empty() {
                    self.data.selected_journal = 0;
                    self.data.journal_list_state.select(Some(0));
                    self.data.reload_journal_entries();
                }
            }
            Action::JournalGotoBottom => {
                if self.ui.page == Page::Journal && !self.data.journal_notes.is_empty() {
                    let last = self.data.journal_notes.len() - 1;
                    self.data.selected_journal = last;
                    self.data.journal_list_state.select(Some(last));
                    self.data.reload_journal_entries();
                }
            }
            Action::JournalDeleteEntry => {
                self.delete_focused_journal_entry();
            }
            Action::EscapeContext => {
                if self.modals.help_open {
                    self.modals.help_open = false;
                } else if self.modals.quick_actions_open {
                    self.modals.quick_actions_open = false;
                } else if self.modals.journal_editor_open {
                    self.modals.journal_editor_open = false;
                    self.modals.journal_editor_buf.clear();
                    self.ui.mode = Mode::Normal;
                } else if self.modals.quick_add_open {
                    self.modals.quick_add_open = false;
                    self.modals.quick_add_buf.clear();
                    self.ui.mode = Mode::Normal;
                } else if self.modals.command_palette_open {
                    self.modals.command_palette_open = false;
                } else if self.modals.search_open {
                    self.modals.search_open = false;
                    self.modals.search_buf.clear();
                } else if self.ui.mode == Mode::Visual {
                    self.ui.mode = Mode::Normal;
                    self.ui.selection.clear();
                } else if self.modals.pomodoro_open {
                    self.modals.pomodoro_open = false;
                    self.modals.pomodoro_started = None;
                } else if self.status_msg.is_some() {
                    self.status_msg = None;
                }
            }
            _ => {}
        }

        // Process any follow-up emitted by substate handlers.
        if let Some(next) = follow_up.take() {
            self.update(next);
        }
    }

    fn jump_selection(&mut self, idx: usize) {
        match self.ui.page {
            Page::Tasks if !self.data.tasks.is_empty() => {
                let prev = self.data.selected_task;
                self.data.selected_task = idx.min(self.data.tasks.len() - 1);
                self.data
                    .task_list_state
                    .select(Some(self.data.selected_task));
                if prev != self.data.selected_task {
                    self.spawn_detail_refresh();
                }
            }
            Page::Journal if !self.data.journal_notes.is_empty() => {
                self.data.selected_journal = idx.min(self.data.journal_notes.len() - 1);
                self.data
                    .journal_list_state
                    .select(Some(self.data.selected_journal));
                self.data.reload_journal_entries();
            }
            _ => {}
        }
    }

    fn jump_selection_end(&mut self) {
        match self.ui.page {
            Page::Tasks if !self.data.tasks.is_empty() => {
                self.jump_selection(self.data.tasks.len() - 1)
            }
            Page::Journal if !self.data.journal_notes.is_empty() => {
                self.jump_selection(self.data.journal_notes.len() - 1)
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i32) {
        match self.ui.page {
            Page::Tasks => {
                if self.ui.focus.pane == Pane::Sidebar {
                    self.move_sidebar(delta);
                    return;
                }
                if self.ui.focus.pane == Pane::Detail {
                    self.move_detail_section_item(delta);
                    return;
                }
                let visible = self.data.visible_task_indices();
                if visible.is_empty() {
                    return;
                }
                let cur = visible
                    .iter()
                    .position(|&i| i == self.data.selected_task)
                    .unwrap_or(0);
                let len = visible.len() as i32;
                let next = (cur as i32 + delta).rem_euclid(len) as usize;
                let new_task = visible[next];
                let changed = new_task != self.data.selected_task;
                self.data.selected_task = new_task;
                // ListState position is relative to visible slice, not full tasks.
                self.data.task_list_state.select(Some(next));
                self.ui.focus.section_item = 0;
                if self.ui.mode == Mode::Visual {
                    if let Some(t) = self.data.tasks.get(self.data.selected_task) {
                        self.ui.selection.insert(t.id);
                    }
                }
                if changed {
                    self.spawn_detail_refresh();
                }
            }
            Page::Journal => {
                if self.data.journal_notes.is_empty() {
                    return;
                }
                let len = self.data.journal_notes.len() as i32;
                let next = (self.data.selected_journal as i32 + delta).rem_euclid(len);
                self.data.selected_journal = next as usize;
                self.data
                    .journal_list_state
                    .select(Some(self.data.selected_journal));
                if let Ok(e) = self
                    .data
                    .store
                    .entries_for_note(self.data.journal_notes[self.data.selected_journal].id)
                {
                    self.data.journal_entries = e;
                }
            }
        }
    }

    fn move_sidebar(&mut self, delta: i32) {
        let total = crate::filter::SIDEBAR_ITEMS.len() as i32;
        let next = (self.ui.focus.sidebar_item as i32 + delta).rem_euclid(total);
        self.ui.focus.sidebar_item = next as usize;
    }

    /// Apply currently-highlighted sidebar item as the active filter.
    pub fn apply_sidebar_selection(&mut self) {
        let idx = self
            .ui
            .focus
            .sidebar_item
            .min(crate::filter::SIDEBAR_ITEMS.len() - 1);
        self.apply_filter(crate::filter::SIDEBAR_ITEMS[idx]);
    }

    /// Switch active filter and reset cursor. Works regardless of current focus.
    pub fn apply_filter(&mut self, new_filter: Filter) {
        self.data.active_filter = new_filter;
        self.toast(format!("filter: {}", new_filter.label()));
        // Move sidebar cursor onto the applied item so visual matches state.
        if let Some(pos) = crate::filter::SIDEBAR_ITEMS
            .iter()
            .position(|f| *f == new_filter)
        {
            self.ui.focus.sidebar_item = pos;
        }
        let visible = self.data.visible_task_indices();
        if let Some(&first) = visible.first() {
            self.data.selected_task = first;
            self.data.task_list_state.select(Some(0));
        } else {
            self.data.task_list_state.select(None);
        }
        self.ui.focus.pane = Pane::List;
    }

    fn move_detail_section_item(&mut self, delta: i32) {
        let len = self.detail_section_len();
        if len == 0 {
            return;
        }
        let len_i = len as i32;
        let next = (self.ui.focus.section_item as i32 + delta).rem_euclid(len_i);
        self.ui.focus.section_item = next as usize;
    }

    fn detail_section_len(&self) -> usize {
        let Some(task) = self.data.tasks.get(self.data.selected_task) else {
            return 0;
        };
        match self.ui.focus.section {
            DetailSection::Header => 0,
            DetailSection::Subtasks => task.subtasks.len(),
            DetailSection::Dependencies => task.blocked_by_ids.len() + task.blocks_ids.len(),
            DetailSection::Notes => task.notes.len(),
        }
    }

    /// Space-bar action: meaning depends on focus context.
    fn handle_space(&mut self) {
        if self.ui.focus.pane != Pane::Detail {
            return;
        }
        if self.ui.focus.section == DetailSection::Subtasks {
            let task_id = match self.data.tasks.get(self.data.selected_task) {
                Some(t) => t.id,
                None => return,
            };
            let item_idx = self.ui.focus.section_item;
            let mut flashed: Option<i64> = None;
            if let Some(task) = self.data.tasks.get_mut(self.data.selected_task) {
                if let Some(st) = task.subtasks.get_mut(item_idx) {
                    st.completed = !st.completed;
                    flashed = Some(st.id);
                    let _ = task_id;
                }
            }
            if let Some(id) = flashed {
                self.ui.flash = Some((FlashTarget::Subtask(id), Instant::now()));
                self.toast(format!("subtask #{} toggled (in-memory)", id));
            }
        }
    }

    /// Trigger the detail-pane refresh animation. Called whenever the cursor
    /// lands on a different task so the detail panel visibly re-paints.
    fn spawn_detail_refresh(&mut self) {
        if self.ui.last_detail_rect.width > 0 {
            let eff = crate::fx::presets::detail_refresh(self.theme.accent);
            self.fx.spawn(
                crate::fx::EffectId::DetailRefresh,
                eff,
                self.ui.last_detail_rect,
            );
        }
    }

    fn submit_quick_add(&mut self, raw: String) {
        self.modals.quick_add_open = false;
        self.ui.mode = Mode::Normal;
        let parsed = parse_quick_add(&raw);
        self.modals.quick_add_buf.clear();
        if parsed.title.is_empty() {
            return;
        }
        if self.ui.last_task_list_rect.width > 0 {
            let eff = crate::fx::presets::quick_add_slide(self.theme.bg);
            self.fx.spawn(
                crate::fx::EffectId::QuickAddInsert,
                eff,
                self.ui.last_task_list_rect,
            );
        }
        self.toast(format!(
            "queued: '{}' (tags={:?} prio={:?} due={:?})",
            parsed.title, parsed.tags, parsed.priority, parsed.due
        ));
    }

    fn submit_journal_entry(&mut self) {
        let body = std::mem::take(&mut self.modals.journal_editor_buf);
        self.modals.journal_editor_open = false;
        self.ui.mode = Mode::Normal;
        if body.trim().is_empty() {
            return;
        }
        match self.data.store.create_or_get_today_note() {
            Ok(note) => match self.data.store.add_journal_entry(note.id, &body) {
                Ok(_) => {
                    self.data.refresh_journal_notes();
                    // Jump cursor to today's note (newest).
                    if let Some(pos) = self.data.journal_notes.iter().position(|n| n.id == note.id)
                    {
                        self.data.selected_journal = pos;
                        self.data.journal_list_state.select(Some(pos));
                        self.data.reload_journal_entries();
                    }
                    self.toast("entry saved".to_string());
                }
                Err(e) => self.toast(format!("save failed: {}", e)),
            },
            Err(e) => self.toast(format!("save failed: {}", e)),
        }
    }

    fn delete_focused_journal_entry(&mut self) {
        let entry_id = match self.data.journal_entries.last() {
            Some(e) => e.id,
            None => return,
        };
        match self.data.store.delete_entry(entry_id) {
            Ok(_) => {
                self.data.reload_journal_entries();
                self.toast(format!("deleted entry #{}", entry_id));
            }
            Err(e) => self.toast(format!("delete failed: {}", e)),
        }
    }

    fn handle_command(&mut self, cmd: String) {
        self.modals.command_palette_open = false;
        match cmd.trim() {
            "tasks" => self.ui.page = Page::Tasks,
            "journal" => self.ui.page = Page::Journal,
            "pomodoro" => {
                self.modals.pomodoro_open = true;
                self.modals.pomodoro_started = Some(Instant::now());
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
