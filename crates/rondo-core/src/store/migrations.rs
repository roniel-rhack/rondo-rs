use rusqlite::Connection;

pub const CURRENT_VERSION: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("schema version {0} newer than supported {1}")]
    FutureVersion(u32, u32),
}

pub fn user_version(conn: &Connection) -> rusqlite::Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .map(|v| v as u32)
}

fn set_user_version(conn: &Connection, v: u32) -> rusqlite::Result<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {}", v))
}

/// Apply all pending migrations. Idempotent: returns Ok if already current.
pub fn migrate(conn: &Connection) -> Result<u32, MigrationError> {
    let from = user_version(conn)?;
    if from > CURRENT_VERSION {
        return Err(MigrationError::FutureVersion(from, CURRENT_VERSION));
    }
    if from < 1 {
        migrate_to_v1(conn)?;
    }
    if from < 2 {
        migrate_to_v2(conn)?;
    }
    if from < 3 {
        migrate_to_v3(conn)?;
    }
    set_user_version(conn, CURRENT_VERSION)?;
    Ok(CURRENT_VERSION)
}

/// v0 → v1: ensure `metadata` column exists on tasks table.
fn migrate_to_v1(conn: &Connection) -> Result<(), MigrationError> {
    let tx = conn.unchecked_transaction()?;
    if !column_exists(&tx, "tasks", "metadata")? {
        tx.execute_batch("ALTER TABLE tasks ADD COLUMN metadata TEXT")?;
    }
    tx.commit()?;
    Ok(())
}

/// v1 → v2: create `focus_sessions` table for persistent pomodoro sessions.
fn migrate_to_v2(conn: &Connection) -> Result<(), MigrationError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS focus_sessions (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          task_id INTEGER,
          kind INTEGER NOT NULL,
          started_at TEXT NOT NULL,
          completed_at TEXT,
          duration_secs INTEGER NOT NULL,
          FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_focus_sessions_started_at ON focus_sessions(started_at);
        "#,
    )?;
    tx.commit()?;
    Ok(())
}

/// v2 → v3: create `plugin_kv` table for plugin-scoped key/value blobs.
fn migrate_to_v3(conn: &Connection) -> Result<(), MigrationError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS plugin_kv (
          plugin_id TEXT NOT NULL,
          key TEXT NOT NULL,
          value BLOB NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (plugin_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_kv_plugin ON plugin_kv(plugin_id);
        "#,
    )?;
    tx.commit()?;
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
