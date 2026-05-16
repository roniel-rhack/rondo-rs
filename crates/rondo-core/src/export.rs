//! Builtin exporters: markdown, json, ndjson.
//!
//! All exporters operate on `&[Task]`; the caller is responsible for
//! sourcing the slice (e.g., `SqliteStore::list_tasks`).

use crate::domain::task::Task;
use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub fn to_markdown(tasks: &[Task]) -> String {
    let mut out = String::new();
    out.push_str("# Tasks\n\n");
    for t in tasks {
        let icon = t.status.icon();
        out.push_str(&format!("- {} **{}** ", icon, t.title));
        out.push_str(&format!("`{}`", t.priority.label()));
        if let Some(due) = t.due_date {
            out.push_str(&format!(" · due {}", due.format("%Y-%m-%d")));
        }
        if !t.tags.is_empty() {
            out.push_str(&format!(" · [{}]", t.tags.join(", ")));
        }
        out.push('\n');
        if let Some(desc) = &t.description {
            if !desc.is_empty() {
                for line in desc.lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        if !t.subtasks.is_empty() {
            for s in &t.subtasks {
                out.push_str(&format!(
                    "  - [{}] {}\n",
                    if s.completed { "x" } else { " " },
                    s.title
                ));
            }
        }
    }
    out
}

pub fn to_json(tasks: &[Task]) -> Result<String, ExportError> {
    Ok(serde_json::to_string_pretty(tasks)?)
}

pub fn to_ndjson<W: Write>(tasks: &[Task], w: &mut W) -> Result<(), ExportError> {
    for t in tasks {
        let line = serde_json::to_string(t)?;
        writeln!(w, "{}", line)?;
    }
    Ok(())
}
