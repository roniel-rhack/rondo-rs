//! End-to-end coverage for the `lang` CLI subcommand tree.
//!
//! Each test pins `HOME` (and `RONDO_CONFIG`) to a `TempDir` so the runs are
//! hermetic — no risk of trampling the developer's real `~/.rondo-rs/lang/`
//! directory or config file.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn isolated() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Build a `rondo-tui` invocation with `HOME` and `RONDO_CONFIG` pointed at
/// the isolated tempdir so neither the lang directory nor the saved config
/// touches the developer's machine.
fn bin(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("rondo-tui").unwrap();
    cmd.env("HOME", home.path())
        .env("RONDO_CONFIG", home.path().join("config.toml"));
    cmd
}

#[test]
fn scaffold_writes_translator_ready_file() {
    let home = isolated();
    let out = home.path().join("es.toml");
    bin(&home)
        .args(["lang", "scaffold", "es", "--name", "Español", "--out"])
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("scaffolded es pack"));
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("# TODO: translate"));
    assert!(body.contains("code = \"es\""));
    assert!(body.contains("name = \"Español\""));
    assert!(body.contains("[strings]"));
}

#[test]
fn install_then_list_then_remove_round_trip() {
    let home = isolated();
    let pack = home.path().join("es.toml");
    bin(&home)
        .args(["lang", "scaffold", "es", "--name", "Español", "--out"])
        .arg(&pack)
        .assert()
        .success();

    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .assert()
        .success()
        .stdout(predicate::str::contains("installed es pack"));

    let list_out = bin(&home)
        .args(["--json", "lang", "list"])
        .output()
        .unwrap();
    assert!(list_out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&list_out.stdout).expect("json");
    let arr = v.as_array().unwrap();
    assert!(arr.iter().any(|e| e["code"] == "en"));
    assert!(arr.iter().any(|e| e["code"] == "es"));

    bin(&home)
        .args(["lang", "remove", "es"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed es pack"));
}

#[test]
fn remove_refuses_builtin_english() {
    let home = isolated();
    bin(&home)
        .args(["lang", "remove", "en"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to remove built-in"));
}

#[test]
fn install_refuses_overwrite_without_force() {
    let home = isolated();
    let pack = home.path().join("es.toml");
    bin(&home)
        .args(["lang", "scaffold", "es", "--name", "Español", "--out"])
        .arg(&pack)
        .assert()
        .success();
    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .assert()
        .success();
    // Re-install without --force must fail.
    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already installed"));
    // --force succeeds.
    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .arg("--force")
        .assert()
        .success();
}

#[test]
fn install_refuses_pack_with_en_code() {
    let home = isolated();
    let pack = home.path().join("bogus.toml");
    std::fs::write(
        &pack,
        "[meta]\ncode = \"en\"\nname = \"BogusEn\"\n[strings]\n",
    )
    .unwrap();
    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to install"));
}

#[test]
fn current_defaults_to_english() {
    let home = isolated();
    let out = bin(&home)
        .args(["--json", "lang", "current"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["code"], "en");
    assert_eq!(v["source"], "built-in");
}

#[test]
fn install_rejects_invalid_code_in_meta() {
    let home = isolated();
    let pack = home.path().join("bad.toml");
    std::fs::write(
        &pack,
        "[meta]\ncode = \"../etc\"\nname = \"x\"\n[strings]\n",
    )
    .unwrap();
    bin(&home)
        .args(["lang", "install"])
        .arg(&pack)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid code"));
}
