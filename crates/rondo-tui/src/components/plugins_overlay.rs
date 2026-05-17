use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw(app: &AppState, f: &mut Frame<'_>, area: Rect) {
    let t = &app.theme;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_style(true))
        .title(Span::styled(" 🧩 plugins ", t.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        " built-in (in-process)",
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));
    for manifest in app.plugins.iter_manifests() {
        let id = manifest.id.clone();
        let name = manifest.name.clone();
        let version = manifest.version.clone();
        let caps = format!("{:?}", manifest.capabilities);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("[{}]", id),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", name), Style::default().fg(t.fg)),
            Span::styled(format!("  v{}", version), Style::default().fg(t.fg_muted)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("caps: ", Style::default().fg(t.fg_muted)),
            Span::styled(caps, Style::default().fg(t.fg)),
        ]));
        lines.push(Line::raw(""));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        " external (WASM)",
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));
    let plugins_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join(".rondo-rs")
        .join("plugins");
    let mut external_count = 0usize;
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let manifest_path = p.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }
            external_count += 1;
            let raw = std::fs::read_to_string(&manifest_path).unwrap_or_default();
            let id = raw
                .lines()
                .find_map(|l| l.strip_prefix("id ="))
                .map(|s| s.trim().trim_matches('"').to_string())
                .or_else(|| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", id),
                    Style::default().fg(t.warn).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}", p.display()),
                    Style::default().fg(t.fg_muted),
                ),
            ]));
        }
    }
    if external_count == 0 {
        lines.push(Line::from(Span::styled(
            format!("  (none installed in {})", plugins_dir.display()),
            Style::default().fg(t.fg_muted),
        )));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("CLI:", t.kbd()),
        Span::styled(
            " rondo-tui plugins list / info <id> / install <path> / remove <id>",
            Style::default().fg(t.fg_muted),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("Esc", t.kbd()),
        Span::styled(" cerrar", t.muted()),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
