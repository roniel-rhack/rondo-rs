# sync-localdir

Minimal Syncer plugin demonstrating the `Capability::Syncer` contract.

## What it does (scaffold, not full implementation)

- On every Tick, if the current minute-of-hour is a multiple of 5,
  records a `last_sync_at` timestamp via the host's KV store and
  emits a `Notify(System)` message.

## What it does NOT do (yet)

- Actual file copy of the SQLite DB. The intent is: when extism
  host-functions for filesystem I/O land, this plugin will do
  `std::fs::copy(db_path, sync_dir/db-<ts>.sqlite)`. For now it is a
  placeholder for the contract surface.

## Permissions

`Syncer`, `Notifier` and `CliSubcommand` are "dangerous" capabilities
in the host policy and require an explicit grant in
`~/.rondo-rs/config.toml`:

```toml
[plugins.permissions]
"sync-localdir" = ["syncer", "notifier", "tick_handler"]
```

Without this, the policy in `rondo-plugin-host` will disable the
plugin at load time and emit a warn-level trace.

## Build

```bash
cd examples/plugins/sync-localdir
rustup target add wasm32-wasip1 2>/dev/null || true
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/sync_localdir.wasm plugin.wasm
```

## Status

Scaffold. Open issues / follow-ups:

- Actual file-copy host-function bridge.
- Configurable sync interval (currently a hard-coded modulus on `minute`).
- Optional remote target (S3, WebDAV) — those should be separate plugins
  per the M11 reclassification (git-backed sync explicitly abandoned).
