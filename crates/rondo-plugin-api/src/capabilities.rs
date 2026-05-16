use serde::{Deserialize, Serialize};

/// Stable capability tags advertised by plugins via their manifest.
///
/// Variants intentionally carry no inline string data so that `Capability`
/// stays `Copy + Hash + Eq` and cheap to round-trip across the host↔guest
/// boundary. Anything that needs identifiers (exporter `format_id`/`mime`,
/// syncer `name`, CLI `name`/`args_spec`) lives on [`crate::PluginManifest`]
/// in dedicated `Option<*Meta>` fields. The host inspects manifest metadata
/// when dispatching to those capabilities.
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
    /// Plugin reads a portion of the host data model.
    QueryAccess(QueryScope),
    /// Plugin requests permission to mutate a portion of the data model.
    /// Mutation is mediated by the host; the plugin only emits intents.
    MutationAccess(MutationScope),
    /// Plugin can export data; host reads [`crate::ExporterMeta`] from manifest.
    Exporter,
    /// Plugin offers a remote sync target; host reads [`crate::SyncerMeta`].
    Syncer,
    /// Plugin can emit notifications on the given channel.
    Notifier(NotifyChannel),
    /// Plugin contributes a CLI subcommand; host reads [`crate::CliMeta`].
    CliSubcommand,
    /// Plugin contributes theme tokens / color overrides.
    ThemeContributor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryScope {
    Tasks,
    Journal,
    FocusSessions,
    Deps,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MutationScope {
    Tasks,
    Journal,
    FocusSessions,
    Deps,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotifyChannel {
    Audio,
    Desktop,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_is_copy_and_hash() {
        // Compile-time check that scoped variants don't break Copy/Hash.
        fn assert_copy<T: Copy + std::hash::Hash + Eq>() {}
        assert_copy::<Capability>();
        let a = Capability::QueryAccess(QueryScope::Tasks);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn capability_round_trips_json() {
        let caps = vec![
            Capability::OverlayView,
            Capability::QueryAccess(QueryScope::All),
            Capability::MutationAccess(MutationScope::Tasks),
            Capability::Notifier(NotifyChannel::Desktop),
            Capability::Exporter,
            Capability::Syncer,
            Capability::CliSubcommand,
            Capability::ThemeContributor,
        ];
        let json = serde_json::to_string(&caps).unwrap();
        let back: Vec<Capability> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, caps);
    }
}
