use crate::domain::journal::{Entry, Note};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum Status {
    Pending = 0,
    InProgress = 1,
    Done = 2,
}

impl Status {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "◐",
            Self::Done => "✓",
        }
    }
    pub fn from_db(v: i64) -> Self {
        match v {
            1 => Self::InProgress,
            2 => Self::Done,
            _ => Self::Pending,
        }
    }
    /// Cycle Pending → InProgress → Done → Pending.
    pub fn next(self) -> Self {
        match self {
            Self::Pending => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Pending,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::InProgress => "InProgress",
            Self::Done => "Done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum Priority {
    Low = 0,
    Med = 1,
    High = 2,
    Urgent = 3,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Med => "MED",
            Self::High => "HIGH",
            Self::Urgent => "URG!",
        }
    }
    pub fn from_db(v: i64) -> Self {
        match v {
            1 => Self::Med,
            2 => Self::High,
            3 => Self::Urgent,
            _ => Self::Low,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum RecurFreq {
    None = 0,
    Daily = 1,
    Weekly = 2,
    Monthly = 3,
    Yearly = 4,
}

impl RecurFreq {
    pub fn from_db(v: i64) -> Self {
        match v {
            1 => Self::Daily,
            2 => Self::Weekly,
            3 => Self::Monthly,
            4 => Self::Yearly,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeLog {
    pub id: i64,
    pub task_id: i64,
    pub duration_secs: i64,
    pub note: Option<String>,
    pub logged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: i64,
    pub task_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub recur_freq: RecurFreq,
    pub recur_interval: i64,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub subtasks: Vec<Subtask>,
    pub time_logs: Vec<TimeLog>,
    pub notes: Vec<TaskNote>,
    pub blocked_by_ids: Vec<i64>,
    pub blocks_ids: Vec<i64>,
}

impl Task {
    pub fn is_blocked(&self) -> bool {
        !self.blocked_by_ids.is_empty()
    }
    pub fn subtask_progress(&self) -> (usize, usize) {
        let done = self.subtasks.iter().filter(|s| s.completed).count();
        (done, self.subtasks.len())
    }
}

#[derive(Debug, Clone)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub recur_freq: RecurFreq,
    pub recur_interval: i64,
    pub tags: Vec<String>,
}

impl NewTask {
    pub fn quick(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            status: Status::Pending,
            priority: Priority::Low,
            due_date: None,
            recur_freq: RecurFreq::None,
            recur_interval: 0,
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub due_date: Option<Option<NaiveDate>>,
    pub recur_freq: Option<RecurFreq>,
    pub recur_interval: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum UndoKind {
    Create,
    Update,
    Delete,
    SetStatus,
    AddSubtask,
    /// Legacy variant — kept for back-compat with `store.toggle_subtask` which
    /// still emits it. New TUI-side toggles use [`UndoKind::SubtaskToggle`] so
    /// the undo doesn't rely on diffing two task snapshots.
    ToggleSubtask,
    AddTag,
    RemoveTag,
    /// Dependency edge `task_id` ← `blocker_id` was added; undo removes it.
    AddDep { task_id: i64, blocker_id: i64 },
    /// Dependency edge `task_id` ← `blocker_id` was removed; undo re-adds it.
    RemoveDep { task_id: i64, blocker_id: i64 },
    /// Subtask was deleted; undo re-creates it (original id is lost but title +
    /// completion state are preserved).
    DeleteSubtask { task_id: i64, subtask: Subtask },
    /// Explicit subtask toggle that doesn't rely on diffing.
    SubtaskToggle {
        task_id: i64,
        subtask_id: i64,
        before: bool,
    },
    /// Task note was added; undo deletes the created note row.
    AddNote { task_id: i64, note_id: i64 },
    /// Task note body was edited; undo restores the previous body.
    UpdateNote {
        task_id: i64,
        note_id: i64,
        before: String,
    },
    /// Task note was deleted; undo re-creates it (original id is lost).
    DeleteNote { task_id: i64, note: TaskNote },
    /// Journal entry deleted; undo recreates it on the same note id.
    JournalDeleteEntry { entry: Entry },
    /// Journal day (note) deleted; undo recreates the day note and its entries.
    JournalDeleteDay { note: Note, entries: Vec<Entry> },
}

#[derive(Debug, Clone)]
pub struct UndoSnapshot {
    pub kind: UndoKind,
    pub task_before: Option<Task>,
    pub created_id: Option<i64>,
}

impl UndoSnapshot {
    /// Convenience: build a snapshot that only carries a `UndoKind` payload —
    /// used by handlers that don't need the legacy `task_before`/`created_id`
    /// fields (dep edges, note CRUD, journal deletes, explicit subtask toggle).
    pub fn from_kind(kind: UndoKind) -> Self {
        Self {
            kind,
            task_before: None,
            created_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trip() {
        for s in [Status::Pending, Status::InProgress, Status::Done] {
            assert_eq!(Status::from_db(s as i64), s);
        }
    }

    #[test]
    fn status_icons_match_go() {
        assert_eq!(Status::Pending.icon(), "○");
        assert_eq!(Status::InProgress.icon(), "◐");
        assert_eq!(Status::Done.icon(), "✓");
    }

    #[test]
    fn priority_labels_match_go() {
        assert_eq!(Priority::Low.label(), "LOW");
        assert_eq!(Priority::Med.label(), "MED");
        assert_eq!(Priority::High.label(), "HIGH");
        assert_eq!(Priority::Urgent.label(), "URG!");
    }
}
