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
fn delete_without_write_errors() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("delete")
        .arg("1");
    cmd.assert().failure();
}

#[test]
fn delete_with_write_succeeds() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("delete")
        .arg("3");
    cmd.assert().success();
}

#[test]
fn journal_list_runs() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("journal")
        .arg("list");
    cmd.assert().success();
}

#[test]
fn journal_add_requires_write() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("journal")
        .arg("add")
        .arg("hello world");
    cmd.assert().failure();
}

#[test]
fn journal_add_with_write_succeeds() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("journal")
        .arg("add")
        .arg("note body");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("journal entry"));
}

#[test]
fn focus_stats_runs() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("focus")
        .arg("stats");
    cmd.assert().success();
}

#[test]
fn focus_start_with_write() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("focus")
        .arg("start");
    cmd.assert().success();
}

#[test]
fn stats_json_parses() {
    let db = make_db();
    let home = isolated_home();
    let out = Command::cargo_bin("rondo-tui")
        .unwrap()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("stats")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.get("tasks").is_some());
    assert!(v.get("focus_streak").is_some());
    assert!(v.get("journal_notes").is_some());
}

#[test]
fn recur_preview_runs() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("recur")
        .arg("preview");
    cmd.assert().success();
}

#[test]
fn completion_emits_bash() {
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.arg("completion").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("rondo-tui"));
}

#[test]
fn completion_emits_zsh() {
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.arg("completion").arg("zsh");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("rondo-tui"));
}

#[test]
fn batch_processes_ndjson() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("batch")
        .write_stdin(
            "{\"op\":\"add\",\"title\":\"From batch\"}\n{\"op\":\"add\",\"title\":\"Another\"}\n",
        );
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("processed 2"));
}

#[test]
fn batch_json_summary() {
    let db = make_db();
    let home = isolated_home();
    let out = Command::cargo_bin("rondo-tui")
        .unwrap()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("--json")
        .arg("batch")
        .write_stdin("{\"op\":\"add\",\"title\":\"X\"}\n{\"op\":\"unknown\"}\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["processed"], 1);
    assert_eq!(v["errors"], 1);
}

#[test]
fn dep_add_requires_write() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("dep")
        .arg("add")
        .arg("1")
        .arg("2");
    cmd.assert().failure();
}

#[test]
fn dep_add_creates_dependency() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("dep")
        .arg("add")
        .arg("1")
        .arg("2");
    cmd.assert().success();
}

#[test]
fn dep_remove_idempotent() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("dep")
        .arg("remove")
        .arg("1")
        .arg("99");
    cmd.assert().success();
}

#[test]
fn tag_add_works() {
    let db = make_db();
    let home = isolated_home();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("tag")
        .arg("add")
        .arg("1")
        .arg("newtag");
    cmd.assert().success();
}

#[test]
fn tag_remove_works() {
    let db = make_db();
    let home = isolated_home();
    Command::cargo_bin("rondo-tui")
        .unwrap()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("tag")
        .arg("add")
        .arg("1")
        .arg("tmp")
        .assert()
        .success();
    Command::cargo_bin("rondo-tui")
        .unwrap()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--write")
        .arg("tag")
        .arg("remove")
        .arg("1")
        .arg("tmp")
        .assert()
        .success();
}
