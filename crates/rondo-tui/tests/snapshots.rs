use insta::assert_snapshot;
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

fn snapshot(
    _name: &str,
    width: u16,
    height: u16,
    mutate: impl FnOnce(&mut AppState),
) -> String {
    let mut app = AppState::new(fixture_store()).unwrap();
    mutate(&mut app);
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| components::root::draw(&mut app, f)).unwrap();
    let raw = term.backend().to_string();
    // Redact wall-clock timestamps so snapshots are stable across runs.
    // Order matters: longer pattern first (HH:MM:SS), then plain HH:MM.
    let re_hms = regex::Regex::new(r"\d{2}:\d{2}:\d{2}").unwrap();
    let stage1 = re_hms.replace_all(&raw, "HH:MM:SS").to_string();
    let re_hm = regex::Regex::new(r"\b\d{2}:\d{2}\b").unwrap();
    re_hm.replace_all(&stage1, "HH:MM").to_string()
}

#[test]
fn tasks_default() {
    let s = snapshot("tasks_default", 120, 32, |_| {});
    assert_snapshot!(s);
}

#[test]
fn tasks_selected_second() {
    let s = snapshot("tasks_selected_second", 120, 32, |a| a.selected_task = 1);
    assert_snapshot!(s);
}

#[test]
fn tasks_blocked() {
    let s = snapshot("tasks_blocked", 120, 32, |a| a.selected_task = 3);
    assert_snapshot!(s);
}

#[test]
fn journal_view() {
    let s = snapshot("journal_view", 120, 32, |a| a.page = Page::Journal);
    assert_snapshot!(s);
}

#[test]
fn pomodoro_overlay() {
    let s = snapshot("pomodoro_overlay", 120, 32, |a| {
        a.pomodoro_open = true;
        a.pomodoro_started = Some(std::time::Instant::now());
    });
    assert_snapshot!(s);
}

#[test]
fn command_palette() {
    let s = snapshot("command_palette", 120, 32, |a| {
        a.command_palette_open = true;
        a.command_buf = "p".to_string();
    });
    assert_snapshot!(s);
}

#[test]
fn narrow_terminal() {
    let s = snapshot("narrow_terminal", 80, 24, |_| {});
    assert_snapshot!(s);
}

#[test]
fn wide_terminal() {
    let s = snapshot("wide_terminal", 160, 40, |_| {});
    assert_snapshot!(s);
}

#[test]
fn full_dashboard_140x42() {
    let s = snapshot("full_dashboard_140x42", 140, 42, |_| {});
    assert_snapshot!(s);
}

#[test]
fn help_overlay() {
    let s = snapshot("help_overlay", 120, 32, |a| a.help_open = true);
    assert_snapshot!(s);
}

#[test]
fn search_overlay() {
    let s = snapshot("search_overlay", 120, 32, |a| {
        a.search_open = true;
        a.search_buf = "deploy".to_string();
    });
    assert_snapshot!(s);
}

#[test]
fn empty_tasks() {
    let s = snapshot("empty_tasks", 120, 32, |a| {
        a.tasks.clear();
        a.task_list_state.select(None);
    });
    assert_snapshot!(s);
}

#[test]
fn visual_mode_multi_select() {
    let s = snapshot("visual_mode_multi_select", 120, 32, |a| {
        a.mode = rondo_tui::focus::Mode::Visual;
        a.selection.insert(1);
        a.selection.insert(2);
        a.selected_task = 1;
        a.task_list_state.select(Some(1));
    });
    assert_snapshot!(s);
}

#[test]
fn quick_actions_overlay() {
    let s = snapshot("quick_actions_overlay", 140, 32, |a| {
        a.quick_actions_open = true;
    });
    assert_snapshot!(s);
}

#[test]
fn quick_add_overlay() {
    let s = snapshot("quick_add_overlay", 120, 32, |a| {
        a.quick_add_open = true;
        a.quick_add_buf = "ship the demo #work !p3 due:tmrw".to_string();
        a.mode = rondo_tui::focus::Mode::Insert;
    });
    assert_snapshot!(s);
}

#[test]
fn detail_focused_subtasks_section() {
    let s = snapshot("detail_focused_subtasks_section", 120, 32, |a| {
        a.focus.pane = rondo_tui::focus::Pane::Detail;
        a.focus.section = rondo_tui::focus::DetailSection::Subtasks;
        a.focus.section_item = 1;
        a.selected_task = 2; // Review API spec has 5 subtasks
        a.task_list_state.select(Some(2));
    });
    assert_snapshot!(s);
}

#[test]
fn empty_journal() {
    let s = snapshot("empty_journal", 120, 32, |a| {
        a.page = Page::Journal;
        a.journal_notes.clear();
        a.journal_entries.clear();
        a.journal_list_state.select(None);
    });
    assert_snapshot!(s);
}
