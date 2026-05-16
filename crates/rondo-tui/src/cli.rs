//! CLI subcommand dispatch. When `rondo-tui` is invoked with a subcommand
//! (e.g. `rondo-tui list`), we run a one-shot CLI path and exit instead of
//! launching the TUI. Mutation commands (`add`, `done`) require `--write`
//! and follow the same backup + lock-acquire dance as the TUI startup.

use color_eyre::eyre::{eyre, Result};
use rondo_core::domain::task::{NewTask, Status, Task};
use rondo_core::store::sqlite::SqliteStore;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::filter::Filter;

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Add a new task (quick-add syntax)
    Add {
        /// Title (and shorthand tokens, joined with spaces)
        title: Vec<String>,
    },
    /// List visible tasks
    List {
        /// Filter: inbox|today|upcoming|all|urgent|highprio|overdue|notag|done
        #[arg(long, default_value = "all")]
        filter: String,
    },
    /// Mark a task as done by id
    Done {
        id: i64,
    },
    /// Export tasks as markdown/json/ndjson
    Export {
        /// Format: md|json|ndjson
        #[arg(long, default_value = "md")]
        format: String,
    },
    /// Manage plugins installed under ~/.todo-app/plugins/
    Plugins {
        #[command(subcommand)]
        action: PluginsAction,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum PluginsAction {
    /// List all installed plugins
    List,
    /// Show full manifest for a plugin
    Info { id: String },
    /// Install a plugin from a local path (dir containing plugin.toml [+ plugin.wasm])
    Install { path: PathBuf },
    /// Remove a plugin by id
    Remove { id: String },
}

/// Subcommands that do not need a SQLite database. Used by `main.rs` to skip
/// the DB-existence check when only metadata work is requested.
pub fn needs_db(cmd: &Command) -> bool {
    !matches!(cmd, Command::Plugins { .. })
}

/// Shared CLI options used by every subcommand.
pub struct CliOpts {
    pub json: bool,
    pub write: bool,
}

pub fn run(cmd: Command, opts: &CliOpts, db_path: &Path) -> Result<()> {
    match cmd {
        Command::Add { title } => cli_add(db_path, opts, title),
        Command::List { filter } => cli_list(db_path, opts, &filter),
        Command::Done { id } => cli_done(db_path, opts, id),
        Command::Export { format } => cli_export(db_path, opts, &format),
        Command::Plugins { action } => cli_plugins(opts, action),
    }
}

/// Open the store read-write, taking a backup snapshot and acquiring the
/// inter-process lock. Mirrors the dance done in `main.rs` for the TUI.
fn open_rw_store(db_path: &Path) -> Result<(Arc<SqliteStore>, rondo_core::store::lock::LockGuard)> {
    let backup_dir = rondo_core::store::backup::default_backup_dir();
    rondo_core::store::backup::rotate(&backup_dir, 30);
    match rondo_core::store::backup::snapshot(db_path, &backup_dir) {
        Ok(p) => tracing::info!("backup snapshot: {}", p.display()),
        Err(e) => tracing::warn!("backup failed (continuing without): {}", e),
    }
    let lock_path = rondo_core::store::lock::LockGuard::default_path();
    let guard = match rondo_core::store::lock::LockGuard::acquire(lock_path.clone()) {
        Ok(g) => g,
        Err(rondo_core::store::lock::LockError::Conflict(pid)) => {
            return Err(eyre!(
                "another rondo process holds the lock (PID {pid}). If you're sure none is running, remove {}",
                lock_path.display()
            ));
        }
        Err(e) => return Err(e.into()),
    };
    let store = Arc::new(SqliteStore::open_readwrite(db_path)?);
    Ok((store, guard))
}

fn require_write(opts: &CliOpts, action: &str) -> Result<()> {
    if !opts.write {
        eprintln!("error: `{action}` needs --write");
        std::process::exit(2);
    }
    Ok(())
}

fn parse_filter(s: &str) -> Result<Filter> {
    let f = match s.to_ascii_lowercase().as_str() {
        "inbox" => Filter::Inbox,
        "today" | "hoy" => Filter::Today,
        "upcoming" | "proximas" | "próximas" => Filter::Upcoming,
        "all" | "todas" => Filter::All,
        "urgent" | "urgentes" => Filter::Urgent,
        "highprio" | "high" | "alta" => Filter::HighPriority,
        "overdue" | "vencidas" => Filter::Overdue,
        "notag" | "sintag" => Filter::NoTag,
        "done" | "completed" | "completadas" => Filter::Completed,
        other => return Err(eyre!("unknown filter: {other}")),
    };
    Ok(f)
}

fn cli_add(db_path: &Path, opts: &CliOpts, title: Vec<String>) -> Result<()> {
    require_write(opts, "add")?;
    if title.is_empty() {
        return Err(eyre!("add: title required"));
    }
    let title = title.join(" ");
    let (store, _guard) = open_rw_store(db_path)?;
    let (id, _undo) = store.create_task(NewTask::quick(title))?;
    if opts.json {
        println!("{}", serde_json::json!({ "id": id }));
    } else {
        println!("task {id} created");
    }
    Ok(())
}

fn cli_list(db_path: &Path, opts: &CliOpts, filter_s: &str) -> Result<()> {
    let filter = parse_filter(filter_s)?;
    let store = SqliteStore::open_readonly(db_path)?;
    let tasks: Vec<Task> = store
        .list_tasks()?
        .into_iter()
        .filter(|t| filter.applies_to(t))
        .collect();
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&tasks)?);
    } else {
        print_table(&tasks);
    }
    Ok(())
}

fn cli_done(db_path: &Path, opts: &CliOpts, id: i64) -> Result<()> {
    require_write(opts, "done")?;
    let (store, _guard) = open_rw_store(db_path)?;
    store.set_status(id, Status::Done)?;
    if opts.json {
        println!("{}", serde_json::json!({ "id": id, "status": "Done" }));
    } else {
        println!("task {id} done");
    }
    Ok(())
}

fn cli_export(db_path: &Path, _opts: &CliOpts, format: &str) -> Result<()> {
    let store = SqliteStore::open_readonly(db_path)?;
    let tasks = store.list_tasks()?;
    match format {
        "md" | "markdown" => {
            print!("{}", rondo_core::export::to_markdown(&tasks));
        }
        "json" => {
            println!("{}", rondo_core::export::to_json(&tasks)?);
        }
        "ndjson" => {
            let mut stdout = std::io::stdout().lock();
            rondo_core::export::to_ndjson(&tasks, &mut stdout)?;
        }
        other => {
            eprintln!("error: unknown export format `{other}` (expected md|json|ndjson)");
            std::process::exit(2);
        }
    }
    Ok(())
}

fn print_table(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("(no tasks)");
        return;
    }
    println!(
        "{:>4}  {:<12}  {:<6}  {:<40}  {:<10}  TAGS",
        "ID", "STATUS", "PRI", "TITLE", "DUE"
    );
    for t in tasks {
        let due = t
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let tags = if t.tags.is_empty() {
            String::new()
        } else {
            t.tags.join(",")
        };
        let title = truncate(&t.title, 40);
        println!(
            "{:>4}  {:<12}  {:<6}  {:<40}  {:<10}  {}",
            t.id,
            format!("{:?}", t.status),
            t.priority.label(),
            title,
            due,
            tags
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

// ---------------------------------------------------------------------------
// plugins subcommands
// ---------------------------------------------------------------------------

fn default_plugins_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".todo-app")
        .join("plugins")
}

fn cli_plugins(opts: &CliOpts, action: PluginsAction) -> Result<()> {
    let dir = default_plugins_dir();
    match action {
        PluginsAction::List => plugins_list(&dir, opts.json),
        PluginsAction::Info { id } => plugins_info(&dir, &id, opts.json),
        PluginsAction::Install { path } => plugins_install(&dir, &path),
        PluginsAction::Remove { id } => plugins_remove(&dir, &id),
    }
}

fn load_manifests(dir: &Path) -> Vec<(rondo_plugin_host::FsManifest, PathBuf)> {
    let mut out = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for entry in read.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let mf = p.join("plugin.toml");
        if !mf.exists() {
            continue;
        }
        match rondo_plugin_host::FsManifest::load(&mf) {
            Ok(m) => out.push((m, p)),
            Err(e) => eprintln!("warn: failed to read {}: {}", mf.display(), e),
        }
    }
    out.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    out
}

fn plugins_list(dir: &Path, json: bool) -> Result<()> {
    let manifests = load_manifests(dir);
    if json {
        let arr: Vec<serde_json::Value> = manifests
            .iter()
            .map(|(m, p)| {
                serde_json::json!({
                    "id": m.id,
                    "version": m.version,
                    "api": m.api,
                    "capabilities": m.capabilities,
                    "has_wasm": p.join("plugin.wasm").exists(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr)?);
        return Ok(());
    }
    if manifests.is_empty() {
        println!("(no plugins installed in {})", dir.display());
        return Ok(());
    }
    println!(
        "{:<24}  {:<10}  {:<6}  {:<6}  CAPABILITIES",
        "ID", "VERSION", "API", "WASM"
    );
    for (m, p) in &manifests {
        let caps = m
            .capabilities
            .iter()
            .map(|c| format!("{:?}", c))
            .collect::<Vec<_>>()
            .join(",");
        let wasm = if p.join("plugin.wasm").exists() {
            "yes"
        } else {
            "no"
        };
        println!(
            "{:<24}  {:<10}  {:<6}  {:<6}  {}",
            truncate(&m.id, 24),
            truncate(&m.version, 10),
            m.api,
            wasm,
            caps
        );
    }
    Ok(())
}

fn plugins_info(dir: &Path, id: &str, json: bool) -> Result<()> {
    let manifests = load_manifests(dir);
    let found = manifests.into_iter().find(|(m, _)| m.id == id);
    let (m, p) = match found {
        Some(t) => t,
        None => {
            eprintln!("error: plugin `{}` not found in {}", id, dir.display());
            std::process::exit(2);
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": m.id,
                "name": m.name,
                "version": m.version,
                "api": m.api,
                "capabilities": m.capabilities,
                "wasi": {
                    "allowed_paths": m.wasi.allowed_paths,
                    "allowed_hosts": m.wasi.allowed_hosts,
                },
                "exporter": m.exporter,
                "syncer": m.syncer,
                "cli": m.cli,
                "dir": p.display().to_string(),
                "has_wasm": p.join("plugin.wasm").exists(),
            }))?
        );
        return Ok(());
    }
    println!("id:           {}", m.id);
    if let Some(n) = &m.name {
        println!("name:         {}", n);
    }
    println!("version:      {}", m.version);
    println!("api:          {}", m.api);
    println!("dir:          {}", p.display());
    println!(
        "wasm:         {}",
        if p.join("plugin.wasm").exists() {
            "yes"
        } else {
            "no"
        }
    );
    println!("capabilities:");
    for c in &m.capabilities {
        println!("  - {:?}", c);
    }
    if !m.wasi.allowed_paths.is_empty() {
        println!("wasi.allowed_paths: {:?}", m.wasi.allowed_paths);
    }
    if !m.wasi.allowed_hosts.is_empty() {
        println!("wasi.allowed_hosts: {:?}", m.wasi.allowed_hosts);
    }
    Ok(())
}

fn plugins_install(dir: &Path, src: &Path) -> Result<()> {
    if !src.exists() {
        return Err(eyre!("source path does not exist: {}", src.display()));
    }
    if !src.is_dir() {
        return Err(eyre!(
            "source must be a directory containing plugin.toml [+ plugin.wasm]: {}",
            src.display()
        ));
    }
    let manifest_path = src.join("plugin.toml");
    if !manifest_path.exists() {
        return Err(eyre!(
            "missing plugin.toml in {} — not a plugin directory",
            src.display()
        ));
    }
    let manifest = rondo_plugin_host::FsManifest::load(&manifest_path)
        .map_err(|e| eyre!("invalid plugin.toml: {}", e))?;
    std::fs::create_dir_all(dir)?;
    let dst = dir.join(&manifest.id);
    if dst.exists() {
        return Err(eyre!(
            "plugin `{}` is already installed at {} — remove it first",
            manifest.id,
            dst.display()
        ));
    }
    copy_dir_all(src, &dst)?;
    println!("installed `{}` -> {}", manifest.id, dst.display());
    Ok(())
}

fn plugins_remove(dir: &Path, id: &str) -> Result<()> {
    let target = dir.join(id);
    if !target.exists() {
        println!("(plugin `{}` not installed; nothing to remove)", id);
        return Ok(());
    }
    if !target.is_dir() {
        return Err(eyre!(
            "refusing to remove non-directory entry: {}",
            target.display()
        ));
    }
    std::fs::remove_dir_all(&target)?;
    println!("removed `{}` ({})", id, target.display());
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
