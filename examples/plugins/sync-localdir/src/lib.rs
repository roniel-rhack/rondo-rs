//! sync-localdir: minimal Syncer plugin demonstrating the contract.
//!
//! On `Tick`, when the current minute-of-hour is a multiple of 5, emits a
//! `KvSet` follow-up recording `last_sync_at` plus a `Notify(System)`
//! message indicating it would copy the DB into `~/.todo-app/sync/`. For
//! now the actual copy is performed by the HOST in response to the
//! sentinel `KvSet` — real WASI file I/O is deferred to a follow-up so
//! this scaffold stays portable.
//!
//! When extism host-functions for file I/O are added, replace the `KvSet`
//! sentinel with a direct `std::fs::copy` call inside the plugin.

use chrono::{DateTime, Timelike, Utc};
use extism_pdk::*;
use rondo_plugin_api::{NotifyChannel, PluginAction, PluginContext, PluginResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Input {
    action: PluginAction,
    ctx: PluginContext,
}

#[plugin_fn]
pub fn handle(input: Json<Input>) -> FnResult<Json<PluginResult>> {
    let Input { action, ctx } = input.0;
    let now: DateTime<Utc> = ctx.now;

    let result = match action {
        PluginAction::Show | PluginAction::Hide => PluginResult::default(),
        PluginAction::Tick { delta_ms: _ } => {
            // Stateless cadence demo: if minute-of-hour is a multiple of 5,
            // ask the host to record a sync attempt. A real plugin would
            // accumulate elapsed time via KvGet/KvSet instead of sampling
            // wall-clock.
            if now.minute() % 5 == 0 {
                let follow_up = vec![
                    PluginAction::KvSet {
                        key: "last_sync_at".into(),
                        value: now.to_rfc3339().into_bytes(),
                    },
                    PluginAction::Notify {
                        channel: NotifyChannel::System,
                        message: format!("sync-localdir: tick {}", now.to_rfc3339()),
                    },
                ];
                PluginResult {
                    view: None,
                    follow_up,
                }
            } else {
                PluginResult::default()
            }
        }
        _ => PluginResult::default(),
    };

    Ok(Json(result))
}
