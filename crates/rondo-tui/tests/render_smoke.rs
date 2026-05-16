use ratatui::{backend::TestBackend, Terminal};
use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::{action::Page, app::AppState, components};
use std::sync::Arc;

fn fixture_store() -> Arc<SqliteStore> {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    let seed = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(&seed).unwrap();
    drop(conn);
    Arc::new(SqliteStore::open_readonly(tmp.path()).unwrap())
}

#[test]
fn task_page_renders_without_panic() {
    let app = AppState::new(fixture_store()).unwrap();
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&app, f)).unwrap();
    let buf = term.backend().to_string();
    assert!(buf.contains("RonDO"));
    assert!(buf.contains("Tasks"));
    assert!(buf.contains("Review API spec"));
}

#[test]
fn journal_page_renders_without_panic() {
    let mut app = AppState::new(fixture_store()).unwrap();
    app.page = Page::Journal;
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&app, f)).unwrap();
    let buf = term.backend().to_string();
    assert!(buf.contains("Journal"));
    assert!(buf.contains("Today"));
}

#[test]
fn pomodoro_overlay_renders() {
    let mut app = AppState::new(fixture_store()).unwrap();
    app.pomodoro_open = true;
    app.pomodoro_started = Some(std::time::Instant::now());
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&app, f)).unwrap();
    let buf = term.backend().to_string();
    assert!(buf.contains("Focus Session"));
}

#[test]
fn command_palette_overlay_renders() {
    let mut app = AppState::new(fixture_store()).unwrap();
    app.command_palette_open = true;
    app.command_buf = "ta".to_string();
    let backend = TestBackend::new(100, 30);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&app, f)).unwrap();
    let buf = term.backend().to_string();
    assert!(buf.contains("command"));
    assert!(buf.contains("tasks"));
}
