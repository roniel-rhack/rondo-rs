use rondo_core::telemetry::{init_logging, rotate_old_logs};

#[test]
fn logging_writes_to_file() {
    let dir = tempfile::tempdir().unwrap();
    let guard = init_logging(dir.path().to_path_buf()).unwrap();
    tracing::info!("smoke test line");
    drop(guard); // flush
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().flatten().collect();
    assert!(!entries.is_empty());
    let found = entries.iter().any(|e| {
        std::fs::read_to_string(e.path())
            .unwrap_or_default()
            .contains("smoke test line")
    });
    assert!(found, "log line not found in any file");
}

#[test]
fn rotation_deletes_old_files() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("rondo-rust-old.log");
    std::fs::write(&old, "old").unwrap();
    let ten_days_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(10 * 86_400);
    filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(ten_days_ago)).unwrap();
    rotate_old_logs(dir.path(), 7);
    assert!(!old.exists());
}
