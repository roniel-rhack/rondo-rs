pub const LIST_TASKS: &str = r#"
SELECT id, title, description, status, priority, due_date, created_at,
       recur_freq, recur_interval, COALESCE(metadata, '{}')
FROM tasks
ORDER BY status ASC, priority DESC, COALESCE(due_date, '9999-12-31') ASC, id DESC
"#;

pub const TASK_BY_ID: &str = r#"
SELECT id, title, description, status, priority, due_date, created_at,
       recur_freq, recur_interval, COALESCE(metadata, '{}')
FROM tasks WHERE id = ?1
"#;

pub const SUBTASKS_FOR_TASK: &str = r#"
SELECT id, task_id, title, completed, position
FROM subtasks WHERE task_id = ?1 ORDER BY position ASC
"#;

pub const TAGS_FOR_TASK: &str = r#"
SELECT name FROM tags WHERE task_id = ?1 ORDER BY name ASC
"#;

pub const TIME_LOGS_FOR_TASK: &str = r#"
SELECT id, task_id, duration, COALESCE(note, ''), logged_at
FROM time_logs WHERE task_id = ?1 ORDER BY logged_at DESC
"#;

pub const NOTES_FOR_TASK: &str = r#"
SELECT id, task_id, body, created_at
FROM task_notes WHERE task_id = ?1 ORDER BY created_at DESC
"#;

pub const BLOCKED_BY: &str = r#"
SELECT blocked_by FROM task_dependencies WHERE task_id = ?1
"#;

pub const BLOCKS: &str = r#"
SELECT task_id FROM task_dependencies WHERE blocked_by = ?1
"#;

pub const LIST_JOURNAL_NOTES: &str = r#"
SELECT id, date, hidden, created_at, updated_at
FROM journal_notes WHERE hidden = 0 ORDER BY date DESC LIMIT 365
"#;

pub const ENTRIES_FOR_NOTE: &str = r#"
SELECT id, note_id, body, created_at
FROM journal_entries WHERE note_id = ?1 ORDER BY created_at ASC
"#;

pub const INSERT_TASK: &str = r#"
INSERT INTO tasks (title, description, status, priority, due_date, created_at, recur_freq, recur_interval, metadata)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '{}')
"#;

pub const UPDATE_TASK_TITLE: &str = "UPDATE tasks SET title = ?1 WHERE id = ?2";
pub const UPDATE_TASK_DESCRIPTION: &str = "UPDATE tasks SET description = ?1 WHERE id = ?2";
pub const UPDATE_TASK_STATUS: &str = "UPDATE tasks SET status = ?1 WHERE id = ?2";
pub const UPDATE_TASK_PRIORITY: &str = "UPDATE tasks SET priority = ?1 WHERE id = ?2";
pub const UPDATE_TASK_DUE_DATE: &str = "UPDATE tasks SET due_date = ?1 WHERE id = ?2";
pub const UPDATE_TASK_RECUR_FREQ: &str = "UPDATE tasks SET recur_freq = ?1 WHERE id = ?2";
pub const UPDATE_TASK_RECUR_INTERVAL: &str = "UPDATE tasks SET recur_interval = ?1 WHERE id = ?2";

pub const DELETE_TASK: &str = "DELETE FROM tasks WHERE id = ?1";

pub const NEXT_SUBTASK_POSITION: &str =
    "SELECT COALESCE(MAX(position), -1) + 1 FROM subtasks WHERE task_id = ?1";

pub const INSERT_SUBTASK: &str =
    "INSERT INTO subtasks (task_id, title, completed, position) VALUES (?1, ?2, 0, ?3)";

pub const SUBTASK_LOOKUP: &str = "SELECT task_id, completed FROM subtasks WHERE id = ?1";

pub const UPDATE_SUBTASK_COMPLETED: &str = "UPDATE subtasks SET completed = ?1 WHERE id = ?2";

pub const INSERT_TAG: &str = "INSERT INTO tags (task_id, name) VALUES (?1, ?2)";
pub const DELETE_TAG: &str = "DELETE FROM tags WHERE task_id = ?1 AND name = ?2";

pub const JOURNAL_NOTE_BY_DATE: &str =
    "SELECT id, hidden, created_at, updated_at FROM journal_notes WHERE date = ?1";

pub const INSERT_JOURNAL_NOTE: &str =
    "INSERT INTO journal_notes (date, hidden, created_at, updated_at) VALUES (?1, 0, ?2, ?3)";

pub const INSERT_JOURNAL_ENTRY: &str =
    "INSERT INTO journal_entries (note_id, body, created_at) VALUES (?1, ?2, ?3)";

pub const TOUCH_JOURNAL_NOTE: &str = "UPDATE journal_notes SET updated_at = ?1 WHERE id = ?2";

pub const HIDE_JOURNAL_NOTE: &str = "UPDATE journal_notes SET hidden = 1 WHERE id = ?1";
pub const UNHIDE_JOURNAL_NOTE: &str = "UPDATE journal_notes SET hidden = 0 WHERE id = ?1";

pub const DELETE_JOURNAL_ENTRY: &str = "DELETE FROM journal_entries WHERE id = ?1";

pub const LIST_ALL_JOURNAL_NOTES: &str = r#"
SELECT id, date, hidden, created_at, updated_at
FROM journal_notes ORDER BY date DESC LIMIT 365
"#;
