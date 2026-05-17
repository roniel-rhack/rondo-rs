//! Capability-grant policy. Plugins that declare "dangerous" capabilities
//! (mutation, syncer, notifier, CLI subcommand) must be explicitly granted
//! permission via `config.plugins.permissions`. When a manifest declares a
//! capability that has not been granted, the host loads the plugin with
//! `enabled = false` and logs a warning telling the user how to grant.

use rondo_plugin_api::Capability;
use std::collections::HashMap;

/// Per-plugin capability grant table.
///
/// Built from [`rondo_core::config::PluginsConfig::permissions`]; the map
/// keys are plugin ids and the values are case-insensitive tokens listing
/// granted capability classes (e.g. `"mutation_access"`, `"notifier"`).
/// Capabilities not in [`needs_grant`] are auto-granted and never appear
/// in the list.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    pub granted: HashMap<String, Vec<String>>,
}

impl Policy {
    pub fn from_config(perms: &HashMap<String, Vec<String>>) -> Self {
        Self {
            granted: perms.clone(),
        }
    }

    /// Returns the capability tokens declared by `caps` that need a grant
    /// but have not yet been granted for `plugin_id`. Empty Vec => OK.
    pub fn missing_for(&self, plugin_id: &str, caps: &[Capability]) -> Vec<String> {
        let granted = self.granted.get(plugin_id);
        let mut missing: Vec<String> = Vec::new();
        for c in caps {
            if !needs_grant(c) {
                continue;
            }
            let token = capability_token(c);
            let allowed = matches!(
                granted,
                Some(list) if list.iter().any(|g| g.eq_ignore_ascii_case(&token))
            );
            if !allowed && !missing.iter().any(|t| t == &token) {
                missing.push(token);
            }
        }
        missing
    }
}

fn needs_grant(c: &Capability) -> bool {
    use Capability::*;
    matches!(c, MutationAccess(_) | Syncer | Notifier(_) | CliSubcommand)
}

fn capability_token(c: &Capability) -> String {
    use Capability::*;
    match c {
        OverlayView => "overlay_view",
        TickHandler => "tick_handler",
        CommandContributor => "command_contributor",
        PageView => "page_view",
        QueryAccess(_) => "query_access",
        MutationAccess(_) => "mutation_access",
        Exporter => "exporter",
        Syncer => "syncer",
        Notifier(_) => "notifier",
        CliSubcommand => "cli_subcommand",
        ThemeContributor => "theme_contributor",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rondo_plugin_api::{MutationScope, NotifyChannel, QueryScope};

    #[test]
    fn empty_grants_block_mutation() {
        let p = Policy::default();
        let miss = p.missing_for("x", &[Capability::MutationAccess(MutationScope::Tasks)]);
        assert_eq!(miss, vec!["mutation_access".to_string()]);
    }

    #[test]
    fn grant_unblocks() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), vec!["mutation_access".to_string()]);
        let p = Policy::from_config(&map);
        let miss = p.missing_for("x", &[Capability::MutationAccess(MutationScope::Tasks)]);
        assert!(miss.is_empty());
    }

    #[test]
    fn read_only_caps_are_free() {
        let p = Policy::default();
        let miss = p.missing_for(
            "x",
            &[
                Capability::OverlayView,
                Capability::QueryAccess(QueryScope::Tasks),
                Capability::TickHandler,
                Capability::CommandContributor,
                Capability::Exporter,
                Capability::ThemeContributor,
            ],
        );
        assert!(miss.is_empty());
    }

    #[test]
    fn token_match_is_case_insensitive() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), vec!["Notifier".to_string()]);
        let p = Policy::from_config(&map);
        let miss = p.missing_for("x", &[Capability::Notifier(NotifyChannel::Desktop)]);
        assert!(miss.is_empty());
    }

    #[test]
    fn duplicates_collapsed() {
        let p = Policy::default();
        let miss = p.missing_for(
            "x",
            &[
                Capability::Notifier(NotifyChannel::Desktop),
                Capability::Notifier(NotifyChannel::System),
            ],
        );
        assert_eq!(miss, vec!["notifier".to_string()]);
    }
}
