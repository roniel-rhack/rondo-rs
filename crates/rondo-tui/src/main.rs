use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rondo_tui::{
    a11y, action::Action, app::AppState, cli, components, event as ev, fx::FxManager,
    theme::Theme, tui,
};

#[derive(Parser)]
#[command(name = "rondo-tui", version, about = "Rust + ratatui MVP of rondo")]
struct Cli {
    /// Path to SQLite DB (default: ~/.todo-app/todo.db)
    #[arg(long, env = "RONDO_DB", global = true)]
    db: Option<std::path::PathBuf>,
    /// Use Color::Reset for all styling (honor NO_COLOR spec)
    #[arg(long, global = true)]
    no_color: bool,
    /// Disable all animations
    #[arg(long, global = true)]
    reduced_motion: bool,
    /// Enable write access. Default: read-only (safer during M1-M3).
    #[arg(long, global = true)]
    write: bool,
    /// Emit JSON to stdout where applicable (CLI subcommands only)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<cli::Command>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let log_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join(".todo-app")
        .join("logs");
    rondo_core::telemetry::rotate_old_logs(&log_dir, 7);
    let _log_guard = rondo_core::telemetry::init_logging(log_dir.clone()).ok();
    rondo_core::telemetry::install_panic_hook(log_dir);
    let cli_args = Cli::parse();
    let db_path = cli_args.db.clone().unwrap_or_else(default_db_path);
    if let Some(cmd) = cli_args.command {
        if cli::needs_db(&cmd) && !db_path.exists() {
            eprintln!(
                "DB no encontrado en {}. Usa --db o setea RONDO_DB.",
                db_path.display()
            );
            std::process::exit(2);
        }
        let opts = cli::CliOpts {
            json: cli_args.json,
            write: cli_args.write,
        };
        return cli::run(cmd, &opts, &db_path);
    }
    if !db_path.exists() {
        eprintln!(
            "DB no encontrado en {}. Usa --db o setea RONDO_DB.",
            db_path.display()
        );
        std::process::exit(2);
    }
    let mut _lock_guard: Option<rondo_core::store::lock::LockGuard> = None;
    let store = if cli_args.write {
        let backup_dir = rondo_core::store::backup::default_backup_dir();
        rondo_core::store::backup::rotate(&backup_dir, 30);
        match rondo_core::store::backup::snapshot(&db_path, &backup_dir) {
            Ok(p) => tracing::info!("backup snapshot: {}", p.display()),
            Err(e) => tracing::warn!("backup failed (continuing without): {}", e),
        }
        let lock_path = rondo_core::store::lock::LockGuard::default_path();
        _lock_guard = Some(
            match rondo_core::store::lock::LockGuard::acquire(lock_path.clone()) {
                Ok(g) => g,
                Err(rondo_core::store::lock::LockError::Conflict(pid)) => {
                    eprintln!(
                        "another rondo process holds the lock (PID {pid}). If you're sure none is running, remove {}",
                        lock_path.display()
                    );
                    std::process::exit(2);
                }
                Err(e) => return Err(e.into()),
            },
        );
        let store = Arc::new(rondo_core::store::sqlite::SqliteStore::open_readwrite(
            &db_path,
        )?);
        let today = chrono::Utc::now().date_naive();
        match rondo_core::recurrence::spawn_recurrent_instances(&store, today) {
            Ok(ids) if !ids.is_empty() => {
                tracing::info!("recurrence: spawned {} instances", ids.len())
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("recurrence spawn failed: {}", e),
        }
        store
    } else {
        Arc::new(rondo_core::store::sqlite::SqliteStore::open_readonly(
            &db_path,
        )?)
    };
    let no_color_active = a11y::no_color() || cli_args.no_color;
    let reduced = a11y::reduced_motion(cli_args.reduced_motion);
    let mut app = AppState::with_writable(store, cli_args.write)?;
    app.theme = if no_color_active {
        Theme::no_color()
    } else {
        Theme::dark()
    };
    app.fx = FxManager::new_with_options(reduced);
    register_builtin_plugins(&mut app);
    let mut terminal = tui::init()?;
    let result = run(&mut terminal, &mut app);
    tui::restore()?;
    result
}

fn default_db_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home)
        .join(".todo-app")
        .join("todo.db")
}

fn register_builtin_plugins(app: &mut AppState) {
    app.plugins.register(Box::new(
        rondo_tui::plugins::builtin::pomodoro::PomodoroPlugin::new(),
    ));
    app.plugins.register(Box::new(
        rondo_tui::plugins::builtin::focus_page::FocusPagePlugin::new(app.data.store.clone()),
    ));
    app.plugins.register(Box::new(
        rondo_tui::plugins::builtin::calendar::CalendarPlugin::new(app.data.store.clone()),
    ));
}

fn run(terminal: &mut tui::Tui, app: &mut AppState) -> Result<()> {
    let flash_tick = Duration::from_millis(40); // 25 Hz while flashing
    let anim_tick = Duration::from_millis(100); // 10 Hz for pomodoro
    let idle_tick = Duration::from_secs(60);
    let mut last_tick = Instant::now();
    let mut dirty = true;
    while !app.should_quit {
        if dirty {
            terminal.draw(|f| components::root::draw(app, f))?;
            dirty = false;
        }
        let tick = if app.ui.flash.is_some() || app.fx.any_running() {
            flash_tick
        } else if app.needs_animation_tick() {
            anim_tick
        } else {
            idle_tick
        };
        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Some(a) = ev::map(event::read()?, app) {
                app.update(a);
                dirty = true;
            }
        }
        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
            if app.needs_animation_tick() {
                app.update(Action::Tick);
                dirty = true;
            }
        }
    }
    Ok(())
}
