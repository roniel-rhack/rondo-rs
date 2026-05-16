//! rondo-plugin-host: WASM runtime + loader for `.wasm` plugins built against
//! [`rondo_plugin_api`].
//!
//! In-process builtin plugins continue to live behind
//! [`rondo_plugin_api::PluginRegistry`]. This crate handles the **external**
//! loader path: scan a directory for `plugin.toml` manifests, optionally
//! load a sibling `plugin.wasm` through [`extism`], and route
//! [`PluginAction`](rondo_plugin_api::PluginAction) calls to the plugin's
//! `handle` export.

pub mod error;
pub mod host;
pub mod manifest;

pub use error::HostError;
pub use host::{LoadedPlugin, PluginHost};
pub use manifest::{FsManifest, Wasi};
