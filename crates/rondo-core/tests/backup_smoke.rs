use rondo_core::store::backup::{rotate, snapshot};

#[test]
fn snapshot_creates_file_with_timestamp() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("source.sqlite");
    std::fs::write(&src, b"sqlite contents stub").unwrap();
    let backup_dir = tmp.path().join("backups");
    let out = snapshot(&src, &backup_dir).unwrap();
    assert!(out.exists());
    assert!(out
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("source.sqlite"));
    assert_eq!(std::fs::read(&out).unwrap(), b"sqlite contents stub");
}

#[test]
fn snapshot_missing_source_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let res = snapshot(&tmp.path().join("nonexistent"), &tmp.path().join("backups"));
    assert!(res.is_err());
}

#[test]
fn rotate_deletes_old_backups() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let old = dir.join("20200101T000000Z-source.sqlite");
    std::fs::write(&old, b"old").unwrap();
    let ancient = std::time::SystemTime::now() - std::time::Duration::from_secs(40 * 86_400);
    filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(ancient)).unwrap();
    rotate(dir, 30);
    assert!(!old.exists());
}

#[test]
fn rotate_keeps_recent_backups() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let recent = dir.join("recent.sqlite");
    std::fs::write(&recent, b"recent").unwrap();
    rotate(dir, 30);
    assert!(recent.exists());
}
