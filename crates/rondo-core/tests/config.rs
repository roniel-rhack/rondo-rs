use rondo_core::config::Config;

#[test]
fn default_values() {
    let c = Config::default();
    assert_eq!(c.ui.theme, "dark");
    assert!(c.ui.sidebar);
    assert!(c.ui.animations);
    assert_eq!(c.pomodoro.work_min, 25);
    assert_eq!(c.pomodoro.short_break_min, 5);
    assert_eq!(c.pomodoro.long_break_min, 15);
}

#[test]
fn load_nonexistent_returns_default() {
    let c = Config::load_or_default(std::path::Path::new(
        "/tmp/definitely-not-a-file-3qweqwe.toml",
    ));
    assert_eq!(c.ui.theme, "dark");
}

#[test]
fn parses_full_toml() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        r#"
[ui]
theme = "light"
sidebar = false
animations = false

[pomodoro]
work_min = 50
short_break_min = 10
long_break_min = 30

[plugins]
enabled = ["builtin.pomodoro", "extra.fizz"]

[plugins.permissions]
"quote-of-the-day" = ["overlay_view", "tick_handler"]
"#,
    )
    .unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.ui.theme, "light");
    assert!(!c.ui.sidebar);
    assert_eq!(c.pomodoro.work_min, 50);
    assert_eq!(c.plugins.enabled.len(), 2);
    assert_eq!(
        c.plugins.permissions.get("quote-of-the-day").unwrap().len(),
        2
    );
}

#[test]
fn invalid_toml_returns_default_no_panic() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), "this is not [valid toml = =").unwrap();
    let c = Config::load_or_default(f.path());
    assert_eq!(c.ui.theme, "dark");
}

#[test]
fn subset_fills_defaults() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        r#"[ui]
theme = "high-contrast"
"#,
    )
    .unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.ui.theme, "high-contrast");
    assert!(c.ui.sidebar);
    assert_eq!(c.pomodoro.work_min, 25);
}

#[test]
fn env_override() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), "[ui]\ntheme = \"light\"\n").unwrap();
    std::env::set_var("RONDO_CONFIG", f.path());
    let c = Config::from_env_or_default();
    std::env::remove_var("RONDO_CONFIG");
    assert_eq!(c.ui.theme, "light");
}
