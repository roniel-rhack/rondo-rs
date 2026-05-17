# rondo-rs — Claude Code project memory

Rust + ratatui terminal task manager + journal. All state is contained
under `~/.rondo-rs/`.

## What this is

A full task manager with: tasks (CRUD, subtasks, dependencies, tags,
time-logs, notes, recurrence), journal (per-day notes with markdown +
multi-line tui-textarea editor), focus sessions (persisted), command
palette, fuzzy search with highlight in list AND detail, plugin system
(builtin in-process + external WASM via extism), 17 CLI subcommands.

## Quick-start commands

```bash
# Default: RW against ~/.rondo-rs/todo.db (auto-seeded on first run)
cargo run --release -p rondo-tui

# Read-only safety mode
cargo run --release -p rondo-tui -- --read-only

# CLI mode (subcommand pre-empts the TUI)
cargo run --release -p rondo-tui -- list
cargo run --release -p rondo-tui -- add "new task #tag !p3 due:tmrw"
cargo run --release -p rondo-tui -- plugins list

# Disable animations / honor NO_COLOR
RONDO_FX=0 cargo run -p rondo-tui
NO_COLOR=1 cargo run -p rondo-tui
cargo run -p rondo-tui -- --reduced-motion --no-color
```

## Where things live

| File | What |
|---|---|
| [README.md](README.md) | User-facing manual, install + plugin authoring |
| [docs/keybindings.md](docs/keybindings.md) | Every binding, modal stack, section-scoped keys |
| [docs/architecture.md](docs/architecture.md) | Crate graph, AppState substate split, plugin contract, tick loop |
| [docs/plugins.md](docs/plugins.md) | How to author + install a WASM plugin, capability cheat sheet |
| [docs/dev.md](docs/dev.md) | Conventions, file map, test commands, deferred work |

## Conventions (the short list)

- No `unsafe`. Theme tokens not hex. No `Modifier::REVERSED` for selection.
- Errors: `color_eyre::eyre::Result` at boundaries, `rondo_core::Result` inside core.
- Snapshot tests via `insta` + `TestBackend` — wall-clock timestamps redacted globally.
- Plugin DSL types must survive `serde_json` round-trip.
- All state under `~/.rondo-rs/` (DB, backups, logs, lock, plugins, sync, config).

## Architecture decisions (short)

1. **Component free-fn** `draw(app, f, rect)` over a trait — simpler, no dynamic dispatch.
2. **Hierarchical focus** (`FocusState { pane, section, section_item, sidebar_item }`)
   so bindings can be section-scoped: `e` is "edit title" in Header, "rename"
   in Subtasks, "edit note" in Notes, etc.
3. **`ViewSpec` as serializable DSL** keeps the plugin contract stable across
   the in-process → WASM boundary. Host owns the rendering via
   `widgets/viewspec.rs`.
4. **No tokio.** Single-threaded event loop with adaptive tick (40/100/60_000 ms).
5. **Read-only / read-write SQLite** via two `open_*` constructors; both wrap a
   `Mutex<Connection>` for `Send + Sync`.
6. **Substate split** of `AppState` into `data`/`ui`/`modals` so writes,
   modals and undo don't sprawl into a single 1000-line `update()` match.
7. **tachyonfx pinned to 0.13** (last version on ratatui 0.29).
8. **extism pinned to 1.10** (newer versions need rustc ≥ 1.90; we target 1.83).
9. **`~/.rondo-rs/` confinement** — every path the binary writes (DB,
   backups, logs, lock, plugins, sync, config) lives under that single
   root; nothing outside is touched.

## Build / test / lint

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
INSTA_UPDATE=always cargo test -p rondo-tui --test snapshots   # regen snapshots
```

## Known limitations

- KvGet round-trip via extism host-function not wired yet (KvSet works).
- TUI permission-prompt overlay deferred; permissions granted via CLI/config.
- Page::DepGraph / Page::Calendar / Page::FocusPage not bound to keys — use
  `:calendar`, `:deps`, `:focus`, `:analytics` from the command palette.
- Cloud sync is a scaffold (`examples/plugins/sync-localdir/`).
- `B` adds/removes dependency by typing a task id; no task-picker overlay yet.
