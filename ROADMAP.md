# RonDO Rust — Roadmap revisado (post-architect review)

Este documento es el plan operativo para llevar rondo-rust de "MVP visual aprobado" a "feature parity + ventaja propia sobre la versión Go". Revisión completa tras feedback de 3 architects paralelos — los cambios estructurales están documentados al final ("Architect review summary").

## Estado actual

✅ visual richness · sidebar interactivo · focus stack jerárquico · filtros · animaciones tachyonfx · plugin contract embrionario · 20 snapshot tests

❌ writes a DB · CRUD desde TUI · CLI subcomandos · journal write · pomodoro persistence · recurrence engine · plugin loader real · sync

## Principios

1. **Antes de tocar writes, garantizar backup automático** y schema migration test.
2. **Writes detrás de `--write` flag explícito** hasta que `M1 + M4 + roundtrip harness verde` pasen.
3. **Roundtrip test Rust↔Go** valida que ambos binarios producen el mismo set de instancias recurrentes antes de quitar el flag.
4. **Plugin loader temprano** — adelantado al puesto 4 (antes de calendar/heatmap/graph) para que esos features nazcan como plugins, no deuda en core.
5. **Cada fase deja la app shippable** (tests verdes, clippy limpio, `cargo run` corre).

---

## M0 — Foundations (NUEVO — añadido por architect review)

**Objetivo:** Refactor de tech debt + harness de testing crítico para que M1+ no se ahoguen en spaghetti.

### Fase M0.1 — Substate split en AppState

Files: `crates/rondo-tui/src/app/{mod.rs,tasks_state.rs,journal_state.rs,modals_state.rs}`.

Hoy `AppState` tiene 30+ campos en un struct plano y `update()` es un match de 700 líneas. Antes de M1 (que sumará undo + modales de confirm + write paths), partir en:

```rust
pub struct AppState {
    pub data: DataState,          // store + tasks vec + journal_notes + entries
    pub ui: UiState,              // focus, mode, split_ratio, selection
    pub modals: ModalsState,      // help/palette/quickadd/search/pomodoro/quick_actions
    pub fx: FxManager,
    pub plugins: PluginRegistry + PluginHost,
    pub config: Config,
    pub theme: Theme,
    pub should_quit: bool,
}
```

Cada substate tiene su propio `update(action)`. `AppState::update()` se vuelve un dispatcher que enruta. ~1-2 noches.

### Fase M0.2 — Roundtrip harness Rust↔Go

Files: `crates/rondo-tui/tests/roundtrip/`.

Script que verifica:
1. Rust crea task X → cierra → Go lee X correctamente
2. Go modifica X → cierra → Rust lee con cambios
3. Recurrencia: Rust spawnea instancias → Go las ve y no duplica
4. Backup creado por Rust → Go puede ignorarlo sin error

Sin esto, M1 vuela ciego cuando quitemos el `--write` flag.

### Fase M0.3 — Telemetry + crash reporter

Files: `crates/rondo-core/src/telemetry.rs`.

`tracing` a `~/.todo-app/logs/rondo-rust-{ts}.log`. Panic hook que escribe stack trace + estado parcial. Cap log a 7 días rotación.

**Total M0:** 2-3 noches. Bloquea M1.3.

---

## M1 — Writes seguros + CRUD básico (revisado)

**Objetivo:** CRUD desde TUI con backup + migrations. **Gate revisado:** no quitar `--write` flag hasta M1+M4+roundtrip verdes.

### Fase M1.1 — SqliteStore RW + backup

Files: `crates/rondo-core/src/store/{sqlite.rs,backup.rs}`, `crates/rondo-tui/src/main.rs`.

- `SqliteStore::open_readwrite(path)` separado de `open_readonly`.
- **Backups en sub-dir propio: `~/.todo-app/backups/rust/`** (no mezclar con la rotación del Go).
- `backup::snapshot(path) -> PathBuf` copia DB con timestamp ISO antes de cada write session. Rotación 30 días.
- CLI flag `--write` en `main.rs`: ausente → RO (default por toda M1-M3).

### Fase M1.2 — Schema migrations

Files: `crates/rondo-core/src/store/migrations.rs`.

- `PRAGMA user_version` versioning explícito.
- Migración idempotente con `ALTER TABLE ADD COLUMN IF NOT EXISTS`.
- **Lock cooperativo**: `~/.todo-app/.rondo-rust.lock` con PID; warning si Go tiene la DB abierta.
- Test: cargar `fixtures/seed-v1.sql` (sin columna `metadata`), verificar migración OK.

### Fase M1.3 — Domain mutations API

Files: `crates/rondo-core/src/store/sqlite.rs`, `crates/rondo-core/src/domain/task.rs`.

```rust
fn create_task(&self, input: NewTask) -> Result<i64>;
fn update_task(&self, id: i64, patch: TaskPatch) -> Result<UndoSnapshot>;
fn delete_task(&self, id: i64) -> Result<UndoSnapshot>;
fn set_status(&self, id: i64, status: Status) -> Result<UndoSnapshot>;
fn add_subtask(&self, task_id: i64, title: &str) -> Result<i64>;
fn toggle_subtask(&self, id: i64) -> Result<bool>;
fn add_tag(&self, task_id: i64, name: &str) -> Result<()>;
fn remove_tag(&self, task_id: i64, name: &str) -> Result<()>;
```

Cada mutate retorna `UndoSnapshot` (Task completa pre-cambio + child rows). Todas en transacciones.

### Fase M4 — Recurrence engine (movida adelante de M1.4)

**Movida aquí desde puesto 4 original por architect review.** Sin esto, activar writes corrompe paridad con Go.

Files: `crates/rondo-core/src/recurrence.rs`.

```rust
pub fn next_occurrence(task: &Task, now: NaiveDate) -> Option<NaiveDate>;
pub fn spawn_recurrent_instances(store: &SqliteStore, now: NaiveDate) -> Result<Vec<i64>>;
```

Portar tests del Go `internal/task/recurrence_test.go` (~30 casos: DST, leap year, month-end, recur_interval>1). **Estimación revisada 3-4 noches** (era 1-2).

`main.rs` al startup llama `spawn_recurrent_instances` (silent, idempotent).

### Fase M1.4 — Wire CRUD desde TUI

Files: `crates/rondo-tui/src/app/tasks_state.rs`, modales, bindings.

- `submit_quick_add` → `store.create_task(parsed)` si RW.
- `handle_space` Subtasks → `store.toggle_subtask(id)`.
- Bulk done Visual → `store.set_status` por id.
- Bind `d` → delete + confirm modal.
- Bind `e` → inline edit.
- **No `app.tasks = store.list_tasks()` después de cada mutate** — invalidación dirigida: aplicar `UndoSnapshot` localmente al vector + invalidar índices afectados.

### Fase M1.5 — Undo stack

Files: `crates/rondo-tui/src/app/undo.rs`.

- `Vec<UndoSnapshot>` cap 50.
- `Ctrl+Z` re-aplica via store.
- Test: crear → undo → estado idéntico inicial.

### **GATE — Quitar `--write` flag por defecto:** sólo cuando M1 + M4 + roundtrip harness verde. Hasta entonces, default RO.

---

## M10.1 — Config schema mínima (adelantado)

**Movida desde puesto 10 a 4 — bloqueante de M8 (plugin loader).** Sin config, los plugins no tienen donde declarar enabled/permissions.

Files: `crates/rondo-core/src/config.rs`.

```toml
[ui]
theme = "dark"
sidebar = true
animations = true
[pomodoro]
work_min = 25
short_break_min = 5
long_break_min = 15
[plugins]
enabled = ["builtin.pomodoro"]
[plugins.permissions]
"quote-of-the-day" = ["overlay_view", "tick_handler"]
```

`Config::load_or_default(path)` + `RONDO_CONFIG` env override. Defaults inline en código. ~medio día.

---

## M8 — Plugin loader (MOVIDO DEL PUESTO 9 AL 5)

**Reframing crítico de architect review:** adelantar plugin loader **antes** de calendar/heatmap/graph para que esos features nazcan como plugins, no como deuda en core.

Runtime decisión: **extism 1.x** (defendido en plugin-runtime review):
- Sandbox WASI gratis (sin fs, sin red por defecto)
- Cold-load 10-30ms para módulos pequeños
- Cross-platform (un `.wasm` cualquier OS)
- SDKs Rust/Go/Zig/JS estables para autores
- JSON serde ya en el contrato (cero migración)

### Fase M8.1 — Plugin contract evolution (BLOQUEANTE)

Files: `crates/rondo-plugin-api/src/{capabilities.rs,plugin.rs}`.

El contrato actual es **insuficiente** para WASM real (plugins read-only no pueden persistir nada → pomodoro-como-plugin imposible). Antes de M8.2 expandir:

```rust
pub enum Capability {
    // Existing
    OverlayView, TickHandler, CommandContributor, PageView,
    // Nuevas
    QueryAccess(QueryScope),       // ReadOnlyStore filtrado
    MutationAccess(MutationScope), // writes auditadas, opt-in usuario
    Exporter { format_id: &'static str, mime: &'static str },
    Syncer { name: &'static str },
    Notifier { channel: NotifyChannel },
    CliSubcommand { name: &'static str },
    ThemeContributor,
}
```

`PluginContext` ahora `#[derive(Serialize, Deserialize)]` con `now: DateTime<Utc>` + manifest reference.

Host-functions exposed via extism: `kv_get(plugin_id, key)`, `kv_set(plugin_id, key, val)`, `query_tasks(filter)`, etc.

Plugins read-only por defecto; `MutationAccess` requiere prompt user explícito en first-load.

### Fase M8.2 — `rondo-plugin-host` crate

Files: nuevo `crates/rondo-plugin-host/` workspace member.

```rust
pub struct PluginHost {
    plugins: HashMap<String, LoadedPlugin>,
    policy: Policy,
}
impl PluginHost {
    fn load_from_dir(&mut self, dir: &Path) -> Result<Vec<String>>;
    fn dispatch(&mut self, action: &PluginAction) -> Vec<(String, PluginResult)>;
    fn set_enabled(&mut self, id: &str, on: bool) -> Result<()>;
}
```

Manifest TOML por plugin (`plugin.toml`):
```toml
id = "calendar-extra"
version = "0.1.0"
api = "0.1"
capabilities = ["page_view", "query_access:journal"]
[wasi]
allowed_paths = []
allowed_hosts = []
```

Backwards compat: `PluginRegistry` builtins (PomodoroPlugin) coexiste — dispatch combina ambos.

### Fase M8.3 — KV store host-functions

Files: `crates/rondo-core/src/store/plugin_kv.rs`.

Tabla `plugin_kv (plugin_id, key, value BLOB)` con migración. Host-functions `kv_get/kv_set` namespaced por `plugin_id`. Plugin no ve DDL ni otras tablas.

### Fase M8.4 — Sample plugin: Quote of the Day

Files: `examples/plugins/quote-of-the-day/`.

Plugin trivial que demuestra `OverlayView` + `TickHandler`. Sin capabilities peligrosas. Cargo build a `wasm32-wasip1` + script.

### Fase M8.5 — Permission prompt + plugins CLI

Files: `crates/rondo-tui/src/components/permission_prompt.rs`, `crates/rondo-tui/src/cli/plugins.rs`.

- First-load: TUI overlay pidiendo aprobar capabilities.
- Sidebar `[5] PLUGINS` lista plugins reales con enable/disable.
- CLI: `rondo-tui plugins {install,list,info,remove}`. `install gh:owner/repo@tag` descarga `.wasm`.

### Performance budget M8

- Cold-load < 50ms por plugin
- Per-tick overhead < 200µs (M1 Pro) / < 500µs (x86_64 general)
- Memoria < 2MB residente por plugin
- Dispatch total < 5ms con 10 plugins
- Bench en `crates/rondo-plugin-host/benches/`. CI fail si regress >20%.

**Total M8:** 7-10 noches.

---

## M2 — Journal completo (revisado)

**M2.4 (calendar widget) reclasificada como plugin builtin** post-M8.

### M2.1 — Journal write API (CORE)

```rust
fn create_or_get_today_note(&self) -> Result<Note>;
fn add_journal_entry(&self, note_id: i64, body: &str) -> Result<i64>;
fn hide_note(&self, note_id: i64) -> Result<()>;
fn delete_entry(&self, entry_id: i64) -> Result<()>;
```

### M2.2 — TUI inline entry editor (CORE)

`tui-textarea` para input multilínea. Markdown preview al guardar.

### M2.3 — Date navigation (CORE)

`gg`/`G`, hidden filter, `H` toggle hide.

### M2.4 — Calendar widget → **PLUGIN BUILTIN**

Files: `crates/rondo-tui/src/plugins/builtin/calendar.rs`.

Plugin in-process (no `.wasm` aún, sólo trait Plugin). Usa `Capability::PageView + QueryAccess(Journal)`. Renderiza mini-calendar 7×N con días que tienen entries highlighted.

Razón de plugin: visualización alternativa, no dominio. Si el usuario no quiere calendario, no carga el plugin.

---

## M3 — Pomodoro persistencia (revisado)

### M3.1 — Focus sessions persistence (CORE)

```rust
fn start_focus_session(&self, task_id: Option<i64>, kind: SessionKind) -> Result<i64>;
fn complete_focus_session(&self, id: i64) -> Result<()>;
fn focus_streak(&self) -> Result<u32>;
```

### M3.2 — Focus page con heatmap → **PLUGIN BUILTIN**

Files: `crates/rondo-tui/src/plugins/builtin/focus_page.rs`.

Plugin con `PageView + QueryAccess(FocusSessions)`. Heatmap 7×N coloreado por sesiones/día.

### M3.3 — Bell sound → **PLUGIN BUILTIN**

Plugin con `Notifier { channel: Audio }`. Side-effect aislable.

### M3.4 — Configurable durations (CONFIG)

`[pomodoro]` en config.toml. Ya cubierto por M10.1.

---

## M9 — CLI subcomandos

**Reclasificado:** `skill install` → plugin. Resto core.

Subcommands core (`rondo-tui <cmd>`):
1. `add` quick-add syntax
2. `list` JSON/table
3. `done <id>`
4. `delete <id>`
5. `journal add`
6. `focus start`
7. `stats` JSON
8. `export` (md/json/ndjson — builtin)
9. `batch` NDJSON stdin
10. `recur preview`
11. `dep add/remove`
12. `note add`
13. `tag add/remove`
14. `timelog start/stop`
15. `config get/set`
16. `completion bash/zsh/fish` (clap_complete)
17. `plugins {install,list,info,remove}` (parte de M8.5)

`--json` global flag, `--quiet` suprime status.

`skill install` → plugin externo con `Capability::CliSubcommand`.

Tests integración con `assert_cmd`. **Empezar con 4** (`add/list/done/export`), el resto bajo demanda.

---

## M7 — Export (revisado)

### M7.1 — Exporters builtin (CORE)

Files: `crates/rondo-core/src/export.rs`.

```rust
pub fn to_markdown(tasks: &[Task]) -> String;
pub fn to_json(tasks: &[Task]) -> Result<String>;
pub fn to_ndjson<W: Write>(tasks: &[Task], w: &mut W) -> Result<()>;
```

Golden tests sobre `fixtures/seed.sql`.

### M7.2 — Exporter trait + plugin contributions (HYBRID)

`Capability::Exporter { format_id, mime }`. Core enruta `rondo-tui export --format icalendar` al plugin matching.

Formatos exóticos (iCal, taskpaper, Org-mode, CSV) → plugins externos.

---

## M6 — Search + sort

### M6.1 — Fuzzy search (HYBRID)

Files: `crates/rondo-tui/src/components/search.rs`, `crates/rondo-tui/src/app/search.rs`.

Crate `nucleo = "0.5"` default. Match sobre title + tags + description. Highlight matches con underline accent.

`Capability::Matcher` para algoritmos alternativos (regex, exact, etc.).

### M6.2 — Sort selector (CORE)

`s` abre overlay. Persiste en `AppState.sort_order` + config.

---

## M5 — Dependencies (revisado)

### M5.1 — Dep mutations (CORE)

```rust
fn add_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()>;  // detects cycles
fn remove_dependency(&self, task_id: i64, blocked_by: i64) -> Result<()>;
```

Cycle detection DFS.

### M5.2 — Dep graph render → **PLUGIN BUILTIN**

Plugin `PageView + QueryAccess(Deps)`. ASCII graph o `tui-tree-widget`.

---

## M10.2/M10.3 — Themes + a11y

Theme variants dark/light/high-contrast. Custom themes via `Capability::ThemeContributor`.

`NO_COLOR` honor, `RONDO_FX=0` (ya existe), `--reduced-motion` flag.

---

## Analytics dashboard → **PLUGIN BUILTIN**

Hoy `components/analytics.rs` está en core. **Promover a plugin builtin** con `PageView + QueryAccess(Tasks, FocusSessions)`. Si usuario no quiere dashboard, desactiva.

---

## M11 — Sync → **PLUGINS EXTERNOS**

**Reclasificado:** ya no es milestone monolítico. Tres caminos posibles, cada uno un plugin separado:

- `rondo-plugin-sync-git` (git-backed, conflict policy = last-write-wins)
- `rondo-plugin-sync-crdt` (Automerge, complejidad alta)
- `rondo-plugin-sync-cloud` (WebDAV/S3 backup)

Cada uno con `Capability::Syncer + MutationAccess(All) + TickHandler`.

**Recomendación:** **abandonar git-backed sync**. Conflict resolution sobre blob SQLite es hell. Empezar con cloud-backup-only.

---

## Plugins externos (descubiertos en `~/.todo-app/plugins/`)

No-core, no-builtin, opcionales por usuario:

- `rondo-plugin-skill-install` — escribe SKILL.md a `~/.claude/skills/`
- `rondo-plugin-templates` — task templates (daily-standup, weekly-review, etc.)
- `rondo-plugin-notify-desktop` — notify-rust / AppleScript / dbus
- `rondo-plugin-attachments-kitty` — kitty image protocol previews
- `rondo-plugin-export-ical`, `-org-mode`, `-csv`
- `rondo-plugin-sync-*` (M11)
- `rondo-plugin-themes-pack` — preset bundles

---

## Cronograma revisado

| Hito | Esfuerzo (noches) | Bloqueado por |
|---|---|---|
| **M0 Foundations** (substates + roundtrip + telemetry) | 2-3 | — |
| M1.1+M1.2 (backup + migrations) | 2 | M0.1 |
| M10.1 (config schema) | 0.5 | — |
| M1.3 (mutations API) | 2 | M0.1, M1.2 |
| **M4** (recurrence + tests Go) | **3-4** (era 1-2) | M1.3 |
| M1.4+M1.5 (TUI CRUD + undo) | 5-7 (era 3-5) | M4 |
| **GATE: roundtrip Rust↔Go verde → quitar `--write` default** | — | M1.5 |
| **M8.1** (contract evolution) | 2 | M1.3 |
| **M8.2-M8.4** (host crate + sample plugin) | 5-7 | M8.1, M10.1 |
| M8.5 (permission prompt + CLI) | 2 | M8.2 |
| M7.1 (export builtin) | 1 | M1.3 |
| M9.1 (CLI básico: 4 cmds) | 2 | M7.1 |
| M2.1-M2.3 (journal write) | 3 | M1.3 |
| **M2.4 calendar widget (PLUGIN)** | 2 | M8 |
| M3.1 (focus persistence) | 2 | M1.3 |
| **M3.2 focus page (PLUGIN)** | 3 | M8 |
| **M3.3 bell (PLUGIN)** | 1 | M8 |
| **Analytics dashboard (PLUGIN, refactor)** | 2 | M8 |
| M6 (search + sort) | 2 | M1.3 |
| M5.1 (dep mutations) | 1 | M1.3 |
| **M5.2 dep graph (PLUGIN)** | 2 | M8 |
| M9 (CLI restantes) | 3-4 | M1.3 |
| M7.2 (exporter trait) | 1 | M8 |
| M10.2/M10.3 (themes + a11y) | 2 | — |
| M11 sync (plugin separado) | 5+ | M8 + telemetry |

**Total revisado:** 51-66 noches (era 27-38 sin contar M0 ni replanteamiento).

Estimación honesta optimista: 2x = **15-20 semanas** weekend-pace.

## Orden recomendado (revisado)

1. **M0.1 + M0.2** — substates + roundtrip harness (bloqueante absoluto)
2. **M1.1 + M1.2** — backup + migrations
3. **M10.1** — config schema (habilita M8)
4. **M1.3** — mutations API
5. **M4** — recurrence engine (antes de M1.4!)
6. **M1.4 + M1.5** — TUI CRUD + undo
7. **GATE** — roundtrip verde → quitar `--write` flag
8. **M8.1 + M8.2 + M8.3 + M8.4** — plugin contract + host + KV + sample (4-5 commits seguidos)
9. **M7.1** — export markdown/json/ndjson (puro)
10. **M9.1** — CLI básico (4 subcommands)
11. **M2.1-M2.3** — journal write core
12. **M2.4** — calendar widget como PLUGIN
13. **M3.1** — focus session persistence core
14. **M3.2 + M3.3** — focus page + bell como PLUGINS
15. **Analytics dashboard refactor** → PLUGIN
16. **M6** — search + sort
17. **M5** — dependencies (core + graph plugin)
18. **M9 resto** + **M7.2** — CLI completo + exporter trait
19. **M8.5** — permission prompt + plugins CLI
20. **M10.2 + M10.3** — themes + a11y
21. **M11** — sync plugin (decisión tras telemetry)

---

## Architect review summary

Tres architects revisaron este roadmap en paralelo. Hallazgos clave:

### Critical-review architect
**Top 3 leverage edits aplicadas:**
1. ✅ Insertado **M0** (AppState substates + roundtrip harness)
2. ✅ Movido **M4** antes de M1.4; gate revisado a `M1 + M4 + roundtrip`
3. ✅ Plugin contract a expandir en **M8.1** (Serialize, KV, MutationAccess) antes de comprometer WASM

**Adicional:**
- M9 depende de M4 (sino CLI produce duplicados recurrentes) → wired
- M8 depende de M10.1 (config para enabled/permissions) → adelantado M10.1 al puesto 3
- AppState god-struct rompe antes de M3 sin substates → M0.1 obligatorio
- `focus_left()` shim no sobrevive a M3 → coordinar con substates
- Backup en sub-dir propio `~/.todo-app/backups/rust/` para no chocar con Go

**Time estimates corregidas:**
- M1: 3-5 → **8-12 noches** (mutation API + UI reload + undo snapshots completos)
- M8: 5-7 → **7-10 noches** (spike + contract evolution + host + sample)
- M11: 5 → **20+** o **abandonar git-backed** completamente

### Plugin-runtime architect
**Decisión:** extism 1.x. Defensa: sandbox WASI gratis, cold-load <50ms, cross-platform single `.wasm`, SDKs estables, JSON serde ya existe. Status quo (sólo builtins) insuficiente — M8 es la feature diferenciadora vs Go.

**Fallback:** mlua con sandbox estricto.

**Plan 6 commits:** skeleton → extism integration → dispatch+KV → wire to TUI → sample plugin → permission prompt + CLI.

**Threat model definido:** trust boundary en `.wasm`. Plugin no puede acceder fs/net por defecto, no puede mutar `AppState`, no puede leer KV de otros plugins (namespaced).

**Performance budget:** cold-load <50ms, per-tick <200µs, mem <2MB. Bench en `crates/rondo-plugin-host/benches/`.

**Distribución:** GitHub releases scanned + curated index. NO crates.io.

### Core-vs-plugin architect
**Clasificación global:** 58% CORE · 30% PLUGIN · 8% HYBRID · 4% CONFIG.

**Reclassificaciones aplicadas al roadmap:**
- M2.4 calendar → PLUGIN BUILTIN
- M3.2 focus page → PLUGIN BUILTIN
- M3.3 bell → PLUGIN BUILTIN
- M5.2 dep graph → PLUGIN BUILTIN
- Analytics dashboard (hoy en core) → PROMOVER a PLUGIN BUILTIN
- M7.2 exporters exóticos → PLUGINS EXTERNOS
- M11 sync → PLUGINS EXTERNOS (3 variantes)
- `skill install` → PLUGIN EXTERNO
- Notifications, templates, attachments → PLUGINS EXTERNOS

**Capabilities nuevas propuestas (en M8.1):**
- `QueryAccess(scope)` — read-only store filtrado
- `MutationAccess(scope)` — writes auditadas, opt-in
- `Exporter { format_id, mime }`
- `Syncer { name }`
- `Notifier { channel }`
- `CliSubcommand { name, args_spec }`
- `ThemeContributor`
- `Matcher` (opcional)

**Helix test:** `cargo install rondo-tui --no-default-features` debería dar task tracker funcional sin pomodoro/sync/gráficos. Eso valida que el core es minimal.

**Reframing más importante (aplicado):** plugin loader al puesto 5 (post M1.4) en vez de puesto 9 — sino calendar/heatmap/graph nacen en core como deuda permanente.

---

## Notas de implementación cross-cutting

- Cada fase: snapshot test antes → cambio → diff revisado
- DB writes en transacciones con rollback en `?` failure
- Sin `unsafe`
- CHANGELOG.md actualizado por hito
- Tests RW usan `tempfile`, nunca `~/.todo-app/` real
- `cargo bench` baseline antes de M8 para detectar regressions
