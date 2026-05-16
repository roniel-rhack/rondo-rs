//! CLI subcommand dispatch. When `rondo-tui` is invoked with a subcommand
//! (e.g. `rondo-tui list`), we run a one-shot CLI path and exit instead of
//! launching the TUI. Mutation commands (`add`, `done`) require `--write`
//! and follow the same backup + lock-acquire dance as the TUI startup.

use color_eyre::eyre::{eyre, Result};
use rondo_core::domain::task::{NewTask, Status, Task};
use rondo_core::store::sqlite::SqliteStore;
use std::path::Path;
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
