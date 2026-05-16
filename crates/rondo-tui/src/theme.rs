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

    pub fn light() -> Self {
        Self {
            name: "light",
            bg: Color::Rgb(0xFA, 0xFA, 0xFA),
            surface: Color::Rgb(0xEE, 0xEE, 0xEE),
            fg: Color::Rgb(0x21, 0x21, 0x21),
            fg_muted: Color::Rgb(0x61, 0x61, 0x61),
            accent: Color::Rgb(0x00, 0x83, 0x95),
            danger: Color::Rgb(0xC6, 0x28, 0x28),
            warn: Color::Rgb(0xF9, 0xA8, 0x25),
            success: Color::Rgb(0x2E, 0x7D, 0x32),
            border_active: Color::Rgb(0x00, 0x83, 0x95),
            border_inactive: Color::Rgb(0xBD, 0xBD, 0xBD),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "high-contrast",
            bg: Color::Rgb(0, 0, 0),
            surface: Color::Rgb(0x0A, 0x0A, 0x0A),
            fg: Color::Rgb(0xFF, 0xFF, 0xFF),
            fg_muted: Color::Rgb(0xBD, 0xBD, 0xBD),
            accent: Color::Rgb(0xFF, 0xD4, 0x00),
            danger: Color::Rgb(0xFF, 0x17, 0x44),
            warn: Color::Rgb(0xFF, 0xAB, 0x00),
            success: Color::Rgb(0x00, 0xE6, 0x76),
            border_active: Color::Rgb(0xFF, 0xD4, 0x00),
            border_inactive: Color::Rgb(0x80, 0x80, 0x80),
        }
    }

    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "high-contrast" | "hc" => Self::high_contrast(),
            _ => Self::dark(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
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
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn kbd(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_name_resolves_variants() {
        assert_eq!(Theme::by_name("dark").name(), "dark");
        assert_eq!(Theme::by_name("light").name(), "light");
        assert_eq!(Theme::by_name("high-contrast").name(), "high-contrast");
        assert_eq!(Theme::by_name("hc").name(), "high-contrast");
    }

    #[test]
    fn unknown_name_falls_back_to_dark() {
        assert_eq!(Theme::by_name("garbage").name(), "dark");
    }

    #[test]
    fn each_variant_has_distinct_fg_bg() {
        for t in [Theme::dark(), Theme::light(), Theme::high_contrast()] {
            assert_ne!(t.fg, t.bg, "{} fg==bg", t.name());
        }
    }
}
