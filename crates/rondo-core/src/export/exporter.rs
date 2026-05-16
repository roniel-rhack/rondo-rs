//! `Exporter` trait + builtin registry.
//!
//! The trait is the extensibility seam: builtin exporters (markdown, json,
//! ndjson) implement it, and plugin-contributed exporters (e.g. ical,
//! org-mode, taskpaper) can register themselves through the same interface.
//! The CLI's `export --format <id>` is a registry lookup.

use crate::domain::task::Task;

/// Errors surfaced by [`Exporter::export`] implementations.
///
/// `Io` is kept around for exporters that stream to a writer or otherwise
/// touch the filesystem; the builtins below only use `Json`.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown format: {0}")]
    UnknownFormat(String),
}

/// Anything that can turn a `&[Task]` into a serialized blob.
///
/// `format_id` is the user-facing token (e.g. `"md"`, `"json"`, `"org"`)
/// passed on the CLI. `mime` is the MIME type a downstream consumer can
/// use to set `Content-Type` headers or pick an editor mode.
pub trait Exporter: Send + Sync {
    fn format_id(&self) -> &str;
    fn mime(&self) -> &str;
    fn export(&self, tasks: &[Task]) -> Result<String, ExportError>;
}

pub struct MarkdownExporter;
impl Exporter for MarkdownExporter {
    fn format_id(&self) -> &str {
        "md"
    }
    fn mime(&self) -> &str {
        "text/markdown"
    }
    fn export(&self, tasks: &[Task]) -> Result<String, ExportError> {
        Ok(super::to_markdown(tasks))
    }
}

pub struct JsonExporter;
impl Exporter for JsonExporter {
    fn format_id(&self) -> &str {
        "json"
    }
    fn mime(&self) -> &str {
        "application/json"
    }
    fn export(&self, tasks: &[Task]) -> Result<String, ExportError> {
        Ok(serde_json::to_string_pretty(tasks)?)
    }
}

pub struct NdjsonExporter;
impl Exporter for NdjsonExporter {
    fn format_id(&self) -> &str {
        "ndjson"
    }
    fn mime(&self) -> &str {
        "application/x-ndjson"
    }
    fn export(&self, tasks: &[Task]) -> Result<String, ExportError> {
        let mut out = String::new();
        for t in tasks {
            out.push_str(&serde_json::to_string(t)?);
            out.push('\n');
        }
        Ok(out)
    }
}

/// Registry of exporters keyed by `format_id`. Builtins are seeded by
/// [`ExporterRegistry::with_builtins`]; the host can `register` additional
/// exporters contributed by plugins that declare
/// [`crate::Capability::Exporter`](../../plugin_api/enum.Capability.html#variant.Exporter).
#[derive(Default)]
pub struct ExporterRegistry {
    exporters: Vec<Box<dyn Exporter>>,
}

impl ExporterRegistry {
    pub fn with_builtins() -> Self {
        let mut r = Self::default();
        r.register(Box::new(MarkdownExporter));
        r.register(Box::new(JsonExporter));
        r.register(Box::new(NdjsonExporter));
        r
    }

    pub fn register(&mut self, e: Box<dyn Exporter>) {
        self.exporters.push(e);
    }

    pub fn get(&self, format_id: &str) -> Option<&dyn Exporter> {
        self.exporters
            .iter()
            .map(|e| e.as_ref())
            .find(|e| e.format_id() == format_id)
    }

    /// `(format_id, mime)` for every registered exporter, in registration order.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.exporters
            .iter()
            .map(|e| (e.format_id(), e.mime()))
            .collect()
    }
}
