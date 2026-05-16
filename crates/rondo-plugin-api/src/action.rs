use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginAction {
    Show,
    Hide,
    Tick { delta_ms: u32 },
    Command { name: String, args: Vec<String> },
    KeyPress { key: String },
}
