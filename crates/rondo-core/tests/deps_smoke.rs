use rondo_core::domain::task::NewTask;
use rondo_core::error::Error;
use rondo_core::store::sqlite::SqliteStore;

fn three_tasks() -> (tempfile::NamedTempFile, SqliteStore, [i64; 3]) {
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
    let store = SqliteStore::open_readwrite(f.path()).unwrap();
    let (a, _) = store.create_task(NewTask::quick("A")).unwrap();
    let (b, _) = store.create_task(NewTask::quick("B")).unwrap();
    let (c, _) = store.create_task(NewTask::quick("C")).unwrap();
    (f, store, [a, b, c])
}

#[test]
fn add_dependency_persists() {
    let (_f, store, [a, b, _]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    let t = store.task_by_id(a).unwrap();
    assert!(t.blocked_by_ids.contains(&b));
}

#[test]
fn add_dependency_idempotent() {
    let (_f, store, [a, b, _]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    store.add_dependency(a, b).unwrap();
    let t = store.task_by_id(a).unwrap();
    assert_eq!(t.blocked_by_ids.iter().filter(|x| **x == b).count(), 1);
}

#[test]
fn self_dependency_rejected() {
    let (_f, store, [a, _, _]) = three_tasks();
    let err = store.add_dependency(a, a).unwrap_err();
    assert!(matches!(err, Error::CycleDetected(_, _)));
}

#[test]
fn direct_cycle_rejected() {
    let (_f, store, [a, b, _]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    let err = store.add_dependency(b, a).unwrap_err();
    assert!(matches!(err, Error::CycleDetected(_, _)));
}

#[test]
fn transitive_cycle_rejected() {
    let (_f, store, [a, b, c]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    store.add_dependency(b, c).unwrap();
    let err = store.add_dependency(c, a).unwrap_err();
    assert!(matches!(err, Error::CycleDetected(_, _)));
}

#[test]
fn parallel_deps_allowed() {
    let (_f, store, [a, b, c]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    store.add_dependency(a, c).unwrap();
    let t = store.task_by_id(a).unwrap();
    assert_eq!(t.blocked_by_ids.len(), 2);
}

#[test]
fn remove_dependency_removes_edge() {
    let (_f, store, [a, b, _]) = three_tasks();
    store.add_dependency(a, b).unwrap();
    store.remove_dependency(a, b).unwrap();
    let t = store.task_by_id(a).unwrap();
    assert!(!t.blocked_by_ids.contains(&b));
}

#[test]
fn remove_dependency_idempotent() {
    let (_f, store, [a, b, _]) = three_tasks();
    store.remove_dependency(a, b).unwrap();
}
