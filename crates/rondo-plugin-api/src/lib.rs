//! rondo-plugin-api: stable contract for plugins (future WASM ABI).

pub mod action;
pub mod capabilities;
pub mod plugin;
pub mod registry;
pub mod view;

pub use action::PluginAction;
pub use capabilities::{Capability, MutationScope, NotifyChannel, QueryScope};
pub use plugin::{
    CliMeta, ExporterMeta, Plugin, PluginContext, PluginManifest, PluginResult, SyncerMeta,
};
pub use registry::PluginRegistry;
pub use view::{Block, ColorToken, Span, TextStyle, ViewKind, ViewSpec};
