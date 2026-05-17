use rondo_core::domain::task::Task;
use rondo_core::export::ExporterRegistry;
use rondo_core::store::sqlite::SqliteStore;

fn tasks() -> Vec<Task> {
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
    SqliteStore::open_readonly(f.path())
        .unwrap()
        .list_tasks()
        .unwrap()
}

#[test]
fn registry_has_3_builtins() {
    let r = ExporterRegistry::with_builtins();
    let ids: Vec<&str> = r.list().iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&"md") && ids.contains(&"json") && ids.contains(&"ndjson"));
}

#[test]
fn markdown_exporter_outputs_heading() {
    let r = ExporterRegistry::with_builtins();
    let exp = r.get("md").unwrap();
    let out = exp.export(&tasks()).unwrap();
    assert!(out.starts_with("# Tasks"));
}

#[test]
fn json_exporter_round_trips() {
    let r = ExporterRegistry::with_builtins();
    let exp = r.get("json").unwrap();
    let out = exp.export(&tasks()).unwrap();
    let _: Vec<Task> = serde_json::from_str(&out).unwrap();
}

#[test]
fn ndjson_exporter_one_line_per_task() {
    let r = ExporterRegistry::with_builtins();
    let exp = r.get("ndjson").unwrap();
    let ts = tasks();
    let out = exp.export(&ts).unwrap();
    assert_eq!(out.lines().count(), ts.len());
    for line in out.lines() {
        let _: Task = serde_json::from_str(line).unwrap();
    }
}

#[test]
fn unknown_format_returns_none() {
    let r = ExporterRegistry::with_builtins();
    assert!(r.get("ical").is_none());
}

#[test]
fn mime_types_are_set() {
    let r = ExporterRegistry::with_builtins();
    assert_eq!(r.get("md").unwrap().mime(), "text/markdown");
    assert_eq!(r.get("json").unwrap().mime(), "application/json");
    assert_eq!(r.get("ndjson").unwrap().mime(), "application/x-ndjson");
}

#[test]
fn custom_exporter_can_register() {
    struct YamlExporter;
    impl rondo_core::export::Exporter for YamlExporter {
        fn format_id(&self) -> &str {
            "yaml"
        }
        fn mime(&self) -> &str {
            "application/yaml"
        }
        fn export(&self, _tasks: &[Task]) -> Result<String, rondo_core::export::ExportError> {
            Ok("tasks: []\n".to_string())
        }
    }
    let mut r = ExporterRegistry::with_builtins();
    r.register(Box::new(YamlExporter));
    assert!(r.get("yaml").is_some());
    assert_eq!(r.get("yaml").unwrap().export(&[]).unwrap(), "tasks: []\n");
}
