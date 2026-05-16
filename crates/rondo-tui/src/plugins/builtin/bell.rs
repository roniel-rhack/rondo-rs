use rondo_plugin_api::{
    Capability, NotifyChannel, Plugin, PluginAction, PluginContext, PluginManifest, PluginResult,
};

/// Builtin plugin that rings a terminal bell when the pomodoro session
/// completes. Listens for `PluginAction::Notify { channel: Audio, .. }` and
/// writes ASCII BEL (`\x07`) to stderr — stderr only, so the ratatui
/// alt-screen buffer is undisturbed.
#[derive(Default)]
pub struct BellPlugin;

impl Plugin for BellPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.bell".into(),
            name: "Pomodoro Bell".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![Capability::Notifier(NotifyChannel::Audio)],
            exporter: None,
            syncer: None,
            cli: None,
        }
    }

    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext) -> PluginResult {
        if let PluginAction::Notify {
            channel: NotifyChannel::Audio,
            message,
        } = action
        {
            use std::io::Write;
            eprint!("\x07");
            let _ = std::io::stderr().flush();
            tracing::debug!("bell: {}", message);
        }
        PluginResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_plugin_api::{NotifyChannel, PluginContext};

    #[test]
    fn manifest_declares_audio_notifier() {
        let p = BellPlugin;
        let m = p.manifest();
        assert_eq!(m.id, "builtin.bell");
        assert!(m.capabilities.iter().any(|c| matches!(
            c,
            rondo_plugin_api::Capability::Notifier(NotifyChannel::Audio)
        )));
    }

    #[test]
    fn handles_notify_without_panicking() {
        let mut p = BellPlugin;
        let ctx = PluginContext::new("builtin.bell");
        let r = p.handle(
            PluginAction::Notify {
                channel: NotifyChannel::Audio,
                message: "test".into(),
            },
            &ctx,
        );
        assert!(r.view.is_none());
        assert!(r.follow_up.is_empty());
    }

    #[test]
    fn ignores_non_audio_notify() {
        let mut p = BellPlugin;
        let ctx = PluginContext::new("builtin.bell");
        let _ = p.handle(
            PluginAction::Notify {
                channel: NotifyChannel::Desktop,
                message: "test".into(),
            },
            &ctx,
        );
    }
}
