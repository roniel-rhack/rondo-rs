use crate::theme::Theme;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

pub fn render(md: &str, theme: &Theme) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut buf: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default().fg(theme.fg);
    let mut in_heading: Option<u8> = None;
    let mut in_code = false;

    let flush_buf = |buf: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !buf.is_empty() {
            lines.push(Line::from(std::mem::take(buf)));
        }
    };

    for ev in Parser::new(md) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_buf(&mut buf, &mut lines);
                in_heading = Some(level as u8);
                let prefix = match level as u8 {
                    1 => "▍▍ ",
                    2 => "▎ ",
                    _ => "▏ ",
                };
                let head_style = match level as u8 {
                    1 => Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                    2 => Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                };
                buf.push(Span::styled(prefix, head_style));
                style = head_style;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_buf(&mut buf, &mut lines);
                lines.push(Line::raw(""));
                in_heading = None;
                style = Style::default().fg(theme.fg);
            }
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::End(TagEnd::Strong) => style = style.remove_modifier(Modifier::BOLD),
            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::End(TagEnd::Emphasis) => style = style.remove_modifier(Modifier::ITALIC),
            Event::Start(Tag::CodeBlock(_)) => {
                flush_buf(&mut buf, &mut lines);
                in_code = true;
                style = Style::default().fg(theme.warn).bg(theme.surface);
            }
            Event::End(TagEnd::CodeBlock) => {
                flush_buf(&mut buf, &mut lines);
                in_code = false;
                style = Style::default().fg(theme.fg);
            }
            Event::Code(s) => {
                buf.push(Span::styled(
                    format!(" {} ", s),
                    Style::default().fg(theme.warn).bg(theme.surface),
                ));
            }
            Event::Text(s) => {
                let text = s.to_string();
                if in_code {
                    for code_line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            format!(" {} ", code_line),
                            style,
                        )));
                    }
                } else {
                    buf.push(Span::styled(text, style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_buf(&mut buf, &mut lines);
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_buf(&mut buf, &mut lines);
                lines.push(Line::raw(""));
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                buf.push(Span::styled("  · ", Style::default().fg(theme.fg_muted)));
            }
            Event::End(TagEnd::Item) => {
                flush_buf(&mut buf, &mut lines);
            }
            _ => {}
        }
    }
    flush_buf(&mut buf, &mut lines);
    let _ = in_heading;
    Text::from(lines)
}
