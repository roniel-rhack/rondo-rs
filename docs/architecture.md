# Architecture

## Workspace

3 first-party crates + 1 example folder. All cargo-workspace members except
`examples/plugins/*` which compile standalone (each has its own
`[workspace]` table so they target `wasm32-wasip1` without dragging the host
toolchain).

```
crates/
├── rondo-core         # domain types + READ_ONLY/READ_WRITE SqliteStore
├── rondo-plugin-api   # stable Plugin trait + ViewSpec DSL (serializable)
├── rondo-plugin-host  # extism wasm runtime + manifest loader + Policy
└── rondo-tui          # ratatui binary (lib + bin), CLI, builtins
examples/plugins/
├── quote-of-the-day   # OverlayView + TickHandler sample (real .wasm checked in)
├── exporter-org-mode  # Exporter capability scaffold
└── sync-localdir      # Syncer capability scaffold
```

Dependency direction: `rondo-tui` → `rondo-plugin-host` → `rondo-plugin-api`.
`rondo-tui` also depends on `rondo-core`. `rondo-plugin-api` has NO
dependencies on `rondo-core` so external plugins don't need to pull the
SQLite dep tree.

## Storage

SQLite at `~/.rondo-rs/todo.db` by default (override with `--db` or
`RONDO_DB`). First run creates the file and applies the embedded
`fixtures/seed.sql` so the UI always has data.

| Object | Concern |
|---|---|
| `OpenFlags::SQLITE_OPEN_READ_WRITE` + WAL + `foreign_keys=ON` | RW mode (`open_readwrite`) |
| `OpenFlags::SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only=true` | RO mode (`open_readonly`) |
| `Mutex<Connection>` | `Send + Sync` for the TUI loop |
| `~/.rondo-rs/backups/` | snapshot before every RW open; rotation 30 days |
| `~/.rondo-rs/rondo.lock` | cooperative PID lock, refuses concurrent RW |
| `~/.rondo-rs/logs/` | tracing output + panic backtraces, rotation 7 days |

Migrations live in `crates/rondo-core/src/store/migrations.rs`. PRAGMA
`user_version` drives them. v0→v1 adds `tasks.metadata`, v1→v2 creates
`focus_sessions`, v2→v3 creates `plugin_kv`.

## Substate split (AppState)

`crates/rondo-tui/src/app/`:

```
AppState
├── data     : DataState     # Arc<SqliteStore> + tasks/journal/selection
├── ui       : UiState       # FocusState, Mode, split_ratio, journal_pane
├── modals   : ModalsState   # every overlay flag + tui-textarea instances
├── fx       : FxManager     # tachyonfx 0.13 effects bucket
├── plugins  : PluginRegistry # in-process Plugin trait dispatch
├── theme    : Theme
├── writable : bool
├── undo     : UndoStack
└── status_msg : Option<String>
```

Each substate has its own `update(action) -> Option<Action>`. The top-level
`AppState::update` dispatches to substates first, then handles cross-cutting
actions (mutations that need access to >1 substate, animations, plugin
dispatch).

## Hierarchical focus

`FocusState { pane, section, section_item, sidebar_item }` plus
`UiState.journal_pane` for the Journal page (no FocusState there). Bindings
look at the focus tuple to pick which action to dispatch. e.g. `e` is
`RequestEditTitle` in Detail::Header but `RequestEditFocusedSubtask` in
Detail::Subtasks.

## Plugin contract

`crates/rondo-plugin-api/src/plugin.rs`:

```rust
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn handle(&mut self, action: PluginAction, ctx: &PluginContext) -> PluginResult;
}
```

`PluginAction` covers `Show`, `Hide`, `Tick`, `KeyPress`, `Query`, `KvGet`,
`KvSet`, `Notify`, `Command`. `PluginResult` returns `Option<ViewSpec>` plus
`follow_up: Vec<PluginAction>` so plugins request host services. All types
are `serde::Serialize/Deserialize` so the SAME types travel both
in-process and across the future extism boundary.

`ViewSpec` blocks: `Heading{text,level}`, `Paragraph{text,style}`,
`Gauge{ratio,label}`, `Throbber{label}`, `Divider`, `Spans(Vec<Span>)`. Each
span carries a `TextStyle` with `ColorToken`s (Accent/Success/Warning/
Danger/Muted/Foreground/Background) resolved against the active `Theme` at
render time by `widgets/viewspec.rs`.

`Capability` enum gates what each plugin can do; dangerous variants
(`MutationAccess`, `Syncer`, `Notifier`, `CliSubcommand`) require explicit
grant via `~/.rondo-rs/config.toml` `[plugins.permissions]`. The policy
check happens at load time in `rondo-plugin-host::host::PluginHost::load_one`.

## Tick / event loop

`crates/rondo-tui/src/main.rs::run`:

- Adaptive tick: 40 ms when an animation is running, 100 ms when pomodoro
  open, 60 s otherwise.
- `crossterm::event::poll(timeout)` is the only blocking call.
- `ev::map(event, app)` produces `Option<Action>`. Modal-aware handlers
  intercept first; everything else flows into `key_to_action`.
- `app.update(action)` synchronously mutates state.
- Render only when dirty.
- Plugin ticks are dispatched once per main tick when
  `needs_animation_tick()` is true.

## Effects (`src/fx.rs`)

`FxManager` owns a small `Vec<LiveEffect>`. Each entry pins an `EffectId`,
the `tachyonfx::Effect`, and the `Rect` to apply it to. Effects rendered
AFTER everything else paints, mutating buffer cells in place. `dt` is
clamped to 64 ms so a slow frame can't fast-forward an animation, and
`last_tick` resets on transitions from empty→non-empty so the first frame
after idle isn't a giant jump.

## Language packs (i18n)

External, user-installable TOML packs under `~/.rondo-rs/lang/<code>.toml`.
English is baked into the binary at `crates/rondo-core/src/i18n/en.toml`
(pulled via `include_str!`) and is the single source of truth for every
key — translators copy it via `lang scaffold` and overwrite values in
place.

| Piece | Where |
|---|---|
| Runtime + `t()` / `tf()` helpers | `crates/rondo-core/src/i18n/mod.rs` |
| Baked baseline | `crates/rondo-core/src/i18n/en.toml` |
| Active pack handle | `arc_swap::ArcSwap<Translations>` in `i18n::ACTIVE` |
| Selection persisted | `[ui].language` in `~/.rondo-rs/config.toml` |
| CLI surface | `rondo-rs lang scaffold|install|list|remove|current` |
| TUI palette | `:lang` opens `components::lang_picker` |

`t(key)` resolves against the active pack first, then the baked baseline,
then falls back to the key verbatim with `tracing::warn!`. The
`:lang` modal calls `i18n::set_active` and `Config::save` together so the
next render frame uses the new pack and the choice survives restart.

Tests pin the active pack to English via `i18n::force_for_tests()` so
snapshots remain locale-stable regardless of host config.

## Reference projects (visual inspiration)

binsider · openapi-tui · slumber · mdfried · plus the NEXUS-TASK mockup
that defined the 3-column dashboard + analytics row.
