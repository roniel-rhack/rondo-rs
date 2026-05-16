# RonDO-Rust MVP — Plan de Mejoras (post-review)

Síntesis de 4 auditorías paralelas: UX/UI, ratatui idiomatic, TUI conventions, Visual design. Cada hallazgo cita el archivo o snapshot original.

**Estado actual:** MVP funcional, 9 tests verdes, clippy limpio. Pero los 4 agentes coinciden en que **se ve como demo default de ratatui**, no como producto diseñado. Hay además 3 bugs reales (no cosméticos): `ListState` reconstruido cada frame pierde scroll, throbber congelado en frame uno, render loop quema CPU a 10fps en idle.

Snapshots de referencia: `review-snapshots/01-tasks.txt` ... `08-wide-160x40.txt`.

---

## Bugs reales (no estéticos)

| # | Bug | Archivo | Por qué pasa |
|---|---|---|---|
| B1 | `ListState::default()` reconstruido cada draw — pierde scroll offset en listas largas | `task_list.rs:87-88`, `journal.rs:51` | State local declarado dentro de `draw()` |
| B2 | `ThrobberState` congelado en frame uno — `(elapsed%1000)%10` casi siempre 0 | `pomodoro.rs:61-64` | Throbber espera state persistente, no re-construido |
| B3 | Render loop dibuja a 10 fps en idle | `main.rs:56-73` | Tick fuerza redraw incondicional |
| B4 | `Esc` siempre dispara `ClosePomodoro` aunque pomodoro cerrado | `event.rs:54` | No-op accidental hoy; rompe cuando llegue otro modal |
| B5 | Command palette se solapa con bordes del panel Tasks | `06-command-palette.txt:20,31`, `root.rs:57-65` | `palette_rect` no garantiza margen |
| B6 | `CLAUDE.md:26` afirma "keybinding parity" — solo paridad de navegación | `CLAUDE.md` vs `internal/app/keys.go:35-142` | 15+ bindings del Go ausentes |

---

## Visión consolidada

Tres ejes para llevar de "demo default" a "producto deliberado":

1. **Restringir REVERSED.** Hoy lo usan header, badges (priority/due/blocked), tags, footer chips, selección. Resultado: stripe noise. Reservar `REVERSED` solo para tab activo y fila seleccionada.
2. **Paleta cohesiva 7 tokens, no Material Design rainbow.** Material da 5 hues compitiendo en una fila. Reducir a 7 roles semánticos. Un solo rojo (urgent+overdue+blocked colapsados), un accent frío único.
3. **Información contextual sobre densidad.** Footer muestra los mismos 7 hints en todas las páginas; debería cambiar por contexto + offload a `?` help overlay.

---

## Plan por fases

Cada fase produce un commit verificable. TDD: snapshot tests con `insta` antes de cambiar render (Fase 0).

### Fase 0 — Guardrails (1 día)

**Objetivo:** Habilitar refactors visuales sin romper. Ya hay `insta` en `dev-dependencies` pero no se usa.

- [ ] **0.1 Migrar `tests/render_smoke.rs` a `insta::assert_snapshot!`**
  - Snapshot full-buffer para cada vista (tasks, journal, pomodoro, command-palette, narrow 80x24, wide 160x40).
  - Reemplazar `buf.contains("RonDO")` por `insta::assert_snapshot!(term.backend())`.
  - Aceptar baselines con `cargo insta accept`.
- [ ] **0.2 Trim de features ratatui**
  - `Cargo.toml:14` quitar `all-widgets` y `macros` — no se usan ni `Tabs`, `Table`, `Chart`, `Canvas`, `Sparkline`.
  - Sustituir por `default-features = false, features = ["crossterm"]`.

```toml
ratatui = { version = "0.29", default-features = false, features = ["crossterm"] }
```

### Fase 1 — Bugs reales (1-2 días)

**Objetivo:** Cero bugs antes de polish visual.

- [ ] **1.1 `ListState` persistente** (B1)
  - Mover `task_list_state: ListState` y `journal_list_state: ListState` a `AppState`.
  - En `task_list.rs::draw`, pasar `&mut state` desde `app`.
  - Esto exige que `draw` reciba `&mut AppState` — empieza el camino al refactor de Fase 3.
- [ ] **1.2 `ThrobberState` persistente** (B2)
  - Añadir `pomodoro_throbber: ThrobberState` a `AppState`.
  - En `app.rs::update` cuando `Action::Tick` y `pomodoro_open`, hacer `self.pomodoro_throbber.calc_next()` **una vez**.
  - En `pomodoro.rs:61-65` borrar las 4 líneas que recrean el state; usar `&mut app.pomodoro_throbber`.
- [ ] **1.3 Render dirty-flag** (B3)
  - Patrón `Action::Render`: solo redibujar cuando hay acción que mutó estado. Idle = no draw.
  - Mientras `pomodoro_open && running` o overlay animado activo, subir tick rate; en otro caso bajar a evento-driven puro.
  ```rust
  let mut dirty = true;
  loop {
      if dirty { terminal.draw(...)?; dirty = false; }
      if let Some(ev) = poll_event(timeout)? {
          if let Some(a) = map(ev, app) { app.update(a); dirty = true; }
      }
      if app.needs_animation_tick() {
          app.update(Action::Tick); dirty = true;
      }
  }
  ```
- [ ] **1.4 `Esc` context-aware** (B4)
  - En `event.rs:54`, generar `Action::EscapeContext` y resolver en `app.update`: 1° palette, 2° pomodoro, 3° clear `status_msg`.
- [ ] **1.5 `palette_rect` margen seguro** (B5)
  - Refactor a `Layout::flex(Flex::End)` con `margin(1)` para no pegarse al borde del panel.
  ```rust
  use ratatui::layout::Flex;
  fn palette_rect(area: Rect) -> Rect {
      let [_, palette] = Layout::vertical([Constraint::Min(0), Constraint::Length(12)])
          .flex(Flex::End).margin(1).areas(area);
      palette
  }
  ```
- [ ] **1.6 Corregir `CLAUDE.md:26`** (B6)
  - Cambiar "Keybindings: ..." por "**Navigation parity only** — see Fase 4 for the full keybinding plan".

### Fase 2 — Paleta + tipografía (1-2 días)

**Objetivo:** Borrar stripe noise. Una paleta de 7 tokens semánticos.

- [ ] **2.1 Reemplazar `theme.rs::dark()`**
  ```rust
  pub fn dark() -> Self {
      Self {
          bg:               Color::Rgb(0x0F, 0x11, 0x15),
          surface:          Color::Rgb(0x18, 0x1B, 0x22),  // panel inactivo, code block bg
          fg:               Color::Rgb(0xE6, 0xE1, 0xCF),  // warm off-white
          fg_muted:         Color::Rgb(0x5C, 0x63, 0x70),
          accent:           Color::Rgb(0x7F, 0xDB, 0xCA),  // teal pastel, solo para focus/active
          danger:           Color::Rgb(0xFF, 0x6B, 0x6B),  // colapsa urgent + overdue + blocked
          warn:             Color::Rgb(0xE5, 0xC0, 0x7B),  // today + med priority
          success:          Color::Rgb(0x98, 0xC3, 0x79),  // done + low priority
          border_active:    Color::Rgb(0x7F, 0xDB, 0xCA),
          border_inactive:  Color::Rgb(0x2A, 0x2E, 0x36),
      }
  }
  ```
  - Eliminar campo `urgent` (colapsa en `danger`).
  - Añadir `surface` (background diferenciado para panel inactivo + code blocks).
- [ ] **2.2 Helpers en `Theme`**
  ```rust
  impl Theme {
      pub fn kbd(&self) -> Style { Style::default().fg(self.accent).add_modifier(Modifier::BOLD) }
      pub fn badge(&self, color: Color) -> Style { Style::default().fg(color).add_modifier(Modifier::BOLD) }
      pub fn border(&self, active: bool) -> Style {
          Style::default().fg(if active { self.border_active } else { self.border_inactive })
      }
      pub fn selection(&self) -> Style { Style::default().add_modifier(Modifier::REVERSED) }
  }
  ```
  - Reemplazar 30+ `Style::default().fg(t.fg_muted)` por `t.muted()` (ya existe pero no se usa consistentemente).
  - Reemplazar `if app.focus_left { t.border_active } else { t.border_inactive }` por `t.border(app.focus_left)`.

### Fase 3 — Restringir REVERSED + badges suaves (1 día)

Hallazgo unánime de UX y Visual.

- [ ] **3.1 `priority_badge.rs` — sin REVERSED**
  ```rust
  pub fn span(p: Priority, theme: &Theme) -> Span<'static> {
      Span::styled(
          format!("· {} ·", p.label()),  // bullet delimiter en lugar de pillow REVERSED
          Style::default().fg(theme.priority_color(p)).add_modifier(Modifier::BOLD),
      )
  }
  ```
  - `URG!` → `URG` (quitar exclamación performativa).
- [ ] **3.2 `due_badge.rs` — ocultar UPCOMING + fechas relativas**
  - Si `due > today + 3 días`: no mostrar badge. Mostrar `in 2d` / `next week` como texto muted.
  - Solo `OVERDUE` y `TODAY` permanecen como badges con color.
- [ ] **3.3 `header.rs` — título sin REVERSED**
  - Quitar `Modifier::REVERSED` del título `RonDO`.
  - Active tab: `UNDERLINED + BOLD` en lugar de `REVERSED + BOLD`.
- [ ] **3.4 `footer.rs` — chips sin REVERSED**
  - `kbd()` helper: solo `accent + BOLD`, sin `REVERSED`.
  - El contraste con `muted` label ya basta.

### Fase 4 — Keybindings honestos + help overlay (2 días)

Hallazgo TUI: paridad declarada pero no cumplida. Half-vim peor que no-vim.

- [ ] **4.1 Crear `components/help.rs` overlay**
  - Tabla key | context | action.
  - Trigger: `?` global.
  - Z-order: encima de palette y pomodoro.
- [ ] **4.2 Añadir bindings vim completos**
  - `g g` jump top, `G` jump bottom (renombrar futuro `stats` a `S`).
  - `Ctrl+D` / `Ctrl+U` half-page.
  - `h` / `l` focus left/right pane.
  - `Tab` / `Shift+Tab` next/prev page (mover `FocusNext` a `h`/`l`).
- [ ] **4.3 `/` search overlay**
  - Filtra `app.tasks` in-memory por substring fuzzy en title+tags.
  - Highlight matches en title.
- [ ] **4.4 `<`/`>` resize +5% (no 2%) + `=` reset 50/50**
  - `event.rs:52-53` cambiar `delta: ±2` a `±5`.
  - Añadir handler `=` → `Action::ResetSplit`.
- [ ] **4.5 Footer contextual**
  - Mostrar solo 4 hints relevantes al contexto actual (page + focus + modal).
  - Añadir `?` help como hint primero (no `q`).

### Fase 5 — Selección + foco visibles (medio día)

Hallazgo UX P0: en los snapshots no hay forma de saber qué fila está seleccionada cuando no hay color.

- [ ] **5.1 Gutter explícito en lista seleccionada**
  - Prepender `▌` en accent color a la fila seleccionada, `  ` a las otras.
  - Mantener `REVERSED` solo cuando el panel tiene focus; sin focus solo el gutter.
- [ ] **5.2 Foco activo diferenciado más allá de color**
  - `BorderType::Double` (o `Rounded`) en panel activo, `Plain` en inactivo.
  - Funciona en terminales sin truecolor.
- [ ] **5.3 Active-tab caret superior**
  - Añadir línea `▔▔▔▔` debajo del active tab en el header (en lugar de invertir).

### Fase 6 — Empty + error states (1 día)

Hallazgo unánime: hoy no existen.

- [ ] **6.1 `task_list.rs` empty**
  ```rust
  if app.tasks.is_empty() {
      let msg = vec![
          Line::from(""),
          Line::from(Span::styled("  No tasks yet", t.muted())),
          Line::from(""),
          Line::from(vec![Span::raw("  Press "), Span::styled("?", t.kbd()), Span::raw(" for help")]),
      ];
      f.render_widget(Paragraph::new(msg).block(block).alignment(Alignment::Center), area);
      return;
  }
  ```
- [ ] **6.2 `task_detail.rs` `No task selected` con acción**
  - Mensaje + hint `j/k navigate · / search · ? help`.
- [ ] **6.3 `journal.rs` `(no entries this day)` con CTA**
- [ ] **6.4 Error banner persistente**
  - Si `app.error.is_some()`, banner danger sobre header con prompt de reintento.

### Fase 7 — Pomodoro real (1-2 días)

Hallazgo UX P1: hoy se ve como loading spinner, no como "modo enfoque".

- [ ] **7.1 Big-text timer**
  - Añadir `tui-big-text = "0.6"` (de los mismos autores de ratatui).
  - Render `25:00` como 5 líneas ASCII grandes en accent.
- [ ] **7.2 Dim backdrop**
  - Cuando pomodoro abierto, redibujar el frame base con un overlay `Block::default().style(Style::default().fg(t.fg_muted))` para "atenuar" todo lo que no es el modal.
- [ ] **7.3 Phase + round indicator**
  - Subtítulo "Focus 2/4" o "Short Break".
  - Color: focus=accent, break=success.
- [ ] **7.4 Quitar throbber animado**
  - Throbber semánticamente = "loading". Pulse del color en el timer ya transmite "vivo".
- [ ] **7.5 Quitar emoji 🍅**
  - Rompe alineamiento monospace. Sustituir por `◉ FOCUS` o `[F]`.
- [ ] **7.6 Wire `:pomodoro` ya funciona; añadir `p pause`, `r reset`, `s skip`**

### Fase 8 — Refactor estructural (3-5 días — opcional, post-MVP)

Hallazgo ratatui: si vamos a seguir creciendo, AppState god-struct + free-fn components no escala.

- [ ] **8.1 Partir `AppState`**
  ```rust
  pub struct AppState {
      pub data: DataState,       // tasks, journal_*, store
      pub ui: UiState,           // page, focus, selection, split_ratio, list states
      pub modals: ModalStack,    // VecDeque<Box<dyn Component>>
      pub plugins: PluginRegistry,
      pub theme: Theme,
      pub should_quit: bool,
  }
  ```
- [ ] **8.2 `trait Component`**
  ```rust
  pub trait Component {
      fn draw(&mut self, f: &mut Frame<'_>, area: Rect, ctx: &DrawCtx);
      fn handle_key(&mut self, k: KeyEvent, ctx: &mut EventCtx) -> EventOutcome { EventOutcome::Ignored }
      fn tick(&mut self, _ctx: &mut EventCtx) {}
  }
  pub enum EventOutcome { Consumed, Ignored, Action(Action) }
  ```
- [ ] **8.3 Modal stack route input top-down**
  - Eliminar peeking de `app.command_palette_open` en `event::map`. Cada componente decide si consume o ignora.
- [ ] **8.4 `PluginRegistry::tick_all(ctx, delta)` helper**
  - Reemplazar el doble-borrow en `app.rs:144-154` con una sola llamada.

### Fase 9 — "Signature visual element" (1 día)

Hallazgo Visual: necesita un elemento reconocible.

- [ ] **9.1 Priority spine**
  - Columna vertical de 1 char a la izquierda del task list, pintada continua según prioridad:
    - `▌` urgent (danger)
    - `▍` high (danger dim)
    - `▎` med (warn)
    - `▏` low (success)
  - Lectura agregada de prioridades de un vistazo, sin parsear badges.
  - Widget nuevo: `widgets/priority_spine.rs`.

### Fase 10 — Markdown + journal polish (1 día)

Hallazgo Visual: `to_uppercase` headings es anti-pattern.

- [ ] **10.1 Quitar `to_uppercase()` en `widgets/markdown.rs:57-58`**
- [ ] **10.2 Prefijos para headings**
  - H1: `▍▍ ` + accent BOLD
  - H2: `▎ ` + accent
  - H3: `▏ ` + fg BOLD
- [ ] **10.3 Code blocks con `bg = surface`**
  - Padding lateral de 1 char, distintivo del body.
- [ ] **10.4 Listas con `·` en muted en lugar de `•` en accent**

### Fase 11 — Narrow degradation (1 día)

Hallazgo UX P1: en 80x24 trunca sin elipsis ni indicador.

- [ ] **11.1 Breakpoint < 100 cols**
  - Single-pane mode: lista full-width, detalle como overlay full-screen activado con `Enter`.
- [ ] **11.2 Elipsis en truncación**
  - Cuando título de task no cabe, terminar en `…` en lugar de cortar.

---

## Top 10 cambios prioritarios (effort:reward)

Si solo se va a hacer una parte, hacer estos en este orden:

1. **Fase 0** — Guardrails con insta snapshots (sin esto los demás cambios son ciegos).
2. **Bug B2** — Throbber persistente (visible en demos, fácil fix).
3. **Bug B3** — Render dirty-flag (deja de quemar CPU).
4. **Fase 5.1** — Gutter `▌` en selección (P0 UX, 5 líneas de código).
5. **Fase 6** — Empty states en tasks/detail/journal (transforma primera impresión).
6. **Fase 3** — Restringir REVERSED + UPCOMING oculto (limpieza masiva).
7. **Fase 2** — Paleta 7 tokens + helpers en Theme.
8. **Fase 4.1** — `?` help overlay (descubribilidad).
9. **Fase 9** — Priority spine (signature element).
10. **Fase 7** — Pomodoro big-text + dim backdrop (sube percepción de calidad).

Refactor Fase 8 (Component trait + ModalStack) solo si se va a continuar más allá del MVP — para validación visual no es necesario.

---

## Reglas de spacing (norma explícita)

1. **1 línea en blanco** entre secciones semánticas.
2. **2 espacios** entre label y valor.
3. **3 espacios** entre badges consecutivos.
4. **Padding interno de badge**: 1 espacio a cada lado solo si el badge usa color de fondo; 0 si solo es color de texto.
5. **1 char de gutter izquierdo** en todo panel — contenido empieza en `inner.x + 1`, nunca pegado al border.

---

## Verificación post-mejoras

- [ ] `cargo test --workspace` verde (insta snapshots aceptados intencionadamente).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` limpio.
- [ ] Re-correr `cargo test -p rondo-tui --test snapshot_dump dump_all_views` y revisar visualmente `review-snapshots/*.txt` actualizado.
- [ ] Side-by-side screenshot vs `rondo` Go: la versión Rust debe verse claramente mejor en ≥3 vistas.
- [ ] Verificar en terminales reales: iTerm2, Alacritty, kitty, Terminal.app (sin truecolor).

---

## Archivos clave a tocar (resumen)

| Archivo | Fases |
|---|---|
| `crates/rondo-tui/src/theme.rs` | 2 |
| `crates/rondo-tui/src/main.rs` | 1.3 |
| `crates/rondo-tui/src/app.rs` | 1.1, 1.2, 1.4, 8.1 |
| `crates/rondo-tui/src/event.rs` | 1.4, 4.x |
| `crates/rondo-tui/src/components/root.rs` | 1.5, 11 |
| `crates/rondo-tui/src/components/header.rs` | 3.3, 5.3 |
| `crates/rondo-tui/src/components/footer.rs` | 3.4, 4.5 |
| `crates/rondo-tui/src/components/task_list.rs` | 1.1, 5.1, 5.2, 6.1, 9.1 |
| `crates/rondo-tui/src/components/task_detail.rs` | 6.2 |
| `crates/rondo-tui/src/components/journal.rs` | 1.1, 6.3 |
| `crates/rondo-tui/src/components/pomodoro.rs` | 1.2, 7.x |
| `crates/rondo-tui/src/components/command_palette.rs` | 1.5 |
| `crates/rondo-tui/src/components/help.rs` *(nuevo)* | 4.1 |
| `crates/rondo-tui/src/widgets/priority_badge.rs` | 3.1 |
| `crates/rondo-tui/src/widgets/due_badge.rs` | 3.2 |
| `crates/rondo-tui/src/widgets/markdown.rs` | 10 |
| `crates/rondo-tui/src/widgets/priority_spine.rs` *(nuevo)* | 9 |
| `Cargo.toml` | 0.2, 7.1 |
| `CLAUDE.md` | 1.6 |
| `tests/render_smoke.rs` → `tests/snapshots.rs` | 0.1 |
