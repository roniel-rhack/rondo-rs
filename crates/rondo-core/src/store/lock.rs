use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("another process holds the lock: PID {0}")]
    Conflict(u32),
}

#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
}

impl LockGuard {
    /// Acquire an exclusive cooperative lock at `path`. The lock file is
    /// created with `O_EXCL` semantics; if it already exists, returns
    /// `Conflict(pid)` with the PID stored inside (or 0 if unreadable).
    /// The file is removed on `Drop`.
    pub fn acquire(path: PathBuf) -> Result<Self, LockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut s = String::new();
                let _ = File::open(&path).and_then(|mut f| f.read_to_string(&mut s));
                let pid = s.trim().parse::<u32>().unwrap_or(0);
                return Err(LockError::Conflict(pid));
            }
            Err(e) => return Err(LockError::Io(e)),
        };
        write!(f, "{}", std::process::id())?;
        Ok(Self { path })
    }

    /// Default lock path: `~/.todo-app/.rondo-rust.lock`.
    pub fn default_path() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".todo-app")
            .join(".rondo-rust.lock")
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
