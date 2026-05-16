use ratatui::style::{Color, Modifier, Style};
use rondo_core::domain::task::Priority;

pub struct Theme {
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub urgent: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub bg: Color,
    pub border_active: Color,
    pub border_inactive: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            accent: Color::Rgb(0x00, 0xBC, 0xD4),
            success: Color::Rgb(0x4C, 0xAF, 0x50),
            warning: Color::Rgb(0xFF, 0xC1, 0x07),
            danger: Color::Rgb(0xF4, 0x43, 0x36),
            urgent: Color::Rgb(0xE9, 0x1E, 0x63),
            fg: Color::Rgb(0xFA, 0xFA, 0xFA),
            fg_muted: Color::Rgb(0x9E, 0x9E, 0x9E),
            bg: Color::Reset,
            border_active: Color::Rgb(0x00, 0xBC, 0xD4),
            border_inactive: Color::Rgb(0x42, 0x42, 0x42),
        }
    }

    pub fn priority_color(&self, p: Priority) -> Color {
        match p {
            Priority::Low => self.success,
            Priority::Med => self.warning,
            Priority::High => self.danger,
            Priority::Urgent => self.urgent,
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

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }
}
