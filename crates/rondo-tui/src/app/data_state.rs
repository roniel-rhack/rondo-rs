use crate::action::{Action, Page};
use crate::filter::{Filter, SIDEBAR_ITEMS};
use chrono::Local;
use ratatui::widgets::ListState;
use rondo_core::domain::{
    journal::{Entry, Note},
    task::Task,
};
use std::collections::HashMap;
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
    /// Index into `journal_entries` for the currently focused entry (used
    /// for "edit this entry" and "delete this entry" actions).
    pub selected_journal_entry: usize,
    pub active_filter: Filter,
    pub journal_show_hidden: bool,
    /// Cached per-filter counts, refreshed alongside `tasks`. Sidebar and
    /// header use these instead of re-scanning `tasks` per render frame.
    pub filter_counts: HashMap<Filter, usize>,
    /// Lazily-built `title + tags + description` strings aligned by index
    /// with `tasks`. Refreshed alongside `tasks` so fuzzy search avoids
    /// re-formatting per call.
    pub task_haystacks: Vec<String>,
    /// Reused fuzzy-search engine. Pulled out of `DataState` so we keep
    /// nucleo's internal scratch buffers across frames instead of
    /// allocating a fresh `Matcher` per render.
    pub search_engine: std::cell::RefCell<crate::search::SearchEngine>,
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
        let mut state = Self {
            store,
            tasks,
            selected_task: 0,
            task_list_state,
            journal_notes,
            journal_entries,
            selected_journal: 0,
            journal_list_state,
            selected_journal_entry: 0,
            active_filter: Filter::Inbox,
            journal_show_hidden: false,
            filter_counts: HashMap::new(),
            task_haystacks: Vec::new(),
            search_engine: std::cell::RefCell::new(crate::search::SearchEngine::new()),
        };
        state.refresh_filter_counts();
        state.rebuild_haystacks();
        Ok(state)
    }

    fn rebuild_haystacks(&mut self) {
        self.task_haystacks.clear();
        self.task_haystacks.reserve(self.tasks.len());
        for t in &self.tasks {
            self.task_haystacks.push(format!(
                "{} {} {}",
                t.title,
                t.tags.join(" "),
                t.description.as_deref().unwrap_or("")
            ));
        }
    }

    /// Recompute the per-filter cache. Cheap: a single linear pass over
    /// `tasks` checking each `Filter` variant, sharing a `today` value.
    pub fn refresh_filter_counts(&mut self) {
        let today = Local::now().date_naive();
        let mut counts: HashMap<Filter, usize> = HashMap::with_capacity(SIDEBAR_ITEMS.len());
        for &f in SIDEBAR_ITEMS {
            counts.insert(f, 0);
        }
        for task in &self.tasks {
            for &f in SIDEBAR_ITEMS {
                if f.applies_to_with_today(task, today) {
                    *counts.entry(f).or_insert(0) += 1;
                }
            }
        }
        self.filter_counts = counts;
    }

    /// Currently selected task (if any), as a reference.
    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected_task)
    }

    /// Currently selected task id, if any.
    pub fn selected_task_id(&self) -> Option<i64> {
        self.tasks.get(self.selected_task).map(|t| t.id)
    }

    /// Reload tasks from the store. Used after mutations to keep the
    /// in-memory list in sync with persisted state.
    pub fn refresh_tasks(&mut self) {
        self.refresh_tasks_keeping_id(None);
    }

    /// Reload tasks from the store and, when `keep_id` is provided, restore
    /// `selected_task` to the row with that task id. Falls back to clamping
    /// the previous index when the id is no longer present (e.g. deleted).
    pub fn refresh_tasks_keeping_id(&mut self, keep_id: Option<i64>) {
        if let Ok(tasks) = self.store.list_tasks() {
            self.tasks = tasks;
        }
        if self.tasks.is_empty() {
            self.selected_task = 0;
            self.task_list_state.select(None);
        } else {
            if let Some(id) = keep_id {
                if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
                    self.selected_task = pos;
                }
            }
            if self.selected_task >= self.tasks.len() {
                self.selected_task = self.tasks.len() - 1;
            }
            self.task_list_state.select(Some(self.selected_task));
        }
        self.refresh_filter_counts();
        self.rebuild_haystacks();
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
        let mut engine = self.search_engine.borrow_mut();
        let mut scored: Vec<(u16, usize)> = base
            .into_iter()
            .filter_map(|i| {
                let hay = self.task_haystacks.get(i)?;
                engine.score_only(q, hay).map(|s| (s, i))
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
        if self.journal_entries.is_empty() {
            self.selected_journal_entry = 0;
        } else if self.selected_journal_entry >= self.journal_entries.len() {
            self.selected_journal_entry = self.journal_entries.len() - 1;
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
