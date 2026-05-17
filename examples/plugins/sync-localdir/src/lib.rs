//! sync-localdir: minimal Syncer plugin demonstrating the contract.
//!
//! - On `Tick`, when the current minute-of-hour is a multiple of 5, emits a
//!   `KvSet` follow-up recording `last_sync_at` plus a `Notify(System)`
//!   message indicating it would copy the DB into `~/.rondo-rs/sync/`.
//! - On `Show` (triggered by the `:sync-now` palette command via the
//!   `CommandContributor` capability), forces the same sync attempt
//!   regardless of cadence so users have a manual trigger.
//!
//! For now the actual copy is performed by the HOST in response to the
//! sentinel `KvSet` — real WASI file I/O is deferred to a follow-up so
//! this scaffold stays portable. When extism host-functions for file I/O
//! land, replace the `KvSet` sentinel with a direct `std::fs::copy` call
//! inside the plugin.

use chrono::{DateTime, Utc};
use extism_pdk::*;
use rondo_plugin_api::{NotifyChannel, PluginAction, PluginContext, PluginResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Input {
    action: PluginAction,
    ctx: PluginContext,
}

fn sync_now(now: DateTime<Utc>) -> PluginResult {
    let follow_up = vec![
        PluginAction::KvSet {
            key: "last_sync_at".into(),
            value: now.to_rfc3339().into_bytes(),
        },
        PluginAction::Notify {
            channel: NotifyChannel::System,
            message: format!("sync-localdir: sync triggered {}", now.to_rfc3339()),
        },
    ];
    PluginResult {
        view: None,
        follow_up,
    }
}

#[plugin_fn]
pub fn handle(input: Json<Input>) -> FnResult<Json<PluginResult>> {
    use chrono::Timelike;
    let Input { action, ctx } = input.0;
    let now: DateTime<Utc> = ctx.now;

    let result = match action {
        PluginAction::Show => sync_now(now),
        PluginAction::Hide => PluginResult::default(),
        PluginAction::Tick { delta_ms: _ } => {
            if now.minute() % 5 == 0 {
                sync_now(now)
            } else {
                PluginResult::default()
            }
        }
        _ => PluginResult::default(),
    };

    Ok(Json(result))
}
