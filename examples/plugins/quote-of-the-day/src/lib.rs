//! Quote-of-the-Day sample plugin.
//!
//! Demonstrates the `OverlayView` + `TickHandler` capabilities. Compiled to
//! `wasm32-wasip1` via `build.sh`; the resulting `plugin.wasm` sits next to
//! `plugin.toml` and is picked up by the host's `load_from_dir`.
//!
//! Wire shape: the host serializes `{"action": PluginAction, "ctx":
//! PluginContext}` and calls the `handle` export. We deserialize using the
//! real `rondo_plugin_api` types so the JSON shape is guaranteed to stay in
//! sync with the host. `PluginResult` is serialized back the same way.
//!
//! State: cross-call state is not yet persisted (the host's `KvGet`
//! round-trip is M8.5). For now the visible quote is selected deterministically
//! from `ctx.now`, so the rotation is observable even without local memory.

use extism_pdk::*;
use rondo_plugin_api::{Block, PluginAction, PluginContext, PluginResult, ViewKind, ViewSpec};
use serde::{Deserialize, Serialize};

const QUOTES: &[&str] = &[
    "Make it work, make it right, make it fast.",
    "Premature optimization is the root of all evil.",
    "Simplicity is the ultimate sophistication.",
    "Talk is cheap. Show me the code.",
    "Code is read more often than it is written.",
];

#[derive(Deserialize, Serialize)]
struct Input {
    action: PluginAction,
    ctx: PluginContext,
}

fn pick_quote(ctx: &PluginContext) -> &'static str {
    // Deterministic rotation: minute-of-hour modulo N. Stable for tests and
    // visibly rotating over time without needing host-side state.
    let idx = (ctx.now.timestamp().unsigned_abs() / 60) as usize % QUOTES.len();
    QUOTES[idx]
}

fn build_overlay(quote: &str) -> ViewSpec {
    ViewSpec {
        kind: ViewKind::Overlay,
        blocks: vec![
            Block::Heading {
                text: "Quote of the Day".to_string(),
                level: 1,
            },
            Block::Divider,
            Block::Paragraph {
                text: quote.to_string(),
                style: None,
            },
        ],
    }
}

#[plugin_fn]
pub fn handle(input: Json<Input>) -> FnResult<Json<PluginResult>> {
    let Input { action, ctx } = input.0;
    let result = match action {
        PluginAction::Show => PluginResult {
            view: Some(build_overlay(pick_quote(&ctx))),
            follow_up: Vec::new(),
        },
        PluginAction::Tick { .. } => {
            // No local state to bump yet; the deterministic time-based picker
            // means a subsequent Show will reflect the wall clock anyway.
            PluginResult::default()
        }
        PluginAction::Hide => PluginResult::default(),
        // We don't currently react to other host->plugin actions.
        _ => PluginResult::default(),
    };
    Ok(Json(result))
}
