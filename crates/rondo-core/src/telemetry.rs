//! Telemetry: tracing logger to file, panic hook, and log rotation.

use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;

pub fn init_logging(log_dir: PathBuf) -> std::io::Result<WorkerGuard> {
    std::fs::create_dir_all(&log_dir)?;
    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("rondo-rs-{}.log", ts);
    let appender = tracing_appender::rolling::never(&log_dir, &filename);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init();
    Ok(guard)
}

pub fn install_panic_hook(log_dir: PathBuf) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::create_dir_all(&log_dir);
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let file = log_dir.join(format!("crash-{}.log", ts));
        let bt = std::backtrace::Backtrace::force_capture();
        let payload = info
            .payload()
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();
        let _ = std::fs::write(
            &file,
            format!(
                "ts: {}\npayload: {}\nlocation: {}\nbacktrace:\n{}\n",
                ts, payload, location, bt
            ),
        );
        prev(info);
    }));
}

pub fn rotate_old_logs(dir: &Path, keep_days: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(keep_days * 86_400));
    let Some(cutoff) = cutoff else { return };
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime < cutoff {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}
