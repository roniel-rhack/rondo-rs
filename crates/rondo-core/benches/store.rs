//! Baseline criterion benchmarks for `SqliteStore`.
//!
//! Measures `list_tasks` cost at three corpus sizes (100/1k/10k) plus a
//! single-row `task_by_id` lookup baseline.
//!
//! Run:
//!
//! ```bash
//! cargo bench -p rondo-core
//! cargo bench -p rondo-core -- --quick   # smoke
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rondo_core::domain::task::NewTask;
use rondo_core::store::sqlite::SqliteStore;
use tempfile::NamedTempFile;

fn seeded_store(n: usize) -> (NamedTempFile, SqliteStore) {
    let f = NamedTempFile::new().unwrap();
    // The fixtures `seed.sql` defines the base `tasks` schema; migrations
    // expect this table to exist before they run.
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
    // Wipe seed rows so the corpus size is exactly `n`.
    conn.execute_batch(
        "DELETE FROM tags; DELETE FROM task_dependencies; DELETE FROM subtasks; \
         DELETE FROM time_logs; DELETE FROM task_notes; DELETE FROM tasks;",
    )
    .unwrap();
    drop(conn);
    let store = SqliteStore::open_readwrite(f.path()).unwrap();
    for i in 0..n {
        store
            .create_task(NewTask::quick(format!("bench task {i}")))
            .unwrap();
    }
    (f, store)
}

fn bench_list_tasks(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_tasks");
    for &size in &[100usize, 1_000, 10_000] {
        let (_f, store) = seeded_store(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let v = store.list_tasks().unwrap();
                criterion::black_box(v);
            });
        });
    }
    group.finish();
}

fn bench_task_by_id(c: &mut Criterion) {
    let (_f, store) = seeded_store(1_000);
    c.bench_function("task_by_id_mid", |b| {
        b.iter(|| {
            let t = store.task_by_id(500).unwrap();
            criterion::black_box(t);
        });
    });
}

criterion_group!(benches, bench_list_tasks, bench_task_by_id);
criterion_main!(benches);
