use crate::domain::{
    focus::{Session, SessionKind},
    journal::{Entry, Note},
    task::{
        NewTask, Priority, RecurFreq, Status, Subtask, Task, TaskNote, TaskPatch, TimeLog,
        UndoKind, UndoSnapshot,
    },
};
use crate::error::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OpenFlags, Row};
use std::path::Path;
use std::sync::Mutex;

/// Maximum byte length for free-form text fields stored in the DB. These
/// are upper bounds against accidental or malicious giant inputs (paste
/// loops, runaway scripts) rather than a UX hint — normal use is well
/// below these caps. Validated at the store boundary so every caller
/// (TUI, CLI, plugins, future RPC) is covered.
const MAX_TITLE: usize = 500;
const MAX_DESC: usize = 50_000;
const MAX_NOTE: usize = 50_000;
const MAX_JOURNAL: usize = 100_000;
const MAX_TAG: usize = 64;

fn check_len(field: &'static str, value: &str, max: usize) -> Result<()> {
    if value.len() > max {
        return Err(crate::error::Error::InputTooLong { field, max });
    }
    Ok(())
}

pub struct SqliteStore {
    pub(super) conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open_readonly<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "query_only", true)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_readwrite<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path.as_ref(),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        super::migrations::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_TASKS)?;
        let mut tasks: Vec<Task> = stmt
            .query_map([], row_to_task_shallow)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for t in &mut tasks {
            hydrate(&conn, t)?;
        }
        Ok(tasks)
    }

    pub fn task_by_id(&self, id: i64) -> Result<Task> {
        let conn = self.conn.lock().unwrap();
        let mut t = conn.query_row(super::queries::TASK_BY_ID, params![id], row_to_task_shallow)?;
        hydrate(&conn, &mut t)?;
        Ok(t)
    }

    pub fn list_journal_notes(&self) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_JOURNAL_NOTES)?;
        let rows = stmt
            .query_map([], row_to_note)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn entries_for_note(&self, note_id: i64) -> Result<Vec<Entry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::ENTRIES_FOR_NOTE)?;
        let rows = stmt
            .query_map(params![note_id], row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn create_task(&self, input: NewTask) -> Result<(i64, UndoSnapshot)> {
        check_len("title", &input.title, MAX_TITLE)?;
        if let Some(desc) = input.description.as_deref() {
            check_len("description", desc, MAX_DESC)?;
        }
        for tag in &input.tags {
            check_len("tag", tag, MAX_TAG)?;
        }
        let due = input.due_date.map(|d| d.format("%Y-%m-%d").to_string());
        let created_at = Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            super::queries::INSERT_TASK,
            params![
                input.title,
                input.description,
                input.status as i64,
                input.priority as i64,
                due,
                created_at,
                input.recur_freq as i64,
                input.recur_interval,
            ],
        )?;
        let id = tx.last_insert_rowid();
        for tag in &input.tags {
            tx.execute(super::queries::INSERT_TAG, params![id, tag])?;
        }
        tx.commit()?;
        Ok((
            id,
            UndoSnapshot {
                kind: UndoKind::Create,
                task_before: None,
                created_id: Some(id),
            },
        ))
    }

    pub fn update_task(&self, id: i64, patch: TaskPatch) -> Result<UndoSnapshot> {
        if let Some(title) = patch.title.as_deref() {
            check_len("title", title, MAX_TITLE)?;
        }
        if let Some(Some(desc)) = patch.description.as_ref() {
            check_len("description", desc, MAX_DESC)?;
        }
        let before = self.task_by_id(id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        if let Some(title) = patch.title.as_ref() {
            tx.execute(super::queries::UPDATE_TASK_TITLE, params![title, id])?;
        }
        if let Some(desc) = &patch.description {
            tx.execute(super::queries::UPDATE_TASK_DESCRIPTION, params![desc, id])?;
        }
        if let Some(s) = patch.status {
            tx.execute(super::queries::UPDATE_TASK_STATUS, params![s as i64, id])?;
        }
        if let Some(p) = patch.priority {
            tx.execute(super::queries::UPDATE_TASK_PRIORITY, params![p as i64, id])?;
        }
        if let Some(due) = &patch.due_date {
            let s = due.map(|d| d.format("%Y-%m-%d").to_string());
            tx.execute(super::queries::UPDATE_TASK_DUE_DATE, params![s, id])?;
        }
        if let Some(r) = patch.recur_freq {
            tx.execute(
                super::queries::UPDATE_TASK_RECUR_FREQ,
                params![r as i64, id],
            )?;
        }
        if let Some(ri) = patch.recur_interval {
            tx.execute(super::queries::UPDATE_TASK_RECUR_INTERVAL, params![ri, id])?;
        }
        tx.commit()?;
        Ok(UndoSnapshot {
            kind: UndoKind::Update,
            task_before: Some(before),
            created_id: None,
        })
    }

    pub fn delete_task(&self, id: i64) -> Result<UndoSnapshot> {
        let before = self.task_by_id(id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(super::queries::DELETE_TASK, params![id])?;
        tx.commit()?;
        Ok(UndoSnapshot {
            kind: UndoKind::Delete,
            task_before: Some(before),
            created_id: None,
        })
    }

    pub fn set_status(&self, id: i64, status: Status) -> Result<UndoSnapshot> {
        let before = self.task_by_id(id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            super::queries::UPDATE_TASK_STATUS,
            params![status as i64, id],
        )?;
        tx.commit()?;
        Ok(UndoSnapshot {
            kind: UndoKind::SetStatus,
            task_before: Some(before),
            created_id: None,
        })
    }

    pub fn add_subtask(&self, task_id: i64, title: &str) -> Result<(i64, UndoSnapshot)> {
        check_len("subtask title", title, MAX_TITLE)?;
        let before = self.task_by_id(task_id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let position: i64 = tx.query_row(
            super::queries::NEXT_SUBTASK_POSITION,
            params![task_id],
            |r| r.get(0),
        )?;
        tx.execute(
            super::queries::INSERT_SUBTASK,
            params![task_id, title, position],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok((
            id,
            UndoSnapshot {
                kind: UndoKind::AddSubtask,
                task_before: Some(before),
                created_id: Some(id),
            },
        ))
    }

    pub fn delete_subtask(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::DELETE_SUBTASK, params![id])?;
        Ok(())
    }

    pub fn update_subtask_title(&self, id: i64, title: &str) -> Result<()> {
        check_len("subtask title", title, MAX_TITLE)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::UPDATE_SUBTASK_TITLE, params![title, id])?;
        Ok(())
    }

    pub fn add_task_note(&self, task_id: i64, body: &str) -> Result<i64> {
        check_len("note", body, MAX_NOTE)?;
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            super::queries::INSERT_TASK_NOTE,
            params![task_id, body, &now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_task_note(&self, id: i64, body: &str) -> Result<()> {
        check_len("note", body, MAX_NOTE)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::UPDATE_TASK_NOTE, params![body, id])?;
        Ok(())
    }

    pub fn delete_task_note(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::DELETE_TASK_NOTE, params![id])?;
        Ok(())
    }

    pub fn toggle_subtask(&self, id: i64) -> Result<(bool, UndoSnapshot)> {
        let task_id: i64;
        let new_completed: i64;
        {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn.transaction()?;
            let (tid, completed): (i64, i64) =
                tx.query_row(super::queries::SUBTASK_LOOKUP, params![id], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            task_id = tid;
            new_completed = if completed == 0 { 1 } else { 0 };
            tx.execute(
                super::queries::UPDATE_SUBTASK_COMPLETED,
                params![new_completed, id],
            )?;
            tx.commit()?;
        }
        let before = self.task_by_id(task_id)?;
        Ok((
            new_completed != 0,
            UndoSnapshot {
                kind: UndoKind::ToggleSubtask,
                task_before: Some(before),
                created_id: None,
            },
        ))
    }

    pub fn add_tag(&self, task_id: i64, name: &str) -> Result<UndoSnapshot> {
        check_len("tag", name, MAX_TAG)?;
        let before = self.task_by_id(task_id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(super::queries::INSERT_TAG, params![task_id, name])?;
        tx.commit()?;
        Ok(UndoSnapshot {
            kind: UndoKind::AddTag,
            task_before: Some(before),
            created_id: None,
        })
    }

    pub fn remove_tag(&self, task_id: i64, name: &str) -> Result<UndoSnapshot> {
        let before = self.task_by_id(task_id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(super::queries::DELETE_TAG, params![task_id, name])?;
        tx.commit()?;
        Ok(UndoSnapshot {
            kind: UndoKind::RemoveTag,
            task_before: Some(before),
            created_id: None,
        })
    }

    /// Adds `blocked_by` as a dependency of `task_id`. Rejects with
    /// `Error::CycleDetected` if this edge would create a cycle in the
    /// dependency DAG. Idempotent: re-adding the same edge is a no-op.
    pub fn add_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if would_create_cycle(&conn, task_id, blocked_by)? {
            return Err(crate::error::Error::CycleDetected(task_id, blocked_by));
        }
        conn.execute(
            super::queries::INSERT_DEPENDENCY,
            params![task_id, blocked_by],
        )?;
        Ok(())
    }

    /// Removes a dependency edge. Returns Ok even if the edge didn't exist.
    pub fn remove_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            super::queries::DELETE_DEPENDENCY,
            params![task_id, blocked_by],
        )?;
        Ok(())
    }

    /// Persist a freshly started focus session. Returns the row id.
    pub fn start_focus_session(
        &self,
        task_id: Option<i64>,
        kind: SessionKind,
        duration_secs: u64,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            super::queries::INSERT_FOCUS_SESSION,
            params![task_id, kind as i64, &now, duration_secs as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Mark a previously-started session as completed (sets `completed_at`).
    pub fn complete_focus_session(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(super::queries::COMPLETE_FOCUS_SESSION, params![&now, id])?;
        Ok(())
    }

    /// Count of consecutive days (going back from today, UTC) with at least
    /// one completed Work session. Returns 0 if today has none.
    pub fn focus_streak(&self) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::FOCUS_STREAK_DATES)?;
        let dates: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if dates.is_empty() {
            return Ok(0);
        }
        let today = Utc::now().date_naive();
        let mut streak: u32 = 0;
        let mut expected = today;
        for d in dates {
            let parsed = chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok();
            match parsed {
                Some(p) if p == expected => {
                    streak += 1;
                    expected = expected.pred_opt().unwrap_or(expected);
                }
                Some(p) if p < expected => break,
                _ => break,
            }
        }
        Ok(streak)
    }

    /// All focus sessions, newest first (capped to 1000).
    pub fn list_focus_sessions(&self) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_FOCUS_SESSIONS)?;
        let rows = stmt
            .query_map([], |r| {
                let started: String = r.get(3)?;
                let completed: Option<String> = r.get(4)?;
                Ok(Session {
                    id: Some(r.get::<_, i64>(0)?),
                    task_id: r.get::<_, Option<i64>>(1)?,
                    kind: SessionKind::from_db(r.get::<_, i64>(2)?),
                    cycle_pos: 0,
                    started_at: parse_dt_sql(&started)?,
                    completed_at: completed.as_deref().map(parse_dt_sql).transpose()?,
                    duration_secs: r.get::<_, i64>(5)? as u64,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Returns the today-dated note, creating it if it doesn't exist yet.
    /// "Today" is interpreted in the user's LOCAL timezone — using UTC would
    /// roll the date over at midnight UTC, producing a "tomorrow" note for
    /// users west of UTC who write entries after their evening cutoff.
    pub fn create_or_get_today_note(&self) -> Result<Note> {
        let today = chrono::Local::now().date_naive();
        self.create_or_get_note_for(today)
    }

    pub fn create_or_get_note_for(&self, date: NaiveDate) -> Result<Note> {
        let conn = self.conn.lock().unwrap();
        let date_s = date.format("%Y-%m-%d").to_string();
        let existing: rusqlite::Result<(i64, i64, String, String)> = conn.query_row(
            super::queries::JOURNAL_NOTE_BY_DATE,
            params![&date_s],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        );
        match existing {
            Ok((id, hidden, created_at, updated_at)) => Ok(Note {
                id,
                date,
                hidden: hidden != 0,
                created_at: parse_dt(&created_at)?,
                updated_at: parse_dt(&updated_at)?,
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    super::queries::INSERT_JOURNAL_NOTE,
                    params![&date_s, &now, &now],
                )?;
                let id = conn.last_insert_rowid();
                Ok(Note {
                    id,
                    date,
                    hidden: false,
                    created_at: parse_dt(&now)?,
                    updated_at: parse_dt(&now)?,
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn add_journal_entry(&self, note_id: i64, body: &str) -> Result<i64> {
        check_len("journal entry", body, MAX_JOURNAL)?;
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            super::queries::INSERT_JOURNAL_ENTRY,
            params![note_id, body, &now],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(super::queries::TOUCH_JOURNAL_NOTE, params![&now, note_id])?;
        tx.commit()?;
        Ok(id)
    }

    pub fn hide_note(&self, note_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::HIDE_JOURNAL_NOTE, params![note_id])?;
        Ok(())
    }

    pub fn unhide_note(&self, note_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::UNHIDE_JOURNAL_NOTE, params![note_id])?;
        Ok(())
    }

    pub fn delete_entry(&self, entry_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::DELETE_JOURNAL_ENTRY, params![entry_id])?;
        Ok(())
    }

    /// Hard-delete a whole day's note. The FK on `journal_entries.note_id`
    /// is declared `ON DELETE CASCADE` so child rows go with it.
    pub fn delete_note(&self, note_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(super::queries::DELETE_JOURNAL_NOTE, params![note_id])?;
        Ok(())
    }

    /// Replace the body text of an existing journal entry. Also bumps the
    /// parent note's `updated_at`. Inside a single transaction.
    pub fn update_journal_entry(&self, entry_id: i64, body: &str) -> Result<()> {
        check_len("journal entry", body, MAX_JOURNAL)?;
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let note_id: i64 = tx.query_row(
            super::queries::NOTE_ID_FOR_ENTRY,
            params![entry_id],
            |r| r.get(0),
        )?;
        let now = Utc::now().to_rfc3339();
        tx.execute(
            super::queries::UPDATE_JOURNAL_ENTRY_BODY,
            params![body, entry_id],
        )?;
        tx.execute(super::queries::TOUCH_JOURNAL_NOTE, params![&now, note_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_all_journal_notes_including_hidden(&self) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_ALL_JOURNAL_NOTES)?;
        let rows = stmt
            .query_map([], row_to_note)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

/// Bound on how many nodes `would_create_cycle` will visit before
/// giving up. Guards against pathological graphs (or future bugs that
/// don't terminate the traversal) eating CPU/RAM. 1000 is far above any
/// realistic dep graph for a personal task manager.
const MAX_CYCLE_DEPTH: usize = 1000;

fn would_create_cycle(conn: &Connection, task_id: i64, blocked_by: i64) -> Result<bool> {
    if task_id == blocked_by {
        return Ok(true);
    }
    let mut stack = vec![blocked_by];
    let mut visited = std::collections::HashSet::new();
    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        if visited.len() > MAX_CYCLE_DEPTH {
            return Err(crate::error::Error::CycleDepthExceeded);
        }
        if current == task_id {
            return Ok(true);
        }
        let mut stmt = conn.prepare(super::queries::BLOCKED_BY)?;
        let rows = stmt.query_map(params![current], |r| r.get::<_, i64>(0))?;
        for row in rows {
            let next = row?;
            if !visited.contains(&next) {
                stack.push(next);
            }
        }
    }
    Ok(false)
}

fn row_to_task_shallow(r: &Row<'_>) -> rusqlite::Result<Task> {
    let task_id: i64 = r.get(0)?;
    let metadata_json: String = r.get(9)?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or_else(|e| {
        tracing::warn!(task_id, error = ?e, "corrupt metadata json, defaulting to empty");
        Default::default()
    });
    let due_str: Option<String> = r.get(5)?;
    let created_str: String = r.get(6)?;
    Ok(Task {
        id: task_id,
        title: r.get(1)?,
        description: r.get(2)?,
        status: Status::from_db(r.get(3)?),
        priority: Priority::from_db(r.get(4)?),
        due_date: due_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        created_at: parse_dt_sql(&created_str)?,
        recur_freq: RecurFreq::from_db(r.get(7)?),
        recur_interval: r.get(8)?,
        metadata,
        tags: vec![],
        subtasks: vec![],
        time_logs: vec![],
        notes: vec![],
        blocked_by_ids: vec![],
        blocks_ids: vec![],
    })
}

fn hydrate(conn: &Connection, t: &mut Task) -> Result<()> {
    {
        let mut s = conn.prepare(super::queries::SUBTASKS_FOR_TASK)?;
        t.subtasks = s
            .query_map(params![t.id], |r| {
                Ok(Subtask {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    title: r.get(2)?,
                    completed: r.get::<_, i64>(3)? != 0,
                    position: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    {
        let mut s = conn.prepare(super::queries::TAGS_FOR_TASK)?;
        t.tags = s
            .query_map(params![t.id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    {
        let mut s = conn.prepare(super::queries::TIME_LOGS_FOR_TASK)?;
        t.time_logs = s
            .query_map(params![t.id], |r| {
                let note_str: String = r.get(3)?;
                Ok(TimeLog {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    duration_secs: r.get(2)?,
                    note: if note_str.is_empty() {
                        None
                    } else {
                        Some(note_str)
                    },
                    logged_at: parse_dt_sql(&r.get::<_, String>(4)?)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    {
        let mut s = conn.prepare(super::queries::NOTES_FOR_TASK)?;
        t.notes = s
            .query_map(params![t.id], |r| {
                Ok(TaskNote {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    body: r.get(2)?,
                    created_at: parse_dt_sql(&r.get::<_, String>(3)?)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    {
        let mut s = conn.prepare(super::queries::BLOCKED_BY)?;
        t.blocked_by_ids = s
            .query_map(params![t.id], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    {
        let mut s = conn.prepare(super::queries::BLOCKS)?;
        t.blocks_ids = s
            .query_map(params![t.id], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    Ok(())
}

fn row_to_note(r: &Row<'_>) -> rusqlite::Result<Note> {
    let date_str: String = r.get(1)?;
    Ok(Note {
        id: r.get(0)?,
        date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch date is valid")),
        hidden: r.get::<_, i64>(2)? != 0,
        created_at: parse_dt_sql(&r.get::<_, String>(3)?)?,
        updated_at: parse_dt_sql(&r.get::<_, String>(4)?)?,
    })
}

fn row_to_entry(r: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?,
        note_id: r.get(1)?,
        body: r.get(2)?,
        created_at: parse_dt_sql(&r.get::<_, String>(3)?)?,
    })
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Ok(d.with_timezone(&Utc));
    }
    if let Ok(nd) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(nd, Utc));
    }
    Err(crate::error::Error::ParseDate(s.to_string()))
}

/// Adapter for use inside rusqlite row-mapper closures (which must return
/// `rusqlite::Result<T>`). Wraps a `parse_dt` failure as a SQLite
/// conversion-failure error so the row read fails loudly instead of
/// silently substituting a default.
fn parse_dt_sql(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    parse_dt(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        )
    })
}
