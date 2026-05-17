//! CLI `due` subcommand: set, clear, and reject invalid inputs.

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

fn rondo() -> Command {
    Command::cargo_bin("rondo-rs").unwrap()
}

#[test]
fn due_sets_iso_date() {
    let db = make_db();
    let home = isolated_home();
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("due")
        .arg("1")
        .arg("2026-12-31")
        .assert()
        .success()
        .stdout(predicate::str::contains("2026-12-31"));
}

#[test]
fn due_accepts_alias() {
    let db = make_db();
    let home = isolated_home();
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("due")
        .arg("1")
        .arg("today")
        .assert()
        .success();
}

#[test]
fn due_clear_removes_date() {
    let db = make_db();
    let home = isolated_home();
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("due")
        .arg("1")
        .arg("--clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("cleared"));
}

#[test]
fn due_rejects_invalid_date() {
    let db = make_db();
    let home = isolated_home();
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("due")
        .arg("1")
        .arg("not-a-date")
        .assert()
        .failure();
}

#[test]
fn due_requires_write() {
    let db = make_db();
    let home = isolated_home();
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--read-only")
        .arg("due")
        .arg("1")
        .arg("today")
        .assert()
        .failure();
}
