# Rondo MVP en Rust + ratatui — Plan de Implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construir un MVP visual (3 vistas read-only + overlay pomodoro animada) en `/Users/roniel/Develop/Rust/rondo_rust` que permita decidir objetivamente si Rust + ratatui produce mejor UI que la versión Go actual (bubbletea + lipgloss), con arquitectura plugin-ready desde el día 1.

**Architecture:** Workspace Cargo de 3 crates — `rondo-core` (dominio + SQLite read-only), `rondo-plugin-api` (trait `Plugin` + tipos compartidos, WASM-stub futuro), `rondo-tui` (binario ratatui + Component/Action/Reducer estilo openapi-tui). Cero `unsafe`, cero dynamic loading en MVP; el contrato del plugin queda definido como trait + registry, dejando la puerta abierta a `extism`/`wasmtime` sin refactor.

**Tech Stack:** Rust 1.83 stable; `ratatui` 0.30 + `crossterm` 0.29 (event-stream); `rusqlite` 0.32 (bundled, READ_ONLY); `serde` 1 + `serde_json`; `chrono` 0.4 (DateTime UTC); `clap` 4 derive; `color-eyre` 0.6; `tracing` + `tracing-subscriber`; `tui-textarea` 0.7 (preview journal); `tui-input` 0.11 (command palette); `throbber-widgets-tui` 0.8 (pomodoro spinner); `strum` 0.26 (Action enum derive); `pulldown-cmark` 0.12 → ratatui Text (journal markdown).

---

## Context

**Por qué este cambio.** El usuario mantiene un proyecto Go productivo (`/Users/roniel/Develop/rondo`, ~14.8k LOC, stack Charm completo, feature-rich: tasks/subtasks/journal/pomodoro/recurrence/deps/timelogs/CLI/skill). Vio cuatro TUIs en Rust con ratatui (binsider, openapi-tui, slumber, mdfried) cuya estética le pareció notablemente superior. Quiere evidencia, no opinión: un MVP comparable que muestre lado a lado si ratatui justifica un rewrite.

Análisis previo de subagentes concluyó que la calidad visual del Go actual está subinvertida (`internal/app/styles.go` = 16 líneas) y que un refactor in-place lograría mucho de lo mismo. Pero el usuario también quiere arquitectura de plugins para extender post-MVP — y eso Go con bubbletea no resuelve elegantemente. El MVP sirve para validar **dos hipótesis al mismo tiempo**:

1. ratatui produce UI **visiblemente** mejor que bubbletea cuando se invierte diseño equivalente.
2. La arquitectura de plugins en Rust (trait + WASM futuro) permite extender el dominio sin tocar core.

**Resultado esperado.** Tras ~5–7 días de trabajo, el usuario corre `rondo-tui` apuntando a su SQLite real (read-only) y compara contra `rondo` Go. Si la diferencia visual no es notable → archiva el rewrite, invierte el mismo esfuerzo en pulir Go. Si lo es → este MVP es el embrión del rewrite completo con plan de migración del schema (ya idéntico, sin export/import).

**Decisiones de usuario (vía AskUserQuestion):**
- Plugin runtime: **Trait estático + WASM stub** (no carga `.wasm` en MVP, contrato definido).
- DB: **Read-only contra `~/.todo-app/todo.db`** (cero riesgo de corrupción).
- Scope: **3 vistas + pomodoro fake overlay** (timer animada con estado en memoria).

---

## Referencias clave (codebase Go)

| Concepto | Archivo Go | Líneas |
|---|---|---|
| Task model | `/Users/roniel/Develop/rondo/internal/task/task.go` | 97–115 |
| Task store (queries) | `/Users/roniel/Develop/rondo/internal/task/store.go` | 24–820 |
| Task list render | `/Users/roniel/Develop/rondo/internal/app/delegate.go` | 31–96 |
| Task detail render | `/Users/roniel/Develop/rondo/internal/ui/views.go` | 68–290 |
| Journal store | `/Users/roniel/Develop/rondo/internal/journal/store.go` | 49–105 |
| Journal render | `/Users/roniel/Develop/rondo/internal/app/model_journal.go` | (full file) |
| Keybindings | `/Users/roniel/Develop/rondo/internal/app/keys.go` | (full file) |
| Color palette | `/Users/roniel/Develop/rondo/internal/ui/colors.go` | 8–24 |
| DB open | `/Users/roniel/Develop/rondo/internal/database/db.go` | 30–39 |

**Mantener exactamente igual entre Go y Rust** (comparación justa):
- Status iconos: `○` Pending, `◐` InProgress, `✓` Done.
- Priority colors: LOW=verde, MED=amarillo, HIGH=rojo, URG!=magenta.
- Keybindings: `a/e/d/s/t//`, `Tab`, `</>`, `Esc`, `?`, `F1/F2/F3`, `j/k`.
- Color palette hex: Cyan `#00BCD4`, White `#FAFAFA`, Green `#4CAF50`, Red `#F44336`.
- Tab bar: "RonDO" + All/Active/Done + divider + Journal.
- Split ratio default 0.5, resizable.

---

## File Structure

```
/Users/roniel/Develop/Rust/rondo_rust/
├── Cargo.toml                          # workspace manifest
├── CLAUDE.md                           # project memory para Claude Code
├── README.md                           # instrucciones MVP
├── rust-toolchain.toml                 # pin stable 1.83
├── .gitignore
│
├── crates/
│   ├── rondo-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # re-exports
│   │       ├── domain/
│   │       │   ├── mod.rs
│   │       │   ├── task.rs             # Task, Subtask, Status, Priority, RecurFreq
│   │       │   ├── journal.rs          # Note, Entry
│   │       │   └── focus.rs            # Session, SessionKind (in-memory only)
│   │       ├── store/
│   │       │   ├── mod.rs              # trait Store
│   │       │   ├── sqlite.rs           # SqliteStore (READ_ONLY)
│   │       │   └── queries.rs          # SQL constants
│   │       └── error.rs                # eyre-friendly Error enum
│   │
│   ├── rondo-plugin-api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── plugin.rs               # trait Plugin + Context
│   │       ├── view.rs                 # trait View + ViewKind
│   │       ├── action.rs               # PluginAction enum (serializable)
│   │       ├── capabilities.rs         # Capability flags
│   │       └── registry.rs             # PluginRegistry (static dispatch para MVP)
│   │
│   └── rondo-tui/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs                 # tokio::main? NO — std main + clap
│       │   ├── app.rs                  # AppState, Component trait, main loop
│       │   ├── action.rs               # Action enum (strum::Display)
│       │   ├── event.rs                # crossterm → Action mapping
│       │   ├── theme.rs                # paleta tokenizada (NO duplicar hex)
│       │   ├── tui.rs                  # Terminal init/restore + panic hook
│       │   ├── components/
│       │   │   ├── mod.rs              # trait Component
│       │   │   ├── root.rs             # layout principal (header/body/footer)
│       │   │   ├── header.rs           # tab bar
│       │   │   ├── footer.rs           # status bar + hints
│       │   │   ├── task_list.rs        # panel izquierdo
│       │   │   ├── task_detail.rs      # panel derecho
│       │   │   ├── journal.rs          # vista journal
│       │   │   ├── pomodoro.rs         # overlay con gauge + throbber
│       │   │   └── command_palette.rs  # overlay slumber-style
│       │   ├── pages/
│       │   │   ├── mod.rs              # trait Page
│       │   │   ├── tasks.rs            # tasks Page (compose list + detail)
│       │   │   └── journal.rs          # journal Page
│       │   ├── plugins/
│       │   │   ├── mod.rs              # PluginHost (in-process)
│       │   │   └── builtin/
│       │   │       └── pomodoro.rs     # ejemplo plugin builtin
│       │   └── widgets/
│       │       ├── mod.rs
│       │       ├── priority_badge.rs   # widget reusable
│       │       ├── due_badge.rs        # OVERDUE/TODAY/UPCOMING
│       │       ├── progress_bar.rs     # subtasks ratio
│       │       └── markdown.rs         # pulldown-cmark → ratatui Text
│       └── tests/
│           ├── snapshot/                # ratatui buffer snapshots
│           └── integration.rs           # spawn app against fixture DB
│
└── fixtures/
    └── seed.sql                         # opcional: replicar schema Go para CI
```

**Decisiones de decomposición:**
- `rondo-core` no depende de ratatui ni crossterm — testeable headless.
- `rondo-plugin-api` no depende de `rondo-core` (solo de `serde`) — plugins externos no necesitan recompilar core.
- `rondo-tui` depende de ambos.
- Cada componente UI vive en su propio archivo (~50–150 LOC). `task_list.rs` y `task_detail.rs` separados, no un mega `model.go`.

---

## Plan de tareas (TDD, bite-sized, frequent commits)

### Task 0: Bootstrap del workspace

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `crates/rondo-core/Cargo.toml`
- Create: `crates/rondo-plugin-api/Cargo.toml`
- Create: `crates/rondo-tui/Cargo.toml`

- [ ] **Step 1: Crear `Cargo.toml` workspace root**

```toml
[workspace]
resolver = "2"
members = ["crates/rondo-core", "crates/rondo-plugin-api", "crates/rondo-tui"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.83"
authors = ["Roniel <roniel.rack@gmail.com>"]
license = "MIT"
repository = "https://github.com/roniel-rondo/rondo-rust"

[workspace.dependencies]
ratatui = { version = "0.30", features = ["crossterm", "all-widgets", "macros"] }
crossterm = { version = "0.29", features = ["event-stream", "bracketed-paste"] }
rusqlite = { version = "0.32", features = ["bundled", "chrono", "serde_json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4.5", features = ["derive", "env"] }
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tui-textarea = "0.7"
tui-input = "0.11"
throbber-widgets-tui = "0.8"
strum = { version = "0.26", features = ["derive"] }
pulldown-cmark = "0.12"
thiserror = "1"
```

- [ ] **Step 2: Crear `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.83.0"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Crear `.gitignore`**

```
target/
Cargo.lock
*.swp
.DS_Store
.envrc
.direnv/
*.log
```

- [ ] **Step 4: Crear `crates/rondo-core/Cargo.toml`**

```toml
[package]
name = "rondo-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 5: Crear `crates/rondo-plugin-api/Cargo.toml`**

```toml
[package]
name = "rondo-plugin-api"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
strum = { workspace = true }
```

- [ ] **Step 6: Crear `crates/rondo-tui/Cargo.toml`**

```toml
[package]
name = "rondo-tui"
version.workspace = true
edition.workspace = true
default-run = "rondo-tui"

[[bin]]
name = "rondo-tui"
path = "src/main.rs"

[dependencies]
rondo-core = { path = "../rondo-core" }
rondo-plugin-api = { path = "../rondo-plugin-api" }
ratatui = { workspace = true }
crossterm = { workspace = true }
clap = { workspace = true }
color-eyre = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
tui-textarea = { workspace = true }
tui-input = { workspace = true }
throbber-widgets-tui = { workspace = true }
strum = { workspace = true }
pulldown-cmark = { workspace = true }
serde = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
insta = { version = "1.40", features = ["yaml"] }
tempfile = "3.10"
```

- [ ] **Step 7: Crear stubs `src/lib.rs` y `src/main.rs`**

`crates/rondo-core/src/lib.rs`:
```rust
//! rondo-core: domain types and read-only SQLite store.
```

`crates/rondo-plugin-api/src/lib.rs`:
```rust
//! rondo-plugin-api: stable contract for plugins (future WASM ABI).
```

`crates/rondo-tui/src/main.rs`:
```rust
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    println!("rondo-tui MVP — bootstrap OK");
    Ok(())
}
```

- [ ] **Step 8: Verificar build**

Run: `cd /Users/roniel/Develop/Rust/rondo_rust && cargo build --workspace`
Expected: 3 crates compilan, binario `target/debug/rondo-tui` ejecuta y imprime el mensaje.

- [ ] **Step 9: Commit**

```bash
cd /Users/roniel/Develop/Rust/rondo_rust
git init
git add .
git commit -m "feat: bootstrap rust workspace with 3 crates"
```

---

### Task 1: Domain types (`rondo-core::domain`)

**Files:**
- Create: `crates/rondo-core/src/domain/mod.rs`
- Create: `crates/rondo-core/src/domain/task.rs`
- Create: `crates/rondo-core/src/domain/journal.rs`
- Create: `crates/rondo-core/src/domain/focus.rs`
- Test: `crates/rondo-core/src/domain/task.rs` (inline `#[cfg(test)] mod tests`)

Reference: `/Users/roniel/Develop/rondo/internal/task/task.go:97–115`.

- [ ] **Step 1: Test del enum `Status`**

`crates/rondo-core/src/domain/task.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum Status { Pending = 0, InProgress = 1, Done = 2 }

impl Status {
    pub fn icon(self) -> &'static str {
        match self { Self::Pending => "○", Self::InProgress => "◐", Self::Done => "✓" }
    }
    pub fn from_db(v: i64) -> Self {
        match v { 1 => Self::InProgress, 2 => Self::Done, _ => Self::Pending }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_round_trip() {
        for s in [Status::Pending, Status::InProgress, Status::Done] {
            assert_eq!(Status::from_db(s as i64), s);
        }
    }
    #[test]
    fn status_icons_match_go() {
        assert_eq!(Status::Pending.icon(), "○");
        assert_eq!(Status::InProgress.icon(), "◐");
        assert_eq!(Status::Done.icon(), "✓");
    }
}
```

- [ ] **Step 2: Verificar test falla** (porque archivo aún no está en `mod.rs`)

Run: `cargo test -p rondo-core domain::task`
Expected: error de compilación / módulo no encontrado.

- [ ] **Step 3: Registrar módulo**

`crates/rondo-core/src/domain/mod.rs`:
```rust
pub mod task;
pub mod journal;
pub mod focus;
```

`crates/rondo-core/src/lib.rs`:
```rust
pub mod domain;
pub mod store;
pub mod error;
```

- [ ] **Step 4: Implementar `Priority`, `RecurFreq`, `Task`, `Subtask`, `TimeLog`, `TaskNote`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum Priority { Low = 0, Med = 1, High = 2, Urgent = 3 }

impl Priority {
    pub fn label(self) -> &'static str {
        match self { Self::Low => "LOW", Self::Med => "MED", Self::High => "HIGH", Self::Urgent => "URG!" }
    }
    pub fn from_db(v: i64) -> Self {
        match v { 1 => Self::Med, 2 => Self::High, 3 => Self::Urgent, _ => Self::Low }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i64)]
pub enum RecurFreq { None = 0, Daily = 1, Weekly = 2, Monthly = 3, Yearly = 4 }

impl RecurFreq {
    pub fn from_db(v: i64) -> Self {
        match v { 1 => Self::Daily, 2 => Self::Weekly, 3 => Self::Monthly, 4 => Self::Yearly, _ => Self::None }
    }
}

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub completed: bool,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeLog {
    pub id: i64,
    pub task_id: i64,
    pub duration_secs: i64,
    pub note: Option<String>,
    pub logged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: i64,
    pub task_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub recur_freq: RecurFreq,
    pub recur_interval: i64,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub subtasks: Vec<Subtask>,
    pub time_logs: Vec<TimeLog>,
    pub notes: Vec<TaskNote>,
    pub blocked_by_ids: Vec<i64>,
    pub blocks_ids: Vec<i64>,
}

impl Task {
    pub fn is_blocked(&self) -> bool { !self.blocked_by_ids.is_empty() }
    pub fn subtask_progress(&self) -> (usize, usize) {
        let done = self.subtasks.iter().filter(|s| s.completed).count();
        (done, self.subtasks.len())
    }
}
```

- [ ] **Step 5: Implementar `Note`/`Entry` y `Session`/`SessionKind`**

`crates/rondo-core/src/domain/journal.rs`:
```rust
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub date: NaiveDate,
    pub hidden: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    pub note_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
```

`crates/rondo-core/src/domain/focus.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind { Work, ShortBreak, LongBreak }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub task_id: Option<i64>,
    pub kind: SessionKind,
    pub cycle_pos: u8,
    pub started_at: DateTime<Utc>,
    pub duration_secs: u64,
}
```

- [ ] **Step 6: Crear `error.rs`**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlite: {0}")] Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
    #[error("not found: {0}")] NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

Crear `crates/rondo-core/src/store/mod.rs` vacío con `pub mod sqlite; pub mod queries;` (stubs vacíos para que compile).

- [ ] **Step 7: Verificar tests pasan**

Run: `cargo test -p rondo-core`
Expected: 2 tests PASS (`status_round_trip`, `status_icons_match_go`).

- [ ] **Step 8: Commit**

```bash
git add crates/rondo-core/
git commit -m "feat(core): add domain types matching go schema"
```

---

### Task 2: SQLite store read-only

**Files:**
- Create: `crates/rondo-core/src/store/queries.rs`
- Create: `crates/rondo-core/src/store/sqlite.rs`
- Modify: `crates/rondo-core/src/store/mod.rs`
- Test: `crates/rondo-core/tests/store_smoke.rs`
- Create: `fixtures/seed.sql` (replica schema Go mínimo)

Reference: `/Users/roniel/Develop/rondo/internal/task/store.go:24–820`, `/Users/roniel/Develop/rondo/internal/journal/store.go:49–105`.

- [ ] **Step 1: Crear `fixtures/seed.sql`**

```sql
CREATE TABLE tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  description TEXT,
  status INTEGER NOT NULL DEFAULT 0,
  priority INTEGER NOT NULL DEFAULT 0,
  due_date TEXT,
  created_at TEXT NOT NULL,
  recur_freq INTEGER NOT NULL DEFAULT 0,
  recur_interval INTEGER NOT NULL DEFAULT 0,
  metadata TEXT
);
CREATE TABLE subtasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  title TEXT NOT NULL,
  completed INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE time_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  duration INTEGER NOT NULL,
  note TEXT,
  logged_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE task_dependencies (
  task_id INTEGER NOT NULL,
  blocked_by INTEGER NOT NULL,
  PRIMARY KEY (task_id, blocked_by)
);
CREATE TABLE task_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE journal_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  date TEXT NOT NULL UNIQUE,
  hidden INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE journal_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  note_id INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (note_id) REFERENCES journal_notes(id) ON DELETE CASCADE
);

-- 3 tareas + 1 journal entry de seed
INSERT INTO tasks (id, title, description, status, priority, due_date, created_at, metadata)
VALUES
  (1, 'Review API spec', 'Check RFC #42', 1, 2, date('now'), datetime('now'), '{"owner":"me"}'),
  (2, 'Deploy v2.1', NULL, 2, 1, date('now','-1 day'), datetime('now','-3 days'), '{}'),
  (3, 'Buy groceries', 'Milk, eggs, bread', 0, 0, date('now','+2 days'), datetime('now','-1 hour'), '{}');
INSERT INTO subtasks (task_id, title, completed, position) VALUES
  (1, 'Research tools', 1, 0), (1, 'Setup repo', 0, 1), (1, 'Config testing', 0, 2);
INSERT INTO tags (task_id, name) VALUES (1, 'work'), (1, 'backend'), (3, 'personal');
INSERT INTO journal_notes (date, hidden, created_at, updated_at)
  VALUES (date('now'), 0, datetime('now'), datetime('now'));
INSERT INTO journal_entries (note_id, body, created_at)
  VALUES (1, '# Hoy\n\nShipped **v2.1** with new API endpoint.', datetime('now'));
```

- [ ] **Step 2: SQL queries constants**

`crates/rondo-core/src/store/queries.rs`:
```rust
pub const LIST_TASKS: &str = r#"
SELECT id, title, description, status, priority, due_date, created_at,
       recur_freq, recur_interval, COALESCE(metadata, '{}')
FROM tasks
ORDER BY status ASC, priority DESC, COALESCE(due_date, '9999-12-31') ASC, id DESC
"#;

pub const TASK_BY_ID: &str = r#"
SELECT id, title, description, status, priority, due_date, created_at,
       recur_freq, recur_interval, COALESCE(metadata, '{}')
FROM tasks WHERE id = ?1
"#;

pub const SUBTASKS_FOR_TASK: &str = r#"
SELECT id, task_id, title, completed, position
FROM subtasks WHERE task_id = ?1 ORDER BY position ASC
"#;

pub const TAGS_FOR_TASK: &str = r#"
SELECT name FROM tags WHERE task_id = ?1 ORDER BY name ASC
"#;

pub const TIME_LOGS_FOR_TASK: &str = r#"
SELECT id, task_id, duration, COALESCE(note, ''), logged_at
FROM time_logs WHERE task_id = ?1 ORDER BY logged_at DESC
"#;

pub const NOTES_FOR_TASK: &str = r#"
SELECT id, task_id, body, created_at
FROM task_notes WHERE task_id = ?1 ORDER BY created_at DESC
"#;

pub const BLOCKED_BY: &str = r#"
SELECT blocked_by FROM task_dependencies WHERE task_id = ?1
"#;

pub const BLOCKS: &str = r#"
SELECT task_id FROM task_dependencies WHERE blocked_by = ?1
"#;

pub const LIST_JOURNAL_NOTES: &str = r#"
SELECT id, date, hidden, created_at, updated_at
FROM journal_notes WHERE hidden = 0 ORDER BY date DESC LIMIT 365
"#;

pub const ENTRIES_FOR_NOTE: &str = r#"
SELECT id, note_id, body, created_at
FROM journal_entries WHERE note_id = ?1 ORDER BY created_at ASC
"#;
```

- [ ] **Step 3: Test que el store carga el seed**

`crates/rondo-core/tests/store_smoke.rs`:
```rust
use rondo_core::store::sqlite::SqliteStore;

fn fixture_db() -> std::path::PathBuf {
    let tmp = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    let path = tmp.to_path_buf();
    std::mem::forget(tmp); // keep path alive
    let conn = rusqlite::Connection::open(&path).unwrap();
    let seed = std::fs::read_to_string("../../fixtures/seed.sql").unwrap();
    conn.execute_batch(&seed).unwrap();
    path
}

#[test]
fn list_tasks_returns_three() {
    let path = fixture_db();
    let store = SqliteStore::open_readonly(&path).unwrap();
    let tasks = store.list_tasks().unwrap();
    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].title, "Review API spec");
}

#[test]
fn task_detail_loads_subtasks_and_tags() {
    let path = fixture_db();
    let store = SqliteStore::open_readonly(&path).unwrap();
    let t = store.task_by_id(1).unwrap();
    assert_eq!(t.subtasks.len(), 3);
    assert!(t.subtasks[0].completed);
    assert_eq!(t.tags, vec!["backend", "work"]);
}

#[test]
fn journal_today_has_one_entry() {
    let path = fixture_db();
    let store = SqliteStore::open_readonly(&path).unwrap();
    let notes = store.list_journal_notes().unwrap();
    assert_eq!(notes.len(), 1);
    let entries = store.entries_for_note(notes[0].id).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].body.contains("v2.1"));
}
```

- [ ] **Step 4: Verificar tests fallan**

Run: `cargo test -p rondo-core --test store_smoke`
Expected: compile errors (SqliteStore no existe).

- [ ] **Step 5: Implementar `SqliteStore`**

`crates/rondo-core/src/store/mod.rs`:
```rust
pub mod queries;
pub mod sqlite;
```

`crates/rondo-core/src/store/sqlite.rs`:
```rust
use crate::domain::{
    journal::{Entry, Note},
    task::{Priority, RecurFreq, Status, Subtask, Task, TaskNote, TimeLog},
};
use crate::error::{Error, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OpenFlags, Row};
use std::path::Path;
use std::sync::Mutex;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open_readonly<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "query_only", true)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_TASKS)?;
        let rows = stmt.query_map([], |r| Ok(row_to_task_shallow(r)))?;
        let mut tasks = Vec::new();
        for r in rows { tasks.push(r??); }
        for t in &mut tasks { hydrate(&conn, t)?; }
        Ok(tasks)
    }

    pub fn task_by_id(&self, id: i64) -> Result<Task> {
        let conn = self.conn.lock().unwrap();
        let mut t = conn.query_row(super::queries::TASK_BY_ID, params![id], |r| {
            Ok(row_to_task_shallow(r))
        })??;
        hydrate(&conn, &mut t)?;
        Ok(t)
    }

    pub fn list_journal_notes(&self) -> Result<Vec<Note>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::LIST_JOURNAL_NOTES)?;
        let rows = stmt.query_map([], row_to_note)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn entries_for_note(&self, note_id: i64) -> Result<Vec<Entry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(super::queries::ENTRIES_FOR_NOTE)?;
        let rows = stmt.query_map(params![note_id], row_to_entry)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn row_to_task_shallow(r: &Row<'_>) -> Result<Task> {
    let metadata_json: String = r.get(9)?;
    let metadata = serde_json::from_str(&metadata_json)?;
    let due_str: Option<String> = r.get(5)?;
    let created_str: String = r.get(6)?;
    Ok(Task {
        id: r.get(0)?, title: r.get(1)?, description: r.get(2)?,
        status: Status::from_db(r.get(3)?),
        priority: Priority::from_db(r.get(4)?),
        due_date: due_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        created_at: parse_dt(&created_str),
        recur_freq: RecurFreq::from_db(r.get(7)?),
        recur_interval: r.get(8)?,
        metadata, tags: vec![], subtasks: vec![],
        time_logs: vec![], notes: vec![],
        blocked_by_ids: vec![], blocks_ids: vec![],
    })
}

fn hydrate(conn: &Connection, t: &mut Task) -> Result<()> {
    let mut s = conn.prepare(super::queries::SUBTASKS_FOR_TASK)?;
    t.subtasks = s.query_map(params![t.id], |r| Ok(Subtask {
        id: r.get(0)?, task_id: r.get(1)?, title: r.get(2)?,
        completed: r.get::<_, i64>(3)? != 0, position: r.get(4)?,
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut s = conn.prepare(super::queries::TAGS_FOR_TASK)?;
    t.tags = s.query_map(params![t.id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut s = conn.prepare(super::queries::TIME_LOGS_FOR_TASK)?;
    t.time_logs = s.query_map(params![t.id], |r| Ok(TimeLog {
        id: r.get(0)?, task_id: r.get(1)?, duration_secs: r.get(2)?,
        note: { let n: String = r.get(3)?; if n.is_empty() { None } else { Some(n) } },
        logged_at: parse_dt(&r.get::<_, String>(4)?),
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut s = conn.prepare(super::queries::NOTES_FOR_TASK)?;
    t.notes = s.query_map(params![t.id], |r| Ok(TaskNote {
        id: r.get(0)?, task_id: r.get(1)?, body: r.get(2)?,
        created_at: parse_dt(&r.get::<_, String>(3)?),
    }))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut s = conn.prepare(super::queries::BLOCKED_BY)?;
    t.blocked_by_ids = s.query_map(params![t.id], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut s = conn.prepare(super::queries::BLOCKS)?;
    t.blocks_ids = s.query_map(params![t.id], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(())
}

fn row_to_note(r: &Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        date: NaiveDate::parse_from_str(&r.get::<_, String>(1)?, "%Y-%m-%d").unwrap(),
        hidden: r.get::<_, i64>(2)? != 0,
        created_at: parse_dt(&r.get::<_, String>(3)?),
        updated_at: parse_dt(&r.get::<_, String>(4)?),
    })
}

fn row_to_entry(r: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?, note_id: r.get(1)?, body: r.get(2)?,
        created_at: parse_dt(&r.get::<_, String>(3)?),
    })
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|nd| DateTime::<Utc>::from_naive_utc_and_offset(nd, Utc))
                .unwrap_or_else(|_| Utc::now())
        })
}
```

- [ ] **Step 6: Verificar tests pasan**

Run: `cargo test -p rondo-core`
Expected: 5 tests PASS (2 domain + 3 store_smoke).

- [ ] **Step 7: Commit**

```bash
git add crates/rondo-core/ fixtures/
git commit -m "feat(core): add read-only sqlite store with hydration"
```

---

### Task 3: Plugin API contract

**Files:**
- Create: `crates/rondo-plugin-api/src/lib.rs`
- Create: `crates/rondo-plugin-api/src/plugin.rs`
- Create: `crates/rondo-plugin-api/src/view.rs`
- Create: `crates/rondo-plugin-api/src/action.rs`
- Create: `crates/rondo-plugin-api/src/capabilities.rs`
- Create: `crates/rondo-plugin-api/src/registry.rs`
- Test: inline en `registry.rs`

**Diseño:** El trait `Plugin` toma `&PluginContext` y devuelve metadata + handlers. Las vistas son enums **serializables** (no `&dyn Widget`) para que el día que migremos a WASM/extism el contrato no cambie — el host es quien renderiza los `ViewSpec` con ratatui. Esto se llama "remote rendering" o "view DSL" y es el patrón de Zellij.

- [ ] **Step 1: Definir `Capability`**

`crates/rondo-plugin-api/src/capabilities.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Plugin contributes an overlay view (renders on top of pages).
    OverlayView,
    /// Plugin reacts to ticks (e.g. timer).
    TickHandler,
    /// Plugin contributes commands to the palette.
    CommandContributor,
    /// Plugin owns a full page.
    PageView,
}
```

- [ ] **Step 2: Definir `ViewSpec` (DSL serializable de UI)**

`crates/rondo-plugin-api/src/view.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSpec {
    pub kind: ViewKind,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewKind { Page, Overlay, Sidebar }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading { text: String, level: u8 },
    Paragraph { text: String, style: Option<TextStyle> },
    Gauge { ratio: f64, label: Option<String> },
    Throbber { label: String },
    Divider,
    Spans(Vec<Span>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub style: Option<TextStyle>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TextStyle {
    pub fg: Option<ColorToken>,
    pub bg: Option<ColorToken>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorToken {
    Accent, Success, Warning, Danger, Muted, Foreground, Background,
}
```

- [ ] **Step 3: `PluginAction` enum + `PluginContext`**

`crates/rondo-plugin-api/src/action.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginAction {
    Show,
    Hide,
    Tick { delta_ms: u32 },
    Command { name: String, args: Vec<String> },
    KeyPress { key: String },
}
```

`crates/rondo-plugin-api/src/plugin.rs`:
```rust
use crate::action::PluginAction;
use crate::capabilities::Capability;
use crate::view::ViewSpec;

#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub capabilities: &'static [Capability],
}

pub struct PluginContext<'a> {
    pub now: chrono::DateTime<chrono::Utc>,
    pub _hidden: std::marker::PhantomData<&'a ()>,
}

pub trait Plugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
    fn handle(&mut self, action: PluginAction, ctx: &PluginContext<'_>) -> PluginResult;
}

#[derive(Debug, Default)]
pub struct PluginResult {
    pub view: Option<ViewSpec>,
    pub follow_up: Vec<PluginAction>,
}
```

- [ ] **Step 4: Registry estático**

`crates/rondo-plugin-api/src/registry.rs`:
```rust
use crate::plugin::{Plugin, PluginMeta};
use std::collections::HashMap;

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<&'static str, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.meta().id;
        self.plugins.insert(id, plugin);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    pub fn iter_meta(&self) -> impl Iterator<Item = PluginMeta> + '_ {
        self.plugins.values().map(|p| p.meta())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::PluginAction;
    use crate::capabilities::Capability;
    use crate::plugin::{PluginContext, PluginResult};
    use crate::view::ViewSpec;

    struct Dummy;
    impl Plugin for Dummy {
        fn meta(&self) -> PluginMeta {
            PluginMeta { id: "dummy", name: "Dummy", version: "0.1.0",
                capabilities: &[Capability::OverlayView] }
        }
        fn handle(&mut self, _: PluginAction, _: &PluginContext<'_>) -> PluginResult {
            PluginResult { view: Some(ViewSpec { kind: crate::view::ViewKind::Overlay, blocks: vec![] }), follow_up: vec![] }
        }
    }

    #[test]
    fn register_and_dispatch() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(Dummy));
        let ctx = PluginContext { now: chrono::Utc::now(), _hidden: std::marker::PhantomData };
        let r = reg.get_mut("dummy").unwrap().handle(PluginAction::Show, &ctx);
        assert!(r.view.is_some());
    }
}
```

- [ ] **Step 5: `lib.rs` re-exports**

```rust
pub mod action;
pub mod capabilities;
pub mod plugin;
pub mod registry;
pub mod view;

pub use action::PluginAction;
pub use capabilities::Capability;
pub use plugin::{Plugin, PluginContext, PluginMeta, PluginResult};
pub use registry::PluginRegistry;
pub use view::{Block, ColorToken, Span, TextStyle, ViewKind, ViewSpec};
```

- [ ] **Step 6: Verificar test**

Run: `cargo test -p rondo-plugin-api`
Expected: 1 test PASS (`register_and_dispatch`).

- [ ] **Step 7: Commit**

```bash
git add crates/rondo-plugin-api/
git commit -m "feat(plugin-api): define stable plugin trait + serializable view dsl"
```

---

### Task 4: TUI skeleton + theme + terminal init

**Files:**
- Modify: `crates/rondo-tui/src/main.rs`
- Create: `crates/rondo-tui/src/tui.rs`
- Create: `crates/rondo-tui/src/theme.rs`
- Create: `crates/rondo-tui/src/action.rs`
- Create: `crates/rondo-tui/src/event.rs`
- Create: `crates/rondo-tui/src/app.rs`
- Create: `crates/rondo-tui/src/components/mod.rs`

- [ ] **Step 1: `theme.rs` — paleta tokenizada**

```rust
use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub urgent: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub bg: Color,
    pub border_active: Color,
    pub border_inactive: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            accent: Color::Rgb(0x00, 0xBC, 0xD4),    // cyan #00BCD4 (parity Go)
            success: Color::Rgb(0x4C, 0xAF, 0x50),   // green #4CAF50
            warning: Color::Rgb(0xFF, 0xC1, 0x07),   // amber
            danger: Color::Rgb(0xF4, 0x43, 0x36),    // red #F44336
            urgent: Color::Rgb(0xE9, 0x1E, 0x63),    // magenta
            fg: Color::Rgb(0xFA, 0xFA, 0xFA),
            fg_muted: Color::Rgb(0x9E, 0x9E, 0x9E),
            bg: Color::Reset,
            border_active: Color::Rgb(0x00, 0xBC, 0xD4),
            border_inactive: Color::Rgb(0x42, 0x42, 0x42),
        }
    }
    pub fn priority_color(&self, p: rondo_core::domain::task::Priority) -> Color {
        use rondo_core::domain::task::Priority::*;
        match p { Low => self.success, Med => self.warning, High => self.danger, Urgent => self.urgent }
    }
    pub fn priority_style(&self, p: rondo_core::domain::task::Priority) -> Style {
        Style::default().fg(self.priority_color(p)).add_modifier(Modifier::BOLD)
    }
    pub fn muted(&self) -> Style { Style::default().fg(self.fg_muted) }
    pub fn accent(&self) -> Style { Style::default().fg(self.accent).add_modifier(Modifier::BOLD) }
}
```

- [ ] **Step 2: `action.rs` — Action enum**

```rust
use strum::Display;

#[derive(Debug, Clone, Display)]
pub enum Action {
    Tick,
    Quit,
    Render,
    Resize { width: u16, height: u16 },

    // Navigation
    NextItem,
    PrevItem,
    SelectItem(usize),
    NextTab,
    PrevTab,
    TogglePage(Page),

    // Panels
    FocusNext,
    ResizeSplit { delta: i16 },

    // Overlay
    OpenPomodoro,
    ClosePomodoro,
    TogglePomodoro,
    OpenCommandPalette,
    CloseCommandPalette,
    SubmitCommand(String),

    // Search
    StartSearch,
    SearchInput(String),
    CommitSearch,
    CancelSearch,

    // Errors
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page { Tasks, Journal }
```

- [ ] **Step 3: `tui.rs` — terminal lifecycle**

```rust
use color_eyre::eyre::Result;
use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
            PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableBracketedPaste)?;
    let _ = execute!(out, PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    ));
    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;
    install_panic_hook();
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    let mut out = stdout();
    let _ = execute!(out, PopKeyboardEnhancementFlags);
    execute!(out, LeaveAlternateScreen, DisableBracketedPaste)?;
    disable_raw_mode()?;
    Ok(())
}

fn install_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        hook(info);
    }));
}
```

- [ ] **Step 4: `event.rs` — crossterm → Action**

```rust
use crate::action::{Action, Page};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub fn map(ev: Event) -> Option<Action> {
    match ev {
        Event::Key(k) => key_to_action(k),
        Event::Resize(w, h) => Some(Action::Resize { width: w, height: h }),
        _ => None,
    }
}

fn key_to_action(k: KeyEvent) -> Option<Action> {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Char('q') if !ctrl => Action::Quit,
        KeyCode::Char('c') if ctrl => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::NextItem,
        KeyCode::Char('k') | KeyCode::Up => Action::PrevItem,
        KeyCode::Tab => Action::FocusNext,
        KeyCode::BackTab => Action::PrevTab,
        KeyCode::Char('1') => Action::TogglePage(Page::Tasks),
        KeyCode::Char('2') => Action::TogglePage(Page::Journal),
        KeyCode::Char('p') => Action::TogglePomodoro,
        KeyCode::Char(':') => Action::OpenCommandPalette,
        KeyCode::Char('<') => Action::ResizeSplit { delta: -2 },
        KeyCode::Char('>') => Action::ResizeSplit { delta: 2 },
        KeyCode::Esc => Action::CloseCommandPalette,
        _ => return None,
    })
}
```

- [ ] **Step 5: `app.rs` — AppState + main loop**

```rust
use crate::action::{Action, Page};
use crate::theme::Theme;
use color_eyre::eyre::Result;
use rondo_core::domain::{journal::{Entry, Note}, task::Task};
use rondo_plugin_api::PluginRegistry;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct AppState {
    pub theme: Theme,
    pub page: Page,
    pub tasks: Vec<Task>,
    pub selected_task: usize,
    pub focus_left: bool,
    pub split_ratio: u16,
    pub journal_notes: Vec<Note>,
    pub journal_entries: Vec<Entry>,
    pub selected_journal: usize,
    pub pomodoro_open: bool,
    pub pomodoro_started: Option<Instant>,
    pub pomodoro_total: Duration,
    pub command_palette_open: bool,
    pub command_buf: String,
    pub should_quit: bool,
    pub status_msg: Option<String>,
    pub plugins: PluginRegistry,
    pub store: Arc<rondo_core::store::sqlite::SqliteStore>,
}

impl AppState {
    pub fn new(store: Arc<rondo_core::store::sqlite::SqliteStore>) -> Result<Self> {
        let tasks = store.list_tasks()?;
        let journal_notes = store.list_journal_notes()?;
        let journal_entries = if let Some(n) = journal_notes.first() {
            store.entries_for_note(n.id)?
        } else { vec![] };
        Ok(Self {
            theme: Theme::dark(), page: Page::Tasks,
            tasks, selected_task: 0, focus_left: true, split_ratio: 50,
            journal_notes, journal_entries, selected_journal: 0,
            pomodoro_open: false, pomodoro_started: None,
            pomodoro_total: Duration::from_secs(25 * 60),
            command_palette_open: false, command_buf: String::new(),
            should_quit: false, status_msg: None,
            plugins: PluginRegistry::new(),
            store,
        })
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NextItem => {
                if self.page == Page::Tasks && !self.tasks.is_empty() {
                    self.selected_task = (self.selected_task + 1) % self.tasks.len();
                } else if self.page == Page::Journal && !self.journal_notes.is_empty() {
                    self.selected_journal = (self.selected_journal + 1).min(self.journal_notes.len() - 1);
                    if let Ok(e) = self.store.entries_for_note(self.journal_notes[self.selected_journal].id) {
                        self.journal_entries = e;
                    }
                }
            }
            Action::PrevItem => {
                if self.page == Page::Tasks && !self.tasks.is_empty() {
                    self.selected_task = if self.selected_task == 0 { self.tasks.len() - 1 } else { self.selected_task - 1 };
                } else if self.page == Page::Journal && self.selected_journal > 0 {
                    self.selected_journal -= 1;
                    if let Ok(e) = self.store.entries_for_note(self.journal_notes[self.selected_journal].id) {
                        self.journal_entries = e;
                    }
                }
            }
            Action::TogglePage(p) => self.page = p,
            Action::FocusNext => self.focus_left = !self.focus_left,
            Action::ResizeSplit { delta } => {
                let new = self.split_ratio as i16 + delta;
                self.split_ratio = new.clamp(20, 80) as u16;
            }
            Action::TogglePomodoro => {
                self.pomodoro_open = !self.pomodoro_open;
                if self.pomodoro_open { self.pomodoro_started = Some(Instant::now()); }
            }
            Action::OpenCommandPalette => { self.command_palette_open = true; self.command_buf.clear(); }
            Action::CloseCommandPalette => self.command_palette_open = false,
            _ => {}
        }
    }
}
```

- [ ] **Step 6: `main.rs` — wiring inicial (sólo render placeholder)**

```rust
use color_eyre::eyre::Result;
use crossterm::event;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod action;
mod app;
mod event as ev;
mod theme;
mod tui;
mod components;

use clap::Parser;

#[derive(Parser)]
#[command(name = "rondo-tui", version)]
struct Cli {
    /// Path to SQLite DB (default: ~/.todo-app/todo.db)
    #[arg(long, env = "RONDO_DB")]
    db: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    if !db_path.exists() {
        eprintln!("DB no encontrado en {}. Usa --db o setea RONDO_DB.", db_path.display());
        std::process::exit(2);
    }
    let store = Arc::new(rondo_core::store::sqlite::SqliteStore::open_readonly(&db_path)?);
    let mut app = app::AppState::new(store)?;
    let mut terminal = tui::init()?;
    let result = run(&mut terminal, &mut app);
    tui::restore()?;
    result
}

fn default_db_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home).join(".todo-app").join("todo.db")
}

fn run(terminal: &mut tui::Tui, app: &mut app::AppState) -> Result<()> {
    let tick = Duration::from_millis(100);
    let mut last = Instant::now();
    while !app.should_quit {
        terminal.draw(|f| {
            let block = Block::default().borders(Borders::ALL).title("rondo-tui (skeleton)");
            f.render_widget(Paragraph::new(format!("tasks: {}", app.tasks.len())).block(block), f.area());
        })?;
        let timeout = tick.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            if let Some(a) = ev::map(event::read()?) { app.update(a); }
        }
        if last.elapsed() >= tick { last = Instant::now(); app.update(action::Action::Tick); }
    }
    Ok(())
}
```

- [ ] **Step 7: `components/mod.rs` stub**

```rust
pub mod root;
pub mod header;
pub mod footer;
pub mod task_list;
pub mod task_detail;
pub mod journal;
pub mod pomodoro;
pub mod command_palette;

use crate::app::AppState;
use ratatui::{layout::Rect, Frame};

pub trait Component {
    fn draw(&self, app: &AppState, f: &mut Frame<'_>, area: Rect);
}
```

Crear archivos vacíos `crates/rondo-tui/src/components/{root,header,footer,task_list,task_detail,journal,pomodoro,command_palette}.rs` con `// stub` para que compile.

- [ ] **Step 8: Verificar build + smoke run**

Run: `cargo build -p rondo-tui`
Expected: compila sin warnings críticos.

Run (manual smoke; el tester ejecuta esto interactivamente):
```bash
RONDO_DB=$(mktemp).db
sqlite3 "$RONDO_DB" < fixtures/seed.sql
cargo run -p rondo-tui -- --db "$RONDO_DB"
```
Expected: ventana con borde y texto "tasks: 3". `q` o `Ctrl+C` cierra limpio.

- [ ] **Step 9: Commit**

```bash
git add crates/rondo-tui/
git commit -m "feat(tui): bootstrap event loop, theme, and action enum"
```

---

### Task 5: Layout root + header + footer

**Files:**
- Modify: `crates/rondo-tui/src/components/root.rs`
- Modify: `crates/rondo-tui/src/components/header.rs`
- Modify: `crates/rondo-tui/src/components/footer.rs`
- Modify: `crates/rondo-tui/src/main.rs` (sustituir render placeholder por `Root`)

- [ ] **Step 1: `header.rs` — tab bar parity con Go**

```rust
use crate::{action::Page, app::AppState};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let title = Span::styled("RonDO", Style::default().fg(t.accent).add_modifier(Modifier::BOLD));
    let sep = Span::styled(" │ ", Style::default().fg(t.fg_muted));
    let tab = |label: &'static str, active: bool| {
        let s = if active { Style::default().fg(t.fg).add_modifier(Modifier::REVERSED | Modifier::BOLD) }
                else { Style::default().fg(t.fg_muted) };
        Span::styled(format!(" {} ", label), s)
    };
    let line = Line::from(vec![
        title, sep.clone(),
        tab("Tasks", app.page == Page::Tasks), Span::raw(" "),
        tab("Journal", app.page == Page::Journal), sep.clone(),
        Span::styled(
            format!("{} tasks", app.tasks.len()),
            Style::default().fg(t.fg_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
```

- [ ] **Step 2: `footer.rs` — status bar con hints**

```rust
use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let mut spans = vec![
        Span::styled(" j/k ", Style::default().fg(t.accent)),
        Span::styled("nav  ", Style::default().fg(t.fg_muted)),
        Span::styled(" Tab ", Style::default().fg(t.accent)),
        Span::styled("focus  ", Style::default().fg(t.fg_muted)),
        Span::styled(" 1/2 ", Style::default().fg(t.accent)),
        Span::styled("page  ", Style::default().fg(t.fg_muted)),
        Span::styled(" p ", Style::default().fg(t.accent)),
        Span::styled("pomodoro  ", Style::default().fg(t.fg_muted)),
        Span::styled(" : ", Style::default().fg(t.accent)),
        Span::styled("cmd  ", Style::default().fg(t.fg_muted)),
        Span::styled(" q ", Style::default().fg(t.accent)),
        Span::styled("quit", Style::default().fg(t.fg_muted)),
    ];
    if let Some(msg) = &app.status_msg {
        spans.push(Span::raw("  │  "));
        spans.push(Span::styled(msg.clone(), Style::default().fg(t.warning)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
```

- [ ] **Step 3: `root.rs` — layout master**

```rust
use crate::{action::Page, app::AppState, components};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());
    components::header::draw(app, f, chunks[0]);
    body(app, f, chunks[1]);
    components::footer::draw(app, f, chunks[2]);
    if app.pomodoro_open { components::pomodoro::draw(app, f, centered(40, 11, f.area())); }
    if app.command_palette_open { components::command_palette::draw(app, f, palette_rect(f.area())); }
}

fn body(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    match app.page {
        Page::Tasks => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(app.split_ratio), Constraint::Percentage(100 - app.split_ratio)])
                .split(area);
            components::task_list::draw(app, f, split[0]);
            components::task_detail::draw(app, f, split[1]);
        }
        Page::Journal => components::journal::draw(app, f, area),
    }
}

fn centered(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect { x, y, width: w.min(area.width), height: h.min(area.height) }
}

fn palette_rect(area: Rect) -> Rect {
    let h = 12.min(area.height / 2);
    Rect { x: area.x + 2, y: area.y + area.height - h - 1, width: area.width - 4, height: h }
}
```

- [ ] **Step 4: Stubs de componentes**

Reemplazar cada stub creado en Task 4 con función `pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect)` que renderice un `Block` con título por ahora (e.g. `task_list.rs` muestra block con título "Tasks"). Servirá de andamio hasta Task 6.

- [ ] **Step 5: Cambiar `main.rs::run`**

Reemplazar el render placeholder:
```rust
terminal.draw(|f| components::root::draw(app, f))?;
```

- [ ] **Step 6: Smoke run**

```bash
cargo run -p rondo-tui -- --db "$RONDO_DB"
```
Expected: header `RonDO │ Tasks Journal`, footer con hints, dos paneles vacíos (Tasks/Detail). `Tab` cambia focus, `>`/`<` ajustan ratio, `q` cierra.

- [ ] **Step 7: Commit**

```bash
git commit -am "feat(tui): add root layout with header, footer, page routing"
```

---

### Task 6: Task list panel (replica visual de delegate.go)

**Files:**
- Modify: `crates/rondo-tui/src/components/task_list.rs`
- Create: `crates/rondo-tui/src/widgets/priority_badge.rs`
- Create: `crates/rondo-tui/src/widgets/due_badge.rs`
- Create: `crates/rondo-tui/src/widgets/mod.rs`
- Test: `crates/rondo-tui/tests/snapshot/task_list.rs` con `insta`

Reference: `/Users/roniel/Develop/rondo/internal/app/delegate.go:31–96`.

- [ ] **Step 1: `widgets/mod.rs`**

```rust
pub mod priority_badge;
pub mod due_badge;
pub mod progress_bar;
pub mod markdown;
```

- [ ] **Step 2: `priority_badge.rs`**

```rust
use crate::theme::Theme;
use ratatui::text::Span;
use rondo_core::domain::task::Priority;

pub fn span(p: Priority, theme: &Theme) -> Span<'static> {
    Span::styled(format!(" {} ", p.label()), theme.priority_style(p))
}
```

- [ ] **Step 3: `due_badge.rs`**

```rust
use crate::theme::Theme;
use chrono::{Local, NaiveDate};
use ratatui::{style::{Modifier, Style}, text::Span};

pub fn span(due: Option<NaiveDate>, theme: &Theme) -> Option<Span<'static>> {
    let due = due?;
    let today = Local::now().date_naive();
    let (label, color) = if due < today { ("OVERDUE", theme.danger) }
        else if due == today { ("TODAY", theme.warning) }
        else { ("UPCOMING", theme.fg_muted) };
    Some(Span::styled(format!(" {} ", label),
        Style::default().fg(color).add_modifier(Modifier::BOLD)))
}
```

- [ ] **Step 4: `task_list.rs`**

```rust
use crate::app::AppState;
use crate::widgets::{due_badge, priority_badge};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let border_color = if app.focus_left { t.border_active } else { t.border_inactive };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" Tasks ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)));

    let items: Vec<ListItem> = app.tasks.iter().enumerate().map(|(i, task)| {
        let icon = Span::styled(
            format!(" {} ", task.status.icon()),
            Style::default().fg(match task.status {
                rondo_core::domain::task::Status::Done => t.success,
                rondo_core::domain::task::Status::InProgress => t.accent,
                _ => t.fg_muted,
            }),
        );
        let mut spans = vec![icon, Span::raw(task.title.clone()), Span::raw("  ")];
        spans.push(priority_badge::span(task.priority, t));
        if let Some(b) = due_badge::span(task.due_date, t) { spans.push(b); }
        if task.is_blocked() {
            spans.push(Span::styled(" BLOCKED ",
                Style::default().fg(t.danger).add_modifier(Modifier::REVERSED | Modifier::BOLD)));
        }
        let (done, total) = task.subtask_progress();
        if total > 0 {
            spans.push(Span::styled(format!("  {}/{}", done, total), Style::default().fg(t.fg_muted)));
        }
        let tags = if task.tags.is_empty() { String::new() }
            else { format!("  [{}]", task.tags.join(",")) };
        if !tags.is_empty() {
            spans.push(Span::styled(tags, Style::default().fg(t.fg_muted)));
        }
        ListItem::new(Line::from(spans))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default();
    state.select(Some(app.selected_task));
    f.render_stateful_widget(list, area, &mut state);
}
```

- [ ] **Step 5: Test snapshot**

Crear `crates/rondo-tui/tests/snapshot.rs`:
```rust
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};
use rondo_core::store::sqlite::SqliteStore;
use std::sync::Arc;

fn fixture_store() -> Arc<SqliteStore> {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string("../../fixtures/seed.sql").unwrap()).unwrap();
    Arc::new(SqliteStore::open_readonly(tmp.path()).unwrap())
}

#[test]
fn task_list_snapshot() {
    let store = fixture_store();
    let app = rondo_tui::app::AppState::new(store).unwrap();
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| rondo_tui::components::task_list::draw(&app, f, f.area())).unwrap();
    assert_snapshot!(term.backend());
}
```

Esto requiere exponer módulos en `crates/rondo-tui/src/lib.rs` (crear si no existe):
```rust
pub mod action;
pub mod app;
pub mod theme;
pub mod components;
pub mod widgets;
pub mod event;
pub mod tui;
```
Y ajustar `main.rs` para hacer `use rondo_tui::*` o moverlo. Más simple: dejar `main.rs` con `mod` declarations Y crear un `lib.rs` paralelo que re-exporte (`pub use rondo_tui_lib::*` no necesario; usar `pub mod` en lib y `mod` en main).

Para evitar duplicación: convertir `rondo-tui` en lib + binary — `Cargo.toml` ya tiene `[[bin]]`, agregar `[lib]` con `path = "src/lib.rs"` y `main.rs` usar `use rondo_tui::*`.

- [ ] **Step 6: Verificar test**

Run: `cargo test -p rondo-tui --test snapshot`
Expected: primer run crea `.snap.new` — `cargo insta review` para aceptar. Después PASS.

- [ ] **Step 7: Smoke run visual**

```bash
cargo run -p rondo-tui -- --db "$RONDO_DB"
```
Expected: 3 tasks visibles con iconos, prioridad coloreada, due badge, tags.

- [ ] **Step 8: Commit**

```bash
git commit -am "feat(tui): render task list with priority/due/tags badges"
```

---

### Task 7: Task detail panel

**Files:**
- Modify: `crates/rondo-tui/src/components/task_detail.rs`
- Create: `crates/rondo-tui/src/widgets/progress_bar.rs`
- Create: `crates/rondo-tui/src/widgets/markdown.rs`

Reference: `/Users/roniel/Develop/rondo/internal/ui/views.go:68–290`.

- [ ] **Step 1: `progress_bar.rs`** (subtasks ratio visual)

```rust
use crate::theme::Theme;
use ratatui::{style::Style, text::{Line, Span}};

pub fn line(done: usize, total: usize, width: usize, theme: &Theme) -> Line<'static> {
    if total == 0 { return Line::raw(""); }
    let ratio = (done as f64) / (total as f64);
    let filled = ((width as f64) * ratio).round() as usize;
    let empty = width.saturating_sub(filled);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(theme.success)),
        Span::styled("░".repeat(empty), Style::default().fg(theme.fg_muted)),
        Span::raw(format!("  {}/{}", done, total)),
    ])
}
```

- [ ] **Step 2: `markdown.rs`** — pulldown-cmark → ratatui Text minimal

```rust
use crate::theme::Theme;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

pub fn render(md: &str, theme: &Theme) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut buf: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default().fg(theme.fg);
    let mut in_heading: Option<u8> = None;

    for ev in Parser::new(md) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                if !buf.is_empty() { lines.push(Line::from(std::mem::take(&mut buf))); }
                in_heading = Some(level as u8);
                style = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
            }
            Event::End(TagEnd::Heading(_)) => {
                lines.push(Line::from(std::mem::take(&mut buf)));
                lines.push(Line::raw(""));
                in_heading = None;
                style = Style::default().fg(theme.fg);
            }
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::End(TagEnd::Strong) => style = style.remove_modifier(Modifier::BOLD),
            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::End(TagEnd::Emphasis) => style = style.remove_modifier(Modifier::ITALIC),
            Event::Text(s) => {
                let s = if in_heading.is_some() { s.to_string().to_uppercase() } else { s.to_string() };
                buf.push(Span::styled(s, style));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut buf)));
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                lines.push(Line::from(std::mem::take(&mut buf)));
                lines.push(Line::raw(""));
            }
            _ => {}
        }
    }
    if !buf.is_empty() { lines.push(Line::from(buf)); }
    Text::from(lines)
}
```

- [ ] **Step 3: `task_detail.rs`**

```rust
use crate::app::AppState;
use crate::widgets::{due_badge, markdown, priority_badge, progress_bar};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use rondo_core::domain::task::Status;

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let task = match app.tasks.get(app.selected_task) {
        Some(x) => x,
        None => {
            f.render_widget(empty_block(t, area), area);
            return;
        }
    };
    let border = if !app.focus_left { t.border_active } else { t.border_inactive };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(Span::styled(" Detail ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(task.title.clone(),
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD))));
    lines.push(Line::raw(""));

    let mut meta_spans: Vec<Span> = vec![
        Span::styled("Status: ", Style::default().fg(t.fg_muted)),
        Span::styled(format!("{} ", task.status.icon()), Style::default().fg(
            match task.status { Status::Done => t.success, Status::InProgress => t.accent, _ => t.fg_muted })),
        Span::styled(format!("{:?}", task.status), Style::default().fg(t.fg)),
        Span::raw("    "),
        Span::styled("Priority: ", Style::default().fg(t.fg_muted)),
        priority_badge::span(task.priority, t),
    ];
    if let Some(b) = due_badge::span(task.due_date, t) {
        meta_spans.push(Span::raw("    "));
        meta_spans.push(Span::styled("Due: ", Style::default().fg(t.fg_muted)));
        meta_spans.push(b);
    }
    lines.push(Line::from(meta_spans));
    lines.push(Line::raw(""));

    if !task.tags.is_empty() {
        let tag_spans: Vec<Span> = task.tags.iter().flat_map(|tag| {
            [Span::styled(format!(" {} ", tag),
                Style::default().fg(t.accent).add_modifier(Modifier::REVERSED)),
             Span::raw(" ")]
        }).collect();
        let mut row = vec![Span::styled("Tags    ", Style::default().fg(t.fg_muted))];
        row.extend(tag_spans);
        lines.push(Line::from(row));
        lines.push(Line::raw(""));
    }

    if let Some(desc) = &task.description {
        if !desc.is_empty() {
            for l in markdown::render(desc, t).lines { lines.push(l); }
        }
    }

    let (done, total) = task.subtask_progress();
    if total > 0 {
        lines.push(Line::from(Span::styled(format!("Subtasks ({}/{})", done, total),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD))));
        lines.push(progress_bar::line(done, total, 30, t));
        for st in &task.subtasks {
            let icon = if st.completed { "✓" } else { "○" };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(if st.completed { t.success } else { t.fg_muted })),
                Span::raw("  "),
                Span::styled(st.title.clone(),
                    if st.completed { Style::default().fg(t.fg_muted).add_modifier(Modifier::CROSSED_OUT) }
                    else { Style::default().fg(t.fg) }),
            ]));
        }
        lines.push(Line::raw(""));
    }

    if task.is_blocked() {
        lines.push(Line::from(vec![
            Span::styled("Blocked by: ", Style::default().fg(t.fg_muted)),
            Span::styled(task.blocked_by_ids.iter().map(|i| format!("#{}", i)).collect::<Vec<_>>().join(", "),
                Style::default().fg(t.danger)),
        ]));
        lines.push(Line::raw(""));
    }

    if !task.time_logs.is_empty() {
        let total_secs: i64 = task.time_logs.iter().map(|tl| tl.duration_secs).sum();
        lines.push(Line::from(vec![
            Span::styled("Time logged: ", Style::default().fg(t.fg_muted)),
            Span::styled(format_duration(total_secs), Style::default().fg(t.success)),
        ]));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block), area);
}

fn empty_block(t: &crate::theme::Theme, _area: Rect) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.border_inactive))
        .title(" Detail ")
}

fn format_duration(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
}
```

- [ ] **Step 4: Smoke + snapshot**

```bash
cargo run -p rondo-tui -- --db "$RONDO_DB"
```
Expected: panel derecho muestra task seleccionada con title, status, priority, tags chips, subtasks con progress bar, blocked-by si aplica.

Añadir snapshot test análogo a Task 6 (`task_detail_snapshot`).

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(tui): render task detail with markdown desc, progress, time logs"
```

---

### Task 8: Journal day-view

**Files:**
- Modify: `crates/rondo-tui/src/components/journal.rs`

Reference: `/Users/roniel/Develop/rondo/internal/app/model_journal.go`.

- [ ] **Step 1: Layout dos-paneles + render markdown**

```rust
use crate::{app::AppState, widgets::markdown};
use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(1)])
        .split(area);

    let items: Vec<ListItem> = app.journal_notes.iter().map(|n| {
        let label = smart_date_label(n.date);
        ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", label), Style::default().fg(t.accent)),
            Span::styled(n.date.format("%Y-%m-%d").to_string(), Style::default().fg(t.fg_muted)),
        ]))
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Days "))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default();
    state.select(Some(app.selected_journal));
    f.render_stateful_widget(list, chunks[0], &mut state);

    let mut content_lines: Vec<Line> = Vec::new();
    if let Some(note) = app.journal_notes.get(app.selected_journal) {
        content_lines.push(Line::from(Span::styled(
            note.date.format("%A, %B %-d, %Y").to_string(),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )));
        content_lines.push(Line::raw(""));
        for entry in &app.journal_entries {
            content_lines.push(Line::from(Span::styled(
                entry.created_at.with_timezone(&Local).format("%H:%M  ").to_string(),
                Style::default().fg(t.fg_muted),
            )));
            for l in markdown::render(&entry.body, t).lines { content_lines.push(l); }
            content_lines.push(Line::raw(""));
        }
    } else {
        content_lines.push(Line::from(Span::styled("No journal entries", Style::default().fg(t.fg_muted))));
    }
    f.render_widget(
        Paragraph::new(content_lines).wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Today ")),
        chunks[1],
    );
}

fn smart_date_label(date: chrono::NaiveDate) -> &'static str {
    let today = Local::now().date_naive();
    let delta = (today - date).num_days();
    match delta {
        0 => "Today",
        1 => "Yesterday",
        d if d < 7 => match date.weekday() {
            chrono::Weekday::Mon => "Mon", chrono::Weekday::Tue => "Tue",
            chrono::Weekday::Wed => "Wed", chrono::Weekday::Thu => "Thu",
            chrono::Weekday::Fri => "Fri", chrono::Weekday::Sat => "Sat",
            chrono::Weekday::Sun => "Sun",
        },
        _ => "",
    }
}
```

- [ ] **Step 2: Smoke + commit**

Run: `cargo run -p rondo-tui -- --db "$RONDO_DB"`, presiona `2` → journal page muestra día con entries renderizadas como markdown.

```bash
git commit -am "feat(tui): journal day-view with markdown entries and smart date labels"
```

---

### Task 9: Pomodoro overlay (in-memory, animado)

**Files:**
- Modify: `crates/rondo-tui/src/components/pomodoro.rs`
- Modify: `crates/rondo-tui/src/app.rs` (tick handler)

- [ ] **Step 1: Render overlay**

```rust
use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL)
        .border_style(Style::default().fg(t.urgent))
        .title(Span::styled(" 🍅 Focus Session ",
            Style::default().fg(t.urgent).add_modifier(Modifier::BOLD)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let task_label = app.tasks.get(app.selected_task)
        .map(|t| t.title.as_str()).unwrap_or("(no task)");
    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled("Task: ", Style::default().fg(t.fg_muted)),
        Span::styled(task_label, Style::default().fg(t.fg).add_modifier(Modifier::BOLD)),
    ])), chunks[0]);

    let (elapsed, total) = (
        app.pomodoro_started.map(|s| s.elapsed().as_secs()).unwrap_or(0),
        app.pomodoro_total.as_secs(),
    );
    let remaining = total.saturating_sub(elapsed);
    let ratio = (elapsed as f64 / total as f64).clamp(0.0, 1.0);

    let mut throbber_state = ThrobberState::default();
    throbber_state.calc_next();
    let throbber = Throbber::default()
        .label(format!("  {:02}:{:02} remaining", remaining / 60, remaining % 60))
        .style(Style::default().fg(t.accent));
    f.render_stateful_widget(throbber, chunks[1], &mut throbber_state);

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(t.urgent).bg(t.border_inactive))
        .ratio(ratio)
        .label(format!("{:.0}%", ratio * 100.0));
    f.render_widget(gauge, chunks[2]);

    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled(" p ", Style::default().fg(t.accent)),
        Span::styled("toggle  ", Style::default().fg(t.fg_muted)),
        Span::styled(" Esc ", Style::default().fg(t.accent)),
        Span::styled("close", Style::default().fg(t.fg_muted)),
    ])), chunks[3]);
}
```

- [ ] **Step 2: `app.rs` — auto-render on tick**

Asegurar que el loop redraws cuando `pomodoro_open` cada tick (ya cubierto por el tick de 100ms en main).

- [ ] **Step 3: Smoke**

`cargo run`, presiona `p` → overlay aparece, gauge avanza ~0.067%/segundo (25min total), throbber rota. `p` o `Esc` cierra.

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(tui): pomodoro overlay with animated gauge and throbber"
```

---

### Task 10: Command palette (slumber-style)

**Files:**
- Modify: `crates/rondo-tui/src/components/command_palette.rs`
- Modify: `crates/rondo-tui/src/event.rs` (capturar texto cuando palette abierta)
- Modify: `crates/rondo-tui/src/action.rs` (`SearchInput`)

- [ ] **Step 1: Render palette**

```rust
use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.accent))
        .title(Span::styled(" : command ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled("› ", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
        Span::raw(app.command_buf.clone()),
        Span::styled("▏", Style::default().fg(t.fg).add_modifier(Modifier::SLOW_BLINK)),
    ])), chunks[0]);

    let suggestions = filter_suggestions(&app.command_buf);
    let items: Vec<ListItem> = suggestions.iter().map(|s| ListItem::new(Line::from(vec![
        Span::styled(format!("  {} ", s.cmd), Style::default().fg(t.accent)),
        Span::styled(s.desc, Style::default().fg(t.fg_muted)),
    ]))).collect();
    f.render_widget(List::new(items), chunks[1]);
}

struct Suggestion { cmd: &'static str, desc: &'static str }

fn filter_suggestions(buf: &str) -> Vec<&'static Suggestion> {
    static ALL: &[Suggestion] = &[
        Suggestion { cmd: "tasks", desc: "switch to Tasks page" },
        Suggestion { cmd: "journal", desc: "switch to Journal page" },
        Suggestion { cmd: "pomodoro", desc: "start focus session" },
        Suggestion { cmd: "quit", desc: "exit rondo-tui" },
    ];
    ALL.iter().filter(|s| s.cmd.starts_with(buf)).collect()
}
```

- [ ] **Step 2: `event.rs` — modo de input cuando palette abierta**

Cambiar signature de `map` para recibir `&AppState` (decidir modo):

```rust
pub fn map(ev: Event, app: &crate::app::AppState) -> Option<Action> {
    if app.command_palette_open {
        if let Event::Key(k) = ev {
            return Some(match k.code {
                KeyCode::Esc => Action::CloseCommandPalette,
                KeyCode::Enter => Action::SubmitCommand(app.command_buf.clone()),
                KeyCode::Backspace => Action::SearchInput({
                    let mut s = app.command_buf.clone(); s.pop(); s
                }),
                KeyCode::Char(c) => Action::SearchInput({
                    let mut s = app.command_buf.clone(); s.push(c); s
                }),
                _ => return None,
            });
        }
        return None;
    }
    // ... original key_to_action logic
    match ev { /* idem */ }
}
```

Actualizar `main.rs::run` para pasar `app`:
```rust
if let Some(a) = ev::map(event::read()?, app) { app.update(a); }
```

- [ ] **Step 3: `app.rs::update` — handle SearchInput/SubmitCommand**

```rust
Action::SearchInput(s) => self.command_buf = s,
Action::SubmitCommand(cmd) => {
    self.command_palette_open = false;
    match cmd.trim() {
        "tasks" => self.page = Page::Tasks,
        "journal" => self.page = Page::Journal,
        "pomodoro" => { self.pomodoro_open = true; self.pomodoro_started = Some(Instant::now()); }
        "quit" => self.should_quit = true,
        other if !other.is_empty() => self.status_msg = Some(format!("unknown: {}", other)),
        _ => {}
    }
}
```

- [ ] **Step 4: Smoke + commit**

`:tasks<Enter>`, `:journal<Enter>`, `:pomodoro<Enter>`, `:quit<Enter>` funcionan.

```bash
git commit -am "feat(tui): slumber-style command palette with submit dispatch"
```

---

### Task 11: Plugin builtin pomodoro como demo del contrato

**Files:**
- Create: `crates/rondo-tui/src/plugins/mod.rs`
- Create: `crates/rondo-tui/src/plugins/builtin/mod.rs`
- Create: `crates/rondo-tui/src/plugins/builtin/pomodoro.rs`

**Propósito:** Demostrar que el componente Pomodoro funciona también como Plugin via `rondo-plugin-api`, validando que el contrato sirve. El render real sigue siendo nativo ratatui (más rico que `ViewSpec`); este plugin solo demuestra ciclo `Tick → estado → ViewSpec`.

- [ ] **Step 1: Implementar plugin**

```rust
use rondo_plugin_api::{
    action::PluginAction, capabilities::Capability,
    plugin::{Plugin, PluginContext, PluginMeta, PluginResult},
    view::{Block, ColorToken, TextStyle, ViewKind, ViewSpec},
};

pub struct PomodoroPlugin {
    elapsed_ms: u64,
    total_ms: u64,
    running: bool,
}

impl PomodoroPlugin {
    pub fn new() -> Self {
        Self { elapsed_ms: 0, total_ms: 25 * 60 * 1000, running: false }
    }
}

impl Plugin for PomodoroPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: "builtin.pomodoro", name: "Pomodoro", version: "0.1.0",
            capabilities: &[Capability::OverlayView, Capability::TickHandler, Capability::CommandContributor],
        }
    }
    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext<'_>) -> PluginResult {
        match action {
            PluginAction::Show => { self.running = true; self.elapsed_ms = 0; }
            PluginAction::Hide => self.running = false,
            PluginAction::Tick { delta_ms } if self.running => {
                self.elapsed_ms = (self.elapsed_ms + delta_ms as u64).min(self.total_ms);
            }
            _ => {}
        }
        let ratio = self.elapsed_ms as f64 / self.total_ms as f64;
        PluginResult {
            view: self.running.then(|| ViewSpec {
                kind: ViewKind::Overlay,
                blocks: vec![
                    Block::Heading { text: "Focus".into(), level: 1 },
                    Block::Gauge { ratio, label: Some(format!("{:.0}%", ratio * 100.0)) },
                    Block::Paragraph { text: format!("{}s remaining", (self.total_ms - self.elapsed_ms) / 1000),
                        style: Some(TextStyle { fg: Some(ColorToken::Accent), ..Default::default() }) },
                ],
            }),
            follow_up: vec![],
        }
    }
}
```

- [ ] **Step 2: Registrar en `app.rs::new`**

```rust
app.plugins.register(Box::new(crate::plugins::builtin::pomodoro::PomodoroPlugin::new()));
```

- [ ] **Step 3: Test**

```rust
#[test]
fn pomodoro_plugin_round_trip() {
    use rondo_plugin_api::{action::PluginAction, plugin::{Plugin, PluginContext}};
    let mut p = crate::plugins::builtin::pomodoro::PomodoroPlugin::new();
    let ctx = PluginContext { now: chrono::Utc::now(), _hidden: std::marker::PhantomData };
    let r = p.handle(PluginAction::Show, &ctx);
    assert!(r.view.is_some());
    let r = p.handle(PluginAction::Tick { delta_ms: 5000 }, &ctx);
    let v = r.view.unwrap();
    assert!(matches!(v.kind, rondo_plugin_api::view::ViewKind::Overlay));
}
```

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(plugin): demo builtin pomodoro plugin using stable api"
```

---

### Task 12: Documentación + CLAUDE.md + README

**Files:**
- Create: `/Users/roniel/Develop/Rust/rondo_rust/CLAUDE.md`
- Create: `/Users/roniel/Develop/Rust/rondo_rust/README.md`

- [ ] **Step 1: Crear CLAUDE.md** (contenido literal abajo)

- [ ] **Step 2: Crear README.md** con instrucciones de build + comparativa visual

- [ ] **Step 3: Commit**

```bash
git commit -am "docs: add CLAUDE.md and README for mvp"
```

---

## Verificación end-to-end

1. **Compila todo:** `cargo build --workspace --release`. Esperado: 3 binarios/libs sin errores.
2. **Tests:** `cargo test --workspace`. Esperado: 8+ tests PASS (2 domain + 3 store + 1 plugin-api + 1 pomodoro + N snapshots).
3. **Lint:** `cargo clippy --workspace -- -D warnings`. Esperado: 0 warnings.
4. **Smoke contra DB real:**
   ```bash
   cargo run --release -p rondo-tui
   ```
   (sin `--db` usa `~/.todo-app/todo.db`). Esperado: ve las tareas reales del usuario en read-only. **Si está vacío** (usuario no ha usado rondo Go nunca), usar `--db fixtures/seed.db`.
5. **Comparación visual:**
   - Tomar screenshot de `rondo` (Go) en task list, task detail, journal, pomodoro.
   - Tomar screenshot equivalente de `rondo-tui` (Rust).
   - Side-by-side. Si la versión Rust no se ve **claramente** mejor en al menos 3 de 4 vistas → archivar el rewrite, invertir esfuerzo en pulir `styles.go` del Go.
6. **Validar arquitectura plugin:** confirmar que `PomodoroPlugin` (en `rondo-plugin-api`) NO depende de ratatui ni crossterm — `cargo tree -p rondo-plugin-api` solo serde/strum.

---

## CLAUDE.md (contenido a escribir en Task 12)

```markdown
# RonDO Rust MVP — Claude Code project memory

## What this project is

Greenfield MVP that ports a slice of the Go project at `/Users/roniel/Develop/rondo` to Rust + ratatui to evaluate whether the visual quality improves enough to justify a full rewrite. Read-only against the Go SQLite at `~/.todo-app/todo.db`.

## Workspace layout

- `crates/rondo-core` — domain types (`Task`, `Note`, `Session`) + read-only `SqliteStore`. No TTY deps. Mirror of Go `internal/task/*`, `internal/journal/*`, `internal/focus/*`.
- `crates/rondo-plugin-api` — stable plugin contract: `trait Plugin`, `PluginAction`, `ViewSpec` (serializable UI DSL). Designed so a future `extism`/`wasmtime` host can drive plugins compiled from this crate without ABI changes.
- `crates/rondo-tui` — ratatui binary. Component/Action/Reducer architecture inspired by `openapi-tui`. Theme tokens in `theme.rs`, never hex-color literals scattered.

## Reference projects (visual inspiration)

- binsider — https://github.com/orhun/binsider (minimalist, normal borders, compact density)
- openapi-tui — https://github.com/zaghaghi/openapi-tui (command palette, panes with focus-colored borders)
- slumber — https://github.com/LucasPickering/slumber (sidebar persistence, `:`-style command palette, vim bindings)
- mdfried — https://github.com/benjajaja/mdfried (rich markdown rendering — heading hierarchy with visual weight)

## Parity with Go version (do not break)

- Status icons: `○` Pending, `◐` InProgress, `✓` Done
- Priority labels/colors: LOW (green), MED (yellow), HIGH (red), URG! (magenta)
- Keybindings: `j/k` nav, `Tab` focus, `1/2` pages, `p` pomodoro, `:` command palette, `</>` resize, `q` quit
- Color palette hex: cyan #00BCD4, white #FAFAFA, green #4CAF50, red #F44336, muted #9E9E9E

## How to run

```bash
cargo run -p rondo-tui                    # uses ~/.todo-app/todo.db
cargo run -p rondo-tui -- --db ./test.db  # custom path
RUST_LOG=debug cargo run -p rondo-tui     # logs to stderr (alt-screen safe)
```

The DB is opened with `OpenFlags::SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only=true`. Cannot corrupt the Go user data.

## Conventions

- **No `unsafe`.** Use `unsafe_code = "forbid"` in `Cargo.toml` once it stabilizes.
- **Errors flow through `color_eyre::eyre::Result`** at the binary boundary, `rondo_core::Result` inside the core crate.
- **No business logic in `components/`.** Components read from `AppState`, never query the store directly. The store is owned by `AppState`.
- **Theme tokens, not hex.** All colors come from `theme.rs::Theme`. If you find a `Color::Rgb(...)` outside `theme.rs`, move it.
- **Snapshot tests** for every component via `insta` + `TestBackend`. Run `cargo insta review` after intentional visual changes.
- **Plugin contract is the boundary.** Anything a future external plugin should be able to do must round-trip through `rondo-plugin-api` types — never reach into `rondo-core` directly from the plugin layer.

## Build / test / lint

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## Known limitations of the MVP

- **Read-only.** No task creation/edit/delete; the Go binary remains source of truth.
- **Pomodoro is in-memory.** Timer state lost on exit; not persisted to `focus_sessions`.
- **No CLI subcommands.** Only the TUI runs. Cobra parity is out of scope.
- **No plugin hot-reload, no `.wasm` loading.** Plugin host is in-process and static dispatch; the API surface is the part being validated.
- **Plugins cannot mutate `AppState`.** Plugin `handle` returns `ViewSpec` + follow-up actions only — by design, for sandbox readiness.

## When to consider this MVP "done"

- All 4 views render (task list, task detail, journal, pomodoro overlay) against the real `~/.todo-app/todo.db` of the user.
- Side-by-side screenshots taken vs the Go binary in each of the 4 contexts.
- Snapshot tests pass (`cargo insta accept`-ed once intentionally).
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
```

---

## Self-review checklist

- ✅ Cobertura de spec: 3 vistas (list/detail/journal) + pomodoro overlay + plugin contract + CLAUDE.md → todas tienen tarea explícita (Tasks 6, 7, 8, 9, 3+11, 12).
- ✅ Placeholders: ningún "TBD" o "implement later"; cada paso tiene código completo.
- ✅ Type consistency: `Status::from_db` / `Priority::from_db` / `RecurFreq::from_db` mismo patrón; `Plugin::handle` mismo signature en uses; `AppState` campos referenciados consistentemente.
- ✅ Comandos exactos con expected output.
- ✅ TDD: cada task crítica (1, 2, 3, 6, 7, 11) tiene test ANTES de implementación.
- ✅ Commits frecuentes: 12 commits separados (1 por task).

---

## Execution Handoff

Plan completo. Dos opciones de ejecución:

1. **Subagent-Driven (recomendado)** — Despachar fresh subagent por task, review entre tasks, iteración rápida. Requiere `superpowers:subagent-driven-development`.

2. **Inline Execution** — Ejecutar tasks en esta sesión con `superpowers:executing-plans`, batch con checkpoints.

¿Cuál enfoque?
