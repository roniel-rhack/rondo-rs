use rondo_core::domain::task::Task;
use rondo_core::export::{to_json, to_markdown, to_ndjson};
use rondo_core::store::sqlite::SqliteStore;

fn fixture_tasks() -> Vec<Task> {
    let f = tempfile::NamedTempFile::new().unwrap();
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
    drop(conn);
    let store = SqliteStore::open_readonly(f.path()).unwrap();
    store.list_tasks().unwrap()
}

#[test]
fn markdown_contains_titles() {
    let tasks = fixture_tasks();
    let md = to_markdown(&tasks);
    assert!(md.starts_with("# Tasks"));
    for t in &tasks {
        assert!(md.contains(&t.title), "title {} missing", t.title);
    }
}

#[test]
fn markdown_marks_done_subtasks() {
    let tasks = fixture_tasks();
    let md = to_markdown(&tasks);
    assert!(md.contains("[x]"));
    assert!(md.contains("[ ]"));
}

#[test]
fn markdown_uses_status_icons() {
    let tasks = fixture_tasks();
    let md = to_markdown(&tasks);
    let any_icon = md.contains('○') || md.contains('◐') || md.contains('✓');
    assert!(any_icon, "expected at least one status icon");
}

#[test]
fn json_roundtrip_through_serde() {
    let tasks = fixture_tasks();
    let json = to_json(&tasks).unwrap();
    let parsed: Vec<Task> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), tasks.len());
    assert_eq!(parsed[0].title, tasks[0].title);
}

#[test]
fn ndjson_one_line_per_task() {
    let tasks = fixture_tasks();
    let mut buf: Vec<u8> = Vec::new();
    to_ndjson(&tasks, &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    let line_count = s.lines().count();
    assert_eq!(line_count, tasks.len());
    for line in s.lines() {
        let _: Task = serde_json::from_str(line).unwrap();
    }
}

#[test]
fn empty_input_is_safe() {
    let md = to_markdown(&[]);
    assert!(md.starts_with("# Tasks"));
    let json = to_json(&[]).unwrap();
    assert_eq!(json, "[]");
    let mut buf: Vec<u8> = Vec::new();
    to_ndjson(&[], &mut buf).unwrap();
    assert!(buf.is_empty());
}
