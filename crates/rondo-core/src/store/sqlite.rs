use crate::domain::{
    journal::{Entry, Note},
    task::{Priority, RecurFreq, Status, Subtask, Task, TaskNote, TimeLog},
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
