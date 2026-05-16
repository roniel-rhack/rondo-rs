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

    for ev in Parser::new(md) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                if !buf.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut buf)));
                }
                in_heading = Some(level as u8);
                style = Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD);
            }
            Event::End(TagEnd::Heading(_)) => {
                lines.push(Line::from(std::mem::take(&mut buf)));
                lines.push(Line::raw(""));
                in_heading = None;
                style = Style::default().fg(theme.fg);
            }
            Event::Start(Tag::Strong) => style = style.add_modifier(Modifier::BOLD),
            Event::End(TagEnd::Strong) => style = style.remove_modifier(Modifier::BOLD),
            Event::Start(Tag::Emphasis) => style = style.add_modifier(Modifier::ITALIC),
            Event::End(TagEnd::Emphasis) => style = style.remove_modifier(Modifier::ITALIC),
            Event::Start(Tag::CodeBlock(_)) => {
                if !buf.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut buf)));
                }
                in_code = true;
                style = Style::default().fg(theme.warning);
            }
            Event::End(TagEnd::CodeBlock) => {
                if !buf.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut buf)));
                }
                in_code = false;
                style = Style::default().fg(theme.fg);
            }
            Event::Code(s) => {
                buf.push(Span::styled(
                    s.to_string(),
                    Style::default().fg(theme.warning),
                ));
            }
            Event::Text(s) => {
                let text = if in_heading.is_some() {
                    s.to_string().to_uppercase()
                } else {
                    s.to_string()
                };
                if in_code {
                    for code_line in text.lines() {
                        lines.push(Line::from(Span::styled(code_line.to_string(), style)));
                    }
                } else {
                    buf.push(Span::styled(text, style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut buf)));
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                lines.push(Line::from(std::mem::take(&mut buf)));
                lines.push(Line::raw(""));
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                buf.push(Span::styled("  • ", Style::default().fg(theme.accent)));
            }
            Event::End(TagEnd::Item) => {
                lines.push(Line::from(std::mem::take(&mut buf)));
            }
            _ => {}
        }
    }
    if !buf.is_empty() {
        lines.push(Line::from(buf));
    }
    Text::from(lines)
}
