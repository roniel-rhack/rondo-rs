use rondo_plugin_api::{Capability, MutationScope, NotifyChannel, QueryScope};
use rondo_plugin_host::{PluginHost, Policy};
use std::collections::HashMap;

#[test]
fn policy_blocks_mutation_access_without_grant() {
    let granted: HashMap<String, Vec<String>> = HashMap::new();
    let policy = Policy::from_config(&granted);
    let caps = vec![
        Capability::OverlayView,
        Capability::MutationAccess(MutationScope::Tasks),
    ];
    let missing = policy.missing_for("scary-plugin", &caps);
    assert_eq!(missing, vec!["mutation_access".to_string()]);
}

#[test]
fn policy_allows_after_grant() {
    let mut granted: HashMap<String, Vec<String>> = HashMap::new();
    granted.insert("scary-plugin".into(), vec!["mutation_access".into()]);
    let policy = Policy::from_config(&granted);
    let caps = vec![Capability::MutationAccess(MutationScope::Tasks)];
    assert!(policy.missing_for("scary-plugin", &caps).is_empty());
}

#[test]
fn policy_query_access_does_not_need_grant() {
    let policy = Policy::from_config(&HashMap::new());
    let caps = vec![Capability::QueryAccess(QueryScope::Tasks)];
    assert!(policy.missing_for("nice-plugin", &caps).is_empty());
}

#[test]
fn policy_blocks_syncer_and_notifier_and_cli() {
    let policy = Policy::default();
    let miss = policy.missing_for(
        "p",
        &[
            Capability::Syncer,
            Capability::Notifier(NotifyChannel::Desktop),
            Capability::CliSubcommand,
        ],
    );
    assert!(miss.contains(&"syncer".to_string()));
    assert!(miss.contains(&"notifier".to_string()));
    assert!(miss.contains(&"cli_subcommand".to_string()));
}

#[test]
fn host_loads_with_disabled_when_policy_blocks() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("scary");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "scary"
version = "0.1.0"
api = "0.1"
capabilities = [{ MutationAccess = "Tasks" }]
"#,
    )
    .unwrap();
    let mut host = PluginHost::with_policy(Policy::default());
    let loaded = host.load_from_dir(tmp.path()).unwrap();
    assert_eq!(loaded, vec!["scary".to_string()]);
    assert!(!host.is_enabled("scary"));
}

#[test]
fn host_loads_enabled_when_grant_present() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("scary");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "scary"
version = "0.1.0"
api = "0.1"
capabilities = [{ MutationAccess = "Tasks" }]
"#,
    )
    .unwrap();
    let mut perms = HashMap::new();
    perms.insert("scary".to_string(), vec!["mutation_access".to_string()]);
    let mut host = PluginHost::with_policy(Policy::from_config(&perms));
    let loaded = host.load_from_dir(tmp.path()).unwrap();
    assert_eq!(loaded, vec!["scary".to_string()]);
    assert!(host.is_enabled("scary"));
}
