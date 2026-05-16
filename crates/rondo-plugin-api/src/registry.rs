use crate::plugin::{Plugin, PluginManifest};
use std::collections::HashMap;

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.manifest().id;
        self.plugins.insert(id, plugin);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    pub fn iter_manifests(&self) -> impl Iterator<Item = PluginManifest> + '_ {
        self.plugins.values().map(|p| p.manifest())
    }

    pub fn ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
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
        fn manifest(&self) -> PluginManifest {
            PluginManifest {
                id: "dummy".into(),
                name: "Dummy".into(),
                version: "0.1.0".into(),
                api_version: env!("CARGO_PKG_VERSION").into(),
                capabilities: vec![Capability::OverlayView],
                exporter: None,
                syncer: None,
                cli: None,
            }
        }
        fn handle(&mut self, _: PluginAction, _: &PluginContext) -> PluginResult {
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
        let ctx = PluginContext::new("dummy");
        let r = reg
            .get_mut("dummy")
            .unwrap()
            .handle(PluginAction::Show, &ctx);
        assert!(r.view.is_some());
        let manifest_ids: Vec<String> = reg.iter_manifests().map(|m| m.id).collect();
        assert_eq!(manifest_ids, vec!["dummy".to_string()]);
    }
}
