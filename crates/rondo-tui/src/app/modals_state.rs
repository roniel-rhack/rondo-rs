use crate::action::Action;
use std::time::{Duration, Instant};
use throbber_widgets_tui::ThrobberState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepOverlayMode {
    Add,
    Remove,
}

/// Single source of truth for modal priority. Higher numeric value =
/// higher priority (closes first on Escape; intercepts input first).
///
/// The list mirrors the order used by `event.rs::map()` modal interception
/// and `EscapeContext` handling in `app/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ModalLayer {
    Pomodoro = 1,
    Help = 2,
    CommandPalette = 3,
    Search = 4,
    QuickAdd = 5,
    JournalEditor = 6,
    SortOverlay = 7,
    ConfirmDelete = 8,
    EditTitle = 9,
    AddSubtask = 10,
    DepOverlay = 11,
    QuickActions = 12,
    PluginsOverlay = 13,
    PluginPage = 14,
    DescriptionEditor = 15,
    EditSubtask = 16,
    NoteEditor = 17,
    EditDueDate = 18,
}

/// Modal/overlay UI state and associated buffers.
pub struct ModalsState {
    pub pomodoro_open: bool,
    pub pomodoro_started: Option<Instant>,
    pub pomodoro_total: Duration,
    pub pomodoro_throbber: ThrobberState,
    /// Row id of the in-flight `focus_sessions` row, if any. None when running
    /// against a read-only store or when no session is active.
    pub pomodoro_session_id: Option<i64>,
    pub command_palette_open: bool,
    pub command_buf: String,
    pub help_open: bool,
    pub search_open: bool,
    pub search_buf: String,
    pub quick_actions_open: bool,
    pub quick_add_open: bool,
    pub quick_add_buf: String,
    pub journal_editor_open: bool,
    pub journal_textarea: tui_textarea::TextArea<'static>,
    /// If `Some(id)`, the journal editor is editing an existing entry and
    /// submit will UPDATE rather than INSERT. `None` = new entry.
    pub journal_editor_entry_id: Option<i64>,
    pub sort_overlay_open: bool,
    pub confirm_delete_open: bool,
    pub edit_title_open: bool,
    pub edit_title_buf: String,
    pub add_subtask_open: bool,
    pub add_subtask_buf: String,
    pub dep_overlay_open: bool,
    pub dep_overlay_buf: String,
    pub dep_overlay_mode: DepOverlayMode,
    /// Highlighted row in the fuzzy task-picker results (E2). Reset whenever
    /// the buffer changes or the overlay opens.
    pub dep_overlay_cursor: usize,
    pub plugins_overlay_open: bool,
    /// If `Some(id)`, render the named plugin's last `Show` response
    /// full-screen as a page overlay. Set via `:<plugin>` commands.
    pub plugin_page: Option<String>,
    pub description_editor_open: bool,
    pub description_textarea: tui_textarea::TextArea<'static>,
    pub description_task_id: Option<i64>,
    pub edit_subtask_open: bool,
    pub edit_subtask_buf: String,
    pub edit_subtask_id: Option<i64>,
    pub note_editor_open: bool,
    pub note_textarea: tui_textarea::TextArea<'static>,
    /// `Some(id)` = editing an existing note; `None` = adding a new note
    /// to `note_task_id`.
    pub note_editing_id: Option<i64>,
    pub note_task_id: Option<i64>,
    /// EditDueDate modal — typed buffer when the user picks `c)ustom`.
    pub edit_due_date_open: bool,
    pub edit_due_date_buf: String,
    /// `true` once the user pressed `c` to start typing a custom date.
    pub edit_due_date_custom_mode: bool,
}

impl Default for ModalsState {
    fn default() -> Self {
        Self {
            pomodoro_open: false,
            pomodoro_started: None,
            pomodoro_total: Duration::from_secs(25 * 60),
            pomodoro_throbber: ThrobberState::default(),
            pomodoro_session_id: None,
            command_palette_open: false,
            command_buf: String::new(),
            help_open: false,
            search_open: false,
            search_buf: String::new(),
            quick_actions_open: false,
            quick_add_open: false,
            quick_add_buf: String::new(),
            journal_editor_open: false,
            journal_textarea: tui_textarea::TextArea::default(),
            journal_editor_entry_id: None,
            sort_overlay_open: false,
            confirm_delete_open: false,
            edit_title_open: false,
            edit_title_buf: String::new(),
            add_subtask_open: false,
            add_subtask_buf: String::new(),
            dep_overlay_open: false,
            dep_overlay_buf: String::new(),
            dep_overlay_mode: DepOverlayMode::Add,
            dep_overlay_cursor: 0,
            plugins_overlay_open: false,
            plugin_page: None,
            description_editor_open: false,
            description_textarea: tui_textarea::TextArea::default(),
            description_task_id: None,
            edit_subtask_open: false,
            edit_subtask_buf: String::new(),
            edit_subtask_id: None,
            note_editor_open: false,
            note_textarea: tui_textarea::TextArea::default(),
            note_editing_id: None,
            note_task_id: None,
            edit_due_date_open: false,
            edit_due_date_buf: String::new(),
            edit_due_date_custom_mode: false,
        }
    }
}

impl ModalsState {
    /// Highest-priority modal currently open, if any. Drives both event
    /// interception (event.rs) and Escape handling (app/mod.rs).
    ///
    /// Priority follows `ModalLayer`'s numeric values: input is routed to
    /// the topmost open layer, and Escape closes it before lower ones.
    pub fn top_modal(&self) -> Option<ModalLayer> {
        // Check from highest to lowest priority.
        if self.edit_due_date_open {
            return Some(ModalLayer::EditDueDate);
        }
        if self.note_editor_open {
            return Some(ModalLayer::NoteEditor);
        }
        if self.edit_subtask_open {
            return Some(ModalLayer::EditSubtask);
        }
        if self.description_editor_open {
            return Some(ModalLayer::DescriptionEditor);
        }
        if self.plugin_page.is_some() {
            return Some(ModalLayer::PluginPage);
        }
        if self.plugins_overlay_open {
            return Some(ModalLayer::PluginsOverlay);
        }
        if self.quick_actions_open {
            return Some(ModalLayer::QuickActions);
        }
        if self.dep_overlay_open {
            return Some(ModalLayer::DepOverlay);
        }
        if self.add_subtask_open {
            return Some(ModalLayer::AddSubtask);
        }
        if self.edit_title_open {
            return Some(ModalLayer::EditTitle);
        }
        if self.confirm_delete_open {
            return Some(ModalLayer::ConfirmDelete);
        }
        if self.sort_overlay_open {
            return Some(ModalLayer::SortOverlay);
        }
        if self.journal_editor_open {
            return Some(ModalLayer::JournalEditor);
        }
        if self.quick_add_open {
            return Some(ModalLayer::QuickAdd);
        }
        if self.search_open {
            return Some(ModalLayer::Search);
        }
        if self.command_palette_open {
            return Some(ModalLayer::CommandPalette);
        }
        if self.help_open {
            return Some(ModalLayer::Help);
        }
        if self.pomodoro_open {
            return Some(ModalLayer::Pomodoro);
        }
        None
    }

    pub fn open_help(&mut self) {
        self.help_open = true;
    }

    pub fn close_help(&mut self) {
        self.help_open = false;
    }

    pub fn open_command_palette(&mut self) {
        self.command_palette_open = true;
        self.command_buf.clear();
    }

    pub fn close_command_palette(&mut self) {
        self.command_palette_open = false;
        self.command_buf.clear();
    }

    pub fn open_search(&mut self) {
        self.search_open = true;
        self.search_buf.clear();
    }

    pub fn close_search(&mut self) {
        self.search_open = false;
        self.search_buf.clear();
    }

    pub fn open_quick_add(&mut self) {
        self.quick_add_open = true;
        self.quick_add_buf.clear();
    }

    pub fn close_quick_add(&mut self) {
        self.quick_add_open = false;
        self.quick_add_buf.clear();
    }

    pub fn open_edit_title(&mut self, current: String) {
        self.edit_title_buf = current;
        self.edit_title_open = true;
    }

    pub fn close_edit_title(&mut self) {
        self.edit_title_open = false;
        self.edit_title_buf.clear();
    }

    pub fn open_add_subtask(&mut self) {
        self.add_subtask_buf.clear();
        self.add_subtask_open = true;
    }

    pub fn close_add_subtask(&mut self) {
        self.add_subtask_open = false;
        self.add_subtask_buf.clear();
    }

    pub fn open_dep_overlay(&mut self, mode: DepOverlayMode) {
        self.dep_overlay_buf.clear();
        self.dep_overlay_open = true;
        self.dep_overlay_mode = mode;
        self.dep_overlay_cursor = 0;
    }

    pub fn close_dep_overlay(&mut self) {
        self.dep_overlay_open = false;
        self.dep_overlay_buf.clear();
        self.dep_overlay_cursor = 0;
    }

    pub fn open_edit_subtask(&mut self, id: i64, current: String) {
        self.edit_subtask_buf = current;
        self.edit_subtask_id = Some(id);
        self.edit_subtask_open = true;
    }

    pub fn close_edit_subtask(&mut self) {
        self.edit_subtask_open = false;
        self.edit_subtask_buf.clear();
        self.edit_subtask_id = None;
    }

    pub fn open_note_editor(&mut self, task_id: i64, editing: Option<(i64, &str)>) {
        let lines: Vec<String> = match editing {
            Some((_id, body)) => body.split('\n').map(|s| s.to_string()).collect(),
            None => Vec::new(),
        };
        self.note_textarea = if lines.is_empty() {
            tui_textarea::TextArea::default()
        } else {
            tui_textarea::TextArea::new(lines)
        };
        self.note_editing_id = editing.map(|(id, _)| id);
        self.note_task_id = Some(task_id);
        self.note_editor_open = true;
    }

    pub fn close_note_editor(&mut self) {
        self.note_editor_open = false;
        self.note_textarea = tui_textarea::TextArea::default();
        self.note_editing_id = None;
        self.note_task_id = None;
    }

    pub fn open_description_editor(&mut self, task_id: i64, body: &str) {
        self.description_textarea =
            tui_textarea::TextArea::new(body.split('\n').map(|s| s.to_string()).collect());
        self.description_task_id = Some(task_id);
        self.description_editor_open = true;
    }

    pub fn close_description_editor(&mut self) {
        self.description_editor_open = false;
        self.description_textarea = tui_textarea::TextArea::default();
        self.description_task_id = None;
    }

    pub fn open_edit_due_date(&mut self) {
        self.edit_due_date_open = true;
        self.edit_due_date_buf.clear();
        self.edit_due_date_custom_mode = false;
    }

    pub fn close_edit_due_date(&mut self) {
        self.edit_due_date_open = false;
        self.edit_due_date_buf.clear();
        self.edit_due_date_custom_mode = false;
    }

    pub fn open_journal_editor(&mut self, editing: Option<(i64, &str)>) {
        let lines: Vec<String> = match editing {
            Some((_id, body)) => body.split('\n').map(|s| s.to_string()).collect(),
            None => Vec::new(),
        };
        self.journal_textarea = if lines.is_empty() {
            tui_textarea::TextArea::default()
        } else {
            tui_textarea::TextArea::new(lines)
        };
        self.journal_editor_entry_id = editing.map(|(id, _)| id);
        self.journal_editor_open = true;
    }

    pub fn close_journal_editor(&mut self) {
        self.journal_editor_open = false;
        self.journal_textarea = tui_textarea::TextArea::default();
        self.journal_editor_entry_id = None;
    }

    pub fn open_sort_overlay(&mut self) {
        self.sort_overlay_open = true;
    }

    pub fn close_sort_overlay(&mut self) {
        self.sort_overlay_open = false;
    }

    pub fn open_confirm_delete(&mut self) {
        self.confirm_delete_open = true;
    }

    pub fn close_confirm_delete(&mut self) {
        self.confirm_delete_open = false;
    }

    /// Close the topmost modal, returning the layer that was closed.
    /// Returns `None` if no modal is open.
    ///
    /// Callers that need to perform cross-substate side-effects (e.g.
    /// resetting `ui.mode = Normal`, notifying plugin Hide, finalising
    /// pomodoro) should match on the returned layer.
    pub fn close_top_modal(&mut self) -> Option<ModalLayer> {
        let layer = self.top_modal()?;
        match layer {
            ModalLayer::EditDueDate => {
                self.edit_due_date_open = false;
                self.edit_due_date_buf.clear();
                self.edit_due_date_custom_mode = false;
            }
            ModalLayer::NoteEditor => {
                self.note_editor_open = false;
                self.note_textarea = tui_textarea::TextArea::default();
                self.note_editing_id = None;
                self.note_task_id = None;
            }
            ModalLayer::EditSubtask => {
                self.edit_subtask_open = false;
                self.edit_subtask_buf.clear();
                self.edit_subtask_id = None;
            }
            ModalLayer::DescriptionEditor => {
                self.description_editor_open = false;
                self.description_textarea = tui_textarea::TextArea::default();
                self.description_task_id = None;
            }
            ModalLayer::PluginPage => {
                // Caller should notify the plugin (needs &mut plugins).
                self.plugin_page = None;
            }
            ModalLayer::PluginsOverlay => {
                self.plugins_overlay_open = false;
            }
            ModalLayer::QuickActions => {
                self.quick_actions_open = false;
            }
            ModalLayer::DepOverlay => {
                self.dep_overlay_open = false;
                self.dep_overlay_buf.clear();
                self.dep_overlay_cursor = 0;
            }
            ModalLayer::AddSubtask => {
                self.add_subtask_open = false;
                self.add_subtask_buf.clear();
            }
            ModalLayer::EditTitle => {
                self.edit_title_open = false;
                self.edit_title_buf.clear();
            }
            ModalLayer::ConfirmDelete => {
                self.confirm_delete_open = false;
            }
            ModalLayer::SortOverlay => {
                self.sort_overlay_open = false;
            }
            ModalLayer::JournalEditor => {
                self.journal_editor_open = false;
                self.journal_textarea = tui_textarea::TextArea::default();
                self.journal_editor_entry_id = None;
            }
            ModalLayer::QuickAdd => {
                self.quick_add_open = false;
                self.quick_add_buf.clear();
            }
            ModalLayer::Search => {
                self.search_open = false;
                self.search_buf.clear();
            }
            ModalLayer::CommandPalette => {
                self.command_palette_open = false;
            }
            ModalLayer::Help => {
                self.help_open = false;
            }
            ModalLayer::Pomodoro => {
                self.pomodoro_open = false;
            }
        }
        Some(layer)
    }

    /// Any modal open?
    pub fn any_open(&self) -> bool {
        self.pomodoro_open
            || self.command_palette_open
            || self.help_open
            || self.search_open
            || self.quick_actions_open
            || self.quick_add_open
            || self.journal_editor_open
            || self.sort_overlay_open
            || self.confirm_delete_open
            || self.edit_title_open
            || self.add_subtask_open
            || self.dep_overlay_open
            || self.plugins_overlay_open
            || self.plugin_page.is_some()
            || self.description_editor_open
            || self.edit_subtask_open
            || self.note_editor_open
            || self.edit_due_date_open
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
            Action::JournalEntryInput(_s) => {
                // Dead code path: kept for action vocabulary stability.
                // `journal_textarea` owns the editor text now.
                None
            }
            Action::JournalCancelEntry => {
                self.journal_editor_open = false;
                self.journal_textarea = tui_textarea::TextArea::default();
                self.journal_editor_entry_id = None;
                None
            }
            Action::EditTitleInput(s) => {
                self.edit_title_buf = s;
                None
            }
            Action::EditSubtaskInput(s) => {
                self.edit_subtask_buf = s;
                None
            }
            Action::AddSubtaskInput(s) => {
                self.add_subtask_buf = s;
                None
            }
            Action::CancelAddSubtask => {
                self.add_subtask_open = false;
                self.add_subtask_buf.clear();
                None
            }
            Action::DepOverlayInput(s) => {
                self.dep_overlay_buf = s;
                self.dep_overlay_cursor = 0;
                None
            }
            Action::CancelDepOverlay => {
                self.dep_overlay_open = false;
                self.dep_overlay_buf.clear();
                self.dep_overlay_cursor = 0;
                None
            }
            Action::ToggleDepOverlayMode => {
                self.dep_overlay_mode = match self.dep_overlay_mode {
                    DepOverlayMode::Add => DepOverlayMode::Remove,
                    DepOverlayMode::Remove => DepOverlayMode::Add,
                };
                None
            }
            Action::OpenSortOverlay => {
                self.sort_overlay_open = true;
                None
            }
            Action::CloseSortOverlay => {
                self.sort_overlay_open = false;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_modal_none_when_idle() {
        let m = ModalsState::default();
        assert_eq!(m.top_modal(), None);
    }

    #[test]
    fn top_modal_respects_priority() {
        // Open several lower-priority modals; a high-priority one wins.
        let mut m = ModalsState {
            help_open: true,
            search_open: true,
            command_palette_open: true,
            ..ModalsState::default()
        };
        assert_eq!(m.top_modal(), Some(ModalLayer::Search));

        m.note_editor_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::NoteEditor));

        m.note_editor_open = false;
        m.edit_subtask_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::EditSubtask));

        m.edit_subtask_open = false;
        m.description_editor_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::DescriptionEditor));

        m.description_editor_open = false;
        m.plugin_page = Some("foo".into());
        assert_eq!(m.top_modal(), Some(ModalLayer::PluginPage));

        m.plugin_page = None;
        m.plugins_overlay_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::PluginsOverlay));

        m.plugins_overlay_open = false;
        m.quick_actions_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::QuickActions));

        m.quick_actions_open = false;
        m.dep_overlay_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::DepOverlay));

        m.dep_overlay_open = false;
        m.add_subtask_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::AddSubtask));

        m.add_subtask_open = false;
        m.edit_title_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::EditTitle));

        m.edit_title_open = false;
        m.confirm_delete_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::ConfirmDelete));

        m.confirm_delete_open = false;
        m.sort_overlay_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::SortOverlay));

        m.sort_overlay_open = false;
        m.journal_editor_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::JournalEditor));

        m.journal_editor_open = false;
        m.quick_add_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::QuickAdd));

        m.quick_add_open = false;
        // Search > CommandPalette > Help.
        assert_eq!(m.top_modal(), Some(ModalLayer::Search));
        m.search_open = false;
        assert_eq!(m.top_modal(), Some(ModalLayer::CommandPalette));
        m.command_palette_open = false;
        assert_eq!(m.top_modal(), Some(ModalLayer::Help));
        m.help_open = false;
        m.pomodoro_open = true;
        assert_eq!(m.top_modal(), Some(ModalLayer::Pomodoro));
        m.pomodoro_open = false;
        assert_eq!(m.top_modal(), None);
    }
}
