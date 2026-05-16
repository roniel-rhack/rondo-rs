# RonDO Rust — Claude Code project memory

## What this project is

Rust + ratatui port of the Go project at `/Users/roniel/Develop/rondo`. The visual-evaluation MVP is done — the verdict came in **YES**, push forward. The codebase is now growing toward feature parity + going beyond what the Go version offers (richer dashboard, plugin architecture, animations).

Current canonical roadmap: `ROADMAP.md` (at project root). Original MVP plan kept at `/Users/roniel/.claude/plans/construyamos-un-mvp-enfocandonos-resilient-lemon.md`.

## Workspace layout

- `crates/rondo-core` — domain types (`Task`, `Subtask`, `TimeLog`, `Note`, `Entry`, `Session`) + read-only `SqliteStore`. No TTY deps. Mirrors of Go `internal/task/*`, `internal/journal/*`, `internal/focus/*`.
- `crates/rondo-plugin-api` — stable plugin contract: `trait Plugin`, `PluginAction`, `ViewSpec` serializable UI DSL. Designed so a future `extism`/`wasmtime` host can drive plugins without ABI changes.
- `crates/rondo-tui` — ratatui binary. Component/Action/Reducer architecture inspired by `openapi-tui`. Theme tokens in `theme.rs`, never hex literals outside.

## Reference projects (visual inspiration)

- **binsider** https://github.com/orhun/binsider — bordered header, key-binding strip
- **openapi-tui** https://github.com/zaghaghi/openapi-tui — command palette, pane focus
- **slumber** https://github.com/LucasPickering/slumber — `:`-style commands, vim bindings
- **mdfried** https://github.com/benjajaja/mdfried — rich markdown rendering
- **NEXUS-TASK** mockup (user-supplied) — 3-column dashboard, bottom analytics row, action grid

## Current visual / UX state (as of commit `0193c5b`)

### Layout
- **Brand strip** (1 row top): `▌rondo vX.Y.Z  //  SISTEMA DE GESTIÓN DE TAREAS AVANZADO  //  ⊙ ONLINE · HH:MM:SS · ◷ done/total · P— · ☑n`
- **Body** 3 columns when terminal ≥ 100 cols:
  - **Sidebar** (26 cols, full borders): NAVEGACIÓN (4 items) + FILTROS RÁPIDOS (5 items), each with `[letter]` shortcut prefix
  - **Task list panel**: bracket-bordered, column header (ESTADO · PRI · TAREA), spaced multi-row task entries, `╌╌╌` dashed separators, bottom PROGRESO GENERAL bar
  - **Detail panel**: card sections (HEADER · METADATA · DESCRIPCIÓN · SUBTAREAS · DEPENDENCIAS · TIEMPO · NOTAS) with section dividers `── SECTION ──`
- **Analytics row** (9 rows, terminal ≥ 30 rows): vista general / próximas 7 días / distribución por tag / sincronización
- **Footer** (1 row): mode pill `[NOR/INS/VIS]` + 5 context-aware hint chips + `? more`

### Interaction model
- **Pane focus stack**: `Page → Pane → Section → Item`. Pane is one of `Sidebar | List | Detail`. `h/l` moves pane, `Tab/Shift+Tab` cycles sections inside Detail.
- **Mode**: vim-style Normal / Insert / Visual. Visual via `v` for multi-select.
- **Filter system**: 9 real filters (Inbox, Hoy, Próximas, Todas, Urgentes, AltaPrio, Vencidas, SinEtiqueta, Completadas). Each has a single-letter shortcut. Apply via `f<letter>` from anywhere, or single-letter when sidebar focused.

### Animations (tachyonfx 0.13)
- **Filter applied**: status toast on footer (fade_in accent → 900ms hold → fade_out muted)
- **Task selection change**: detail panel coalesce + fade_from_fg accent
- **Bulk task done**: sweep_in over task list
- **Pomodoro open**: fade_from_fg accent over modal
- **Quick-add submit**: slide_in UpToDown over task list
- **Page swap** (Tab/1/2): sweep_in over body
- **Flag**: `RONDO_FX=0` disables all effects

### Bindings (current full set)
| Key | Context | Action |
|---|---|---|
| `j` / `k` / arrows | global | move cursor |
| `g` / `G` | list | jump top / bottom |
| `Ctrl+D` / `Ctrl+U` | list | half page |
| `h` / `l` | global | focus pane Sidebar ↔ List ↔ Detail |
| `Tab` / `Shift+Tab` | Detail | cycle section (Header/Subtasks/Deps/Notes) |
| `1` / `2` | global | Tasks / Journal page |
| `space` | Detail · Subtasks | toggle subtask done |
| `v` | List | enter Visual multi-select |
| `d` | Visual | bulk done |
| `P` | Visual | bulk priority cycle |
| `a` | List | quick-add inline overlay |
| `f<letter>` | global | apply filter (i/t/p/A/u/H/o/n/c) |
| `/` | global | search overlay |
| `:` | global | command palette |
| `.` | global | quick actions grid overlay |
| `?` | global | help overlay |
| `p` | global | toggle pomodoro |
| `<` / `>` / `=` | global | resize split ±5% / reset |
| `Esc` | global | close top modal (stacked: help > palette > quickadd > search > visual > pomodoro > status) |
| `q` / `Ctrl+C` | global | quit |

## How to run

```bash
# Real user data (read-only)
cargo run -p rondo-tui

# Fixture for safe poking
rm -f /tmp/seed.db && sqlite3 /tmp/seed.db < fixtures/seed.sql
cargo run -p rondo-tui -- --db /tmp/seed.db

# Disable animations
RONDO_FX=0 cargo run -p rondo-tui

# Logs (stderr, alt-screen safe)
RUST_LOG=debug cargo run -p rondo-tui
```

DB is opened READ_ONLY + `PRAGMA query_only=true`. The Rust binary cannot corrupt Go's user data.

## Conventions

- **No `unsafe`** anywhere.
- **Errors**: `color_eyre::eyre::Result` at binary boundary, `rondo_core::Result` inside core crate.
- **No business logic in `components/`**. Components read from `AppState`, never query the store directly. Store owned by `AppState`.
- **Theme tokens, not hex.** All colors come from `theme.rs::Theme`. `Color::Rgb(...)` outside `theme.rs` is a bug.
- **No `Modifier::REVERSED` for selection.** Theme-safe: foreground emphasis (bold + underlined + accent fg) only. The accent `▌` gutter is the cursor indicator.
- **Snapshot tests** under `crates/rondo-tui/tests/snapshots.rs` via `insta::assert_snapshot!` over `TestBackend`. Wall-clock timestamps (`HH:MM`, `HH:MM:SS`) auto-redacted globally.
- **Animations via `crate::fx`**: spawn via `AppState::fx.spawn(EffectId::*, presets::*(theme), rect)`. Rects are cached in `last_*_rect` fields populated by `root::draw`. Effects run as overlays in `tick_and_render(f)` at end of frame.
- **Plugin contract is the boundary.** External plugins round-trip through `rondo-plugin-api` types only — never reach into `rondo-core` directly. `ViewSpec` is serializable so the same plugin can run in-process today and over WASM tomorrow.

## Architecture decisions

1. **Component free-fn `draw(app, f, rect)`** chosen over a trait — simpler, no dynamic dispatch, and `&mut AppState` chain is straightforward.
2. **Hierarchical focus stack** (`FocusState { pane, section, section_item, sidebar_item }`) instead of `focus_left: bool`. Each level owns its own keybindings + cursor; eliminates the "I'm in detail but j/k still moves the task list" bug.
3. **`ViewSpec` as serializable DSL** keeps the plugin contract stable across the future WASM boundary. Host owns the ratatui rendering.
4. **No tokio.** Single-threaded event loop with `crossterm::event::poll` + adaptive tick (40ms while fx running, 100ms while pomodoro idle, 60s deep idle). Plugin `Tick` actions dispatch each tick.
5. **Read-only SQLite** with `Mutex<Connection>` for `Send + Sync`. No pool needed for personal-scale usage.
6. **`Filter` enum** in `crate::filter` is the authoritative source for "what's visible". `visible_task_indices()` filters at render time. Sidebar items are entries in `SIDEBAR_ITEMS: &[Filter]`.
7. **`FxManager`** wraps `Vec<(EffectId, Effect, Rect)>` + `last_tick`. Effect dt is clamped to 64ms so a laggy frame can't fast-forward an effect to completion. `last_tick` resets when the bucket transitions from empty to non-empty, so an idle gap doesn't break the first dt.
8. **tachyonfx pinned to 0.13** (last version that uses ratatui 0.29 directly; v0.14+ requires ratatui-core 0.1 split which lands in ratatui 0.30). Revisit on the ratatui 0.30 upgrade.

## Build / test / lint

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Snapshot maintenance:
```bash
INSTA_UPDATE=always cargo test -p rondo-tui --test snapshots  # accept all
cargo insta review                                             # interactive
```

## Known limitations (still — see ROADMAP.md for the plan)

- **Read-only against SQLite.** Toggles and edits are in-memory only; the binary cannot mutate user data yet.
- **Pomodoro in-memory.** Timer lost on exit, no persistence to `focus_sessions`.
- **No CLI subcommands.** Only the TUI runs. Cobra-style parity is roadmap item.
- **No plugin loader.** Contract exists, host can register builtin plugins; no `.wasm` / dylib loading.
- **Plugins read-only.** Plugin `handle` returns `ViewSpec` + follow-up actions only; cannot mutate `AppState`.
- **Sidebar items not all real.** PROYECTOS, GRAFO, AUTOMAT., CALENDARIO, ANÁLISIS, PAPELERA were dropped in commit `b0bf41f`; only filters that apply to the actual data model remain.

## What "done" looks like now

The MVP-visual-evaluation gate already passed. The next gate is **feature parity with rondo-go** + a working **plugin loader**. Tracked in `ROADMAP.md`.

## Reference: file map

| File | Purpose |
|---|---|
| `crates/rondo-core/src/domain/task.rs` | `Task`/`Subtask`/`TimeLog`/`TaskNote` + `Status`/`Priority`/`RecurFreq` |
| `crates/rondo-core/src/store/sqlite.rs` | `SqliteStore` READ_ONLY; `list_tasks` / `task_by_id` / `list_journal_notes` / `entries_for_note` |
| `crates/rondo-core/src/store/queries.rs` | SQL constants |
| `crates/rondo-plugin-api/src/{plugin,action,view,registry,capabilities}.rs` | plugin contract |
| `crates/rondo-tui/src/app.rs` | `AppState` (god-struct, intentional — splittable later); `update(action)` reducer |
| `crates/rondo-tui/src/action.rs` | `Action` enum (all reducer messages) |
| `crates/rondo-tui/src/event.rs` | crossterm → `Action` mapping; modal-aware |
| `crates/rondo-tui/src/focus.rs` | `Pane`/`DetailSection`/`FocusState`/`Mode` |
| `crates/rondo-tui/src/filter.rs` | `Filter` enum + `SIDEBAR_ITEMS` |
| `crates/rondo-tui/src/fx.rs` | `FxManager` + `presets` for tachyonfx effects |
| `crates/rondo-tui/src/theme.rs` | 7-token semantic palette + helpers |
| `crates/rondo-tui/src/components/{root,header,sidebar,task_list,task_detail,journal,pomodoro,help,search,command_palette,quick_add,quick_actions,footer,analytics,filter_strip}.rs` | UI panels |
| `crates/rondo-tui/src/widgets/{bracket_panel,priority_badge,due_badge,priority_spine,progress_bar,markdown,ring,sparkline}.rs` | reusable widget primitives |
| `crates/rondo-tui/src/plugins/builtin/pomodoro.rs` | sample plugin using the API |
| `crates/rondo-tui/tests/snapshots.rs` | insta tests over `TestBackend` |
| `fixtures/seed.sql` | reproducible 4-task seed DB |
