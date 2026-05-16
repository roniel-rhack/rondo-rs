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
