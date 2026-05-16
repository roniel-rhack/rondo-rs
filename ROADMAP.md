# RonDO Rust — Roadmap de funcionalidades faltantes

Este documento es el plan operativo para llevar rondo-rust de "MVP visual aprobado" a "feature parity + ventaja propia sobre la versión Go". Cada hito tiene fases ordenadas por dependencia + reward:risk, con archivos a tocar.

## Estado actual

✅ visual richness · sidebar interactivo · focus stack jerárquico · filtros · animaciones tachyonfx · plugin contract · 20 snapshot tests

❌ writes a la DB · CRUD desde TUI · CLI subcomandos · journal write · pomodoro persistence · recurrence engine · export · stats reales · plugin loader real · multi-device sync

## Principios

1. **Antes de tocar writes, garantizar backup automático** — el Go ya tiene backup diario. Honrar eso desde Rust.
2. **Writes detrás de un flag explícito** (`--db` distinto OR `--write` flag). Mientras el flag esté OFF, la app es read-only para todas las paths nuevas. Cero riesgo de corromper datos reales.
3. **Test de migración compatible** — Rust escribe → Go lee → Rust lee. Roundtrip verde antes de quitar el flag.
4. **Cada fase deja la app shippable** (tests verdes, clippy limpio, `cargo run` corre).

---

## M1 — Writes seguros + CRUD de tareas básico

**Objetivo:** El usuario puede crear, editar, marcar y borrar tareas desde la TUI sin perder paridad con el binario Go.

### Fase M1.1 — `SqliteStore` modo read-write + backup pre-write

Files: `crates/rondo-core/src/store/sqlite.rs`, nuevo `crates/rondo-core/src/store/backup.rs`, `crates/rondo-tui/src/main.rs`, `CLAUDE.md`.

- Añadir `SqliteStore::open_readwrite(path) -> Result<Self>` con `OpenFlags::SQLITE_OPEN_READ_WRITE | SQLITE_OPEN_CREATE`. Mantener `open_readonly` separado.
- Pre-write backup: `backup::snapshot(db_path) -> PathBuf` copia DB con timestamp ISO a `~/.todo-app/backups/rondo-rust-{ts}.db`. Rotación 30 días (paridad con Go en `internal/database/backup.go`).
- CLI flag `--write` en `main.rs`: si presente, abre RW; si no, RO (default).
- Test: abrir RW + abrir RO sobre el mismo path verifican comportamiento.

### Fase M1.2 — Schema migrations & `addColumnIfNotExists` parity

Files: `crates/rondo-core/src/store/migrations.rs`, `crates/rondo-core/src/store/sqlite.rs`.

- Module `migrations` con función `ensure_schema(conn) -> Result<()>`:
  - `PRAGMA user_version` para versioning explícito (Go usa `addColumnIfNotExists` por shape — equivalente).
  - Bootstraps fresh DB con todas las tablas del seed.sql.
  - Migración idempotente: si una columna falta, `ALTER TABLE ADD COLUMN`.
- `SqliteStore::open_readwrite` corre migrations.
- Test: tomar `fixtures/seed-v1.sql` (sin columna `metadata`), abrir RW, verificar migración OK.

### Fase M1.3 — Domain mutations en `rondo-core`

Files: `crates/rondo-core/src/store/sqlite.rs`, `crates/rondo-core/src/domain/task.rs`.

API mínima:
```rust
impl SqliteStore {
    fn create_task(&self, input: NewTask) -> Result<i64>;
    fn update_task(&self, id: i64, patch: TaskPatch) -> Result<()>;
    fn delete_task(&self, id: i64) -> Result<()>;
    fn set_status(&self, id: i64, status: Status) -> Result<()>;
    fn add_subtask(&self, task_id: i64, title: &str) -> Result<i64>;
    fn toggle_subtask(&self, subtask_id: i64) -> Result<bool>;
    fn delete_subtask(&self, subtask_id: i64) -> Result<()>;
    fn add_tag(&self, task_id: i64, name: &str) -> Result<()>;
    fn remove_tag(&self, task_id: i64, name: &str) -> Result<()>;
    fn append_metadata(&self, task_id: i64, key: &str, value: &str) -> Result<()>;
}
```
`NewTask` / `TaskPatch` structs en `domain::task`. Patch usa `Option<T>` por campo para soportar partial updates. Todas las writes dentro de transacciones (`conn.transaction(...)`).

Tests por método contra fixture temp DB.

### Fase M1.4 — Wire CRUD desde TUI

Files: `crates/rondo-tui/src/app.rs`, `crates/rondo-tui/src/components/quick_add.rs`, `crates/rondo-tui/src/components/task_detail.rs`, `crates/rondo-tui/src/event.rs`.

- `submit_quick_add` ahora llama `store.create_task(parsed.into())` si modo RW. Toast cambia: "creada #N".
- `handle_space` en Subtasks: llama `store.toggle_subtask(id)` si RW. Caída a in-memory si RO.
- Bulk done en Visual: `store.set_status(id, Done)` por cada selected.
- Bind `d` (List): `store.delete_task(id)` + confirm modal.
- Bind `e` (Detail · Header section): inline edit del título.
- Reload `app.tasks` después de cada mutación (re-query) — costo aceptable para personal scale.

Test: snapshot del overlay de confirm delete + smoke test que cuenta tasks antes/después.

### Fase M1.5 — Undo stack

Files: `crates/rondo-tui/src/app.rs`, nuevo `crates/rondo-tui/src/undo.rs`.

- `UndoEntry` enum con variants `TaskCreated(id)`, `TaskDeleted(snapshot)`, `StatusChanged{id, prev}`, etc.
- `app.undo_stack: VecDeque<UndoEntry>` capacidad 50.
- Bind `Ctrl+Z` (paridad Go). Aplica inverse op via store.
- Snapshot test: crear → undo → estado igual al inicial.

---

## M2 — Journal completo

**Objetivo:** Escribir entradas, esconder/restaurar notas, navegar fechas con calendario.

### Fase M2.1 — Journal write API

Files: `crates/rondo-core/src/store/sqlite.rs`.

```rust
fn create_or_get_today_note(&self) -> Result<Note>;
fn add_journal_entry(&self, note_id: i64, body: &str) -> Result<i64>;
fn hide_note(&self, note_id: i64) -> Result<()>;
fn restore_note(&self, note_id: i64) -> Result<()>;
fn delete_entry(&self, entry_id: i64) -> Result<()>;
```

### Fase M2.2 — TUI inline entry editor

Files: `crates/rondo-tui/src/components/journal.rs`, `crates/rondo-tui/src/app.rs`.

- En journal day-view, presionar `a` abre un editor de 5 filas (usar `tui-textarea`, ya en deps).
- `Enter` en linea vacía + `Ctrl+S` guarda. `Esc` cancela.
- Markdown se renderiza en preview al guardar.

### Fase M2.3 — Date jumping + hidden filter

- Bind `g` `g` jump top, `G` jump bottom (paridad list).
- `H` / `Shift+H` toggle hide.
- Filter `[h]` HIDDEN en sidebar (estado de la nota, no del task).

### Fase M2.4 — Calendar widget

Files: nuevo `crates/rondo-tui/src/widgets/calendar.rs`, `crates/rondo-tui/src/components/journal.rs`.

- Ratatui `ratatui-widgets` feature `calendar` ya disponible (depende de `time` crate). Activar.
- Renderiza mini-calendar a la derecha del journal con highlight en días que tienen entradas. Click/Enter jump.

---

## M3 — Pomodoro persistente + Focus stats

**Objetivo:** Sesiones pomodoro se persisten en `focus_sessions`. La tab `[4] FOCUS` (hoy stub) muestra streak + heatmap.

### Fase M3.1 — Persist focus sessions

Files: `crates/rondo-core/src/store/sqlite.rs`, `crates/rondo-tui/src/components/pomodoro.rs`.

```rust
fn start_focus_session(&self, task_id: Option<i64>, kind: SessionKind) -> Result<i64>;
fn complete_focus_session(&self, id: i64) -> Result<()>;
fn list_focus_sessions(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Session>>;
fn focus_streak(&self) -> Result<u32>;
```

Pomodoro modal llama `start_focus_session` al `p` press. Al expire del timer, `complete_focus_session`. Bell + status toast "ciclo X de 4 completado".

### Fase M3.2 — Focus page

Files: nuevo `crates/rondo-tui/src/components/focus_page.rs`, `crates/rondo-tui/src/components/root.rs`.

- Nueva `Page::Focus` (al wire de `[4] FOCUS` en sidebar).
- Heatmap 7×N (semanas verticales) de sesiones por día, coloreado por intensidad.
- Streak counter `↟ 5d` (ya en header telemetría — leer de la misma fuente).
- Top tasks by time logged (table sortable).

### Fase M3.3 — Configurable durations

Files: `crates/rondo-tui/src/components/command_palette.rs` (cmd `:focus-config`), `crates/rondo-core/src/config.rs` (nuevo).

- Settings: work=25min, short_break=5min, long_break=15min, cycles_to_long=4.
- Persistir en `~/.todo-app/config.toml`. Crate `toml` ya transitively disponible.

---

## M4 — Recurrence engine

**Objetivo:** Tasks recurrentes se autospawnean (paridad Go `internal/task/recurrence.go`).

### Fase M4.1 — Recurrence calculation

Files: `crates/rondo-core/src/recurrence.rs` (nuevo), tests.

```rust
pub fn next_occurrence(task: &Task, now: NaiveDate) -> Option<NaiveDate>;
pub fn spawn_recurrent_instances(store: &SqliteStore, now: NaiveDate) -> Result<Vec<i64>>;
```

Lógica: si `task.recur_freq != None` AND `task.due_date < now`, crea copia con `due_date = next_occurrence`, marca original como `Done`. Test exhaustivo con calendario edge-cases (last day of month, leap year).

### Fase M4.2 — TUI integration

- `main.rs` al startup llama `spawn_recurrent_instances` (silent, idempotente).
- Status toast: "N recurrentes generados".
- Detail panel muestra ◑ next occurrence en metadata.

---

## M5 — Dependencies + cycle prevention

**Objetivo:** Dependencias bloqueantes manejables desde la TUI.

### Fase M5.1 — Dep mutations

```rust
fn add_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()>;  // detects cycles
fn remove_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()>;
fn dependency_graph(&self) -> Result<DepGraph>;
```

Cycle detection: DFS sobre `task_dependencies`.

### Fase M5.2 — TUI

- Detail · Dependencies section: cursor + `Enter` para abrir target. Bind `a` añade nueva (palette `:dep #N`).
- Delete con `--cascade` confirm modal (paridad Go).
- Sidebar item `[g]` Grafo (nueva nav item, rehabilita el stub): renderiza ASCII graph con `Block`/edges. Optionally usar `tui-tree-widget`.

---

## M6 — Search / fuzzy / sort

**Objetivo:** Búsqueda real + ordenación múltiple.

### Fase M6.1 — Fuzzy implementation

Files: `crates/rondo-tui/src/components/search.rs`, `crates/rondo-tui/src/app.rs`.

- Crate `fuzzy-matcher = "0.3"` o `nucleo = "0.5"`.
- `search_buf` aplica fuzzy match sobre title + tags + description (configurable). Score ordena resultados.
- `Enter` ejecuta — aplica search como filter persistente. `Esc` cancela.
- Highlight matched chars en title con underline accent.

### Fase M6.2 — Sort selector

- Bind `s` abre overlay "Sort by: due | priority | created | title (asc/desc)".
- Persiste en `AppState.sort_order`. Sidebar title agrega "↕ due".
- Paridad Go `F1/F2/F3`.

---

## M7 — Export / import

**Objetivo:** Replicar `rondo export markdown|json|ndjson`.

### Fase M7.1 — Exporters en core

Files: `crates/rondo-core/src/export.rs` (nuevo).

```rust
pub fn to_markdown(tasks: &[Task]) -> String;
pub fn to_json(tasks: &[Task]) -> Result<String>;
pub fn to_ndjson<W: Write>(tasks: &[Task], w: &mut W) -> Result<()>;
```

Test golden: snapshot del output sobre `fixtures/seed.sql` cargado.

### Fase M7.2 — TUI hook + CLI subcommand

- Palette `:export markdown ./tasks.md`.
- CLI `rondo-tui export --format markdown --out tasks.md` (ver M9).

---

## M8 — Plugin loader real

**Objetivo:** Cargar plugins externos. Hoy el `PluginRegistry` solo acepta builtins compilados.

### Fase M8.1 — Decisión de runtime

Subagente original recomendó WASM via `extism`. Confirmar con spike:
- 3 días: cargar un `.wasm` trivial que implementa `Plugin` y registra una página custom.
- Si extism overhead aceptable (cold load < 50ms) → adelante. Si no, fallback a `mlua` (Lua).

Files: `crates/rondo-plugin-host/` (nuevo workspace member), `crates/rondo-tui/src/plugins/mod.rs`.

### Fase M8.2 — Plugin discovery

- `~/.todo-app/plugins/*.wasm` auto-cargados al startup.
- Sidebar item `[5] PLUGINS` (hoy stub) lista cargados + estado + permite enable/disable.
- Capability checks: plugin debe declarar capabilities, host las honra.

### Fase M8.3 — Sample external plugin

- Repo `examples/plugins/calendar-extra/` con un plugin que añade una pagina con vista calendario diferente.
- Build script + instrucciones de instalación.

---

## M9 — CLI subcomandos (paridad Cobra)

**Objetivo:** `rondo-tui add "..."`, `rondo-tui list`, etc. para scripting. Paridad con los 19 subcommands del Go (`internal/cli/`).

### Fase M9.1 — clap subcommand tree

Files: `crates/rondo-tui/src/main.rs` (refactor a `Command` enum).

Subcommands a implementar (prioridad descendente):
1. `add` — crear task (acepta sintaxis quick-add)
2. `list` — JSON o table output
3. `done` — marcar done por id
4. `delete` — borrar por id
5. `journal add` — agregar entry
6. `focus start` — iniciar sesión
7. `stats` — JSON stats
8. `export` — re-uses M7
9. `batch` — NDJSON via stdin (paridad `internal/cli/batch.go`)
10. `recur preview` — ver próximas ocurrencias
11. `dep add/remove` — manejo de dependencies
12. `note add` — task note
13. `tag add/remove`
14. `timelog start/stop`
15. `config get/set`
16. `skill install` — paridad Go (escribir SKILL.md a `~/.claude/skills/`)
17. `completion bash/zsh/fish` — `clap_complete`

### Fase M9.2 — Output format flag

`--json` global flag fuerza output structured. `--quiet` suprime status. Paridad output con Go para scripts ya existentes.

### Fase M9.3 — Tests integración

`crates/rondo-tui/tests/cli/` — spawn binary con `assert_cmd`, verifica salida vs golden. Mantener equivalencia con `internal/cli/cli_integration_test.go` del Go.

---

## M10 — Config + temas

**Objetivo:** Configuración persistente, dark/light mode, custom theme.

### Fase M10.1 — Config schema

Files: `crates/rondo-core/src/config.rs`.

```toml
[ui]
theme = "dark"           # "dark" | "light" | "high-contrast" | "<custom-name>"
sidebar = true
analytics = "auto"       # "auto" | "always" | "never"
animations = true

[pomodoro]
work_min = 25
short_break_min = 5
long_break_min = 15
cycles_to_long = 4
bell = true

[journal]
date_format = "%A, %B %-d, %Y"

[plugins]
enabled = ["builtin.pomodoro", "calendar-extra"]
```

Load at startup. Env var `RONDO_CONFIG` override path. `RONDO_FX=0` sigue funcionando como override binario.

### Fase M10.2 — Theme variants

Files: `crates/rondo-tui/src/theme.rs`.

- `Theme::dark()`, `Theme::light()`, `Theme::high_contrast()`.
- Custom themes from `~/.todo-app/themes/<name>.toml` (7 hex tokens).
- Live reload: `:theme light` sin restart.

### Fase M10.3 — A11y

- Modifier `NO_COLOR` env: drop hue, keep BOLD/UNDERLINED/REVERSED (where allowed).
- `--reduced-motion` flag honra `prefers-reduced-motion` simulation: `RONDO_FX=0` equivalente.

---

## M11 — Sync (long-shot)

**Objetivo:** El "sincronización" panel del dashboard funciona de verdad.

### Fase M11.1 — Decision

Tres caminos posibles, pick one:
- **A — git-backed:** SQLite + auto-commit cada N min. Pull on startup. Conflicts trivial (last-write-wins).
- **B — CRDT** (Automerge / Yjs Rust port): per-task diffs sync. Más complejo.
- **C — Cloud:** WebDAV / S3 backup periódico. Más simple, sin merge real.

Recomendación tentativa: **A** primero (zero infrastructure), CRDT como evolución.

### Fase M11.2 — Implementation por path elegido

Esquema concreto se diseña tras spike de la opción seleccionada.

---

## Cronograma estimado (sin compromisos)

| Hito | Esfuerzo estimado | Bloqueado por |
|---|---|---|
| M1 | 3-5 días nocturnos | — |
| M2 | 2-3 días | M1.1, M1.2 |
| M3 | 2-3 días | M1.1 |
| M4 | 1-2 días | M1.3 |
| M5 | 2 días | M1.3 |
| M6 | 1-2 días | M1.3 |
| M7 | 1 día | M1.3 |
| M8 | 5-7 días | nada (independiente) |
| M9 | 3-4 días | M1.3, M7 |
| M10 | 2 días | — |
| M11 | 5+ días | M1.2 |

**Total bruto:** 27-38 días nocturnos. Estimación honesta optimista: 2x = 8-12 semanas con jornadas de fin de semana.

## Orden recomendado

1. **M1.1 + M1.2** (writes + backup + migrations) — bloqueante para todo lo demás.
2. **M10.1** (config schema) — barato, habilita feature flags.
3. **M1.3 + M1.4 + M1.5** (CRUD + undo) — el grueso del valor de usuario.
4. **M2** (journal write) — completa la segunda página.
5. **M9.1** (CLI subcommands list/add/done) — abre uso scripting.
6. **M6** (search/sort) — pulido productividad.
7. **M3** (pomodoro persistence) + **M4** (recurrence) — paridad Go restante.
8. **M5** (deps) + **M7** (export) — paridad final.
9. **M8** (plugin loader) — feature diferenciadora vs Go.
10. **M10.2/M10.3** (themes/a11y) — pulido.
11. **M11** (sync) — última, gran apuesta.

---

## Notas de implementación cross-cutting

- **Cada fase**: snapshot test antes (estado actual) → cambio → snapshot diff revisado.
- **DB writes**: dentro de transacciones con rollback en cualquier `?` failure.
- **Sin `unsafe`**.
- **Sin global state**: `AppState` reducer, store inyectado.
- **CHANGELOG.md** al root, actualizado en cada hito (paridad Go).
- **AGENTS.md** o **docs/contributing.md** explicando cómo abrir PR sin tocar paridad Go.
- **Tests RW** usan `tempfile::NamedTempFile` con `Drop`, nunca tocan `~/.todo-app/`.
- **`#[deny(clippy::pedantic)]`** opcional a futuro, hoy `-D warnings` cubre.
