use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    Work,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub task_id: Option<i64>,
    pub kind: SessionKind,
    pub cycle_pos: u8,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
}
