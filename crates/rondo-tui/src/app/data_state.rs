use crate::action::{Action, Page};
use crate::filter::Filter;
use ratatui::widgets::ListState;
use rondo_core::domain::{
    journal::{Entry, Note},
    task::Task,
};
use std::sync::Arc;

/// Domain data + read-only store + selection indexes.
pub struct DataState {
    pub store: Arc<rondo_core::store::sqlite::SqliteStore>,
    pub tasks: Vec<Task>,
    pub selected_task: usize,
    pub task_list_state: ListState,
    pub journal_notes: Vec<Note>,
    pub journal_entries: Vec<Entry>,
    pub selected_journal: usize,
    pub journal_list_state: ListState,
    pub active_filter: Filter,
    pub journal_show_hidden: bool,
}

impl DataState {
    pub fn new(
        store: Arc<rondo_core::store::sqlite::SqliteStore>,
    ) -> color_eyre::eyre::Result<Self> {
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
            store,
            tasks,
            selected_task: 0,
            task_list_state,
            journal_notes,
            journal_entries,
            selected_journal: 0,
            journal_list_state,
            active_filter: Filter::Inbox,
            journal_show_hidden: false,
        })
    }

    /// Reload journal notes from the store, honoring the `journal_show_hidden` flag.
    /// Also refreshes entries for the currently-selected note (clamped).
    pub fn refresh_journal_notes(&mut self) {
        let notes = if self.journal_show_hidden {
            self.store
                .list_all_journal_notes_including_hidden()
                .unwrap_or_default()
        } else {
            self.store.list_journal_notes().unwrap_or_default()
        };
        self.journal_notes = notes;
        if self.journal_notes.is_empty() {
            self.selected_journal = 0;
            self.journal_list_state.select(None);
            self.journal_entries.clear();
            return;
        }
        if self.selected_journal >= self.journal_notes.len() {
            self.selected_journal = self.journal_notes.len() - 1;
        }
        self.journal_list_state.select(Some(self.selected_journal));
        self.reload_journal_entries();
    }

    /// Indices of tasks that pass the current filter, in their original order.
    pub fn visible_task_indices(&self) -> Vec<usize> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| self.active_filter.applies_to(t))
            .map(|(i, _)| i)
            .collect()
    }

    /// Number of tasks passing the current filter.
    pub fn visible_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| self.active_filter.applies_to(t))
            .count()
    }

    /// Reload entries for the currently selected journal note.
    pub fn reload_journal_entries(&mut self) {
        if let Some(n) = self.journal_notes.get(self.selected_journal) {
            if let Ok(e) = self.store.entries_for_note(n.id) {
                self.journal_entries = e;
            }
        }
    }

    /// Pure data-mutation handler. Returns optional follow-up for the dispatcher.
    /// Most data changes are driven by other substates (which need cursor + focus
    /// info) so this handler stays small for now.
    pub fn update(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::TogglePage(p) if p == Page::Tasks || p == Page::Journal => {
                // Page is owned by UiState; this is a no-op at the data layer.
                None
            }
            _ => None,
        }
    }
}
