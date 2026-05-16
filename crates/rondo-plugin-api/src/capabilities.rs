use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Plugin contributes an overlay view (renders on top of pages).
    OverlayView,
    /// Plugin reacts to ticks (e.g. timer).
    TickHandler,
    /// Plugin contributes commands to the palette.
    CommandContributor,
    /// Plugin owns a full page.
    PageView,
}
