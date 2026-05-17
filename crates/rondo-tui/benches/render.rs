//! Baseline criterion benchmarks for the TUI render path and helpers.
//!
//! `compute_counts` lives behind module-private visibility so we measure the
//! full `root::draw` cost over a `TestBackend` (which exercises the sidebar
//! count computation alongside everything else).
//!
//! Run:
//!
//! ```bash
//! cargo bench -p rondo-tui
//! cargo bench -p rondo-tui -- --quick
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ratatui::{backend::TestBackend, Terminal};
use rondo_core::domain::task::NewTask;
use rondo_core::store::sqlite::SqliteStore;
use rondo_tui::{app::AppState, components};
use std::sync::Arc;
use tempfile::NamedTempFile;

fn store_with(n: usize) -> Arc<SqliteStore> {
    let f = NamedTempFile::new().unwrap();
    let path = f.path().to_path_buf();

    // Apply fixture schema first so migrations don't trip on a missing
    // `tasks` table (v1 migrate is an ALTER, not CREATE).
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed).unwrap())
        .unwrap();
    conn.execute_batch(
        "DELETE FROM tags; DELETE FROM task_dependencies; DELETE FROM subtasks; \
         DELETE FROM time_logs; DELETE FROM task_notes; DELETE FROM tasks;",
    )
    .unwrap();
    drop(conn);

    let store = SqliteStore::open_readwrite(&path).unwrap();
    for i in 0..n {
        store
            .create_task(NewTask::quick(format!("render bench {i}")))
            .unwrap();
    }
    drop(store);
    // Keep the temp file alive for the duration of the bench by leaking it.
    std::mem::forget(f);
    Arc::new(SqliteStore::open_readwrite(&path).unwrap())
}

fn bench_full_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("root_draw");
    for &size in &[10usize, 100, 1_000] {
        let store = store_with(size);
        let mut app = AppState::new(store).unwrap();
        let backend = TestBackend::new(140, 40);
        let mut term = Terminal::new(backend).unwrap();
        // Warm up: one draw before measuring stabilizes sqlite caches.
        term.draw(|f| components::root::draw(&mut app, f)).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                term.draw(|f| components::root::draw(&mut app, f)).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_visible_indices(c: &mut Criterion) {
    let store = store_with(1_000);
    let app = AppState::new(store).unwrap();
    c.bench_function("visible_task_indices_1000", |b| {
        b.iter(|| {
            let v = app.visible_task_indices();
            criterion::black_box(v);
        });
    });
}

criterion_group!(benches, bench_full_render, bench_visible_indices);
criterion_main!(benches);
