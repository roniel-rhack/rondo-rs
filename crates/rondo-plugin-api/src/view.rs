use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSpec {
    pub kind: ViewKind,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewKind {
    Page,
    Overlay,
    Sidebar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading {
        text: String,
        level: u8,
    },
    Paragraph {
        text: String,
        style: Option<TextStyle>,
    },
    Gauge {
        ratio: f64,
        label: Option<String>,
    },
    Throbber {
        label: String,
    },
    Divider,
    Spans(Vec<Span>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub style: Option<TextStyle>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TextStyle {
    pub fg: Option<ColorToken>,
    pub bg: Option<ColorToken>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorToken {
    Accent,
    Success,
    Warning,
    Danger,
    Muted,
    Foreground,
    Background,
}
