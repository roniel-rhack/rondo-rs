//! First-run seed for a fresh `~/.rondo-rs/todo.db`.
//!
//! The seed schema lives in `fixtures/seed.sql` at workspace root and is
//! embedded at compile-time. `ensure_seeded(path)` is a no-op if the DB
//! file already exists; otherwise it creates the parent directory, opens
//! a fresh SQLite file, and applies the seed script.

use std::path::Path;

const SEED_SQL: &str = include_str!("../../../../fixtures/seed.sql");

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Creates `db_path` (and its parent dir) with the embedded seed schema +
/// sample rows when the file does not yet exist. Returns `true` if a fresh
/// DB was created, `false` if the file was already there.
pub fn ensure_seeded(db_path: &Path) -> Result<bool, SeedError> {
    if db_path.exists() {
        return Ok(false);
    }
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.execute_batch(SEED_SQL)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_seeded_creates_db_with_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nested").join("todo.db");
        let created = ensure_seeded(&db).unwrap();
        assert!(created);
        assert!(db.exists());
        let conn = rusqlite::Connection::open(&db).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert!(count > 0, "seed should populate tasks");
    }

    #[test]
    fn ensure_seeded_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("todo.db");
        assert!(ensure_seeded(&db).unwrap());
        // second call sees existing file and does nothing
        assert!(!ensure_seeded(&db).unwrap());
    }
}
