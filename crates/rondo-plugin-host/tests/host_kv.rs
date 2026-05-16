//! Integration test for the KV-follow-up passthrough wiring in
//! [`rondo_plugin_host::host::resolve_plugin_follow_up`].
//!
//! A real wasm plugin is not required: we synthesize a `PluginResult` with
//! `KvSet`/`KvGet` follow-ups and verify the host resolves them against the
//! injected `SqliteStore`.

use rondo_core::store::sqlite::SqliteStore;
use rondo_plugin_api::{PluginAction, PluginResult};
use rondo_plugin_host::host::resolve_plugin_follow_up;

fn fixture() -> (tempfile::NamedTempFile, SqliteStore) {
    let f = tempfile::NamedTempFile::new().unwrap();
    let seed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("seed.sql");
    let conn = rusqlite::Connection::open(f.path()).unwrap();
    conn.execute_batch(&std::fs::read_to_string(seed_path).unwrap())
        .unwrap();
    drop(conn);
    let s = SqliteStore::open_readwrite(f.path()).unwrap();
    (f, s)
}

#[test]
fn kv_set_follow_up_persists_to_store() {
    let (_f, store) = fixture();
    let result = PluginResult {
        view: None,
        follow_up: vec![PluginAction::KvSet {
            key: "counter".into(),
            value: b"42".to_vec(),
        }],
    };
    let resolved = resolve_plugin_follow_up("plug-a", result, Some(&store));
    assert!(
        resolved.follow_up.is_empty(),
        "KvSet should be drained from follow_up"
    );
    let stored = store.kv_get("plug-a", "counter").unwrap();
    assert_eq!(stored.as_deref(), Some(b"42".as_slice()));
}

#[test]
fn kv_get_follow_up_is_dropped() {
    let (_f, store) = fixture();
    let result = PluginResult {
        view: None,
        follow_up: vec![PluginAction::KvGet {
            key: "anything".into(),
        }],
    };
    let resolved = resolve_plugin_follow_up("plug-a", result, Some(&store));
    assert!(
        resolved.follow_up.is_empty(),
        "KvGet round-trip is deferred, must be dropped for now"
    );
}

#[test]
fn other_follow_ups_pass_through() {
    let (_f, store) = fixture();
    let result = PluginResult {
        view: None,
        follow_up: vec![
            PluginAction::KvSet {
                key: "k".into(),
                value: b"v".to_vec(),
            },
            PluginAction::Tick { delta_ms: 100 },
            PluginAction::KvGet { key: "k".into() },
        ],
    };
    let resolved = resolve_plugin_follow_up("p", result, Some(&store));
    assert_eq!(resolved.follow_up.len(), 1);
    assert!(matches!(
        resolved.follow_up[0],
        PluginAction::Tick { delta_ms: 100 }
    ));
}

#[test]
fn kv_set_without_store_is_noop() {
    let result = PluginResult {
        view: None,
        follow_up: vec![PluginAction::KvSet {
            key: "k".into(),
            value: b"v".to_vec(),
        }],
    };
    let resolved = resolve_plugin_follow_up("p", result, None);
    assert!(resolved.follow_up.is_empty());
}

#[test]
fn namespacing_uses_plugin_id_argument() {
    let (_f, store) = fixture();
    let r1 = PluginResult {
        view: None,
        follow_up: vec![PluginAction::KvSet {
            key: "shared".into(),
            value: b"from-pa".to_vec(),
        }],
    };
    let r2 = PluginResult {
        view: None,
        follow_up: vec![PluginAction::KvSet {
            key: "shared".into(),
            value: b"from-pb".to_vec(),
        }],
    };
    let _ = resolve_plugin_follow_up("pa", r1, Some(&store));
    let _ = resolve_plugin_follow_up("pb", r2, Some(&store));
    assert_eq!(
        store.kv_get("pa", "shared").unwrap().as_deref(),
        Some(b"from-pa".as_slice())
    );
    assert_eq!(
        store.kv_get("pb", "shared").unwrap().as_deref(),
        Some(b"from-pb".as_slice())
    );
}
