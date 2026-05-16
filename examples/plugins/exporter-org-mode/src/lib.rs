//! Placeholder org-mode exporter.
//!
//! This crate documents the shape a plugin-contributed exporter takes once
//! the wasm host (M8.5) is wired up. Concretely:
//!
//! 1. `plugin.toml` declares `capabilities = ["Exporter"]` and an
//!    `[exporter]` table with `format_id = "org"` + `mime = "text/x-org"`.
//! 2. The host's `rondo_core::export::ExporterRegistry` first checks its
//!    builtins; if `format` is unknown, it walks loaded plugins, picks the
//!    one whose `ExporterMeta.format_id` matches, and calls its `export`
//!    entrypoint with the serialized `&[Task]` payload.
//! 3. The plugin returns a `String` (org-mode text) which the host writes
//!    to stdout exactly like a builtin exporter.
//!
//! Full implementation is deferred to M8.5 (runtime wiring). The
//! scaffolding here keeps the contract surface visible and lets us iterate
//! on the manifest schema before paying the wasm-toolchain cost.
