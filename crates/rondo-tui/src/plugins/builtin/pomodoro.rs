use rondo_core::i18n;
use rondo_plugin_api::{
    action::PluginAction,
    capabilities::Capability,
    plugin::{Plugin, PluginContext, PluginManifest, PluginResult},
    view::{Block, ColorToken, TextStyle, ViewKind, ViewSpec},
};

pub struct PomodoroPlugin {
    elapsed_ms: u64,
    total_ms: u64,
    running: bool,
}

impl PomodoroPlugin {
    pub fn new() -> Self {
        Self {
            elapsed_ms: 0,
            total_ms: 25 * 60 * 1000,
            running: false,
        }
    }
}

impl Default for PomodoroPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PomodoroPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "builtin.pomodoro".into(),
            name: "Pomodoro".into(),
            version: "0.1.0".into(),
            api_version: env!("CARGO_PKG_VERSION").into(),
            capabilities: vec![
                Capability::OverlayView,
                Capability::TickHandler,
                Capability::CommandContributor,
            ],
            exporter: None,
            syncer: None,
            cli: None,
        }
    }

    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext) -> PluginResult {
        match action {
            PluginAction::Show => {
                self.running = true;
                self.elapsed_ms = 0;
            }
            PluginAction::Hide => self.running = false,
            PluginAction::Tick { delta_ms } if self.running => {
                self.elapsed_ms = (self.elapsed_ms + delta_ms as u64).min(self.total_ms);
            }
            _ => {}
        }
        let ratio = self.elapsed_ms as f64 / self.total_ms as f64;
        PluginResult {
            view: self.running.then(|| ViewSpec {
                kind: ViewKind::Overlay,
                blocks: vec![
                    Block::Heading {
                        text: i18n::t("pomodoro_plugin.heading"),
                        level: 1,
                    },
                    Block::Gauge {
                        ratio,
                        label: Some(format!("{:.0}%", ratio * 100.0)),
                    },
                    Block::Paragraph {
                        text: format!(
                            "{}{}",
                            (self.total_ms - self.elapsed_ms) / 1000,
                            i18n::t("pomodoro_plugin.remaining_suffix")
                        ),
                        style: Some(TextStyle {
                            fg: Some(ColorToken::Accent),
                            ..Default::default()
                        }),
                    },
                ],
            }),
            follow_up: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pomodoro_plugin_round_trip() {
        let mut p = PomodoroPlugin::new();
        let ctx = PluginContext::new("builtin.pomodoro");
        let r = p.handle(PluginAction::Show, &ctx);
        assert!(r.view.is_some());
        let r = p.handle(PluginAction::Tick { delta_ms: 5000 }, &ctx);
        let v = r.view.unwrap();
        assert!(matches!(v.kind, ViewKind::Overlay));
    }

    #[test]
    fn pomodoro_manifest_has_expected_caps() {
        let p = PomodoroPlugin::new();
        let m = p.manifest();
        assert_eq!(m.id, "builtin.pomodoro");
        assert!(m.capabilities.contains(&Capability::TickHandler));
        assert!(m.exporter.is_none());
    }
}
