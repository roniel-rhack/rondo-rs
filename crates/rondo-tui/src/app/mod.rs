pub mod data_state;
pub mod handlers;
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

    /// Reload the task list from the store, preserving the current
    /// selection by id when possible. Sync's `data.selected_task` back
    /// to whatever row currently holds `ui.selected_task_id`.
    pub fn refresh_tasks(&mut self) {
        self.data
            .refresh_tasks_keeping_id(self.ui.selected_task_id);
        if let Some(t) = self.data.tasks.get(self.data.selected_task) {
            self.ui.selected_task_id = Some(t.id);
        } else {
            self.ui.selected_task_id = None;
        }
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

        // Extracted domain handlers. Each `handle()` returns true if it
        // consumed the action; the main match below is skipped in that
        // case (still running the follow-up tail).
        if handlers::journal::handle(self, &action)
            || handlers::pomodoro::handle(self, &action)
            || handlers::task::handle(self, &action)
            || handlers::subtask::handle(self, &action)
        {
            if let Some(next) = follow_up.take() {
                self.update(next);
            }
            return;
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
                        self.refresh_tasks();
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
            // TogglePomodoro/OpenPomodoro/ClosePomodoro handled in
            // `handlers::pomodoro` (dispatched above).
            Action::SubmitCommand(cmd) => self.handle_command(cmd),
            // Journal* actions handled in `handlers::journal` (dispatched above).
            Action::SetSortOrder(order) => {
                self.ui.sort_order = order;
                self.modals.sort_overlay_open = false;
                self.toast(format!("sort: {}", order.label()));
            }
            // Task delete + edit title handled in `handlers::task`.
            // Subtask add handled in `handlers::subtask`.
            Action::RequestAddDependency => {
                if !self.writable {
                    self.toast(ro_msg("dep"));
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
                                self.undo
                                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                                        rondo_core::domain::task::UndoKind::AddDep {
                                            task_id,
                                            blocker_id: blocker,
                                        },
                                    ));
                                self.refresh_tasks();
                                self.toast(format!("dep added: #{} blocks #{}", blocker, task_id));
                            }
                            Err(rondo_core::error::Error::CycleDetected(a, b)) => {
                                self.toast(format!("can't add: would create cycle #{a} → #{b}"));
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
                                self.undo
                                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                                        rondo_core::domain::task::UndoKind::RemoveDep {
                                            task_id,
                                            blocker_id: blocker,
                                        },
                                    ));
                                self.refresh_tasks();
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
            // Edit-description handled in `handlers::task`.

            // Subtask edit/delete handled in `handlers::subtask`.

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
                        let note_clone = note.clone();
                        let task_id = task.id;
                        let note_id = note.id;
                        match self.data.store.delete_task_note(note_id) {
                            Ok(_) => {
                                self.undo
                                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                                        rondo_core::domain::task::UndoKind::DeleteNote {
                                            task_id,
                                            note: note_clone,
                                        },
                                    ));
                                self.refresh_tasks();
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
                match (editing, task_id) {
                    (Some(note_id), Some(tid)) => {
                        // Capture the before-body so undo can restore exactly
                        // what was there before this edit.
                        let before_body = self
                            .data
                            .selected_task()
                            .and_then(|t| t.notes.iter().find(|n| n.id == note_id))
                            .map(|n| n.body.clone());
                        match self.data.store.update_task_note(note_id, &body) {
                            Ok(_) => {
                                if let Some(before) = before_body {
                                    self.undo.push(
                                        rondo_core::domain::task::UndoSnapshot::from_kind(
                                            rondo_core::domain::task::UndoKind::UpdateNote {
                                                task_id: tid,
                                                note_id,
                                                before,
                                            },
                                        ),
                                    );
                                }
                                self.refresh_tasks();
                                self.toast("note updated");
                            }
                            Err(e) => self.toast(format!("note failed: {}", e)),
                        }
                    }
                    (None, Some(tid)) => match self.data.store.add_task_note(tid, &body) {
                        Ok(note_id) => {
                            self.undo
                                .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                                    rondo_core::domain::task::UndoKind::AddNote {
                                        task_id: tid,
                                        note_id,
                                    },
                                ));
                            self.refresh_tasks();
                            self.toast("note added");
                        }
                        Err(e) => self.toast(format!("note failed: {}", e)),
                    },
                    _ => {}
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
                            let touches_journal = matches!(
                                snap.kind,
                                rondo_core::domain::task::UndoKind::JournalDeleteEntry { .. }
                                    | rondo_core::domain::task::UndoKind::JournalDeleteDay { .. }
                            );
                            if let Err(e) = self.apply_undo(snap) {
                                self.toast(format!("undo failed: {}", e));
                            } else {
                                self.refresh_tasks();
                                if touches_journal {
                                    self.data.refresh_journal_notes();
                                    self.data.reload_journal_entries();
                                }
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
                use crate::app::modals_state::ModalLayer;
                let top = self.modals.top_modal();
                // Visual mode wins over a bare pomodoro overlay (preserves
                // pre-ModalLayer ordering).
                if matches!(top, None | Some(ModalLayer::Pomodoro))
                    && self.ui.mode == Mode::Visual
                {
                    self.ui.mode = Mode::Normal;
                    self.ui.selection.clear();
                } else {
                    match top {
                        Some(ModalLayer::PluginPage) => {
                            // Notify plugin BEFORE the field is cleared.
                            if let Some(id) = self.modals.plugin_page.take() {
                                let ctx = rondo_plugin_api::PluginContext::new(&id);
                                if let Some(p) = self.plugins.get_mut(&id) {
                                    let _ =
                                        p.handle(rondo_plugin_api::PluginAction::Hide, &ctx);
                                }
                            }
                        }
                        Some(layer) => {
                            let needs_normal_mode = matches!(
                                layer,
                                ModalLayer::DescriptionEditor
                                    | ModalLayer::EditSubtask
                                    | ModalLayer::NoteEditor
                                    | ModalLayer::EditTitle
                                    | ModalLayer::AddSubtask
                                    | ModalLayer::DepOverlay
                                    | ModalLayer::JournalEditor
                                    | ModalLayer::QuickAdd
                            );
                            let was_pomodoro = matches!(layer, ModalLayer::Pomodoro);
                            self.modals.close_top_modal();
                            if needs_normal_mode {
                                self.ui.mode = Mode::Normal;
                            }
                            if was_pomodoro {
                                self.finalize_pomodoro_close();
                            }
                        }
                        None => {
                            if self.status_msg.is_some() {
                                self.status_msg = None;
                            }
                        }
                    }
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
        let kind = snap.kind.clone();
        match kind {
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
            UndoKind::AddDep {
                task_id,
                blocker_id,
            } => {
                self.data.store.remove_dependency(task_id, blocker_id)?;
            }
            UndoKind::RemoveDep {
                task_id,
                blocker_id,
            } => {
                self.data.store.add_dependency(task_id, blocker_id)?;
            }
            UndoKind::DeleteSubtask { subtask, .. } => {
                self.data.store.restore_subtask(&subtask)?;
            }
            UndoKind::SubtaskToggle {
                subtask_id, before, ..
            } => {
                self.data.store.set_subtask_completed(subtask_id, before)?;
            }
            UndoKind::AddNote { note_id, .. } => {
                self.data.store.delete_task_note(note_id)?;
            }
            UndoKind::UpdateNote {
                note_id, before, ..
            } => {
                self.data.store.update_task_note(note_id, &before)?;
            }
            UndoKind::DeleteNote { note, .. } => {
                self.data.store.restore_task_note(&note)?;
            }
            UndoKind::JournalDeleteEntry { entry } => {
                self.data.store.restore_journal_entry(&entry)?;
            }
            UndoKind::JournalDeleteDay { note, entries } => {
                self.data.store.restore_journal_day(&note, &entries)?;
            }
        }
        Ok(())
    }

    fn jump_selection(&mut self, idx: usize) {
        match self.ui.page {
            Page::Tasks if !self.data.tasks.is_empty() => {
                let prev = self.data.selected_task;
                self.data.selected_task = idx.min(self.data.tasks.len() - 1);
                self.ui.selected_task_id =
                    self.data.tasks.get(self.data.selected_task).map(|t| t.id);
                self.data
                    .task_list_state
                    .select(Some(self.data.selected_task));
                let visible_len = self.data.visible_task_indices().len();
                self.adjust_task_list_scroll(self.data.selected_task, visible_len);
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

    /// Keep `selected_pos` (index into the visible slice) inside the
    /// viewport derived from `last_task_list_rect`. Subtracts a small
    /// constant for the column header + progress bar that share the
    /// inner panel area.
    fn adjust_task_list_scroll(&mut self, selected_pos: usize, total: usize) {
        let area_h = self.ui.last_task_list_rect.height as usize;
        // panel border (top + bottom) + column header + progress bar (2 lines)
        let chrome = 6usize;
        let viewport = area_h.saturating_sub(chrome).max(1);
        let scroll = self.ui.task_list_scroll;
        let new_scroll = if total == 0 {
            0
        } else if selected_pos < scroll {
            selected_pos
        } else if selected_pos >= scroll + viewport {
            selected_pos + 1 - viewport
        } else {
            scroll
        };
        let max_scroll = total.saturating_sub(1);
        self.ui.task_list_scroll = new_scroll.min(max_scroll);
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
                self.ui.selected_task_id = self.data.tasks.get(new_task).map(|t| t.id);
                // ListState position is relative to visible slice, not full tasks.
                self.data.task_list_state.select(Some(next));
                self.adjust_task_list_scroll(next, visible.len());
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

    pub(crate) fn move_journal_day(&mut self, delta: i32) {
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
        match self.ui.focus.pane {
            Pane::Detail if self.ui.focus.section == DetailSection::Subtasks => {
                self.toggle_focused_subtask();
            }
            Pane::List if self.ui.page == Page::Tasks => {
                self.cycle_focused_task_status();
            }
            _ => {}
        }
    }

    /// Cycle the currently-selected task's status Pending → InProgress → Done →
    /// Pending. Persists when `writable`; otherwise mutates the in-memory copy
    /// and toasts so the user still sees the cycle visually.
    fn cycle_focused_task_status(&mut self) {
        let (task_id, current) = match self.data.tasks.get(self.data.selected_task) {
            Some(t) => (t.id, t.status),
            None => return,
        };
        let next = current.next();
        if self.writable {
            match self.data.store.set_status(task_id, next) {
                Ok(snap) => {
                    self.undo.push(snap);
                    self.data.refresh_tasks();
                    self.ui.flash = Some((FlashTarget::Task(task_id), Instant::now()));
                    self.toast(format!("task #{} → {}", task_id, next.label()));
                }
                Err(e) => self.toast(format!("status change failed: {}", e)),
            }
        } else if let Some(t) = self.data.tasks.get_mut(self.data.selected_task) {
            t.status = next;
            self.ui.flash = Some((FlashTarget::Task(task_id), Instant::now()));
            self.toast(format!(
                "task #{} → {} (read-only, in-memory)",
                task_id,
                next.label()
            ));
        }
    }

    /// Toggle the subtask under the Detail::Subtasks cursor. Persists when
    /// `writable`; otherwise mutates the in-memory copy and toasts.
    fn toggle_focused_subtask(&mut self) {
        let item_idx = self.ui.focus.section_item;
        let (task_id, subtask_id, before_done) = match self.data.tasks.get(self.data.selected_task)
        {
            Some(t) => match t.subtasks.get(item_idx) {
                Some(s) => (t.id, s.id, s.completed),
                None => return,
            },
            None => return,
        };
        if self.writable {
            match self.data.store.toggle_subtask(subtask_id) {
                Ok((_, _legacy_snap)) => {
                    // Push an explicit-state snapshot instead of the legacy
                    // diff-based one so undo doesn't get confused if another
                    // subtask changes between toggle and undo.
                    self.undo
                        .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                            rondo_core::domain::task::UndoKind::SubtaskToggle {
                                task_id,
                                subtask_id,
                                before: before_done,
                            },
                        ));
                    self.refresh_tasks();
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
            self.toast(ro_msg("quick-add"));
            return;
        }
        let new_task = rondo_core::domain::task::NewTask {
            title: parsed.title.clone(),
            description: None,
            status: rondo_core::domain::task::Status::Pending,
            priority: parsed
                .priority
                .unwrap_or(rondo_core::domain::task::Priority::Low),
            due_date: parsed.due.as_deref().and_then(parse_due),
            recur_freq: rondo_core::domain::task::RecurFreq::None,
            recur_interval: 0,
            tags: parsed.tags.clone(),
        };
        match self.data.store.create_task(new_task) {
            Ok((_id, snap)) => {
                self.undo.push(snap);
                self.refresh_tasks();
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

    pub(crate) fn submit_journal_entry(&mut self) {
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

    pub(crate) fn delete_focused_journal_day(&mut self) {
        if self.data.journal_notes.is_empty() {
            return;
        }
        let idx = self
            .data
            .selected_journal
            .min(self.data.journal_notes.len() - 1);
        let note_clone = self.data.journal_notes[idx].clone();
        let note_id = note_clone.id;
        // Capture all entries before deletion so undo can fully restore them
        // (delete_note cascades through the FK).
        let entries = self
            .data
            .store
            .entries_for_note(note_id)
            .unwrap_or_default();
        match self.data.store.delete_note(note_id) {
            Ok(_) => {
                self.undo
                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                        rondo_core::domain::task::UndoKind::JournalDeleteDay {
                            note: note_clone,
                            entries,
                        },
                    ));
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

    pub(crate) fn delete_focused_journal_entry(&mut self) {
        if self.data.journal_entries.is_empty() {
            return;
        }
        let idx = self
            .data
            .selected_journal_entry
            .min(self.data.journal_entries.len() - 1);
        let entry_clone = self.data.journal_entries[idx].clone();
        let entry_id = entry_clone.id;
        match self.data.store.delete_entry(entry_id) {
            Ok(_) => {
                self.undo
                    .push(rondo_core::domain::task::UndoSnapshot::from_kind(
                        rondo_core::domain::task::UndoKind::JournalDeleteEntry {
                            entry: entry_clone,
                        },
                    ));
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
        match cmd.trim() {
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
            "" => {}
            other => self.status_msg = Some(format!("unknown: {}", other)),
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
    pub(crate) fn persist_pomodoro_start(&mut self) {
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
    pub(crate) fn finalize_pomodoro_close(&mut self) {
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
    }
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

/// Build the toast string shown when an action is rejected because the store was
/// opened read-only. The binary has no `--write` flag; the only way to mutate is
/// to relaunch without `--read-only`.
pub(crate) fn ro_msg(action: &str) -> String {
    format!("{action}: read-only mode (restart without --read-only)")
}

/// Parse a `due:` token value into a `NaiveDate`.
///
/// Accepts the natural-language aliases `today`/`hoy`, `tmrw`/`tomorrow`/`mañana`,
/// `next-week`/`semana`, or an ISO `YYYY-MM-DD` date.
pub fn parse_due(raw: &str) -> Option<chrono::NaiveDate> {
    use chrono::{Duration, Local, NaiveDate};
    let today = Local::now().date_naive();
    match raw.trim().to_lowercase().as_str() {
        "today" | "hoy" => Some(today),
        "tmrw" | "tomorrow" | "mañana" => today.checked_add_signed(Duration::days(1)),
        "next-week" | "semana" => today.checked_add_signed(Duration::days(7)),
        other => NaiveDate::parse_from_str(other, "%Y-%m-%d").ok(),
    }
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

    #[test]
    fn quick_add_sets_due_date_for_known_tokens() {
        use chrono::{Duration, Local};
        let today = Local::now().date_naive();
        assert_eq!(parse_due("today"), Some(today));
        assert_eq!(parse_due("hoy"), Some(today));
        assert_eq!(parse_due("tmrw"), Some(today + Duration::days(1)));
        assert_eq!(parse_due("tomorrow"), Some(today + Duration::days(1)));
        assert_eq!(parse_due("mañana"), Some(today + Duration::days(1)));
        assert_eq!(parse_due("next-week"), Some(today + Duration::days(7)));
        assert_eq!(parse_due("semana"), Some(today + Duration::days(7)));
        assert_eq!(
            parse_due("2026-12-31"),
            chrono::NaiveDate::from_ymd_opt(2026, 12, 31)
        );
        assert_eq!(parse_due("not-a-date"), None);
    }
}
