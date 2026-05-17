# rondo-rs

A keyboard-driven terminal task manager + journal in Rust, with a plugin
system that runs both in-process Rust plugins and external WASM modules.

Coexists with the Go [rondo](https://github.com/roniel-rondo/rondo) binary:
every byte rondo-rs writes lives under `~/.rondo-rs/`, isolated from the Go
binary's `~/.todo-app/`.

## Highlights

- **Tasks**: status (○/◐/✓), priority (LOW/MED/HIGH/URG), tags, due dates,
  subtasks, dependencies with cycle detection, time logs, notes, recurrence
  (daily/weekly/monthly/yearly with configurable interval and end-of-month
  clamping).
- **Journal**: per-day notes with multi-line `tui-textarea` editor, full
  cursor navigation, markdown rendering (H1-H6, lists, task lists,
  blockquotes, strikethrough, links, inline + fenced code, horizontal rules),
  and entry-level edit/delete.
- **Focus / pomodoro**: animated 25-min timer with throbber + gauge, sessions
  persisted to a dedicated table, streak counter, optional bell on completion.
- **Plugins**: in-process builtins (pomodoro, bell, calendar, focus heatmap,
  dependency graph, analytics dashboard) plus external WASM via
  [extism](https://extism.org). Sandboxed by WASI; dangerous capabilities
  require explicit user grant in `~/.rondo-rs/config.toml`.
- **CLI mode**: same binary doubles as a non-TUI CLI with 17 subcommands
  (add, list, done, delete, journal, focus, stats, batch (NDJSON stdin),
  recur preview, dep, tag, completion, plugins, export).
- **Animations**: tachyonfx 0.13 effects on selection change, page swap,
  task completion sweep, status toast, pomodoro open, quick-add slide.
  Toggle off with `RONDO_FX=0` or `--reduced-motion`.
- **Theming**: dark / light / high-contrast. `NO_COLOR` env honored.
- **Read-only or read-write**: launches RW by default (auto-creates +
  seeds `~/.rondo-rs/todo.db` on first run); pass `--read-only` for safety.
  Every RW open snapshots the DB into `~/.rondo-rs/backups/` (30-day
  rotation) and acquires a cooperative PID lock.
- **Undo**: bounded stack of 50 entries; `Ctrl+Z` replays the inverse of any
  CRUD action via the store.
- **Fuzzy search** with highlighted matches in both the task list and the
  detail panel (title, tags, description, subtasks, notes).
- **Bracketed clipboard paste** in every input surface — multi-line text
  goes into the description / journal / note editors at the cursor;
  single-line surfaces take the first line.

## Install

Requires Rust 1.83 (toolchain pinned via `rust-toolchain.toml`).

```bash
git clone https://github.com/roniel-rhack/rondo-rs
cd rondo-rs
cargo build --release
./target/release/rondo-tui
```

First run creates `~/.rondo-rs/` and seeds the DB with sample tasks.

## Configuration

Optional TOML at `~/.rondo-rs/config.toml` (override path via
`RONDO_CONFIG`):

```toml
[ui]
theme = "dark"            # "dark" | "light" | "high-contrast"
sidebar = true
animations = true

[pomodoro]
work_min = 25
short_break_min = 5
long_break_min = 15

[plugins]
enabled = ["builtin.pomodoro", "my-external-plugin"]

[plugins.permissions]
"my-external-plugin" = ["mutation_access", "notifier"]
```

Env overrides:

| Variable | Effect |
|---|---|
| `RONDO_DB` | DB path (default `~/.rondo-rs/todo.db`) |
| `RONDO_CONFIG` | Config path |
| `RONDO_FX=0` | Disable animations |
| `RONDO_REDUCED_MOTION=1` | Same as `--reduced-motion` |
| `NO_COLOR=1` | Honor `NO_COLOR` spec |
| `RUST_LOG=debug` | tracing level (logs to `~/.rondo-rs/logs/`) |

## CLI subcommands

The same binary acts as a non-TUI CLI when a subcommand is provided:

```bash
rondo-tui add "Review PR #42 #work !p3 due:tmrw"
rondo-tui list --filter all
rondo-tui done 7
rondo-tui delete 3
rondo-tui journal add "Shipped v0.1.0 today"
rondo-tui journal list
rondo-tui focus start
rondo-tui focus stats
rondo-tui stats --json
rondo-tui export --format md
rondo-tui export --format json
rondo-tui export --format ndjson
rondo-tui recur preview
rondo-tui dep add 4 1                  # task 4 blocked by task 1
rondo-tui dep remove 4 1
rondo-tui tag add 3 personal
rondo-tui tag remove 3 personal
rondo-tui batch < bulk-ops.ndjson      # one {"op":"...","..."} per line
rondo-tui completion bash > rondo.bash
rondo-tui plugins list
rondo-tui plugins info my-plugin
rondo-tui plugins install ./path/to/my-plugin
rondo-tui plugins remove my-plugin
```

Global flags: `--db <PATH>`, `--read-only`, `--json`, `--no-color`,
`--reduced-motion`.

## Keybindings cheat sheet

See [docs/keybindings.md](docs/keybindings.md) for the complete list.

| Context | Key | Action |
|---|---|---|
| Global | `?` | help overlay |
| Global | `:` | command palette |
| Global | `/` | fuzzy search |
| Global | `q` / `Ctrl+C` | quit |
| Global | `Ctrl+Z` | undo |
| Global | `1` / `2` | Tasks / Journal page |
| Global | `h` / `l` | focus left / right |
| Tasks | `a` | quick-add |
| Tasks | `e` / `E` | edit title / edit description |
| Tasks | `A` / `B` | add subtask / dependency |
| Tasks | `d` | delete (confirm) |
| Tasks | `v` | Visual multi-select |
| Tasks | `p` | pomodoro overlay |
| Tasks | `s` | sort overlay |
| Tasks | `f<letter>` | apply filter (i/t/p/A/u/H/o/n/c) |
| Detail pane | `Tab` / `Shift+Tab` | cycle sections |
| Detail pane | `1` / `2` / `3` / `4` | jump to Header / Subtasks / Deps / Notes |
| Detail pane | section-scoped `e`, `d`, `a` | act on focused item |
| Journal | `i` / `A` | new entry |
| Journal | `e` | edit focused entry |
| Journal | `d` / `D` | delete focused entry |
| Journal | `X` | delete focused DAY |
| Journal | `J` / `K` | cycle days |
| Journal | `H` | toggle hidden |

## Plugins

See [docs/plugins.md](docs/plugins.md) for the full guide.

Plugins implement a single trait + a serializable view DSL:

```rust
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn handle(&mut self, action: PluginAction, ctx: &PluginContext) -> PluginResult;
}
```

`PluginAction` variants: `Show`, `Hide`, `Tick { delta_ms }`,
`KeyPress { key }`, `Query { scope_id, params }`, `KvGet { key }`,
`KvSet { key, value }`, `Notify { channel, message }`, `Command { name, args }`.

`PluginResult` returns an `Option<ViewSpec>` and a `follow_up: Vec<PluginAction>`
queue the host processes (e.g. `KvSet` persists to a namespaced
`plugin_kv` table).

### Builtin plugins shipped

| Id | Capabilities | What |
|---|---|---|
| `builtin.pomodoro` | OverlayView, TickHandler, CommandContributor | Animated 25-min timer with gauge + throbber |
| `builtin.bell` | Notifier(Audio) | BEL on pomodoro completion |
| `builtin.calendar` | PageView, QueryAccess(Journal) | Mini-month grid with dots on entry days, interactive cursor (h/l/j/k/J/K/t), entry preview |
| `builtin.focus-page` | PageView, QueryAccess(FocusSessions) | 5×7 shade heatmap + streak |
| `builtin.dep-graph` | PageView, QueryAccess(Deps) | ASCII tree of `blocked_by` chain with cycle detection |
| `builtin.analytics` | PageView, QueryAccess(Tasks, FocusSessions) | 4-panel dashboard (vista general donut, próximas 7d bars, tag distribution, sync block) |

Open any of them with `:calendar`, `:focus`, `:deps`, `:analytics`.
List + manage with `rondo-tui plugins list`.

### Creating a WASM plugin

```bash
cargo new --lib my-plugin
cd my-plugin
```

`Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
rondo-plugin-api = { git = "https://github.com/roniel-rhack/rondo-rs", default-features = false }
extism-pdk = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[profile.release]
opt-level = "s"
lto = true
```

`.cargo/config.toml`:

```toml
[build]
target = "wasm32-wasip1"
```

`src/lib.rs`:

```rust
use extism_pdk::*;
use rondo_plugin_api::{
    PluginAction, PluginContext, PluginResult,
    view::{Block, ViewKind, ViewSpec},
};

#[derive(serde::Deserialize)]
struct Input { action: PluginAction, ctx: PluginContext }

#[plugin_fn]
pub fn handle(input: Json<Input>) -> FnResult<Json<PluginResult>> {
    let view = match input.0.action {
        PluginAction::Show => Some(ViewSpec {
            kind: ViewKind::Page,
            blocks: vec![
                Block::Heading { text: "Hello".into(), level: 1 },
                Block::Paragraph { text: "World".into(), style: None },
            ],
        }),
        _ => None,
    };
    Ok(Json(PluginResult { view, follow_up: vec![] }))
}
```

`plugin.toml`:

```toml
id = "my-plugin"
name = "My Plugin"
version = "0.1.0"
api = "0.1"
capabilities = ["OverlayView", "TickHandler"]

[wasi]
allowed_paths = []
allowed_hosts = []
```

Build + install:

```bash
rustup target add wasm32-wasip1
cargo build --release
cp target/wasm32-wasip1/release/my_plugin.wasm plugin.wasm
rondo-tui plugins install /path/to/my-plugin
rondo-tui plugins list
```

If your manifest declares `MutationAccess`, `Syncer`, `Notifier` or
`CliSubcommand`, grant in `~/.rondo-rs/config.toml`:

```toml
[plugins.permissions]
"my-plugin" = ["mutation_access", "notifier"]
```

Without that grant the plugin loads with `enabled: false` and shows up in
`plugins list` with a warning.

Three sample plugins live in `examples/plugins/`:

- `quote-of-the-day` — OverlayView + TickHandler demo (real `.wasm` checked in)
- `exporter-org-mode` — Exporter capability scaffold
- `sync-localdir` — Syncer capability scaffold

## Filesystem layout

```
~/.rondo-rs/
├── todo.db                       # SQLite database (RW)
├── todo.db-shm                   # WAL shared memory
├── todo.db-wal                   # WAL write-ahead log
├── config.toml                   # optional TOML config
├── rondo.lock                    # cooperative PID lock (RW mode)
├── logs/
│   └── rondo-rs-YYYYMMDD-HHMMSS.log      # rotation 7 days
├── backups/
│   └── YYYYMMDDTHHMMSSZ-todo.db          # rotation 30 days
└── plugins/
    └── <plugin-id>/
        ├── plugin.toml
        └── plugin.wasm
```

The Go binary at `~/.todo-app/` is completely separate; rondo-rs never
reads or writes there.

## Documentation map

- [docs/architecture.md](docs/architecture.md) — workspace layout, storage,
  substate split, plugin contract, tick loop, effects.
- [docs/keybindings.md](docs/keybindings.md) — every binding, every modal,
  section-scoped keys.
- [docs/plugins.md](docs/plugins.md) — author + install guide, capability
  cheat sheet, host↔plugin protocol.
- [docs/dev.md](docs/dev.md) — conventions, file map, test commands.
- [ROADMAP.md](ROADMAP.md) — milestone tracking.

## License

MIT. Personal project, no warranty.
