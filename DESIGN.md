# RonDO Rust + ratatui — Diseño del puerto

Puerto del TUI Go (Bubbletea/Bubbles/Huh/Cobra/modernc.sqlite) a Rust con ratatui. El esquema SQLite en `~/.todo-app/todo.db` (WAL, FK on, `recur_freq`, `recur_interval`, `metadata` JSON columns ya migradas) se reutiliza tal cual. Convivencia binaria con el Go.

## 1. Crate plan

Workspace con dos crates — separa el dominio del front-end para poder reutilizarlo en un futuro daemon o tests sin pintar terminal.

```
rondo_rust/
├── Cargo.toml                  # [workspace]
├── crates/
│   ├── rondo-core/             # lib: dominio + persistencia (sin ratatui)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/{task.rs, subtask.rs, journal.rs, focus.rs, timelog.rs, recur.rs, deps.rs}
│   │       ├── store/{mod.rs, schema.rs, task_store.rs, journal_store.rs, focus_store.rs, backup.rs}
│   │       ├── config.rs
│   │       ├── export.rs
│   │       └── error.rs        # thiserror
│   └── rondo-tui/              # bin: TUI + CLI dispatch
│       └── src/
│           ├── main.rs         # clap dispatch: subcommand → CLI, sino → TUI
│           ├── app.rs          # struct App, Action enum, loop
│           ├── event.rs        # crossterm event → Action
│           ├── action.rs       # Action enum centralizado
│           ├── ui/{mod.rs, tabs.rs, task_list.rs, task_detail.rs, journal.rs, pomodoro.rs, help.rs, stats.rs, form.rs, theme.rs}
│           ├── components/{mod.rs, component.rs, modal.rs, status_bar.rs}
│           ├── cli/{mod.rs, tasks.rs, journal.rs, focus.rs, export.rs, batch.rs, stats.rs, config.rs}
│           └── tracing_setup.rs
```

`rondo-core` no conoce ratatui ni crossterm. Toda lógica de negocio (deps cycle detection, recurrencia, parse de duración tipo `1h30m`, fusión de metadata) vive ahí y es testeable sin TTY. Espeja exactamente los paquetes `internal/task`, `internal/journal`, `internal/focus`, `internal/config`, `internal/database`, `internal/export` del Go.

## 2. Dependencias

```toml
# crates/rondo-tui/Cargo.toml (extracto)
[dependencies]
rondo-core      = { path = "../rondo-core" }
ratatui         = { version = "0.29", features = ["crossterm", "unstable-widget-ref"] }
crossterm       = "0.28"
tui-textarea    = "0.7"     # editor multilínea para entries del journal
tui-input       = "0.10"    # single-line input para quick-add y forms
clap            = { version = "4.5", features = ["derive", "env"] }
clap_complete   = "4.5"     # equivalente a `cobra completion`
color-eyre      = "0.6"
tracing         = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-appender = "0.2"
crossterm-event-stream = false   # usamos blocking + tick thread

# crates/rondo-core/Cargo.toml
[dependencies]
rusqlite        = { version = "0.32", features = ["bundled", "chrono", "backup"] }
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"
chrono          = { version = "0.4", features = ["serde", "clock"] }
thiserror       = "2"
directories     = "5"       # ~/.todo-app cross-platform
fuzzy-matcher   = "0.3"     # equivalente a sahilm/fuzzy
humantime       = "2"       # parse "1h30m" para timelogs
```

**Por qué cada uno:**

- **ratatui 0.29 + crossterm 0.28**: par estándar, `unstable-widget-ref` permite widgets que mantienen estado sin reasignar buffers (lista, tabla).
- **Sin tokio**. La app Go es síncrona: SQLite local sin red, sin websockets, sin múltiples streams. Un loop `crossterm::event::poll(tick)` en un hilo + canal `mpsc` para eventos compuestos (tick del pomodoro, autosave) basta. tokio añadiría 200kB y un runtime sin beneficio.
- **rusqlite con `bundled`**. Empareja `modernc.org/sqlite` (CGO-free en Go → bundled en Rust = un único binario sin libsqlite del sistema). Descartado **sqlx** (async-first, requiere tokio y queries verificadas en compilación que rompen con migraciones runtime `addColumnIfNotExists`). Descartado **sea-orm** (ORM pesado, oculta los `WHERE IN` batch que el Go ya optimizó). `rusqlite` mapea 1:1 al `database/sql` actual.
- **serde_json para metadata y config**. El config Go es JSON (`~/.todo-app/config.json`); no se cambia el formato. Descartado TOML.
- **chrono, no time**. El Go usa layouts estilo `time.Format` con tokens `2006-01-02`; chrono ofrece `format` con `strftime`-style y es suficiente. Los presets (`pretty`, `iso`, `european`, `us`) se traducen a strftime en `config.rs`.
- **clap (derive)** reemplaza Cobra. `clap_complete` da `bash|zsh|fish|powershell`.
- **color-eyre** para reports bonitos en CLI; `tracing` + `tracing-appender` escribiendo a `~/.todo-app/rondo.log` (debug only) — el Go no lo tiene y conviene.
- **tui-textarea** para edit/add de entradas del journal y description multilínea; **tui-input** para forms (título, tags, due). Ratatui no trae editor.
- **fuzzy-matcher** para el filtro `/` (sahilm/fuzzy en Go).
- **humantime** parsea `1h30m` exactamente como el `timelog add`.

## 3. Patrón arquitectónico

**Component + Action enum, estilo `openapi-tui`/`atac`.** Descartado Elm puro: rondo tiene formularios mutables, focus de panel, modales apilados — pattern matching de `Msg → (Model, Cmd)` en un solo update gigante se vuelve inmanejable (el `model.go` actual ya está partido en 6 ficheros precisamente por eso). Descartado state-mutation raw: pierde el journal de acciones que habilita `Ctrl+Z` undo gratis.

```rust
// crates/rondo-tui/src/action.rs
pub enum Action {
    Tick,                          // 250ms, refresca pomodoro
    Quit,
    Resize(u16, u16),
    SwitchTab(Tab),                // All|Active|Done|Journal
    FocusPanel(PanelId),           // 1|2
    ResizePanels(i8),              // '<' / '>'
    Task(TaskAction),              // Add, Edit{id}, Delete{id, cascade:bool}, CycleStatus{id}, ...
    Subtask(SubtaskAction),
    Journal(JournalAction),
    Focus(FocusAction),            // pomodoro Start/Pause/Skip/Stop
    OpenModal(Modal),              // Help, Stats, Export, PomodoroSettings, ConfirmDelete{...}
    CloseModal,
    Reload(ReloadScope),
    Undo,
    Render,
}

pub struct App {
    pub state: AppState,           // tab, panel_focus, panel_ratio, search_query, sort, tag_filter
    pub tasks: Vec<Task>,
    pub journal_notes: Vec<Note>,
    pub focus_session: Option<Session>,
    pub modal: Option<Modal>,
    pub history: UndoStack,        // Vec<Action::Inverse> para Ctrl+Z
    pub store: Stores,             // task_store, journal_store, focus_store
    pub cfg: Config,
    pub action_tx: mpsc::Sender<Action>,
}

impl App {
    fn handle_event(&self, e: crossterm::event::Event) -> Option<Action> { ... }
    fn update(&mut self, a: Action) -> color_eyre::Result<Vec<Action>> { ... } // returns follow-ups
    fn draw(&mut self, f: &mut Frame) { ... }
}
```

Cada `Component` (TaskList, TaskDetail, JournalPane, PomodoroBar, StatusBar, Modal) implementa:

```rust
trait Component {
    fn handle_key(&mut self, k: KeyEvent, ctx: &AppCtx) -> Option<Action>;
    fn draw(&mut self, f: &mut Frame, area: Rect, ctx: &AppCtx);
}
```

El `App` enruta eventos al componente con foco. Un solo `Action` enum centraliza el log para undo.

## 4. Mockups ASCII

### Task list (Tab All, Panel 1 con foco)

```
┌ RonDO ────────────────────────────────────────────────────────────────────┐
│ [All 7]  Active 4   Done 3   Journal 5             🍅 18:42  ▰▰▰▱  2/4   │
├──────────────────────────────┬────────────────────────────────────────────┤
│ ▸ ○ URG! Fix auth bug   #api │ #2 Fix auth bug                            │
│   ◐ HIGH Refactor store  3d  │ Status: ◐ In Progress   Priority: HIGH     │
│   ○ MED  Write docs     ──   │ Due: Mar 18  (in 2 days)                   │
│   ✓ LOW  Reply PR review     │ Tags: #api #backend                        │
│                              │ ─────────────────────────────────────────  │
│                              │ Subtasks  (2/3)  ▰▰▰▰▰▰▱▱▱                 │
│                              │   ✓ Reproduce locally                      │
│                              │   ✓ Patch JWT validator                    │
│                              │ ▸ ○ Add regression test                    │
│                              │ ─────────────────────────────────────────  │
│                              │ Time:  2h 15m logged                       │
│                              │ Blocks: #5 Deploy                          │
├──────────────────────────────┴────────────────────────────────────────────┤
│ j/k nav  a add  e edit  d del  s status  t subtask  /search  Tab next  ? │
└────────────────────────────────────────────────────────────────────────────┘
```

Keys: `j/k`, `a` add, `e` edit, `d` delete (modal confirm; si bloquea → segundo confirm rojo), `s` cycle status, `t` subtask, `/` fuzzy, `F1/F2/F3` sort, `F4` tag filter, `Tab` cambia pestaña, `1/2` foco, `<>` resize.

### Journal day view (Tab Journal)

```
├──────────────────────────────┬────────────────────────────────────────────┤
│ ▸ Today, May 16         (3)  │ Today, Fri May 16                          │
│   Yesterday, May 15     (5)  │ ─────────────────────────────────────────  │
│   Wed, May 14           (2)  │  08:42  Stand-up done, blocked on infra    │
│   May 10                (1)  │  11:20  Shipped journal export to JSON     │
│   [hidden] Apr 28 ······(0)  │  ▸ 14:05  Reviewed PR #41, requested ...  │
│                              │                                            │
│                              │ ───────────────────── (a) add  (e) edit ── │
└──────────────────────────────┴────────────────────────────────────────────┘
```

`a` abre modal con `tui-textarea` (multilínea, Ctrl+S submit, Esc cancel). `h` oculta nota, `H` toggle ver ocultas.

### Pomodoro overlay (tecla `p`)

```
        ╭────────────  Focus Session  ────────────╮
        │  🍅 Work     #2 Fix auth bug             │
        │                                          │
        │           ▰▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱   62%        │
        │              15:32 / 25:00               │
        │                                          │
        │   Cycle:  ●●●○        Today goal: 2/8   │
        │                                          │
        │   [Space] pause   [s] skip   [q] stop    │
        ╰──────────────────────────────────────────╯
```

`ratatui::widgets::Gauge` para la barra, tick cada 250 ms con thread `mpsc::Sender<Action::Tick>`. Bell terminal (`\x07`) al completar.

## 5. Migración de datos

**Compatibilidad binaria al 100% con el SQLite del Go.** El esquema (`tasks`, `subtasks`, `tags`, `time_logs`, `task_dependencies`, `task_notes`, `journal_notes`, `journal_entries`, `focus_sessions`, columnas `recur_freq`/`recur_interval`/`metadata`) es ANSI SQLite plano sin extensiones. `rusqlite` con `bundled` lo abre sin tocar. Apertura idéntica al Go (`PRAGMA journal_mode=WAL`, `PRAGMA foreign_keys=ON`, single connection vía `Mutex<Connection>` en Rust). Timestamps en UTC ISO-8601 igual que el Go.

**Plan de despliegue:**
1. `rondo` (Rust) usa la misma ruta `~/.todo-app/todo.db`.
2. Antes del primer write, copia el DB a `~/.todo-app/backups/pre-rust-{ts}.db` (extiende `backup.go` 1:1).
3. Se puede alternar binarios Go ↔ Rust en la misma máquina sin migración — útil para validar.
4. Config JSON se reusa idéntico (mismas keys `panel_ratio`, `focus.*`, etc.).

No hace falta export/import. **Riesgo controlado**: la migración `addColumnIfNotExists` del Go se replica en Rust al `open()` para que un DB nuevo en Rust quede idéntico al de Go.

## 6. Riesgos específicos de este puerto

1. **Layouts de tiempo Go vs strftime de chrono**: el Go acepta `"02.01.2006 15:04"` y custom layouts. chrono usa `%d.%m.%Y %H:%M`. Hay que mantener una capa `format_compat` que traduzca los presets Go (`pretty`, `iso`, `european`, `us`) y, para custom layouts persistidos, parsearlos con un converter o documentar break en custom-only. Decisión: traducir presets, romper custom, migrar custom a strftime en primer arranque con warning.

2. **Huh forms → ratatui no tiene equivalente**. Huh hace forms validados con focus rings. Hay que componer a mano sobre `tui-input`/`tui-textarea` un `FormComponent` con campos, navegación Tab, validación inline. Es el subsistema con más código nuevo respecto al Go.

3. **Lipgloss adaptive colors**. Lipgloss detecta light/dark terminal. ratatui no. Detectar con `crossterm::terminal::supports_keyboard_enhancement` + `COLORFGBG` env var; si falla, asumir dark (default rondo). Tema "Dracula" hardcoded como en el Go.

4. **`rondo batch` con stdin JSON**: en Go re-instancia el árbol Cobra por comando para aislar flags. En clap con `derive` los `ArgMatches` son inmutables — implementar `Command::parse_from(args)` por línea es directo, pero hay que asegurarse de no compartir state. No usar `OnceLock` para el `Stores` dentro del batch; pasar `&mut Stores` explícito.

5. **Recurrencia + `auto-spawn next on completion`**: la lógica en `task/recur.go` calcula la próxima fecha respetando DST y meses cortos (`time.AddDate` colapsa Jan 31 + month → Feb 28). chrono `Months::new(n)` se comporta distinto (devuelve None en overflow). Hay que portar tests `recur_test.go` 1:1 y elegir comportamiento explícito (preferir saturar a último día del mes para igualar Go).

6. **Detección de ciclos en `task_dependencies`** (`deps.go`): es DFS sobre la tabla. Trivial de portar pero hay un test con depths > 1000 que mata el stack en Rust release con `RUST_MIN_STACK` default. Usar iteración explícita con `VecDeque`, no recursión.

7. **`modernc.org/sqlite` (CGO-free) vs `rusqlite bundled`**: ambos embeben SQLite, pero versiones distintas. Verificar que `WITHOUT ROWID`, `STRICT`, JSON1 (usado para `metadata`?) están disponibles. rusqlite `bundled` trae JSON1 por defecto desde 0.31, OK.

8. **Sin Bubbletea `tea.Cmd` async**: tareas como "guardar y recargar" en Go retornan `tea.Cmd`. En Rust con loop sync se hacen inline. Riesgo: queries lentas (export grande) bloquean render. Mitigación: ejecutar export en thread dedicado y enviar `Action::ExportDone(path)` por el canal.

---

Cargo del workspace queda servido. Primer commit del puerto: `cargo new --lib crates/rondo-core && cargo new crates/rondo-tui` + el `Cargo.toml` de arriba + portar `schema.rs` con el `CREATE TABLE` literal del Go.
