use crate::capabilities::NotifyChannel;
use serde::{Deserialize, Serialize};

/// Bidirectional message envelope between host and plugin.
///
/// - **host → plugin**: `Show`, `Hide`, `Tick`, `Command`, `KeyPress`.
/// - **plugin → host** (emitted via [`crate::PluginResult::follow_up`]):
///   `Query`, `KvGet`, `KvSet`, `Notify`. The host drains those after every
///   `handle()` and dispatches them out-of-band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginAction {
    /// Host: plugin should reveal its overlay / become active.
    Show,
    /// Host: plugin should hide.
    Hide,
    /// Host: clock tick with elapsed delta.
    Tick { delta_ms: u32 },
    /// Host: a palette/CLI command was invoked.
    Command { name: String, args: Vec<String> },
    /// Host: raw key press while plugin owns focus.
    KeyPress { key: String },
    /// Plugin: generic query channel. Host interprets `scope_id` (e.g.
    /// "tasks.list", "journal.today") and answers via a future Plugin
    /// host-call response on the next `handle()`.
    Query {
        scope_id: String,
        params: serde_json::Value,
    },
    /// Plugin: read a key from the host-managed KV store.
    KvGet { key: String },
    /// Plugin: write a key into the host-managed KV store. Host owns
    /// persistence + tx semantics.
    KvSet { key: String, value: Vec<u8> },
    /// Plugin: emit a notification on the given channel.
    Notify {
        channel: NotifyChannel,
        message: String,
    },
}
