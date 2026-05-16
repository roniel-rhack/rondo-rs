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
        })
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

    /// Indices of tasks that pass the current filter AND match the given
    /// fuzzy query (`title + tags + description`). Results are sorted by
    /// score descending. Empty/whitespace query falls back to the filter
    /// result in declared order.
    pub fn visible_task_indices_with_search(&self, query: &str) -> Vec<usize> {
        let q = query.trim();
        let base = self.visible_task_indices();
        if q.is_empty() {
            return base;
        }
        let mut engine = crate::search::SearchEngine::new();
        let mut scored: Vec<(u16, usize)> = base
            .into_iter()
            .filter_map(|i| {
                let t = &self.tasks[i];
                let hay = format!(
                    "{} {} {}",
                    t.title,
                    t.tags.join(" "),
                    t.description.as_deref().unwrap_or("")
                );
                engine.score_only(q, &hay).map(|s| (s, i))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i)| i).collect()
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
