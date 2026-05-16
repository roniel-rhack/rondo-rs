use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};
use rondo_core::domain::task::Priority;

/// Cohesive 7-token semantic palette.
/// Designed to feel warm and restrained, not Material-Design rainbow.
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub surface: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub accent: Color,
    pub danger: Color,
    pub warn: Color,
    pub success: Color,
    pub border_active: Color,
    pub border_inactive: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "dark",
            bg: Color::Rgb(0x0F, 0x11, 0x15),
            surface: Color::Rgb(0x18, 0x1B, 0x22),
            fg: Color::Rgb(0xE6, 0xE1, 0xCF),
            fg_muted: Color::Rgb(0x5C, 0x63, 0x70),
            accent: Color::Rgb(0x7F, 0xDB, 0xCA),
            danger: Color::Rgb(0xFF, 0x6B, 0x6B),
            warn: Color::Rgb(0xE5, 0xC0, 0x7B),
            success: Color::Rgb(0x98, 0xC3, 0x79),
            border_active: Color::Rgb(0x7F, 0xDB, 0xCA),
            border_inactive: Color::Rgb(0x2A, 0x2E, 0x36),
        }
    }

    /// Monochrome theme honoring the NO_COLOR convention.
    /// Every color falls back to `Color::Reset` so the terminal applies its
    /// own default foreground/background.
    pub fn no_color() -> Self {
        Self {
            name: "no-color",
            bg: Color::Reset,
            surface: Color::Reset,
            fg: Color::Reset,
            fg_muted: Color::Reset,
            accent: Color::Reset,
            danger: Color::Reset,
            warn: Color::Reset,
            success: Color::Reset,
            border_active: Color::Reset,
            border_inactive: Color::Reset,
        }
    }

    pub fn priority_color(&self, p: Priority) -> Color {
        match p {
            Priority::Low => self.success,
            Priority::Med => self.warn,
            Priority::High => self.danger,
            Priority::Urgent => self.danger,
        }
    }

    pub fn priority_style(&self, p: Priority) -> Style {
        Style::default()
            .fg(self.priority_color(p))
            .add_modifier(Modifier::BOLD)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    pub fn fg_style(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn kbd(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn badge(&self, color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn selection(&self) -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }

    pub fn border_style(&self, active: bool) -> Style {
        Style::default().fg(if active {
            self.border_active
        } else {
            self.border_inactive
        })
    }

    /// Standard panel block with focus-aware border + accent title.
    pub fn panel<'a>(&'a self, title: &'a str, active: bool) -> Block<'a> {
        let border_type = if active {
            BorderType::Rounded
        } else {
            BorderType::Plain
        };
        Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(self.border_style(active))
            .title(ratatui::text::Span::styled(
                format!(" {} ", title),
                self.accent_style(),
            ))
    }
}
