use crate::theme::Theme;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

#[derive(Debug, Clone, Copy)]
enum ListKind {
    Bullet,
    Ordered(u64),
}

pub fn render(md: &str, theme: &Theme) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut buf: Vec<Span<'static>> = Vec::new();
    let base = Style::default().fg(theme.fg);
    let mut style = base;
    let mut in_heading: Option<u8> = None;
    let mut in_code = false;
    let mut in_blockquote = false;
    let mut in_strikethrough = false;
    let mut list_stack: Vec<ListKind> = Vec::new();
    let mut link_text_buf: Option<String> = None;

    let flush_buf = |buf: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !buf.is_empty() {
            lines.push(Line::from(std::mem::take(buf)));
        }
    };

    let indent = |depth: usize| -> Span<'static> {
        Span::raw("  ".repeat(depth))
    };

    let heading_style = |lvl: u8, theme: &Theme| -> Style {
        match lvl {
            1 => Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED | Modifier::DIM),
            3 => Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
            4 => Style::default().fg(theme.warn).add_modifier(Modifier::BOLD | Modifier::DIM),
            _ => Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::BOLD),
        }
    };

    for ev in Parser::new(md) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_buf(&mut buf, &mut lines);
                let lvl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                in_heading = Some(lvl);
                let hs = heading_style(lvl, theme);
                style = hs;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_buf(&mut buf, &mut lines);
                lines.push(Line::raw(""));
                in_heading = None;
                style = base;
            }
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::End(TagEnd::Strong) => style = style.remove_modifier(Modifier::BOLD),
            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::End(TagEnd::Emphasis) => style = style.remove_modifier(Modifier::ITALIC),
            Event::Start(Tag::Strikethrough) => {
                in_strikethrough = true;
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            Event::End(TagEnd::Strikethrough) => {
                in_strikethrough = false;
                style = style.remove_modifier(Modifier::CROSSED_OUT);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_buf(&mut buf, &mut lines);
                in_blockquote = true;
                style = Style::default()
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::ITALIC);
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_buf(&mut buf, &mut lines);
                in_blockquote = false;
                style = base;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush_buf(&mut buf, &mut lines);
                in_code = true;
                style = Style::default().fg(theme.warn).bg(theme.surface);
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_buf(&mut buf, &mut lines);
                in_code = false;
                style = base;
            }
            Event::Code(s) => {
                buf.push(Span::styled(
                    format!(" {} ", s),
                    Style::default()
                        .fg(theme.warn)
                        .bg(theme.surface)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Event::Start(Tag::Link { .. }) => {
                link_text_buf = Some(String::new());
                style = style
                    .fg(theme.accent)
                    .add_modifier(Modifier::UNDERLINED);
            }
            Event::End(TagEnd::Link) => {
                if let Some(t) = link_text_buf.take() {
                    buf.push(Span::styled(t, style));
                }
                style = if let Some(lvl) = in_heading {
                    heading_style(lvl, theme)
                } else if in_blockquote {
                    Style::default()
                        .fg(theme.fg_muted)
                        .add_modifier(Modifier::ITALIC)
                } else if in_strikethrough {
                    base.add_modifier(Modifier::CROSSED_OUT)
                } else {
                    base
                };
            }
            Event::Rule => {
                flush_buf(&mut buf, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(48),
                    Style::default().fg(theme.border_inactive),
                )));
            }
            Event::TaskListMarker(checked) => {
                let mark = if checked { "[x] " } else { "[ ] " };
                let mstyle = if checked {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg_muted)
                };
                buf.push(Span::styled(mark, mstyle));
            }
            Event::Text(s) => {
                let text = s.to_string();
                if let Some(link) = link_text_buf.as_mut() {
                    link.push_str(&text);
                    continue;
                }
                if in_code {
                    for code_line in text.lines() {
                        lines.push(Line::from(Span::styled(format!(" {} ", code_line), style)));
                    }
                } else {
                    buf.push(Span::styled(text, style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_buf(&mut buf, &mut lines);
                if in_blockquote {
                    buf.push(Span::styled(
                        "▏ ",
                        Style::default().fg(theme.accent),
                    ));
                }
            }
            Event::Start(Tag::Paragraph) => {
                if in_blockquote {
                    buf.push(Span::styled(
                        "▏ ",
                        Style::default().fg(theme.accent),
                    ));
                }
            }
            Event::End(TagEnd::Paragraph) => {
                flush_buf(&mut buf, &mut lines);
                if !in_blockquote {
                    lines.push(Line::raw(""));
                }
            }
            Event::Start(Tag::List(start)) => {
                list_stack.push(match start {
                    Some(n) => ListKind::Ordered(n),
                    None => ListKind::Bullet,
                });
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    flush_buf(&mut buf, &mut lines);
                }
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                buf.push(indent(depth));
                let bullet = match list_stack.last() {
                    Some(ListKind::Bullet) => "• ".to_string(),
                    Some(ListKind::Ordered(n)) => format!("{}. ", n),
                    None => "• ".to_string(),
                };
                buf.push(Span::styled(
                    bullet,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ));
                if let Some(ListKind::Ordered(n)) = list_stack.last_mut() {
                    *n += 1;
                }
            }
            Event::End(TagEnd::Item) => {
                flush_buf(&mut buf, &mut lines);
            }
            _ => {}
        }
    }
    flush_buf(&mut buf, &mut lines);
    Text::from(lines)
}
