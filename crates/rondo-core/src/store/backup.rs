use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("source db missing: {0}")]
    Missing(PathBuf),
}

/// Default backup root: `~/.todo-app/backups/rust/`.
pub fn default_backup_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".todo-app")
        .join("backups")
        .join("rust")
}

/// Copies `db_path` into `backup_dir` with ISO timestamp.
/// Returns path to created snapshot. Creates dir if missing.
pub fn snapshot(db_path: &Path, backup_dir: &Path) -> Result<PathBuf, BackupError> {
    if !db_path.exists() {
        return Err(BackupError::Missing(db_path.to_path_buf()));
    }
    std::fs::create_dir_all(backup_dir)?;
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let name = db_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("db.sqlite");
    let dest = backup_dir.join(format!("{}-{}", ts, name));
    std::fs::copy(db_path, &dest)?;
    Ok(dest)
}

/// Deletes snapshots older than `keep_days`. Best-effort.
pub fn rotate(backup_dir: &Path, keep_days: u64) {
    let Ok(entries) = std::fs::read_dir(backup_dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(keep_days * 86_400));
    let Some(cutoff) = cutoff else { return };
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime < cutoff {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}
