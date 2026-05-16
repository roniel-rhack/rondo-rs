use assert_cmd::Command;
use predicates::prelude::*;

fn plugin_dir_in(home: &std::path::Path) -> std::path::PathBuf {
    home.join(".rondo-rs").join("plugins")
}

#[test]
fn plugins_list_empty_runs() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", dir.path()).arg("plugins").arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no plugins"));
}

#[test]
fn plugins_list_json_runs() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", dir.path())
        .arg("--json")
        .arg("plugins")
        .arg("list");
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid json");
    assert!(v.is_array());
}

#[test]
fn plugins_remove_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", dir.path())
        .arg("plugins")
        .arg("remove")
        .arg("nonexistent");
    cmd.assert().success();
}

#[test]
fn plugins_install_then_list_and_info_and_remove() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    std::fs::write(
        src.path().join("plugin.toml"),
        r#"
id = "hello"
version = "0.2.0"
api = "0.1"
capabilities = ["OverlayView"]
"#,
    )
    .unwrap();

    // install
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("plugins")
        .arg("install")
        .arg(src.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("installed `hello`"));
    assert!(plugin_dir_in(home.path()).join("hello").join("plugin.toml").exists());

    // list
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path()).arg("plugins").arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello"));

    // info
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("plugins")
        .arg("info")
        .arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("id:"))
        .stdout(predicate::str::contains("hello"));

    // duplicate install errors
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("plugins")
        .arg("install")
        .arg(src.path());
    cmd.assert().failure();

    // remove
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .arg("plugins")
        .arg("remove")
        .arg("hello");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("removed `hello`"));
    assert!(!plugin_dir_in(home.path()).join("hello").exists());
}

#[test]
fn plugins_info_missing_id_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", dir.path())
        .arg("plugins")
        .arg("info")
        .arg("ghost");
    cmd.assert().failure();
}

#[test]
fn plugins_install_rejects_bad_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", dir.path())
        .arg("plugins")
        .arg("install")
        .arg("/nonexistent/path/xyz123");
    cmd.assert().failure();
}
