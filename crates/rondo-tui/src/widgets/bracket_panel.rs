use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// Full-bordered panel with asymmetric title rail.
/// Heavy `┏━━┓ ┃ ┗━━┛` when active, light `┌──┐ │ └──┘` when not.
/// Title sits inline in the top rail (left-aligned with `━ title `).
/// Optional badge appears at the right end of the top rail.
pub struct BracketPanel<'a> {
    title: &'a str,
    badge: Option<&'a str>,
    active: bool,
    theme: &'a Theme,
}

impl<'a> BracketPanel<'a> {
    pub fn new(title: &'a str, theme: &'a Theme) -> Self {
        Self {
            title,
            badge: None,
            active: false,
            theme,
        }
    }
    pub fn active(mut self, v: bool) -> Self {
        self.active = v;
        self
    }
    pub fn badge(mut self, b: &'a str) -> Self {
        self.badge = Some(b);
        self
    }

    /// Returns the inner area for body content (1 col gutter inside borders).
    pub fn inner(&self, area: Rect) -> Rect {
        Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        }
    }
}

impl<'a> Widget for BracketPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 2 {
            return;
        }
        let t = self.theme;
        let (tl, tr, bl, br, h, v, color) = if self.active {
            ("┏", "┓", "┗", "┛", "━", "┃", t.border_active)
        } else {
            ("┌", "┐", "└", "┘", "─", "│", t.border_inactive)
        };
        let style = Style::default().fg(color);

        let x0 = area.x;
        let y0 = area.y;
        let x1 = area.x + area.width - 1;
        let y1 = area.y + area.height - 1;

        // Corners
        buf[(x0, y0)].set_symbol(tl).set_style(style);
        buf[(x1, y0)].set_symbol(tr).set_style(style);
        buf[(x0, y1)].set_symbol(bl).set_style(style);
        buf[(x1, y1)].set_symbol(br).set_style(style);

        // Top + bottom rails
        for x in (x0 + 1)..x1 {
            buf[(x, y0)].set_symbol(h).set_style(style);
            buf[(x, y1)].set_symbol(h).set_style(style);
        }

        // Left + right verticals
        for y in (y0 + 1)..y1 {
            buf[(x0, y)].set_symbol(v).set_style(style);
            buf[(x1, y)].set_symbol(v).set_style(style);
        }

        // Title overlay on the top rail
        let title_spans = vec![
            Span::styled(format!("{} ", h), style),
            Span::styled(
                self.title.to_string(),
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];
        let mut title_x = x0 + 1;
        let title_line = Line::from(title_spans);
        for span in title_line.spans {
            let s = span.style;
            for grapheme in span.content.chars() {
                if title_x > x1.saturating_sub(1) {
                    break;
                }
                buf[(title_x, y0)]
                    .set_symbol(&grapheme.to_string())
                    .set_style(s);
                title_x += 1;
            }
        }

        // Right-side badge on the top rail
        if let Some(b) = self.badge {
            let badge_text = format!(" {} {}", b, h);
            let badge_len = badge_text.chars().count() as u16;
            if badge_len < area.width.saturating_sub(8) {
                let start = x1.saturating_sub(badge_len);
                let badge_style = Style::default().fg(t.fg_muted);
                let mut bx = start;
                for ch in badge_text.chars() {
                    if bx >= x1 {
                        break;
                    }
                    buf[(bx, y0)]
                        .set_symbol(&ch.to_string())
                        .set_style(badge_style);
                    bx += 1;
                }
            }
        }
    }
}
