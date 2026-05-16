use crate::action::PluginAction;
use crate::capabilities::Capability;
use crate::view::ViewSpec;

#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub capabilities: &'static [Capability],
}

pub struct PluginContext<'a> {
    pub now: chrono::DateTime<chrono::Utc>,
    pub _hidden: std::marker::PhantomData<&'a ()>,
}

impl PluginContext<'_> {
    pub fn now() -> Self {
        Self {
            now: chrono::Utc::now(),
            _hidden: std::marker::PhantomData,
        }
    }
}

pub trait Plugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
    fn handle(&mut self, action: PluginAction, ctx: &PluginContext<'_>) -> PluginResult;
}

#[derive(Debug, Default)]
pub struct PluginResult {
    pub view: Option<ViewSpec>,
    pub follow_up: Vec<PluginAction>,
}
