#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GoBinary {
    path: PathBuf,
}

impl GoBinary {
    /// Returns None if RONDO_GO env not set OR binary not executable.
    pub fn discover() -> Option<Self> {
        let path = std::env::var("RONDO_GO").ok().map(PathBuf::from)?;
        if path.is_file() {
            Some(Self { path })
        } else {
            None
        }
    }

    pub fn list_json(&self, db: &Path) -> std::io::Result<String> {
        let out = Command::new(&self.path)
            .args(["list", "--json"])
            .env("RONDO_DB", db)
            .output()?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

pub fn seed_db(target: &Path) -> std::io::Result<()> {
    let seed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let seed = std::fs::read_to_string(&seed_path)?;
    let conn =
        rusqlite::Connection::open(target).map_err(|e| std::io::Error::other(e.to_string()))?;
    conn.execute_batch(&seed)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}
