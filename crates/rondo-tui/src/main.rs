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
    let tick = Duration::from_millis(100);
    let mut last = Instant::now();
    while !app.should_quit {
        terminal.draw(|f| components::root::draw(app, f))?;
        let timeout = tick.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            if let Some(a) = ev::map(event::read()?, app) {
                app.update(a);
            }
        }
        if last.elapsed() >= tick {
            last = Instant::now();
            app.update(Action::Tick);
        }
    }
    Ok(())
}
