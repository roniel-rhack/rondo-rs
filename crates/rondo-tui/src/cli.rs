//! CLI subcommand dispatch. When `rondo-tui` is invoked with a subcommand
//! (e.g. `rondo-tui list`), we run a one-shot CLI path and exit instead of
//! launching the TUI. Mutation commands (`add`, `done`) require `--write`
//! and follow the same backup + lock-acquire dance as the TUI startup.

use color_eyre::eyre::{eyre, Result};
use rondo_core::domain::task::{NewTask, Status, Task};
use rondo_core::store::sqlite::SqliteStore;
use std::io::{BufRead, Read};
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
    Done { id: i64 },
    /// Export tasks as markdown/json/ndjson
    Export {
        /// Format: md|json|ndjson
        #[arg(long, default_value = "md")]
        format: String,
    },
    /// Manage plugins installed under ~/.rondo-rs/plugins/
    Plugins {
        #[command(subcommand)]
        action: PluginsAction,
    },
    /// Delete a task by id
    Delete { id: i64 },
    /// Journal operations
    Journal {
        #[command(subcommand)]
        action: JournalAction,
    },
    /// Focus session operations
    Focus {
        #[command(subcommand)]
        action: FocusAction,
    },
    /// Print a summary of tasks / focus streak / journal counts
    Stats,
    /// Read NDJSON ops from stdin (one per line) and apply them
    Batch,
    /// Recurrence helpers
    Recur {
        #[command(subcommand)]
        action: RecurAction,
    },
    /// Task dependency operations
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },
    /// Task tag operations
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },
    /// Emit shell completion script (bash|zsh|fish|powershell|elvish)
    Completion { shell: clap_complete::Shell },
}

#[derive(clap::Subcommand, Debug)]
pub enum JournalAction {
    /// Append an entry to today's note
    Add { body: Vec<String> },
    /// List recent journal notes
    List,
}

#[derive(clap::Subcommand, Debug)]
pub enum FocusAction {
    /// Start a 25-minute Work focus session
    Start,
    /// Print streak + total completed today
    Stats,
}

#[derive(clap::Subcommand, Debug)]
pub enum RecurAction {
    /// List pending recurrent spawns without creating them
    Preview,
}

#[derive(clap::Subcommand, Debug)]
pub enum DepAction {
    /// Add `task_id` is blocked by `blocked_by`
    Add { task_id: i64, blocked_by: i64 },
    /// Remove the edge `task_id` -> `blocked_by`
    Remove { task_id: i64, blocked_by: i64 },
}

#[derive(clap::Subcommand, Debug)]
pub enum TagAction {
    /// Attach a tag to a task
    Add { task_id: i64, name: String },
    /// Detach a tag from a task
    Remove { task_id: i64, name: String },
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
    !matches!(cmd, Command::Plugins { .. } | Command::Completion { .. })
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
        Command::Delete { id } => cli_delete(db_path, opts, id),
        Command::Journal { action } => cli_journal(db_path, opts, action),
        Command::Focus { action } => cli_focus(db_path, opts, action),
        Command::Stats => cli_stats(db_path, opts),
        Command::Batch => cli_batch(db_path, opts),
        Command::Recur { action } => cli_recur(db_path, opts, action),
        Command::Dep { action } => cli_dep(db_path, opts, action),
        Command::Tag { action } => cli_tag(db_path, opts, action),
        Command::Completion { .. } => {
            // Completion is handled in main.rs where the full Cli struct is in
            // scope. Reaching here means main forgot to intercept it.
            Err(eyre!("completion must be handled by the binary entrypoint"))
        }
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
        eprintln!("error: `{action}` cannot run in --read-only mode");
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
    let reg = rondo_core::export::ExporterRegistry::with_builtins();
    let key = if format == "markdown" { "md" } else { format };
    let Some(exp) = reg.get(key) else {
        let ids = reg
            .list()
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
            .join("|");
        eprintln!("error: unknown export format `{format}` (expected {ids})");
        std::process::exit(2);
    };
    let out = exp.export(&tasks)?;
    if exp.format_id() == "ndjson" {
        print!("{}", out);
    } else if exp.format_id() == "json" {
        println!("{}", out);
    } else {
        print!("{}", out);
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
        .join(".rondo-rs")
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

// ---------------------------------------------------------------------------
// M9 rest: delete / journal / focus / stats / batch / recur / dep / tag
// ---------------------------------------------------------------------------

fn cli_delete(db_path: &Path, opts: &CliOpts, id: i64) -> Result<()> {
    require_write(opts, "delete")?;
    let (store, _guard) = open_rw_store(db_path)?;
    let _undo = store.delete_task(id)?;
    if opts.json {
        println!("{}", serde_json::json!({ "id": id, "deleted": true }));
    } else {
        println!("task {id} deleted");
    }
    Ok(())
}

fn cli_journal(db_path: &Path, opts: &CliOpts, action: JournalAction) -> Result<()> {
    match action {
        JournalAction::Add { body } => cli_journal_add(db_path, opts, body),
        JournalAction::List => cli_journal_list(db_path, opts),
    }
}

fn cli_journal_add(db_path: &Path, opts: &CliOpts, body: Vec<String>) -> Result<()> {
    require_write(opts, "journal add")?;
    if body.is_empty() {
        return Err(eyre!("journal add: body required"));
    }
    let (store, _guard) = open_rw_store(db_path)?;
    let note = store.create_or_get_today_note()?;
    let body_str = body.join(" ");
    let entry_id = store.add_journal_entry(note.id, &body_str)?;
    if opts.json {
        println!(
            "{}",
            serde_json::json!({ "entry_id": entry_id, "note_id": note.id })
        );
    } else {
        println!("journal entry {entry_id} added to note {}", note.id);
    }
    Ok(())
}

fn cli_journal_list(db_path: &Path, opts: &CliOpts) -> Result<()> {
    let store = SqliteStore::open_readonly(db_path)?;
    let notes = store.list_journal_notes()?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&notes)?);
        return Ok(());
    }
    if notes.is_empty() {
        println!("(no journal notes)");
        return Ok(());
    }
    println!("{:>4}  {:<12}  HIDDEN", "ID", "DATE");
    for n in &notes {
        let date = n.date.format("%Y-%m-%d").to_string();
        println!("{:>4}  {:<12}  {}", n.id, date, n.hidden);
    }
    Ok(())
}

fn cli_focus(db_path: &Path, opts: &CliOpts, action: FocusAction) -> Result<()> {
    match action {
        FocusAction::Start => cli_focus_start(db_path, opts),
        FocusAction::Stats => cli_focus_stats(db_path, opts),
    }
}

fn cli_focus_start(db_path: &Path, opts: &CliOpts) -> Result<()> {
    require_write(opts, "focus start")?;
    let (store, _guard) = open_rw_store(db_path)?;
    let id = store.start_focus_session(
        None,
        rondo_core::domain::focus::SessionKind::Work,
        25 * 60,
        1,
    )?;
    if opts.json {
        println!("{}", serde_json::json!({ "session_id": id }));
    } else {
        println!("focus session {id} started");
    }
    Ok(())
}

fn cli_focus_stats(db_path: &Path, opts: &CliOpts) -> Result<()> {
    let store = SqliteStore::open_readonly(db_path)?;
    let streak = store.focus_streak().unwrap_or(0);
    let sessions = store.list_focus_sessions().unwrap_or_default();
    let today = chrono::Utc::now().date_naive();
    let completed_today = sessions
        .iter()
        .filter(|s| {
            s.completed_at
                .map(|c| c.date_naive() == today)
                .unwrap_or(false)
                && matches!(s.kind, rondo_core::domain::focus::SessionKind::Work)
        })
        .count();
    if opts.json {
        println!(
            "{}",
            serde_json::json!({
                "streak_days": streak,
                "completed_today": completed_today,
            })
        );
    } else {
        println!("focus streak: {streak} day(s)");
        println!("completed today: {completed_today} session(s)");
    }
    Ok(())
}

fn cli_stats(db_path: &Path, opts: &CliOpts) -> Result<()> {
    let store = SqliteStore::open_readonly(db_path)?;
    let tasks = store.list_tasks()?;
    let total = tasks.len();
    let done = tasks
        .iter()
        .filter(|t| matches!(t.status, Status::Done))
        .count();
    let pending = total - done;
    let streak = store.focus_streak().unwrap_or(0);
    let journal = store.list_journal_notes().map(|n| n.len()).unwrap_or(0);
    if opts.json {
        println!(
            "{}",
            serde_json::json!({
                "tasks": { "total": total, "done": done, "pending": pending },
                "focus_streak": streak,
                "journal_notes": journal,
            })
        );
    } else {
        println!("tasks: {total} total ({done} done, {pending} pending)");
        println!("focus streak: {streak} day(s)");
        println!("journal notes: {journal}");
    }
    Ok(())
}

/// Maximum NDJSON lines processed per `batch` invocation. Bounds work
/// to a sane amount even if the caller pipes an unbounded stream.
const MAX_BATCH_LINES: usize = 10_000;
/// Maximum bytes per NDJSON line. Larger lines are rejected rather than
/// buffered — guards against runaway producers that omit newlines.
const MAX_LINE_LEN: usize = 64 * 1024;

fn cli_batch(db_path: &Path, opts: &CliOpts) -> Result<()> {
    require_write(opts, "batch")?;
    let (store, _guard) = open_rw_store(db_path)?;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut count: usize = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut line_no: usize = 0;
    loop {
        if line_no >= MAX_BATCH_LINES {
            errors.push(format!(
                "aborted: max {MAX_BATCH_LINES} lines per batch exceeded"
            ));
            break;
        }
        // Cap each line at MAX_LINE_LEN + 1 so we can detect overflow
        // without buffering arbitrary input. We read up to the limit and,
        // if no newline is found in that window, skip ahead to the next
        // newline and record an error.
        let mut buf: Vec<u8> = Vec::new();
        let mut limited = (&mut handle).take((MAX_LINE_LEN + 1) as u64);
        let read = limited.read_until(b'\n', &mut buf)?;
        if read == 0 {
            break;
        }
        line_no += 1;
        let trailing_newline = buf.last() == Some(&b'\n');
        if buf.len() > MAX_LINE_LEN && !trailing_newline {
            // Drain to next newline so the next iteration starts fresh.
            let mut sink: Vec<u8> = Vec::new();
            let _ = handle.read_until(b'\n', &mut sink);
            errors.push(format!(
                "line {line_no}: exceeds max length of {MAX_LINE_LEN} bytes"
            ));
            continue;
        }
        if trailing_newline {
            buf.pop();
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
        }
        let line = match String::from_utf8(buf) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("line {line_no}: invalid utf-8: {e}"));
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("parse: {e}"));
                continue;
            }
        };
        let op = v.get("op").and_then(|s| s.as_str()).unwrap_or("");
        match op {
            "add" => {
                let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("");
                if title.is_empty() {
                    errors.push("add: empty title".into());
                    continue;
                }
                match store.create_task(NewTask::quick(title)) {
                    Ok(_) => count += 1,
                    Err(e) => errors.push(format!("add: {e}")),
                }
            }
            "done" => {
                let id = v.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
                if id == 0 {
                    errors.push("done: missing id".into());
                    continue;
                }
                match store.set_status(id, Status::Done) {
                    Ok(_) => count += 1,
                    Err(e) => errors.push(format!("done: {e}")),
                }
            }
            other => errors.push(format!("unknown op: {other}")),
        }
    }
    if opts.json {
        println!(
            "{}",
            serde_json::json!({ "processed": count, "errors": errors.len() })
        );
    } else {
        println!("processed {count} ops, {} errors", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
    }
    Ok(())
}

fn cli_recur(db_path: &Path, opts: &CliOpts, action: RecurAction) -> Result<()> {
    match action {
        RecurAction::Preview => cli_recur_preview(db_path, opts),
    }
}

fn cli_recur_preview(db_path: &Path, opts: &CliOpts) -> Result<()> {
    let store = SqliteStore::open_readonly(db_path)?;
    let tasks = store.list_tasks()?;
    let now = chrono::Utc::now().date_naive();
    let pending: Vec<_> = tasks
        .iter()
        .filter(|t| matches!(t.status, Status::Done))
        .filter_map(|t| {
            rondo_core::recurrence::next_occurrence(t, now)
                .map(|next| (t.id, t.title.clone(), next))
        })
        .collect();
    if opts.json {
        let arr: Vec<serde_json::Value> = pending
            .iter()
            .map(|(id, title, next)| {
                serde_json::json!({ "id": id, "title": title, "next": next.to_string() })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr)?);
    } else if pending.is_empty() {
        println!("(no pending recurrent spawns)");
    } else {
        for (id, title, next) in &pending {
            println!("[#{id}] {title} -> next {next}");
        }
    }
    Ok(())
}

fn cli_dep(db_path: &Path, opts: &CliOpts, action: DepAction) -> Result<()> {
    match action {
        DepAction::Add {
            task_id,
            blocked_by,
        } => {
            require_write(opts, "dep add")?;
            let (store, _guard) = open_rw_store(db_path)?;
            store.add_dependency(task_id, blocked_by)?;
            if opts.json {
                println!(
                    "{}",
                    serde_json::json!({ "task_id": task_id, "blocked_by": blocked_by, "added": true })
                );
            } else {
                println!("dep added: {task_id} blocked by {blocked_by}");
            }
            Ok(())
        }
        DepAction::Remove {
            task_id,
            blocked_by,
        } => {
            require_write(opts, "dep remove")?;
            let (store, _guard) = open_rw_store(db_path)?;
            store.remove_dependency(task_id, blocked_by)?;
            if opts.json {
                println!(
                    "{}",
                    serde_json::json!({ "task_id": task_id, "blocked_by": blocked_by, "removed": true })
                );
            } else {
                println!("dep removed: {task_id} blocked by {blocked_by}");
            }
            Ok(())
        }
    }
}

fn cli_tag(db_path: &Path, opts: &CliOpts, action: TagAction) -> Result<()> {
    match action {
        TagAction::Add { task_id, name } => {
            require_write(opts, "tag add")?;
            let (store, _guard) = open_rw_store(db_path)?;
            let _undo = store.add_tag(task_id, &name)?;
            if opts.json {
                println!(
                    "{}",
                    serde_json::json!({ "task_id": task_id, "tag": name, "added": true })
                );
            } else {
                println!("tag `{name}` added to task {task_id}");
            }
            Ok(())
        }
        TagAction::Remove { task_id, name } => {
            require_write(opts, "tag remove")?;
            let (store, _guard) = open_rw_store(db_path)?;
            let _undo = store.remove_tag(task_id, &name)?;
            if opts.json {
                println!(
                    "{}",
                    serde_json::json!({ "task_id": task_id, "tag": name, "removed": true })
                );
            } else {
                println!("tag `{name}` removed from task {task_id}");
            }
            Ok(())
        }
    }
}

/// Emit a shell-completion script for the binary using the provided clap
/// command (passed in by `main.rs` since the top-level `Cli` lives there).
pub fn emit_completion(shell: clap_complete::Shell, cmd: &mut clap::Command) {
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, cmd, name, &mut std::io::stdout());
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
