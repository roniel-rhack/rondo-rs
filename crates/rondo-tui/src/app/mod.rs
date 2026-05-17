pub mod data_state;
pub mod modals_state;
pub mod ui_state;
pub mod undo;

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
    /// External WASM plugins loaded from `~/.rondo-rs/plugins/`. Empty until
    /// `main::load_external_plugins` runs at startup. Lives alongside the
    /// in-process `PluginRegistry`; the two are queried together by
    /// `handle_command` and the command palette.
    pub external: rondo_plugin_host::PluginHost,
    pub theme: Theme,
    pub should_quit: bool,
    pub status_msg: Option<String>,
    /// True when the underlying store was opened RW (so we can persist mutations).
    pub writable: bool,
    pub undo: undo::UndoStack,
}

impl AppState {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Result<Self> {
        Self::with_writable(store, false)
    }

    pub fn with_writable(
        store: Arc<rondo_core::store::sqlite::SqliteStore>,
        writable: bool,
    ) -> Result<Self> {
        let mut plugins = PluginRegistry::new();
        plugins.register(Box::new(crate::plugins::builtin::bell::BellPlugin));
        plugins.register(Box::new(
            crate::plugins::builtin::dep_graph::DepGraphPlugin::new(Arc::clone(&store)),
        ));
        plugins.register(Box::new(
            crate::plugins::builtin::analytics::AnalyticsPlugin,
        ));
        Ok(Self {
            data: DataState::new(store)?,
            ui: UiState::default(),
            modals: ModalsState::default(),
            fx: crate::fx::FxManager::new(),
            plugins,
            external: rondo_plugin_host::PluginHost::new(),
            theme: Theme::dark(),
            should_quit: false,
            status_msg: None,
            writable,
            undo: undo::UndoStack::default(),
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
        if self.modals.search_open && !self.modals.search_buf.trim().is_empty() {
            self.data
                .visible_task_indices_with_search(&self.modals.search_buf)
        } else {
            self.data.visible_task_indices()
        }
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

        // Auto-dismiss the quick-actions overlay whenever a key dispatched
        // from it opens a different modal or triggers a stateful change.
        // Keeps it from sitting visually behind every subsequent overlay.
        if self.modals.quick_actions_open && action_dismisses_quick_actions(&action) {
            self.modals.quick_actions_open = false;
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
                    if self.writable {
                        let mut ok = 0usize;
                        let mut err: Option<String> = None;
                        for id in &ids {
                            match self
                                .data
                                .store
                                .set_status(*id, rondo_core::domain::task::Status::Done)
                            {
                                Ok(snap) => {
                                    self.undo.push(snap);
                                    ok += 1;
                                }
                                Err(e) => err = Some(format!("{}", e)),
                            }
                        }
                        self.data.refresh_tasks();
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
                        match err {
                            Some(e) => self.toast(format!("bulk done: {} ok, error: {}", ok, e)),
                            None => self.toast(format!("marked {} tasks done", ok)),
                        }
                    } else {
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
                        self.toast(format!(
                            "toggled {} tasks (read-only, in-memory)",
                            self.ui.selection.len()
                        ));
                    }
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
                    self.persist_pomodoro_start();
                }
                if !self.modals.pomodoro_open {
                    self.finalize_pomodoro_close();
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
                self.finalize_pomodoro_close();
            }
            Action::SubmitCommand(cmd) => self.handle_command(cmd),
            Action::JournalStartEntry => {
                if self.ui.page == Page::Journal {
                    self.modals.journal_editor_open = true;
                    self.modals.journal_editor_buf.clear();
                    self.modals.journal_textarea = tui_textarea::TextArea::default();
                    self.modals.journal_editor_entry_id = None;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::JournalEditFocusedEntry => {
                if self.ui.page == Page::Journal && !self.data.journal_entries.is_empty() {
                    let idx = self
                        .data
                        .selected_journal_entry
                        .min(self.data.journal_entries.len() - 1);
                    let entry = &self.data.journal_entries[idx];
                    self.modals.journal_editor_buf = entry.body.clone();
                    self.modals.journal_textarea = tui_textarea::TextArea::new(
                        entry.body.split('\n').map(|s| s.to_string()).collect(),
                    );
                    self.modals.journal_editor_entry_id = Some(entry.id);
                    self.modals.journal_editor_open = true;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::JournalEditorKey(k) => {
                let input = tui_textarea::Input::from(crossterm::event::Event::Key(k));
                self.modals.journal_textarea.input(input);
                self.modals.journal_editor_buf = self.modals.journal_textarea.lines().join("\n");
            }
            Action::JournalNextEntry => {
                let n = self.data.journal_entries.len();
                if n > 0 {
                    self.data.selected_journal_entry =
                        (self.data.selected_journal_entry + 1).min(n - 1);
                }
            }
            Action::JournalPrevEntry => {
                if self.data.selected_journal_entry > 0 {
                    self.data.selected_journal_entry -= 1;
                }
            }
            Action::JournalSubmitEntry => self.submit_journal_entry(),
            Action::JournalCancelEntry => {
                self.modals.journal_editor_open = false;
                self.modals.journal_editor_buf.clear();
                self.modals.journal_textarea = tui_textarea::TextArea::default();
                self.modals.journal_editor_entry_id = None;
                self.ui.mode = Mode::Normal;
            }
            Action::JournalDeleteDay => {
                self.delete_focused_journal_day();
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
            Action::JournalNextDay => self.move_journal_day(1),
            Action::JournalPrevDay => self.move_journal_day(-1),
            Action::SetSortOrder(order) => {
                self.ui.sort_order = order;
                self.modals.sort_overlay_open = false;
                self.toast(format!("sort: {}", order.label()));
            }
            Action::RequestDeleteTask => {
                if !self.writable {
                    self.toast("delete: read-only (start with --write)");
                } else if self.data.selected_task_id().is_some() {
                    self.modals.confirm_delete_open = true;
                }
            }
            Action::ConfirmDeleteTask => {
                self.modals.confirm_delete_open = false;
                if let Some(id) = self.data.selected_task_id() {
                    match self.data.store.delete_task(id) {
                        Ok(snap) => {
                            self.undo.push(snap);
                            self.data.refresh_tasks();
                            self.toast("task deleted");
                        }
                        Err(e) => self.toast(format!("delete failed: {}", e)),
                    }
                }
            }
            Action::CancelDelete => self.modals.confirm_delete_open = false,
            Action::RequestEditTitle => {
                if !self.writable {
                    self.toast("edit: read-only (start with --write)");
                } else if let Some(t) = self.data.selected_task() {
                    self.modals.edit_title_buf = t.title.clone();
                    self.modals.edit_title_open = true;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::SubmitEditTitle(new_title) => {
                let trimmed = new_title.trim().to_string();
                if !trimmed.is_empty() {
                    if let Some(id) = self.data.selected_task_id() {
                        let patch = rondo_core::domain::task::TaskPatch {
                            title: Some(trimmed),
                            ..Default::default()
                        };
                        match self.data.store.update_task(id, patch) {
                            Ok(snap) => {
                                self.undo.push(snap);
                                self.data.refresh_tasks();
                                self.toast("title updated");
                            }
                            Err(e) => self.toast(format!("update failed: {}", e)),
                        }
                    }
                }
                self.modals.edit_title_open = false;
                self.modals.edit_title_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::CancelEditTitle => {
                self.modals.edit_title_open = false;
                self.modals.edit_title_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::RequestAddSubtask => {
                if !self.writable {
                    self.toast("subtask: read-only (start with --write)");
                } else if self.data.selected_task_id().is_some() {
                    self.modals.add_subtask_buf.clear();
                    self.modals.add_subtask_open = true;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::SubmitAddSubtask(title) => {
                let trimmed = title.trim().to_string();
                if !trimmed.is_empty() {
                    if let Some(task_id) = self.data.selected_task_id() {
                        match self.data.store.add_subtask(task_id, &trimmed) {
                            Ok((_id, snap)) => {
                                self.undo.push(snap);
                                self.data.refresh_tasks();
                                self.toast("subtask added");
                            }
                            Err(e) => self.toast(format!("subtask failed: {}", e)),
                        }
                    }
                }
                self.modals.add_subtask_open = false;
                self.modals.add_subtask_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::CancelAddSubtask => {
                self.modals.add_subtask_open = false;
                self.modals.add_subtask_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::RequestAddDependency => {
                if !self.writable {
                    self.toast("dep: read-only (start with --write)");
                } else if self.data.selected_task_id().is_some() {
                    self.modals.dep_overlay_buf.clear();
                    self.modals.dep_overlay_open = true;
                    self.modals.dep_overlay_mode = crate::app::modals_state::DepOverlayMode::Add;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::SubmitAddDependency(buf) => {
                let parsed = buf.trim().parse::<i64>();
                match (parsed, self.data.selected_task_id()) {
                    (Ok(blocker), Some(task_id)) if blocker > 0 && blocker != task_id => {
                        match self.data.store.add_dependency(task_id, blocker) {
                            Ok(()) => {
                                self.data.refresh_tasks();
                                self.toast(format!("dep added: #{} blocks #{}", blocker, task_id));
                            }
                            Err(e) => self.toast(format!("dep add failed: {}", e)),
                        }
                    }
                    (Ok(_), _) => self.toast("dep: invalid id"),
                    (Err(_), _) => self.toast("dep: enter a numeric task id"),
                }
                self.modals.dep_overlay_open = false;
                self.modals.dep_overlay_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::SubmitRemoveDependency(buf) => {
                let parsed = buf.trim().parse::<i64>();
                match (parsed, self.data.selected_task_id()) {
                    (Ok(blocker), Some(task_id)) => {
                        match self.data.store.remove_dependency(task_id, blocker) {
                            Ok(()) => {
                                self.data.refresh_tasks();
                                self.toast(format!("dep removed: #{}", blocker));
                            }
                            Err(e) => self.toast(format!("dep remove failed: {}", e)),
                        }
                    }
                    _ => self.toast("dep: enter a numeric task id"),
                }
                self.modals.dep_overlay_open = false;
                self.modals.dep_overlay_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::CancelDepOverlay => {
                self.modals.dep_overlay_open = false;
                self.modals.dep_overlay_buf.clear();
                self.ui.mode = Mode::Normal;
            }
            Action::ToggleDepOverlayMode => {
                // handled inside ModalsState::update already
            }
            Action::RequestEditDescription => {
                if !self.writable {
                    self.toast("description: read-only (start with --write)");
                } else if let Some(task) = self.data.selected_task() {
                    let body = task.description.clone().unwrap_or_default();
                    self.modals.description_textarea = tui_textarea::TextArea::new(
                        body.split('\n').map(|s| s.to_string()).collect(),
                    );
                    self.modals.description_task_id = Some(task.id);
                    self.modals.description_editor_open = true;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::DescriptionEditorKey(k) => {
                self.modals
                    .description_textarea
                    .input(tui_textarea::Input::from(crossterm::event::Event::Key(k)));
            }
            Action::SubmitEditDescription => {
                let body = self.modals.description_textarea.lines().join("\n");
                let task_id = self.modals.description_task_id;
                self.modals.description_editor_open = false;
                self.modals.description_textarea = tui_textarea::TextArea::default();
                self.modals.description_task_id = None;
                self.ui.mode = Mode::Normal;
                if let Some(id) = task_id {
                    let patch = rondo_core::domain::task::TaskPatch {
                        description: Some(if body.is_empty() { None } else { Some(body) }),
                        ..Default::default()
                    };
                    match self.data.store.update_task(id, patch) {
                        Ok(snap) => {
                            self.undo.push(snap);
                            self.data.refresh_tasks();
                            self.toast("description updated");
                        }
                        Err(e) => self.toast(format!("update failed: {}", e)),
                    }
                }
            }
            Action::CancelEditDescription => {
                self.modals.description_editor_open = false;
                self.modals.description_textarea = tui_textarea::TextArea::default();
                self.modals.description_task_id = None;
                self.ui.mode = Mode::Normal;
            }

            Action::RequestEditFocusedSubtask => {
                if !self.writable {
                    self.toast("subtask: read-only (start with --write)");
                } else if let Some(task) = self.data.selected_task() {
                    if let Some(sub) = task.subtasks.get(self.ui.focus.section_item) {
                        self.modals.edit_subtask_buf = sub.title.clone();
                        self.modals.edit_subtask_id = Some(sub.id);
                        self.modals.edit_subtask_open = true;
                        self.ui.mode = Mode::Insert;
                    }
                }
            }
            Action::SubmitEditSubtask(new_title) => {
                let trimmed = new_title.trim().to_string();
                let sub_id = self.modals.edit_subtask_id;
                self.modals.edit_subtask_open = false;
                self.modals.edit_subtask_buf.clear();
                self.modals.edit_subtask_id = None;
                self.ui.mode = Mode::Normal;
                if !trimmed.is_empty() {
                    if let Some(id) = sub_id {
                        match self.data.store.update_subtask_title(id, &trimmed) {
                            Ok(_) => {
                                self.data.refresh_tasks();
                                self.toast("subtask renamed");
                            }
                            Err(e) => self.toast(format!("rename failed: {}", e)),
                        }
                    }
                }
            }
            Action::CancelEditSubtask => {
                self.modals.edit_subtask_open = false;
                self.modals.edit_subtask_buf.clear();
                self.modals.edit_subtask_id = None;
                self.ui.mode = Mode::Normal;
            }
            Action::RequestDeleteFocusedSubtask => {
                if !self.writable {
                    self.toast("subtask: read-only");
                } else if let Some(task) = self.data.selected_task() {
                    if let Some(sub) = task.subtasks.get(self.ui.focus.section_item) {
                        let sub_id = sub.id;
                        match self.data.store.delete_subtask(sub_id) {
                            Ok(_) => {
                                self.data.refresh_tasks();
                                let total = self
                                    .data
                                    .selected_task()
                                    .map(|t| t.subtasks.len())
                                    .unwrap_or(0);
                                if self.ui.focus.section_item >= total && total > 0 {
                                    self.ui.focus.section_item = total - 1;
                                }
                                self.toast(format!("deleted subtask #{}", sub_id));
                            }
                            Err(e) => self.toast(format!("delete failed: {}", e)),
                        }
                    }
                }
            }

            Action::RequestAddNote => {
                if !self.writable {
                    self.toast("note: read-only");
                } else if let Some(id) = self.data.selected_task_id() {
                    self.modals.note_textarea = tui_textarea::TextArea::default();
                    self.modals.note_editing_id = None;
                    self.modals.note_task_id = Some(id);
                    self.modals.note_editor_open = true;
                    self.ui.mode = Mode::Insert;
                }
            }
            Action::RequestEditFocusedNote => {
                if !self.writable {
                    self.toast("note: read-only");
                } else if let Some(task) = self.data.selected_task() {
                    if let Some(note) = task.notes.get(self.ui.focus.section_item) {
                        self.modals.note_textarea = tui_textarea::TextArea::new(
                            note.body.split('\n').map(|s| s.to_string()).collect(),
                        );
                        self.modals.note_editing_id = Some(note.id);
                        self.modals.note_task_id = Some(task.id);
                        self.modals.note_editor_open = true;
                        self.ui.mode = Mode::Insert;
                    }
                }
            }
            Action::RequestDeleteFocusedNote => {
                if !self.writable {
                    self.toast("note: read-only");
                } else if let Some(task) = self.data.selected_task() {
                    if let Some(note) = task.notes.get(self.ui.focus.section_item) {
                        let note_id = note.id;
                        match self.data.store.delete_task_note(note_id) {
                            Ok(_) => {
                                self.data.refresh_tasks();
                                let total = self
                                    .data
                                    .selected_task()
                                    .map(|t| t.notes.len())
                                    .unwrap_or(0);
                                if self.ui.focus.section_item >= total && total > 0 {
                                    self.ui.focus.section_item = total - 1;
                                }
                                self.toast(format!("deleted note #{}", note_id));
                            }
                            Err(e) => self.toast(format!("delete failed: {}", e)),
                        }
                    }
                }
            }
            Action::NoteEditorKey(k) => {
                self.modals
                    .note_textarea
                    .input(tui_textarea::Input::from(crossterm::event::Event::Key(k)));
            }
            Action::SubmitNote => {
                let body = self.modals.note_textarea.lines().join("\n");
                let editing = self.modals.note_editing_id;
                let task_id = self.modals.note_task_id;
                self.modals.note_editor_open = false;
                self.modals.note_textarea = tui_textarea::TextArea::default();
                self.modals.note_editing_id = None;
                self.modals.note_task_id = None;
                self.ui.mode = Mode::Normal;
                if body.trim().is_empty() {
                    return;
                }
                let result = match (editing, task_id) {
                    (Some(id), _) => self
                        .data
                        .store
                        .update_task_note(id, &body)
                        .map(|_| "note updated".to_string()),
                    (None, Some(tid)) => self
                        .data
                        .store
                        .add_task_note(tid, &body)
                        .map(|_| "note added".to_string()),
                    _ => return,
                };
                match result {
                    Ok(msg) => {
                        self.data.refresh_tasks();
                        self.toast(msg);
                    }
                    Err(e) => self.toast(format!("note failed: {}", e)),
                }
            }
            Action::CancelNote => {
                self.modals.note_editor_open = false;
                self.modals.note_textarea = tui_textarea::TextArea::default();
                self.modals.note_editing_id = None;
                self.modals.note_task_id = None;
                self.ui.mode = Mode::Normal;
            }

            Action::Paste(text) => {
                // Multiline textareas accept paste as-is.
                if self.modals.description_editor_open {
                    self.modals.description_textarea.insert_str(&text);
                } else if self.modals.note_editor_open {
                    self.modals.note_textarea.insert_str(&text);
                } else if self.modals.journal_editor_open {
                    self.modals.journal_textarea.insert_str(&text);
                    self.modals.journal_editor_buf =
                        self.modals.journal_textarea.lines().join("\n");
                } else {
                    // Single-line surfaces: keep the first line only and
                    // strip trailing newlines so the modal doesn't break.
                    let first_line = text
                        .split('\n')
                        .next()
                        .unwrap_or("")
                        .trim_end_matches('\r')
                        .to_string();
                    if self.modals.quick_add_open {
                        self.modals.quick_add_buf.push_str(&first_line);
                    } else if self.modals.edit_title_open {
                        self.modals.edit_title_buf.push_str(&first_line);
                    } else if self.modals.edit_subtask_open {
                        self.modals.edit_subtask_buf.push_str(&first_line);
                    } else if self.modals.command_palette_open {
                        self.modals.command_buf.push_str(&first_line);
                    } else if self.modals.search_open {
                        self.modals.search_buf.push_str(&first_line);
                    } else if self.modals.dep_overlay_open {
                        // dep overlay accepts digits only
                        for c in first_line.chars().filter(|c| c.is_ascii_digit()) {
                            self.modals.dep_overlay_buf.push(c);
                        }
                    } else if self.modals.add_subtask_open {
                        self.modals.add_subtask_buf.push_str(&first_line);
                    }
                }
            }
            Action::PluginKeyPress(key) => {
                if let Some(id) = self.modals.plugin_page.clone() {
                    let ctx = rondo_plugin_api::PluginContext::new(&id);
                    if let Some(plugin) = self.plugins.get_mut(&id) {
                        let _ =
                            plugin.handle(rondo_plugin_api::PluginAction::KeyPress { key }, &ctx);
                    }
                }
            }
            Action::Undo => {
                if !self.writable {
                    self.toast("undo: read-only");
                } else {
                    match self.undo.pop() {
                        None => self.toast("nothing to undo"),
                        Some(snap) => {
                            if let Err(e) = self.apply_undo(snap) {
                                self.toast(format!("undo failed: {}", e));
                            } else {
                                self.data.refresh_tasks();
                                self.toast("undone");
                            }
                        }
                    }
                }
            }
            Action::ToggleFocusedSubtask => {
                if self.ui.focus.pane == Pane::Detail
                    && self.ui.focus.section == DetailSection::Subtasks
                {
                    self.toggle_focused_subtask();
                }
            }
            Action::EscapeContext => {
                if self.modals.description_editor_open {
                    self.modals.description_editor_open = false;
                    self.modals.description_textarea = tui_textarea::TextArea::default();
                    self.modals.description_task_id = None;
                    self.ui.mode = Mode::Normal;
                } else if self.modals.edit_subtask_open {
                    self.modals.edit_subtask_open = false;
                    self.modals.edit_subtask_buf.clear();
                    self.modals.edit_subtask_id = None;
                    self.ui.mode = Mode::Normal;
                } else if self.modals.note_editor_open {
                    self.modals.note_editor_open = false;
                    self.modals.note_textarea = tui_textarea::TextArea::default();
                    self.modals.note_editing_id = None;
                    self.modals.note_task_id = None;
                    self.ui.mode = Mode::Normal;
                } else if self.modals.plugin_overlay.is_some() {
                    if let Some((id, _)) = self.modals.plugin_overlay.take() {
                        if self.plugins.get_mut(&id).is_some() {
                            let ctx = rondo_plugin_api::PluginContext::new(&id);
                            if let Some(p) = self.plugins.get_mut(&id) {
                                let _ = p.handle(rondo_plugin_api::PluginAction::Hide, &ctx);
                            }
                        } else {
                            let _ = self
                                .external
                                .dispatch_one(&id, &rondo_plugin_api::PluginAction::Hide);
                        }
                    }
                } else if self.modals.plugin_page.is_some() {
                    // Notify plugin its page is hiding.
                    if let Some(id) = self.modals.plugin_page.take() {
                        let ctx = rondo_plugin_api::PluginContext::new(&id);
                        if let Some(p) = self.plugins.get_mut(&id) {
                            let _ = p.handle(rondo_plugin_api::PluginAction::Hide, &ctx);
                        }
                    }
                } else if self.modals.plugins_overlay_open {
                    self.modals.plugins_overlay_open = false;
                } else if self.modals.help_open {
                    self.modals.help_open = false;
                } else if self.modals.confirm_delete_open {
                    self.modals.confirm_delete_open = false;
                } else if self.modals.edit_title_open {
                    self.modals.edit_title_open = false;
                    self.modals.edit_title_buf.clear();
                    self.ui.mode = Mode::Normal;
                } else if self.modals.add_subtask_open {
                    self.modals.add_subtask_open = false;
                    self.modals.add_subtask_buf.clear();
                    self.ui.mode = Mode::Normal;
                } else if self.modals.dep_overlay_open {
                    self.modals.dep_overlay_open = false;
                    self.modals.dep_overlay_buf.clear();
                    self.ui.mode = Mode::Normal;
                } else if self.modals.sort_overlay_open {
                    self.modals.sort_overlay_open = false;
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
                    self.finalize_pomodoro_close();
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

    /// Apply the inverse of a captured mutation. Called from the `Undo`
    /// action handler; never pushes onto the undo stack itself.
    ///
    /// Known limitation: `Delete` undo re-creates the row via
    /// `create_task`, which produces a **new** id. Subtasks, tags
    /// beyond the initial set, time logs, and notes attached to the
    /// original are NOT restored — only the core task plus initial
    /// tags from `NewTask`. Dependency edges are also lost.
    fn apply_undo(
        &mut self,
        snap: rondo_core::domain::task::UndoSnapshot,
    ) -> rondo_core::Result<()> {
        use rondo_core::domain::task::{NewTask, TaskPatch, UndoKind};
        match snap.kind {
            UndoKind::Create => {
                if let Some(id) = snap.created_id {
                    self.data.store.delete_task(id)?;
                }
            }
            UndoKind::Update => {
                if let Some(before) = snap.task_before {
                    let patch = TaskPatch {
                        title: Some(before.title.clone()),
                        description: Some(before.description.clone()),
                        status: Some(before.status),
                        priority: Some(before.priority),
                        due_date: Some(before.due_date),
                        recur_freq: Some(before.recur_freq),
                        recur_interval: Some(before.recur_interval),
                    };
                    self.data.store.update_task(before.id, patch)?;
                }
            }
            UndoKind::Delete => {
                if let Some(before) = snap.task_before {
                    let new = NewTask {
                        title: before.title.clone(),
                        description: before.description.clone(),
                        status: before.status,
                        priority: before.priority,
                        due_date: before.due_date,
                        recur_freq: before.recur_freq,
                        recur_interval: before.recur_interval,
                        tags: before.tags.clone(),
                    };
                    self.data.store.create_task(new)?;
                }
            }
            UndoKind::SetStatus => {
                if let Some(before) = snap.task_before {
                    self.data.store.set_status(before.id, before.status)?;
                }
            }
            UndoKind::AddSubtask => {
                if let Some(id) = snap.created_id {
                    self.data.store.delete_subtask(id)?;
                }
            }
            UndoKind::ToggleSubtask => {
                if let Some(before) = snap.task_before {
                    let after = self.data.store.task_by_id(before.id)?;
                    for (bs, as_) in before.subtasks.iter().zip(after.subtasks.iter()) {
                        if bs.completed != as_.completed {
                            self.data.store.toggle_subtask(as_.id)?;
                            break;
                        }
                    }
                }
            }
            UndoKind::AddTag => {
                if let Some(before) = snap.task_before {
                    let after = self.data.store.task_by_id(before.id)?;
                    for tag in after.tags.iter() {
                        if !before.tags.contains(tag) {
                            self.data.store.remove_tag(before.id, tag)?;
                            break;
                        }
                    }
                }
            }
            UndoKind::RemoveTag => {
                if let Some(before) = snap.task_before {
                    let after = self.data.store.task_by_id(before.id)?;
                    for tag in before.tags.iter() {
                        if !after.tags.contains(tag) {
                            self.data.store.add_tag(before.id, tag)?;
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
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
            Page::Journal => match self.ui.journal_pane {
                crate::app::ui_state::JournalPane::Days => self.move_journal_day(delta),
                crate::app::ui_state::JournalPane::Entries => {
                    if !self.data.journal_entries.is_empty() {
                        let len = self.data.journal_entries.len() as i32;
                        let next =
                            (self.data.selected_journal_entry as i32 + delta).rem_euclid(len);
                        self.data.selected_journal_entry = next as usize;
                    }
                }
            },
        }
    }

    fn move_journal_day(&mut self, delta: i32) {
        if self.data.journal_notes.is_empty() {
            return;
        }
        let len = self.data.journal_notes.len() as i32;
        let prev = self.data.selected_journal;
        let next = (self.data.selected_journal as i32 + delta).rem_euclid(len);
        self.data.selected_journal = next as usize;
        self.data
            .journal_list_state
            .select(Some(self.data.selected_journal));
        self.data.selected_journal_entry = 0;
        if let Ok(e) = self
            .data
            .store
            .entries_for_note(self.data.journal_notes[self.data.selected_journal].id)
        {
            self.data.journal_entries = e;
        }
        if prev != self.data.selected_journal {
            self.spawn_journal_refresh();
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
            self.toggle_focused_subtask();
        }
    }

    /// Toggle the subtask under the Detail::Subtasks cursor. Persists when
    /// `writable`; otherwise mutates the in-memory copy and toasts.
    fn toggle_focused_subtask(&mut self) {
        let item_idx = self.ui.focus.section_item;
        let subtask_id = match self.data.tasks.get(self.data.selected_task) {
            Some(t) => match t.subtasks.get(item_idx) {
                Some(s) => s.id,
                None => return,
            },
            None => return,
        };
        if self.writable {
            match self.data.store.toggle_subtask(subtask_id) {
                Ok((_, snap)) => {
                    self.undo.push(snap);
                    self.data.refresh_tasks();
                    self.ui.flash = Some((FlashTarget::Subtask(subtask_id), Instant::now()));
                    self.toast(format!("subtask #{} toggled", subtask_id));
                }
                Err(e) => self.toast(format!("subtask toggle failed: {}", e)),
            }
        } else if let Some(task) = self.data.tasks.get_mut(self.data.selected_task) {
            if let Some(st) = task.subtasks.get_mut(item_idx) {
                st.completed = !st.completed;
                let id = st.id;
                self.ui.flash = Some((FlashTarget::Subtask(id), Instant::now()));
                self.toast(format!("subtask #{} toggled (read-only, in-memory)", id));
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

    /// Coalesce + accent fade over the journal entries pane when the user
    /// flips to a different day. Reuses the same preset as task detail.
    fn spawn_journal_refresh(&mut self) {
        if self.ui.last_journal_entries_rect.width > 0 {
            let eff = crate::fx::presets::detail_refresh(self.theme.accent);
            self.fx.spawn(
                crate::fx::EffectId::DetailRefresh,
                eff,
                self.ui.last_journal_entries_rect,
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
        if !self.writable {
            self.toast("quick-add: read-only (start with --write)");
            return;
        }
        let new_task = rondo_core::domain::task::NewTask {
            title: parsed.title.clone(),
            description: None,
            status: rondo_core::domain::task::Status::Pending,
            priority: parsed
                .priority
                .unwrap_or(rondo_core::domain::task::Priority::Low),
            due_date: None,
            recur_freq: rondo_core::domain::task::RecurFreq::None,
            recur_interval: 0,
            tags: parsed.tags.clone(),
        };
        match self.data.store.create_task(new_task) {
            Ok((_id, snap)) => {
                self.undo.push(snap);
                self.data.refresh_tasks();
                if self.ui.last_task_list_rect.width > 0 {
                    let eff = crate::fx::presets::quick_add_slide(self.theme.bg);
                    self.fx.spawn(
                        crate::fx::EffectId::QuickAddInsert,
                        eff,
                        self.ui.last_task_list_rect,
                    );
                }
                self.toast(format!("added: '{}'", parsed.title));
            }
            Err(e) => self.toast(format!("add failed: {}", e)),
        }
    }

    fn submit_journal_entry(&mut self) {
        let body = self.modals.journal_textarea.lines().join("\n");
        let editing_id = self.modals.journal_editor_entry_id.take();
        self.modals.journal_editor_open = false;
        self.modals.journal_editor_buf.clear();
        self.modals.journal_textarea = tui_textarea::TextArea::default();
        self.ui.mode = Mode::Normal;
        if body.trim().is_empty() {
            return;
        }
        match editing_id {
            Some(id) => match self.data.store.update_journal_entry(id, &body) {
                Ok(_) => {
                    self.data.reload_journal_entries();
                    self.toast(format!("entry #{} updated", id));
                }
                Err(e) => self.toast(format!("update failed: {}", e)),
            },
            None => match self.data.store.create_or_get_today_note() {
                Ok(note) => match self.data.store.add_journal_entry(note.id, &body) {
                    Ok(_) => {
                        self.data.refresh_journal_notes();
                        if let Some(pos) =
                            self.data.journal_notes.iter().position(|n| n.id == note.id)
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
            },
        }
    }

    fn delete_focused_journal_day(&mut self) {
        if self.data.journal_notes.is_empty() {
            return;
        }
        let idx = self
            .data
            .selected_journal
            .min(self.data.journal_notes.len() - 1);
        let note_id = self.data.journal_notes[idx].id;
        match self.data.store.delete_note(note_id) {
            Ok(_) => {
                self.data.refresh_journal_notes();
                if self.data.selected_journal >= self.data.journal_notes.len()
                    && !self.data.journal_notes.is_empty()
                {
                    self.data.selected_journal = self.data.journal_notes.len() - 1;
                    self.data
                        .journal_list_state
                        .select(Some(self.data.selected_journal));
                }
                self.data.reload_journal_entries();
                self.toast(format!("deleted day #{}", note_id));
            }
            Err(e) => self.toast(format!("delete day failed: {}", e)),
        }
    }

    fn delete_focused_journal_entry(&mut self) {
        if self.data.journal_entries.is_empty() {
            return;
        }
        let idx = self
            .data
            .selected_journal_entry
            .min(self.data.journal_entries.len() - 1);
        let entry_id = self.data.journal_entries[idx].id;
        match self.data.store.delete_entry(entry_id) {
            Ok(_) => {
                self.data.reload_journal_entries();
                if self.data.selected_journal_entry >= self.data.journal_entries.len()
                    && !self.data.journal_entries.is_empty()
                {
                    self.data.selected_journal_entry = self.data.journal_entries.len() - 1;
                }
                self.toast(format!("deleted entry #{}", entry_id));
            }
            Err(e) => self.toast(format!("delete failed: {}", e)),
        }
    }

    fn handle_command(&mut self, cmd: String) {
        self.modals.command_palette_open = false;
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        let resolved = match self.resolve_command_prefix(trimmed) {
            CommandResolution::Exact(s) | CommandResolution::UniquePrefix(s) => s,
            CommandResolution::Ambiguous(matches) => {
                self.status_msg = Some(format!(
                    "ambiguous: {} (matches: {})",
                    trimmed,
                    matches.join(", ")
                ));
                return;
            }
            CommandResolution::None => {
                if !self.try_invoke_plugin_command(trimmed) {
                    self.status_msg = Some(format!("unknown: {}", trimmed));
                }
                return;
            }
        };
        match resolved.as_str() {
            "tasks" => self.ui.page = Page::Tasks,
            "journal" => self.ui.page = Page::Journal,
            "pomodoro" => {
                let already = self.modals.pomodoro_open;
                self.modals.pomodoro_open = true;
                if !already {
                    self.modals.pomodoro_started = Some(Instant::now());
                    self.persist_pomodoro_start();
                }
            }
            "plugins" => self.modals.plugins_overlay_open = true,
            "help" => self.modals.help_open = true,
            "calendar" => self.open_plugin_page("builtin.calendar"),
            "focus" | "focus-page" => self.open_plugin_page("builtin.focus-page"),
            "deps" | "dep-graph" => self.open_plugin_page("builtin.dep-graph"),
            "analytics" => self.open_plugin_page("builtin.analytics"),
            "quit" => self.should_quit = true,
            other => {
                if !self.try_invoke_plugin_command(other) {
                    self.status_msg = Some(format!("unknown: {}", other));
                }
            }
        }
    }

    /// Enumerate every command name the palette knows about: hardcoded
    /// builtins plus every plugin's `[cli].name` and `id` (in-process and
    /// external). Used by `resolve_command_prefix` to expand a typed prefix
    /// to a unique canonical command name.
    fn known_command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = [
            "tasks",
            "journal",
            "pomodoro",
            "plugins",
            "help",
            "calendar",
            "focus",
            "focus-page",
            "deps",
            "dep-graph",
            "analytics",
            "quit",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for m in self.plugins.iter_manifests() {
            if let Some(cli) = m.cli.as_ref() {
                names.push(cli.name.clone());
            }
            names.push(m.id.clone());
        }
        for m in self.external.manifests() {
            if let Some(n) = m.command_name() {
                names.push(n.to_string());
            }
            names.push(m.id.clone());
        }
        names.sort();
        names.dedup();
        names
    }

    /// Map a typed string to a canonical command. Exact match wins; if no
    /// exact match exists, a prefix that uniquely identifies one command is
    /// promoted. Multiple prefix matches → `Ambiguous`; zero → `None`.
    fn resolve_command_prefix(&self, input: &str) -> CommandResolution {
        let all = self.known_command_names();
        if all.iter().any(|n| n == input) {
            return CommandResolution::Exact(input.to_string());
        }
        let matches: Vec<String> = all.into_iter().filter(|n| n.starts_with(input)).collect();
        match matches.len() {
            0 => CommandResolution::None,
            1 => CommandResolution::UniquePrefix(matches.into_iter().next().unwrap()),
            _ => CommandResolution::Ambiguous(matches),
        }
    }

    /// Resolve `cmd` against in-process + external plugins, dispatch
    /// `PluginAction::Show`, and route the returned `ViewSpec` to the
    /// appropriate modal slot. Returns `true` when a plugin handled the
    /// command (so the caller can suppress the "unknown" toast).
    fn try_invoke_plugin_command(&mut self, cmd: &str) -> bool {
        // 1) In-process registry — match by manifest id, or by [cli].name.
        let in_proc_id = self
            .plugins
            .iter_manifests()
            .find(|m| m.id == cmd || m.cli.as_ref().map(|c| c.name.as_str()) == Some(cmd))
            .map(|m| m.id.clone());
        if let Some(id) = in_proc_id {
            self.invoke_in_process_plugin(&id);
            return true;
        }
        // 2) External WASM host.
        if let Some(id) = self.external.resolve_command(cmd) {
            self.invoke_external_plugin(&id);
            return true;
        }
        false
    }

    fn invoke_in_process_plugin(&mut self, id: &str) {
        let ctx = rondo_plugin_api::PluginContext::new(id);
        let Some(r) = self
            .plugins
            .get_mut(id)
            .map(|p| p.handle(rondo_plugin_api::PluginAction::Show, &ctx))
        else {
            self.toast(format!("plugin not registered: {}", id));
            return;
        };
        let notify_msg = r.follow_up.iter().find_map(|fa| match fa {
            rondo_plugin_api::PluginAction::Notify { message, .. } => Some(message.clone()),
            _ => None,
        });
        self.route_show_view(id, r.view, notify_msg);
    }

    fn invoke_external_plugin(&mut self, id: &str) {
        let result = self
            .external
            .dispatch_one(id, &rondo_plugin_api::PluginAction::Show);
        let Some(r) = result else {
            self.toast(format!("plugin `{}` failed (see logs)", id));
            return;
        };
        let view = r.view;
        // Surface any Notify follow-up as a toast so background plugins
        // (e.g. sync-localdir's `:sync-now`) confirm execution.
        let notify_msg = r.follow_up.iter().find_map(|fa| match fa {
            rondo_plugin_api::PluginAction::Notify { message, .. } => Some(message.clone()),
            _ => None,
        });
        self.route_show_view(id, view, notify_msg);
    }

    /// Route a `Show` response into the right modal slot based on
    /// `ViewKind`. Overlay → `plugin_overlay`, Page → reuse the existing
    /// full-screen `plugin_page` path, no view → toast (either the
    /// plugin's `Notify` message or a generic "invoked" confirmation).
    fn route_show_view(
        &mut self,
        id: &str,
        view: Option<rondo_plugin_api::ViewSpec>,
        notify_msg: Option<String>,
    ) {
        match view {
            Some(v) if v.kind == rondo_plugin_api::ViewKind::Overlay => {
                self.modals.plugin_overlay = Some((id.to_string(), v));
            }
            Some(v) if v.kind == rondo_plugin_api::ViewKind::Page => {
                self.modals.plugin_page = Some(id.to_string());
                let _ = v;
            }
            _ => {
                let msg = notify_msg.unwrap_or_else(|| format!("plugin `{}` invoked", id));
                self.toast(msg);
            }
        }
    }

    fn open_plugin_page(&mut self, id: &str) {
        let exists = self.plugins.iter_manifests().any(|m| m.id == id);
        if !exists {
            self.toast(format!("plugin not registered: {}", id));
            return;
        }
        // Dispatch a Show so plugins with internal state flip to "active".
        let ctx = rondo_plugin_api::PluginContext::new(id);
        if let Some(plugin) = self.plugins.get_mut(id) {
            let _ = plugin.handle(rondo_plugin_api::PluginAction::Show, &ctx);
        }
        self.modals.plugin_page = Some(id.to_string());
    }

    /// Insert a `focus_sessions` row for the just-opened pomodoro. No-op when
    /// the store is read-only; logs a debug line instead.
    fn persist_pomodoro_start(&mut self) {
        if !self.writable {
            tracing::debug!("pomodoro: read-only store, skipping focus_sessions insert");
            return;
        }
        let task_id = self.data.tasks.get(self.data.selected_task).map(|t| t.id);
        let total = self.modals.pomodoro_total.as_secs();
        match self.data.store.start_focus_session(
            task_id,
            rondo_core::domain::focus::SessionKind::Work,
            total,
        ) {
            Ok(id) => {
                self.modals.pomodoro_session_id = Some(id);
                tracing::debug!("pomodoro: started focus_sessions id={}", id);
            }
            Err(e) => tracing::warn!("pomodoro: failed to persist start: {}", e),
        }
    }

    /// On modal close, mark the session completed iff the timer reached 100%.
    /// Always clears in-memory pomodoro state (started_at, session_id).
    fn finalize_pomodoro_close(&mut self) {
        let reached_total = match self.modals.pomodoro_started {
            Some(started) => started.elapsed() >= self.modals.pomodoro_total,
            None => false,
        };
        if let Some(id) = self.modals.pomodoro_session_id.take() {
            if reached_total && self.writable {
                if let Err(e) = self.data.store.complete_focus_session(id) {
                    tracing::warn!("pomodoro: failed to persist complete: {}", e);
                } else {
                    tracing::debug!("pomodoro: completed focus_sessions id={}", id);
                }
            } else {
                tracing::debug!(
                    "pomodoro: closed early (reached_total={}, writable={}), id={} left incomplete",
                    reached_total,
                    self.writable,
                    id
                );
            }
        }
        if reached_total {
            self.ring_pomodoro_bell();
        }
        self.modals.pomodoro_started = None;
    }

    /// Dispatch an Audio `Notify` to every plugin whose manifest declares
    /// `Capability::Notifier(NotifyChannel::Audio)`. Today that's just
    /// `builtin.bell`, but the dispatch is capability-driven so future
    /// plugins (e.g. desktop-notifier) wire up the same way.
    fn ring_pomodoro_bell(&mut self) {
        use rondo_plugin_api::{Capability, NotifyChannel, PluginAction, PluginContext};
        let targets: Vec<String> = self
            .plugins
            .iter_manifests()
            .filter(|m| {
                m.capabilities
                    .iter()
                    .any(|c| matches!(c, Capability::Notifier(NotifyChannel::Audio)))
            })
            .map(|m| m.id)
            .collect();
        for id in targets {
            if let Some(p) = self.plugins.get_mut(&id) {
                let ctx = PluginContext::new(&id);
                let _ = p.handle(
                    PluginAction::Notify {
                        channel: NotifyChannel::Audio,
                        message: "pomodoro complete".into(),
                    },
                    &ctx,
                );
            }
        }
    }

    fn dispatch_plugin_ticks(&mut self) {
        use rondo_plugin_api::action::PluginAction;
        use rondo_plugin_api::plugin::PluginContext;
        let ids = self.plugins.ids();
        for id in ids {
            if let Some(p) = self.plugins.get_mut(&id) {
                let ctx = PluginContext::new(&id);
                let _ = p.handle(PluginAction::Tick { delta_ms: 100 }, &ctx);
            }
        }
        // Also tick external WASM plugins so TickHandler implementations
        // (e.g. sync-localdir's 5-minute scheduler, quote rotation timers)
        // actually advance.
        if !self.external.is_empty() {
            let _ = self
                .external
                .dispatch(&PluginAction::Tick { delta_ms: 100 });
        }
    }
}

/// Outcome of resolving a typed palette string against the known command
/// set. `Exact` and `UniquePrefix` are both actionable; the latter exists
/// only so that future UI (e.g. a status hint "expanded → analytics") can
/// tell the user what was matched.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandResolution {
    Exact(String),
    UniquePrefix(String),
    Ambiguous(Vec<String>),
    None,
}

#[derive(Debug, Default)]
pub struct QuickAddInput {
    pub title: String,
    pub tags: Vec<String>,
    pub priority: Option<rondo_core::domain::task::Priority>,
    pub due: Option<String>,
}

/// True for actions emitted from the quick-actions grid that should also
/// close the grid (since they open a new modal / change focus). Keeps a
/// fresh overlay from sitting behind every subsequent modal.
fn action_dismisses_quick_actions(a: &Action) -> bool {
    matches!(
        a,
        Action::OpenQuickAdd
            | Action::RequestEditTitle
            | Action::RequestDeleteTask
            | Action::RequestAddSubtask
            | Action::RequestAddDependency
            | Action::ToggleSelected
            | Action::EnterVisual
            | Action::TogglePomodoro
            | Action::OpenSearch
            | Action::OpenCommandPalette
            | Action::OpenSortOverlay
            | Action::LeaderGoto
            | Action::Undo
            | Action::ToggleHelp
            | Action::TogglePage(_)
    )
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
