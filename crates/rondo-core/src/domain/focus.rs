use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    Work,
    ShortBreak,
    LongBreak,
}

impl SessionKind {
    pub fn from_db(v: i64) -> Self {
        match v {
            1 => SessionKind::ShortBreak,
            2 => SessionKind::LongBreak,
            _ => SessionKind::Work,
        }
    }
}

/// Focus session record. Represents both an in-memory active session
/// and a row read back from `focus_sessions`. `id` is `None` until the
/// row is persisted; `completed_at` is `None` while running or abandoned.
/// `cycle_pos` is in-memory bookkeeping for the pomodoro UI — not stored
/// in the DB; queries default it to 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub task_id: Option<i64>,
    pub kind: SessionKind,
    pub cycle_pos: u8,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_secs: u64,
}
