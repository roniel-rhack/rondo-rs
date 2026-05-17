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
    for manifest in app.external.manifests() {
        external_count += 1;
        let id = manifest.id.clone();
        let enabled = app.external.is_enabled(&id);
        let caps = manifest
            .capabilities
            .iter()
            .map(|c| format!("{:?}", c))
            .collect::<Vec<_>>()
            .join(", ");
        let id_color = if enabled { t.warn } else { t.fg_muted };
        let status = if enabled { "enabled" } else { "DISABLED" };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("[{}]", id),
                Style::default().fg(id_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  v{}", manifest.version),
                Style::default().fg(t.fg_muted),
            ),
            Span::styled(format!("  ({})", status), Style::default().fg(t.fg_muted)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("caps: ", Style::default().fg(t.fg_muted)),
            Span::styled(caps, Style::default().fg(t.fg)),
        ]));
        if let Some(cmd) = manifest.command_name() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("cmd:  ", Style::default().fg(t.fg_muted)),
                Span::styled(format!(":{}", cmd), Style::default().fg(t.accent)),
            ]));
        }
        lines.push(Line::raw(""));
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
