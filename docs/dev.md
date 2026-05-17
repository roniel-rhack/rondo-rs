# Development guide

## Conventions

- **No `unsafe`** anywhere in the workspace.
- Errors flow through `color_eyre::eyre::Result` at the binary boundary and
  `rondo_core::error::Result` inside the core crate.
- Components are free functions `draw(app, f, rect)`. They read from
  `AppState` and never query the store directly.
- Theme tokens, not hex. `Color::Rgb(...)` lives only in `theme.rs`.
- No `Modifier::REVERSED` for selection: cursor row marks via accent fg +
  bold + underlined + a `▌` gutter.
- Snapshot tests via `insta` + `TestBackend` cover every overlay and layout.
  Wall-clock timestamps (`HH:MM` and `HH:MM:SS`) are regex-redacted globally
  so they don't churn the snapshots.
- Effects (animations) spawn through `AppState::fx.spawn(...)` with cached
  rects captured during `root::draw`.
- Plugin DSL types must round-trip through `serde_json` because the same
  types travel the in-process Plugin trait AND the future extism boundary.

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Snapshot maintenance:

```bash
INSTA_UPDATE=always cargo test -p rondo-tui --test snapshots
cargo insta review
```

Run against a custom DB without touching the default location:

```bash
cargo run --release -p rondo-tui -- --db /tmp/scratch.db
```

CLI smoke tests use `assert_cmd` and live in:

```
crates/rondo-tui/tests/cli_smoke.rs       # add / list / done / export
crates/rondo-tui/tests/cli_more.rs        # delete / journal / focus / stats / batch / recur / dep / tag / completion
crates/rondo-tui/tests/cli_plugins.rs     # plugins list / info / install / remove
```

Roundtrip Rust↔Go SQLite harness:

```
crates/rondo-tui/tests/roundtrip/
```

Tests stay `#[ignore]` until `RONDO_GO` env is set. The skeleton smoke test
always runs and validates the seed fixture loads.

## File map

| File | Purpose |
|---|---|
| `crates/rondo-core/src/domain/task.rs` | `Task`/`Subtask`/`TimeLog`/`TaskNote` + `Status`/`Priority`/`RecurFreq` |
| `crates/rondo-core/src/store/sqlite.rs` | `SqliteStore` RO/RW + all mutations |
| `crates/rondo-core/src/store/migrations.rs` | PRAGMA `user_version` migrations |
| `crates/rondo-core/src/store/backup.rs` | snapshot + rotation |
| `crates/rondo-core/src/store/lock.rs` | cooperative PID lock |
| `crates/rondo-core/src/store/seed.rs` | `ensure_seeded` on first run |
| `crates/rondo-core/src/store/plugin_kv.rs` | namespaced KV for plugins |
| `crates/rondo-core/src/recurrence.rs` | `next_occurrence`, `spawn_recurrent_instances` |
| `crates/rondo-core/src/export.rs` | builtin md/json/ndjson + `Exporter` trait |
| `crates/rondo-core/src/config.rs` | `~/.rondo-rs/config.toml` schema |
| `crates/rondo-core/src/telemetry.rs` | tracing logger + panic hook |
| `crates/rondo-plugin-api/src/*` | trait Plugin + ViewSpec DSL + Capability enum |
| `crates/rondo-plugin-host/src/host.rs` | extism loader + `dispatch` |
| `crates/rondo-plugin-host/src/policy.rs` | capability grant gate |
| `crates/rondo-tui/src/app/mod.rs` | `AppState::update` dispatcher |
| `crates/rondo-tui/src/app/{data,ui,modals}_state.rs` | substates |
| `crates/rondo-tui/src/action.rs` | every reducer action |
| `crates/rondo-tui/src/event.rs` | crossterm → Action mapping (modal-aware) |
| `crates/rondo-tui/src/focus.rs` | `Pane`/`DetailSection`/`FocusState`/`Mode` |
| `crates/rondo-tui/src/filter.rs` | `Filter` enum + `SIDEBAR_ITEMS` |
| `crates/rondo-tui/src/fx.rs` | `FxManager` + `presets` for tachyonfx |
| `crates/rondo-tui/src/theme.rs` | 7-token semantic palette |
| `crates/rondo-tui/src/search.rs` | nucleo fuzzy matcher + line highlighter |
| `crates/rondo-tui/src/components/*.rs` | UI panels (root, header, sidebar, task_list, task_detail, journal, pomodoro, help, search, command_palette, quick_add, quick_actions, footer, analytics, filter_strip, plugin_page, plugins_overlay, multiline_editor, edit_subtask, edit_title, dep_overlay, add_subtask, confirm, sort_overlay) |
| `crates/rondo-tui/src/widgets/*.rs` | reusable widget primitives (bracket_panel, priority_badge, due_badge, priority_spine, progress_bar, markdown, ring, sparkline, viewspec) |
| `crates/rondo-tui/src/plugins/builtin/*.rs` | builtin Plugins (pomodoro, bell, calendar, focus_page, dep_graph, analytics) |
| `crates/rondo-tui/src/cli.rs` | clap subcommands (add/list/done/delete/export/journal/focus/stats/batch/recur/dep/tag/completion/plugins) |
| `crates/rondo-tui/tests/snapshots.rs` | insta tests over `TestBackend` |
| `fixtures/seed.sql` | embedded first-run seed |
| `fixtures/seed-v1.sql` | pre-migration fixture for migrations smoke test |

## What "done" looks like for a feature

- snapshot tests cover the new visual state
- unit / integration tests cover the new behavior
- clippy `--all-targets -- -D warnings` clean
- `~/.rondo-rs/` writes confined to `--write` flow (default ON)
- footer hints + help overlay mention the new bindings
- CHANGELOG-worthy commit message

## Out of scope (deferred)

- Real KvGet round-trip via extism host-functions
- WASM hot-reload (manifest-only re-scan exists)
- TUI permission prompt overlay (CLI grant works today)
- Page::DepGraph / Page::Calendar / Page::FocusPage routes (use `:` commands)
- Cloud sync implementation (scaffold lives in `examples/plugins/sync-localdir/`)
