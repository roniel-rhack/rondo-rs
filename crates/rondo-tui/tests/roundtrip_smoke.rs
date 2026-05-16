mod roundtrip;

use roundtrip::harness::*;

#[test]
fn harness_seed_smoke() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    seed_db(tmp.path()).unwrap();
    let store = rondo_core::store::sqlite::SqliteStore::open_readonly(tmp.path()).unwrap();
    let tasks = store.list_tasks().unwrap();
    assert!(!tasks.is_empty(), "fixture seeded zero tasks");
}

#[test]
#[ignore = "requires RONDO_GO env + M1.3 mutations API"]
fn rust_creates_go_reads() {
    let _go = GoBinary::discover().expect("set RONDO_GO");
    unimplemented!("M1.3: rondo-core needs create_task before this can run");
}

#[test]
#[ignore = "requires RONDO_GO env"]
fn go_writes_rust_reads() {
    let _go = GoBinary::discover().expect("set RONDO_GO");
    unimplemented!("invoke go binary's add command then read with SqliteStore::open_readonly");
}

#[test]
#[ignore = "requires RONDO_GO env + M4 recurrence engine"]
fn recurrence_no_duplicates() {
    let _go = GoBinary::discover().expect("set RONDO_GO");
    unimplemented!("M4: spawn_recurrent_instances + verify Go doesn't double-spawn");
}

#[test]
#[ignore = "requires RONDO_GO env + M1.1 backup support"]
fn backup_files_ignored_by_go() {
    let _go = GoBinary::discover().expect("set RONDO_GO");
    unimplemented!(
        "M1.1: create a .sqlite in ~/.todo-app/backups/rust/ and verify go list does not fail"
    );
}
