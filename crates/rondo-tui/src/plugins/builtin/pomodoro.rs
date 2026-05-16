use rondo_plugin_api::{
    action::PluginAction,
    capabilities::Capability,
    plugin::{Plugin, PluginContext, PluginMeta, PluginResult},
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
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: "builtin.pomodoro",
            name: "Pomodoro",
            version: "0.1.0",
            capabilities: &[
                Capability::OverlayView,
                Capability::TickHandler,
                Capability::CommandContributor,
            ],
        }
    }

    fn handle(&mut self, action: PluginAction, _ctx: &PluginContext<'_>) -> PluginResult {
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
                        text: "Focus".into(),
                        level: 1,
                    },
                    Block::Gauge {
                        ratio,
                        label: Some(format!("{:.0}%", ratio * 100.0)),
                    },
                    Block::Paragraph {
                        text: format!("{}s remaining", (self.total_ms - self.elapsed_ms) / 1000),
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
        let ctx = PluginContext::now();
        let r = p.handle(PluginAction::Show, &ctx);
        assert!(r.view.is_some());
        let r = p.handle(PluginAction::Tick { delta_ms: 5000 }, &ctx);
        let v = r.view.unwrap();
        assert!(matches!(v.kind, ViewKind::Overlay));
    }
}
