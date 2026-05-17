//! Renders a `rondo_plugin_api::ViewSpec` into a `Vec<Line<'static>>`
//! resolving the abstract `ColorToken`s against the active `Theme`.
//!
//! Used by the plugin overlay (Show response) and any other surface that
//! consumes a plugin's DSL output.

use crate::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use rondo_plugin_api::view::{Block, ColorToken, Span as PSpan, TextStyle, ViewSpec};

pub fn render(view: &ViewSpec, theme: &Theme) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    for block in &view.blocks {
        match block {
            Block::Heading { text, level } => {
                let mut style = Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD);
                if *level >= 2 {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                out.push(Line::from(Span::styled(text.clone(), style)));
                if *level == 1 {
                    out.push(Line::raw(""));
                }
            }
            Block::Paragraph { text, style } => {
                let s = resolve_style(*style, theme, Style::default().fg(theme.fg));
                for line in text.split('\n') {
                    out.push(Line::from(Span::styled(line.to_string(), s)));
                }
            }
            Block::Gauge { ratio, label } => {
                let ratio = ratio.clamp(0.0, 1.0);
                let width = 30;
                let filled = (ratio * width as f64).round() as usize;
                let empty = width - filled;
                let mut spans = vec![
                    Span::styled(
                        "▰".repeat(filled),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "▱".repeat(empty),
                        Style::default().fg(theme.border_inactive),
                    ),
                ];
                if let Some(lbl) = label {
                    spans.push(Span::styled(
                        format!("  {}", lbl),
                        Style::default().fg(theme.fg),
                    ));
                }
                out.push(Line::from(spans));
            }
            Block::Throbber { label } => {
                out.push(Line::from(vec![
                    Span::styled("⠿ ", Style::default().fg(theme.accent)),
                    Span::styled(label.clone(), Style::default().fg(theme.fg)),
                ]));
            }
            Block::Divider => {
                out.push(Line::from(Span::styled(
                    "─".repeat(60),
                    Style::default().fg(theme.border_inactive),
                )));
            }
            Block::Spans(parts) => {
                let spans: Vec<Span<'static>> = parts
                    .iter()
                    .map(|p| render_span(p, theme))
                    .collect();
                out.push(Line::from(spans));
            }
        }
    }
    out
}

fn render_span(p: &PSpan, theme: &Theme) -> Span<'static> {
    let style = resolve_style(p.style, theme, Style::default().fg(theme.fg));
    Span::styled(p.text.clone(), style)
}

fn resolve_style(s: Option<TextStyle>, theme: &Theme, base: Style) -> Style {
    let Some(s) = s else { return base };
    let mut style = base;
    if let Some(fg) = s.fg {
        style = style.fg(token(fg, theme));
    }
    if let Some(bg) = s.bg {
        style = style.bg(token(bg, theme));
    }
    if s.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.dim {
        style = style.add_modifier(Modifier::DIM);
    }
    if s.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if s.reverse {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn token(tok: ColorToken, theme: &Theme) -> Color {
    match tok {
        ColorToken::Accent => theme.accent,
        ColorToken::Success => theme.success,
        ColorToken::Warning => theme.warn,
        ColorToken::Danger => theme.danger,
        ColorToken::Muted => theme.fg_muted,
        ColorToken::Foreground => theme.fg,
        ColorToken::Background => theme.bg,
    }
}
