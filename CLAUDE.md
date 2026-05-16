# RonDO Rust MVP — Claude Code project memory

## What this project is

Greenfield MVP that ports a slice of the Go project at `/Users/roniel/Develop/rondo` to Rust + ratatui to evaluate whether the visual quality improves enough to justify a full rewrite. Read-only against the Go SQLite at `~/.todo-app/todo.db`.

The implementation plan lives at `/Users/roniel/.claude/plans/construyamos-un-mvp-enfocandonos-resilient-lemon.md`.

## Workspace layout

- `crates/rondo-core` — domain types (`Task`, `Note`, `Session`) + read-only `SqliteStore`. No TTY deps. Mirrors of Go `internal/task/*`, `internal/journal/*`, `internal/focus/*`.
- `crates/rondo-plugin-api` — stable plugin contract: `trait Plugin`, `PluginAction`, `ViewSpec` (serializable UI DSL). Designed so a future `extism`/`wasmtime` host can drive plugins compiled from this crate without ABI changes.
- `crates/rondo-tui` — ratatui binary. Component/Action/Reducer architecture inspired by `openapi-tui`. Theme tokens in `theme.rs`; never use hex-color literals outside of it.

## Reference projects (visual inspiration)

- binsider — https://github.com/orhun/binsider (minimalist, normal borders, compact density)
- openapi-tui — https://github.com/zaghaghi/openapi-tui (command palette, panes with focus-colored borders)
- slumber — https://github.com/LucasPickering/slumber (sidebar persistence, `:`-style command palette, vim bindings)
- mdfried — https://github.com/benjajaja/mdfried (rich markdown rendering — heading hierarchy with visual weight)

## Parity with Go version (do not break)

- Status icons: `○` Pending, `◐` InProgress, `✓` Done
- Priority labels/colors: LOW (green), MED (yellow), HIGH (red), URG! (magenta)
- Keybindings: `j/k` nav, `Tab` focus, `1/2` pages, `p` pomodoro, `:` command palette, `</>` resize, `q` quit, `Esc` close overlay
- Color palette hex: cyan #00BCD4, white #FAFAFA, green #4CAF50, red #F44336, muted #9E9E9E

## How to run

```bash
cargo run -p rondo-tui                       # uses ~/.todo-app/todo.db (real user data)
cargo run -p rondo-tui -- --db ./fixture.db  # custom path
RUST_LOG=debug cargo run -p rondo-tui        # logs to stderr (alt-screen safe)
```

Build a fixture DB:
```bash
FIXTURE=$(mktemp).db
sqlite3 "$FIXTURE" < fixtures/seed.sql
cargo run -p rondo-tui -- --db "$FIXTURE"
```

The DB is opened with `OpenFlags::SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only=true`. The Rust binary cannot corrupt the Go user data.

## Conventions

- **No `unsafe`** in `rondo-core` or `rondo-plugin-api`. The TUI uses none either.
- **Errors flow through `color_eyre::eyre::Result`** at the binary boundary, `rondo_core::Result` inside the core crate.
- **No business logic in `components/`.** Components read from `AppState`, never query the store directly. The store is owned by `AppState`.
- **Theme tokens, not hex.** All colors come from `theme.rs::Theme`. If a `Color::Rgb(...)` shows up outside `theme.rs`, move it.
- **Snapshot tests** for components live under `crates/rondo-tui/tests/`. Use `ratatui::backend::TestBackend` + assertions on rendered buffer text.
- **Plugin contract is the boundary.** Anything a future external plugin should do must round-trip through `rondo-plugin-api` types — never reach into `rondo-core` directly from plugins. `ViewSpec` is serializable so the same plugin can run in-process today and over WASM-ABI tomorrow.

## Build / test / lint

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## Architecture decisions

1. **Component/Action/Reducer** chosen over Elm-pure-update because overlays need to stack (pomodoro + command palette + page beneath) and the Component trait keeps draw functions colocated with their state.
2. **`ViewSpec` as a serializable DSL** (not `Box<dyn Widget>`) keeps the plugin contract stable across the future WASM boundary. The host owns the actual ratatui rendering.
3. **No tokio in MVP**. Single-threaded event loop with `crossterm::event::poll` + a 100ms tick. Plugin `Tick` actions get dispatched each tick. Adding tokio is a Task-3-of-the-real-rewrite concern.
4. **Read-only SQLite** with `Mutex<Connection>` for `Send + Sync`. No connection pool needed for personal-scale usage.

## Known limitations of the MVP

- **Read-only.** No task creation/edit/delete; the Go binary remains source of truth.
- **Pomodoro is in-memory.** Timer state lost on exit; not persisted to `focus_sessions`.
- **No CLI subcommands.** Only the TUI runs. Cobra parity is out of scope.
- **No plugin hot-reload, no `.wasm` loading.** Plugin host is in-process and static dispatch; the API surface is the part being validated.
- **Plugins cannot mutate `AppState`.** Plugin `handle` returns `ViewSpec` + follow-up actions only — by design, for sandbox readiness.

## When to consider this MVP "done"

- All 4 views render (task list, task detail, journal, pomodoro overlay) against the real `~/.todo-app/todo.db` of the user.
- Side-by-side screenshots taken vs the Go binary in each of the 4 contexts.
- Render smoke tests pass (`cargo test --workspace` is green).
- The user can answer: "Does this look meaningfully better than rondo-go?" with yes/no, not "kinda."

## Decision deferred

Whether to ship a full Rust rewrite depends entirely on the answer above. If yes: the plugin architecture in `rondo-plugin-api` is the foundation. If no: archive this workspace, port the visual ideas (theme tokens, command palette, markdown journal rendering) back to the Go version's `lipgloss` layer.

## Out-of-scope (will be plugins later, not core)

- Export (markdown/JSON)
- Batch NDJSON / CLI mode
- Skill installer / AUR-specific paths
- Recurrence preview, dependency graph view
- Image/attachment support (kitty protocol)
- Focus session persistence + stats
- Search and fuzzy filter (planned right after MVP)
