use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rondo_tui::{action::Action, app::AppState, components, event as ev, tui};

#[derive(Parser)]
#[command(name = "rondo-tui", version, about = "Rust + ratatui MVP of rondo")]
struct Cli {
    /// Path to SQLite DB (default: ~/.todo-app/todo.db)
    #[arg(long, env = "RONDO_DB")]
    db: Option<std::path::PathBuf>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    if !db_path.exists() {
        eprintln!(
            "DB no encontrado en {}. Usa --db o setea RONDO_DB.",
            db_path.display()
        );
        std::process::exit(2);
    }
    let store = Arc::new(rondo_core::store::sqlite::SqliteStore::open_readonly(
        &db_path,
    )?);
    let mut app = AppState::new(store)?;
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
        let tick = if app.flash.is_some() || app.fx.any_running() {
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
