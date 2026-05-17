use rondo_plugin_host::PluginHost;

#[test]
fn empty_dir_returns_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    let mut host = PluginHost::new();
    let loaded = host.load_from_dir(tmp.path()).unwrap();
    assert!(loaded.is_empty());
    assert!(host.list().is_empty());
}

#[test]
fn missing_dir_does_not_error() {
    let mut host = PluginHost::new();
    let res = host.load_from_dir(std::path::Path::new(
        "/tmp/nonexistent-rondo-plugin-dir-xyz",
    ));
    assert!(res.is_ok());
}

#[test]
fn manifest_only_plugin_loads_without_wasm() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("myplugin");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "myplugin"
version = "0.1.0"
api = "0.1"
capabilities = []
"#,
    )
    .unwrap();
    let mut host = PluginHost::new();
    let loaded = host.load_from_dir(tmp.path()).unwrap();
    assert_eq!(loaded, vec!["myplugin".to_string()]);
    assert_eq!(host.list().len(), 1);
    assert!(!host.get("myplugin").unwrap().has_wasm());
}

#[test]
fn enable_disable() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("p1");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "p1"
version = "0.1.0"
api = "0.1"
capabilities = []
"#,
    )
    .unwrap();
    let mut host = PluginHost::new();
    host.load_from_dir(tmp.path()).unwrap();
    assert!(host.is_enabled("p1"));
    host.set_enabled("p1", false).unwrap();
    assert!(!host.is_enabled("p1"));
    assert!(host.set_enabled("nonexistent", false).is_err());
}

#[test]
fn unsupported_api_version_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("future");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "future"
version = "0.1.0"
api = "9.9"
capabilities = []
"#,
    )
    .unwrap();
    let mut host = PluginHost::new();
    let loaded = host.load_from_dir(tmp.path()).unwrap();
    assert!(loaded.is_empty());
}

#[test]
fn dispatch_to_manifest_only_plugin_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("p1");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.toml"),
        r#"
id = "p1"
version = "0.1.0"
api = "0.1"
capabilities = []
"#,
    )
    .unwrap();
    let mut host = PluginHost::new();
    host.load_from_dir(tmp.path()).unwrap();
    let results = host.dispatch(&rondo_plugin_api::PluginAction::Show);
    assert!(results.is_empty());
}
