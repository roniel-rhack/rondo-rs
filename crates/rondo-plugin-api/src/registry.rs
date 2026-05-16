use crate::plugin::{Plugin, PluginMeta};
use std::collections::HashMap;

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<&'static str, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.meta().id;
        self.plugins.insert(id, plugin);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    pub fn iter_meta(&self) -> impl Iterator<Item = PluginMeta> + '_ {
        self.plugins.values().map(|p| p.meta())
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::PluginAction;
    use crate::capabilities::Capability;
    use crate::plugin::{PluginContext, PluginResult};
    use crate::view::{ViewKind, ViewSpec};

    struct Dummy;
    impl Plugin for Dummy {
        fn meta(&self) -> PluginMeta {
            PluginMeta {
                id: "dummy",
                name: "Dummy",
                version: "0.1.0",
                capabilities: &[Capability::OverlayView],
            }
        }
        fn handle(&mut self, _: PluginAction, _: &PluginContext<'_>) -> PluginResult {
            PluginResult {
                view: Some(ViewSpec {
                    kind: ViewKind::Overlay,
                    blocks: vec![],
                }),
                follow_up: vec![],
            }
        }
    }

    #[test]
    fn register_and_dispatch() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(Dummy));
        assert_eq!(reg.len(), 1);
        let ctx = PluginContext::now();
        let r = reg
            .get_mut("dummy")
            .unwrap()
            .handle(PluginAction::Show, &ctx);
        assert!(r.view.is_some());
    }
}
