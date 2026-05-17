use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("random: {0}")]
    Random(String),
    #[error("another process holds the lock: PID {0}")]
    Conflict(u32),
}

impl From<getrandom::Error> for LockError {
    fn from(e: getrandom::Error) -> Self {
        LockError::Random(e.to_string())
    }
}

#[derive(Debug)]
pub struct LockGuard {
    path: PathBuf,
    nonce: String,
}

impl LockGuard {
    /// Acquire an exclusive cooperative lock at `path`. The lock file is
    /// created with `O_EXCL` semantics; if it already exists, returns
    /// `Conflict(pid)` with the PID stored inside (or 0 if unreadable).
    ///
    /// The lock file payload is `<pid>:<hex-nonce>`. The nonce is a
    /// fresh 16-byte random value written on acquire. On `Drop` we only
    /// remove the file if its on-disk nonce still matches the one we
    /// wrote — protecting against the case where the OS recycles our
    /// PID for another process that then wrote its own lock at the same
    /// path while we were running.
    pub fn acquire(path: PathBuf) -> Result<Self, LockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let nonce = random_nonce_hex()?;
        let mut f = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut s = String::new();
                let _ = File::open(&path).and_then(|mut f| f.read_to_string(&mut s));
                let pid = s
                    .trim()
                    .split(':')
                    .next()
                    .and_then(|p| p.parse::<u32>().ok())
                    .unwrap_or(0);
                return Err(LockError::Conflict(pid));
            }
            Err(e) => return Err(LockError::Io(e)),
        };
        write!(f, "{}:{}", std::process::id(), nonce)?;
        Ok(Self { path, nonce })
    }

    /// Default lock path: `~/.rondo-rs/rondo.lock`.
    pub fn default_path() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(".rondo-rs")
            .join("rondo.lock")
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Read the `<pid>:<nonce>` payload back from disk. Returns `None`
    /// if the file is missing or malformed.
    fn read_payload(&self) -> Option<(u32, String)> {
        let mut s = String::new();
        File::open(&self.path)
            .and_then(|mut f| f.read_to_string(&mut s))
            .ok()?;
        let mut parts = s.trim().splitn(2, ':');
        let pid = parts.next()?.parse::<u32>().ok()?;
        let nonce = parts.next()?.to_string();
        Some((pid, nonce))
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Only remove the file if its nonce still matches what we wrote.
        // If another process (e.g. one that inherited our recycled PID)
        // has since taken the lock with its own nonce, leave it alone.
        match self.read_payload() {
            Some((_, nonce)) if nonce == self.nonce => {
                let _ = std::fs::remove_file(&self.path);
            }
            _ => {}
        }
    }
}

fn random_nonce_hex() -> Result<String, LockError> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf)?;
    let mut out = String::with_capacity(buf.len() * 2);
    for b in buf {
        out.push_str(&format!("{b:02x}"));
    }
    Ok(out)
}
