# rondo-rust

MVP en Rust + ratatui para evaluar si reescribir el [rondo Go](https://github.com/roniel-rondo/rondo) en Rust mejora la calidad visual de la TUI. Read-only contra el SQLite del Go (`~/.todo-app/todo.db`).

## Estado

MVP de 3 vistas + overlay pomodoro animada + arquitectura plugin-ready desde el día 1.

- `crates/rondo-core` — dominio + `SqliteStore` (READ_ONLY)
- `crates/rondo-plugin-api` — contrato estable `trait Plugin` + `ViewSpec` serializable (futuro extism/wasmtime)
- `crates/rondo-tui` — binario ratatui (Component/Action/Reducer)

## Run

```bash
cargo run -p rondo-tui                          # apunta a ~/.todo-app/todo.db (datos reales)
cargo run -p rondo-tui -- --db ./fixture.db    # DB custom
RUST_LOG=debug cargo run -p rondo-tui          # logs a stderr
```

### Fixture DB para pruebas sin tocar datos reales

```bash
FIXTURE=$(mktemp).db
sqlite3 "$FIXTURE" < fixtures/seed.sql
cargo run -p rondo-tui -- --db "$FIXTURE"
```

## Keybindings

| Tecla | Acción |
|---|---|
| `j` / `k` o `↓` / `↑` | Navegar lista |
| `Tab` | Cambiar focus de panel |
| `1` / `2` | Cambiar página (Tasks / Journal) |
| `p` | Toggle Pomodoro overlay |
| `:` | Abrir command palette |
| `<` / `>` | Resize del split horizontal |
| `Esc` | Cerrar overlay |
| `q` o `Ctrl+C` | Salir |

Dentro del command palette: escribe `tasks`, `journal`, `pomodoro`, `quit` y `Enter`.

## Build / test

```bash
cargo build --workspace
cargo test --workspace      # 9 tests
cargo clippy --workspace -- -D warnings
```

## Comparación visual con la versión Go

1. Tomar screenshot de `rondo` (Go) en task list / task detail / journal / pomodoro.
2. Tomar screenshot de `rondo-tui` (Rust) en los mismos contextos.
3. Side-by-side. Si la versión Rust no se ve **claramente** mejor en ≥3 vistas → archivar el rewrite.

## Diseño y trade-offs

- Plugin de pomodoro built-in (`crates/rondo-tui/src/plugins/builtin/pomodoro.rs`) demuestra que el contrato del API funciona: produce `ViewSpec` serializable que un host WASM podría renderizar igual.
- Sin tokio (loop síncrono con tick de 100ms). Suficiente para un task TUI personal.
- Sin `unsafe`.

Plan detallado: `/Users/roniel/.claude/plans/construyamos-un-mvp-enfocandonos-resilient-lemon.md`.
Análisis de diseño completo: [DESIGN.md](DESIGN.md).
