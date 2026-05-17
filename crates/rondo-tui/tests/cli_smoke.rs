use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn make_db() -> NamedTempFile {
    let f = NamedTempFile::new().unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(f.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
        .unwrap();
    f
}

fn isolated_home() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn list_default_format() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("list")
        .arg("--filter")
        .arg("all");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ID"));
}

#[test]
fn list_json() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all");
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    let _: serde_json::Value = serde_json::from_str(&s).expect("valid json");
}

#[test]
fn export_markdown() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("export")
        .arg("--format")
        .arg("md");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Tasks"));
}

#[test]
fn export_ndjson_one_line_per_task() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("export")
        .arg("--format")
        .arg("ndjson");
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = s.lines().collect();
    assert!(!lines.is_empty());
    for line in lines {
        let _: serde_json::Value = serde_json::from_str(line).expect("each line valid json");
    }
}

#[test]
fn add_with_read_only_flag_errors() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--read-only")
        .arg("add")
        .arg("new task");
    cmd.assert().failure();
}

#[test]
fn add_with_write_creates_task() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("add")
        .arg("CLI added");
    cmd.assert().success();
    let mut list = Command::cargo_bin("rondo-rs").unwrap();
    list.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("list")
        .arg("--filter")
        .arg("all");
    list.assert()
        .success()
        .stdout(predicate::str::contains("CLI added"));
}

#[test]
fn done_marks_task() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("done")
        .arg("3");
    cmd.assert().success();
}

#[test]
fn unknown_filter_errors() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("list")
        .arg("--filter")
        .arg("nonsense");
    cmd.assert().failure();
}

#[test]
fn unknown_export_format_errors() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-rs").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("export")
        .arg("--format")
        .arg("xml");
    cmd.assert().failure();
}
