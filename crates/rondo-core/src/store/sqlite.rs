use crate::domain::{
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

pub struct SqliteStore {
    conn: Mutex<Connection>,
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
            tx.execute(super::queries::UPDATE_TASK_RECUR_FREQ, params![r as i64, id])?;
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
        let before = self.task_by_id(task_id)?;
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let position: i64 = tx.query_row(
            super::queries::NEXT_SUBTASK_POSITION,
            params![task_id],
            |r| r.get(0),
        )?;
        tx.execute(super::queries::INSERT_SUBTASK, params![task_id, title, position])?;
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

    pub fn toggle_subtask(&self, id: i64) -> Result<(bool, UndoSnapshot)> {
        let task_id: i64;
        let new_completed: i64;
        {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn.transaction()?;
            let (tid, completed): (i64, i64) = tx.query_row(
                super::queries::SUBTASK_LOOKUP,
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
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
}

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
    let metadata_json: String = r.get(9)?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();
    let due_str: Option<String> = r.get(5)?;
    let created_str: String = r.get(6)?;
    Ok(Task {
        id: r.get(0)?,
        title: r.get(1)?,
        description: r.get(2)?,
        status: Status::from_db(r.get(3)?),
        priority: Priority::from_db(r.get(4)?),
        due_date: due_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        created_at: parse_dt(&created_str),
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
                    logged_at: parse_dt(&r.get::<_, String>(4)?),
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
                    created_at: parse_dt(&r.get::<_, String>(3)?),
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
        date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap_or_else(|_| {
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch date is valid")
        }),
        hidden: r.get::<_, i64>(2)? != 0,
        created_at: parse_dt(&r.get::<_, String>(3)?),
        updated_at: parse_dt(&r.get::<_, String>(4)?),
    })
}

fn row_to_entry(r: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?,
        note_id: r.get(1)?,
        body: r.get(2)?,
        created_at: parse_dt(&r.get::<_, String>(3)?),
    })
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return d.with_timezone(&Utc);
    }
    if let Ok(nd) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return DateTime::<Utc>::from_naive_utc_and_offset(nd, Utc);
    }
    Utc::now()
}
