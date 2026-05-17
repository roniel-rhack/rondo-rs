# Plugins

Two kinds:

1. **Builtin plugins** — Rust types compiled into the `rondo-tui` binary.
   Implement `rondo_plugin_api::Plugin` directly, registered at startup in
   `main.rs::register_builtin_plugins`. Currently shipped:
   `builtin.pomodoro`, `builtin.bell`, `builtin.calendar`,
   `builtin.focus-page`, `builtin.dep-graph`, `builtin.analytics`.
2. **External plugins** — compiled to `wasm32-wasip1`, dropped into
   `~/.rondo-rs/plugins/<id>/` with a `plugin.toml` + `plugin.wasm`. Loaded
   via `extism` 1.10 at startup.

Both speak the SAME `rondo_plugin_api` types. The only differences are
discovery and where the code lives.

## Authoring a WASM plugin

### 1. Bootstrap

```bash
cargo new --lib my-plugin
cd my-plugin
```

`Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
rondo-plugin-api = { git = "https://github.com/roniel-rhack/rondo-rs", default-features = false }
# or path = "/path/to/rondo-rs/crates/rondo-plugin-api"
extism-pdk = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[profile.release]
opt-level = "s"
lto = true
```

`.cargo/config.toml`:

```toml
[build]
target = "wasm32-wasip1"
```

### 2. The contract

The host calls a single exported function:

```rust
#[plugin_fn]
pub fn handle(input: Json<HostInput>) -> FnResult<Json<PluginResult>> {
    // ...
}
```

Where:

```rust
#[derive(serde::Deserialize)]
struct HostInput {
    action: rondo_plugin_api::PluginAction,
    ctx: rondo_plugin_api::PluginContext,
}
```

Return `PluginResult { view: Option<ViewSpec>, follow_up: Vec<PluginAction> }`.
The host paints the `ViewSpec` and processes `follow_up` calls (e.g. `KvSet`
persisted to `~/.rondo-rs/todo.db::plugin_kv`).

### 3. Minimal example

```rust
use extism_pdk::*;
use rondo_plugin_api::{
    PluginAction, PluginContext, PluginResult,
    view::{Block, ViewKind, ViewSpec},
};

#[derive(serde::Deserialize)]
struct Input { action: PluginAction, ctx: PluginContext }

#[plugin_fn]
pub fn handle(input: Json<Input>) -> FnResult<Json<PluginResult>> {
    let Input { action, .. } = input.0;
    let view = match action {
        PluginAction::Show => Some(ViewSpec {
            kind: ViewKind::Page,
            blocks: vec![
                Block::Heading { text: "Hello".into(), level: 1 },
                Block::Paragraph { text: "World".into(), style: None },
            ],
        }),
        _ => None,
    };
    Ok(Json(PluginResult { view, follow_up: vec![] }))
}
```

### 4. Manifest (`plugin.toml`)

```toml
id = "my-plugin"
name = "My Plugin"
version = "0.1.0"
api = "0.1"
capabilities = ["OverlayView", "TickHandler"]

[wasi]
allowed_paths = []
allowed_hosts = []

# optional, declares a custom exporter
# [exporter]
# format_id = "myformat"
# mime = "text/x-my"

# optional, syncer plugins declare their name
# [syncer]
# name = "my-sync"

# optional, CLI subcommand contribution
# [cli]
# name = "myplugin"
# args_spec = []
```

### 5. Build

```bash
rustup target add wasm32-wasip1
cargo build --release
cp target/wasm32-wasip1/release/my_plugin.wasm plugin.wasm
```

### 6. Install

```bash
rondo-tui plugins install /path/to/my-plugin
```

That copies the directory into `~/.rondo-rs/plugins/my-plugin/`. Verify:

```bash
rondo-tui plugins list
rondo-tui plugins info my-plugin
```

### 7. Grant permissions

If your `plugin.toml` declares dangerous capabilities (MutationAccess,
Syncer, Notifier, CliSubcommand), the host will load you with
`enabled: false` until the user adds you to `~/.rondo-rs/config.toml`:

```toml
[plugins.permissions]
"my-plugin" = ["mutation_access", "notifier"]
```

`QueryAccess`, `OverlayView`, `TickHandler`, `PageView`, `Exporter`,
`ThemeContributor` are auto-granted.

## Capability cheat sheet

| Capability | Auto-granted? | Effect |
|---|---|---|
| `OverlayView` | yes | Plugin may return an Overlay `ViewSpec` for a transient float |
| `PageView` | yes | Plugin may return a Page `ViewSpec` for full-screen render |
| `TickHandler` | yes | Plugin receives `PluginAction::Tick { delta_ms }` |
| `CommandContributor` | yes | Plugin may declare CLI/palette commands |
| `QueryAccess(scope)` | yes | Host honors `Query` follow-ups; future scopes: Tasks, Journal, FocusSessions, Deps, All |
| `Exporter` | yes | Manifest's `[exporter]` registers a format the CLI's `export --format` dispatches to |
| `ThemeContributor` | yes | Plugin can supply Theme presets (future host wiring) |
| `MutationAccess(scope)` | NO | Plugin can issue mutation follow-ups; user grant required |
| `Syncer` | NO | Plugin acts as a sync backend; user grant required |
| `Notifier(channel)` | NO | Plugin emits desktop/system/audio notifications; user grant required |
| `CliSubcommand` | NO | Plugin contributes a `rondo-tui <name>` subcommand; user grant required |

## Plugin <-> host communication

```
host  --PluginAction::Show-->                            plugin
host  <--PluginResult{ view, follow_up }--               plugin
host  process(follow_up)
       ├─ KvSet { key, value } → SqliteStore.kv_set(plugin_id, key, value)
       ├─ KvGet { key }        → (TODO: round-trip via host-function)
       ├─ Query { scope, params } → (TODO: host data shipped back)
       └─ Notify { channel, message } → dispatched to a Notifier plugin
```

Plugins are isolated by the extism WASI sandbox: no host filesystem unless
`[wasi].allowed_paths` lists it, no network unless `[wasi].allowed_hosts`
lists it, and no shared memory between plugins. The `plugin_kv` table is
namespaced by `plugin_id` so two plugins can't read each other's blobs.

## Builtin plugin pattern

For Rust-internal plugins (no WASM), implement the trait directly:

```rust
pub struct MyBuiltin { /* fields */ }

impl rondo_plugin_api::Plugin for MyBuiltin {
    fn manifest(&self) -> rondo_plugin_api::PluginManifest { /* ... */ }
    fn handle(&mut self, action: rondo_plugin_api::PluginAction,
              ctx: &rondo_plugin_api::PluginContext) -> rondo_plugin_api::PluginResult {
        /* ... */
    }
}
```

Register in `crates/rondo-tui/src/main.rs::register_builtin_plugins`:

```rust
app.plugins.register(Box::new(MyBuiltin::new(app.data.store.clone())));
```

Builtin plugins can take `Arc<SqliteStore>` directly because they're
in-process and trusted.

## Sample plugins shipped

| Path | Capabilities | Status |
|---|---|---|
| `examples/plugins/quote-of-the-day/` | OverlayView + TickHandler + CommandContributor | real `.wasm` checked in (201 KiB); `:quote-of-the-day` opens a quote overlay |
| `examples/plugins/exporter-org-mode/` | Exporter | scaffold + manifest only |
| `examples/plugins/sync-localdir/` | Syncer + TickHandler + CommandContributor | real `.wasm` checked in (210 KiB); `:sync-now` forces a sync attempt |

## Invoking external plugins from the TUI

`rondo-tui plugins install <dir>` copies the plugin to
`~/.rondo-rs/plugins/<id>/`. On the next TUI start the runtime calls
`PluginHost::load_from_dir` automatically (see
`crates/rondo-tui/src/main.rs::load_external_plugins`) and the plugin
is reachable from the command palette via the `name` declared in its
`[cli]` block. Resolution is prefix-aware: typing a unique prefix
(e.g. `quo` for `quote-of-the-day`) is enough; ambiguous prefixes
toast the list of candidates. The host routes the `Show` response by
`ViewKind`: `Overlay` lands in `modals.plugin_overlay`, `Page` reuses
`modals.plugin_page`, `view: None` produces a toast (the plugin's
`Notify` follow-up message wins over the generic "invoked" string).
