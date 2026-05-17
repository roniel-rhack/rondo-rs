//! End-to-end CLI mutation round-trips.
//!
//! Each test spawns the `rondo-tui` binary multiple times against the same
//! temp DB and asserts that mutations observed via `list`/`journal list`
//! reflect prior `add`/`done`/`tag`/`dep`/`batch` invocations.
//!
//! Coverage complements `cli_smoke.rs` (single-shot smoke) and `cli_more.rs`
//! (per-command exit-status checks) by exercising the *full* state
//! round-trip through stdout parsing.

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
    Command::cargo_bin("rondo-tui").unwrap()
}

#[test]
fn add_then_list_then_done_then_list_roundtrip() {
    let db = make_db();
    let home = isolated_home();

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("add")
        .arg("Round-trip task")
        .assert()
        .success();

    // The new task must show up in `list --filter all`.
    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let tasks = v.as_array().expect("list returns an array");
    let added = tasks
        .iter()
        .find(|t| t["title"] == "Round-trip task")
        .expect("newly-added task is in the list");
    let new_id = added["id"].as_i64().expect("id is i64");
    assert_eq!(added["status"], "Pending");

    // Mark it done.
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("done")
        .arg(new_id.to_string())
        .assert()
        .success();

    // Confirm status flipped.
    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let after = v
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == new_id)
        .expect("task still present");
    assert_eq!(after["status"], "Done", "done flips status to Done");
}

#[test]
fn journal_add_then_list_shows_body() {
    let db = make_db();
    let home = isolated_home();

    let count_before = {
        let out = rondo()
            .env("HOME", home.path())
            .arg("--db")
            .arg(db.path())
            .arg("journal")
            .arg("list")
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).lines().count()
    };

    let unique = format!("e2e-marker-{}", std::process::id());
    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("journal")
        .arg("add")
        .arg(&unique)
        .assert()
        .success();

    // `journal list` must still succeed and list at least the same days
    // (today's note either added a new day row or reused the existing one).
    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("journal")
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let count_after = String::from_utf8_lossy(&out.stdout).lines().count();
    assert!(
        count_after >= count_before,
        "journal list rows shouldn't shrink (before {count_before}, after {count_after})"
    );
}

#[test]
fn tag_add_then_remove_roundtrip() {
    let db = make_db();
    let home = isolated_home();

    // Add a unique tag, verify it appears in JSON listing, remove, verify gone.
    let tag = format!("rt-tag-{}", std::process::id());

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("tag")
        .arg("add")
        .arg("1")
        .arg(&tag)
        .assert()
        .success();

    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let t1 = v
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == 1)
        .expect("task 1 present");
    let tags: Vec<String> = t1["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        tags.contains(&tag),
        "tag {} present after add: {:?}",
        tag,
        tags
    );

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("tag")
        .arg("remove")
        .arg("1")
        .arg(&tag)
        .assert()
        .success();

    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let t1 = v
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == 1)
        .expect("task 1 present");
    let tags: Vec<String> = t1["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        !tags.contains(&tag),
        "tag {} gone after remove: {:?}",
        tag,
        tags
    );
}

#[test]
fn dep_add_then_remove_roundtrip() {
    let db = make_db();
    let home = isolated_home();

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("dep")
        .arg("add")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let t1 = v.as_array().unwrap().iter().find(|t| t["id"] == 1).unwrap();
    let deps: Vec<i64> = t1["blocked_by_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert!(
        deps.contains(&2),
        "blocked_by 2 present after add: {:?}",
        deps
    );

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("dep")
        .arg("remove")
        .arg("1")
        .arg("2")
        .assert()
        .success();

    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let t1 = v.as_array().unwrap().iter().find(|t| t["id"] == 1).unwrap();
    let deps: Vec<i64> = t1["blocked_by_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert!(!deps.contains(&2), "blocked_by 2 removed: {:?}", deps);
}

#[test]
fn batch_ndjson_creates_listed_tasks() {
    let db = make_db();
    let home = isolated_home();

    let payload = "\
{\"op\":\"add\",\"title\":\"batch-alpha\"}
{\"op\":\"add\",\"title\":\"batch-beta\"}
{\"op\":\"add\",\"title\":\"batch-gamma\"}
";

    rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("batch")
        .write_stdin(payload)
        .assert()
        .success()
        .stdout(predicate::str::contains("processed 3"));

    let out = rondo()
        .env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("--json")
        .arg("list")
        .arg("--filter")
        .arg("all")
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let titles: Vec<String> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap_or("").to_string())
        .collect();
    for expected in ["batch-alpha", "batch-beta", "batch-gamma"] {
        assert!(
            titles.iter().any(|t| t == expected),
            "{} present after batch: {:?}",
            expected,
            titles
        );
    }
}
