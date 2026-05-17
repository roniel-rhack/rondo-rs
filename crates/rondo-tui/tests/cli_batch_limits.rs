//! D3: `rondo-tui batch` enforces hard caps on the NDJSON stream
//! shape (per-line byte length and total line count) so a runaway
//! producer can't burn arbitrary CPU/RAM. These tests pipe synthetic
//! input via stdin and assert the reported error / processed counts.

use assert_cmd::Command;
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
fn batch_rejects_oversize_line() {
    let db = make_db();
    let home = isolated_home();
    // 70 KiB single line (no newline) — well over the 64 KiB cap.
    let huge_title = "a".repeat(70 * 1024);
    let stdin_payload = format!(r#"{{"op":"add","title":"{huge_title}"}}"#);

    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("batch")
        .write_stdin(stdin_payload);
    let out = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(
        stderr.contains("exceeds max length"),
        "expected oversize-line error in stderr, got: {stderr}",
    );
}

#[test]
fn batch_aborts_after_max_lines() {
    let db = make_db();
    let home = isolated_home();
    // 10_001 valid add lines — one over the cap. The 10_001st line
    // (and anything after) must be rejected with the abort message.
    let mut payload = String::new();
    for i in 0..10_001 {
        payload.push_str(&format!(r#"{{"op":"add","title":"t{i}"}}"#));
        payload.push('\n');
    }

    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("batch")
        .write_stdin(payload);
    let out = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).into_owned();
    assert!(
        stderr.contains("max 10000 lines per batch exceeded"),
        "expected batch-limit error in stderr, got: {stderr}",
    );
}

#[test]
fn batch_accepts_normal_input() {
    let db = make_db();
    let home = isolated_home();
    let payload = "{\"op\":\"add\",\"title\":\"hello\"}\n";
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("--db")
        .arg(db.path())
        .arg("batch")
        .write_stdin(payload);
    cmd.assert().success();
}
