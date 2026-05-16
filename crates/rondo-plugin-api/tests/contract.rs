use rondo_plugin_api::{
    Capability, MutationScope, NotifyChannel, PluginAction, PluginContext, PluginManifest,
    PluginResult, QueryScope, ViewKind, ViewSpec,
};

#[test]
fn capability_serializes_round_trip() {
    let caps = vec![
        Capability::OverlayView,
        Capability::QueryAccess(QueryScope::Tasks),
        Capability::MutationAccess(MutationScope::Journal),
        Capability::Notifier(NotifyChannel::Audio),
        Capability::Exporter,
        Capability::Syncer,
        Capability::CliSubcommand,
        Capability::ThemeContributor,
    ];
    let json = serde_json::to_string(&caps).unwrap();
    let back: Vec<Capability> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, caps);
}

#[test]
fn plugin_context_serializes() {
    let ctx = PluginContext {
        now: chrono::Utc::now(),
        manifest_id: "foo".into(),
        host_version: "0.1.0".into(),
    };
    let json = serde_json::to_string(&ctx).unwrap();
    let back: PluginContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back.manifest_id, "foo");
    assert_eq!(back.host_version, "0.1.0");
}

#[test]
fn plugin_result_serializes() {
    let r = PluginResult {
        view: Some(ViewSpec {
            kind: ViewKind::Overlay,
            blocks: vec![],
        }),
        follow_up: vec![PluginAction::Tick { delta_ms: 100 }],
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: PluginResult = serde_json::from_str(&json).unwrap();
    assert!(back.view.is_some());
    assert_eq!(back.follow_up.len(), 1);
}

#[test]
fn plugin_action_host_to_plugin_round_trip() {
    let actions = vec![
        PluginAction::Show,
        PluginAction::Hide,
        PluginAction::Tick { delta_ms: 16 },
        PluginAction::Command {
            name: "go".into(),
            args: vec!["a".into(), "b".into()],
        },
        PluginAction::KeyPress { key: "q".into() },
    ];
    for a in actions {
        let json = serde_json::to_string(&a).unwrap();
        let _: PluginAction = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn plugin_action_plugin_to_host_round_trip() {
    let actions = vec![
        PluginAction::Query {
            scope_id: "tasks.list".into(),
            params: serde_json::json!({"status": "open"}),
        },
        PluginAction::KvGet { key: "k".into() },
        PluginAction::KvSet {
            key: "k".into(),
            value: vec![1, 2, 3],
        },
        PluginAction::Notify {
            channel: NotifyChannel::Desktop,
            message: "hi".into(),
        },
    ];
    for a in actions {
        let json = serde_json::to_string(&a).unwrap();
        let _: PluginAction = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn plugin_manifest_round_trip() {
    let m = PluginManifest {
        id: "x".into(),
        name: "X".into(),
        version: "0.1.0".into(),
        api_version: "0.1.0".into(),
        capabilities: vec![
            Capability::OverlayView,
            Capability::QueryAccess(QueryScope::All),
        ],
        exporter: None,
        syncer: None,
        cli: None,
    };
    let json = serde_json::to_string(&m).unwrap();
    let back: PluginManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "x");
    assert_eq!(back.capabilities.len(), 2);
}
